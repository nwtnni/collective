use std::cmp;
use std::env;
use std::ffi;
use std::mem;

use mpi::traits::Communicator as _;

use crate::barrier::Barrier;
use crate::datatype::MpiType;
use crate::metrics;
use crate::mutex::Mutex;

#[no_mangle]
pub unsafe extern "C" fn MPI_Allreduce(
    buffer_send: *const ffi::c_void,
    buffer_receive: *mut ffi::c_void,
    count: ffi::c_int,
    datatype: mpi::ffi::MPI_Datatype,
    _: mpi::ffi::MPI_Op,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    metrics::time!(metrics::timers::TOTAL, {
        if f32::matches(datatype) {
            allreduce::<f32>(buffer_send, buffer_receive, count, comm);
        } else if i32::matches(datatype) {
            allreduce::<i32>(buffer_send, buffer_receive, count, comm);
        } else if i8::matches(datatype) {
            allreduce::<i8>(buffer_send, buffer_receive, count, comm);
        }
    });

    metrics::dump();
    mpi::ffi::MPI_SUCCESS as ffi::c_int
}

unsafe fn allreduce<T: MpiType + Copy>(
    buffer_send: *const ffi::c_void,
    buffer_receive: *mut ffi::c_void,
    count: ffi::c_int,
    comm: mpi::ffi::MPI_Comm,
) {
    let buffer_send = std::slice::from_raw_parts(buffer_send as *const T, count as usize);
    let buffer_receive = std::slice::from_raw_parts_mut(buffer_receive as *mut T, count as usize);
    let comm = crate::Communicator(comm);

    let algorithm = env::var("COLLECTIVE_ALLREDUCE_ALGORITHM");

    match algorithm.as_deref() {
        Ok("single") | Err(_) => allreduce_single(buffer_send, buffer_receive, comm),
        Ok("multiple") => allreduce_multiple(buffer_send, buffer_receive, comm),
        Ok(algorithm) => panic!("Unknown allreduce algorithm: {}", algorithm),
    }
}

unsafe fn allreduce_single<T: MpiType + Copy>(
    buffer_send: &[T],
    buffer_receive: &mut [T],
    comm: crate::Communicator,
) {
    // | Barrier             |
    // | Region 0 Lock       |
    // | Region 1 Lock       |
    // | ...                 |
    // | Region 0 (4KiB)     | <- P0
    // | ...                 |
    // | Region 8 (4KiB)     | <- P1
    // | ...                 |
    // | Region 16 (4KiB)    | <- P2
    // | ...                 |
    // | Region 24 (4KiB)    | <- P3
    // | ...                 |
    let data_size = buffer_send.len() * mem::size_of::<T>();
    let region_size = crate::PAGE_SIZE;
    let region_count = (data_size + region_size - 1) / region_size;
    let region_offset = comm.rank() as usize * (region_count / comm.size() as usize);

    let mut pci_map = crate::PCI_MAP.lock().unwrap();

    // Partition shared memory into disjoint areas
    let (barrier, locks, buffer_shared) = {
        let (barrier, remainder) = pci_map.split_at_mut(Barrier::SIZE);
        let (locks, remainder) = remainder.split_at_mut(Mutex::SIZE * region_count);

        let offset = remainder.as_ptr().align_offset(crate::PAGE_SIZE);

        // Zero memory
        if comm.rank() == 0 {
            metrics::time!(metrics::timers::ZERO, {
                locks.fill(0);
                remainder[offset..][..data_size].fill(0);
            });
        }

        let (prefix, data, suffix) = remainder[offset..][..data_size].align_to_mut::<T>();

        assert!(prefix.is_empty());
        assert!(suffix.is_empty());

        let barrier = unsafe { Barrier::new(barrier.as_ptr()) };
        let locks = (0..region_count)
            .map(|region| region * Mutex::SIZE)
            .map(|offset| locks[offset..].as_ptr())
            .map(|address| Mutex::new(address))
            .collect::<Vec<_>>();

        (barrier, locks, data)
    };

    barrier.wait(comm.rank(), comm.size());

    // Start at different offsets
    for region in (0..region_count)
        .cycle()
        .skip(region_offset)
        .take(region_count)
    {
        let offset = region * region_size / mem::size_of::<T>();
        let count = cmp::min(
            region_size / mem::size_of::<T>(),
            buffer_shared[offset..].len(),
        );

        locks[region].lock();
        metrics::time!(metrics::timers::COMPUTE, {
            buffer_shared[offset..][..count]
                .iter_mut()
                .zip(&buffer_send[offset..][..count])
                .for_each(|(shared, send)| shared.sum_mut(send));
        });
        locks[region].unlock();
    }

    // Wait for all processes to finish writes
    barrier.wait(comm.rank(), comm.size());

    metrics::time!(metrics::timers::COPY, {
        buffer_receive.copy_from_slice(buffer_shared);
    });
}

unsafe fn allreduce_multiple<T: MpiType + Copy>(
    buffer_send: &[T],
    buffer_receive: &mut [T],
    comm: crate::Communicator,
) {
    let comm_rank = comm.rank() as usize;
    let comm_size = comm.size() as usize;

    let byte_size = buffer_send.len() * mem::size_of::<T>();
    let byte_size_aligned = align(byte_size);

    let data_size = buffer_send.len();
    let data_size_aligned = byte_size_aligned / mem::size_of::<T>();

    let mut pci_map = crate::PCI_MAP.lock().unwrap();

    let (barrier, remainder) = pci_map.split_at_mut(Barrier::SIZE);
    let barrier = Barrier::new(barrier.as_ptr());
    let offset = remainder.as_ptr().align_offset(crate::PAGE_SIZE);

    let (buffer_shared_send_all, remainder) =
        remainder[offset..].split_at_mut(byte_size_aligned * comm_size);

    let (prefix, buffer_shared_send_all, suffix) = buffer_shared_send_all.align_to_mut::<T>();
    assert_eq!(prefix.len(), 0);
    assert_eq!(suffix.len(), 0);

    let buffer_shared = &mut remainder[..byte_size];
    if comm.rank() == 0 {
        metrics::time!(metrics::timers::ZERO, {
            buffer_shared.fill(0);
        });
    }

    let (prefix, buffer_shared, suffix) = buffer_shared.align_to_mut::<T>();
    assert_eq!(prefix.len(), 0);
    assert_eq!(suffix.len(), 0);

    metrics::time!(metrics::timers::COPY, {
        buffer_shared_send_all[data_size_aligned * comm_rank..][..data_size]
            .copy_from_slice(buffer_send);
    });

    barrier.wait(comm_rank as i32, comm_size as i32);

    let partition = cmp::max(crate::PAGE_SIZE, align(byte_size / comm_size)) / mem::size_of::<T>();

    if partition * comm_rank < data_size {
        metrics::time!(metrics::timers::COMPUTE, {
            (0..comm_size)
                .map(|rank| {
                    let send = &buffer_shared_send_all[data_size_aligned * rank..][..data_size]
                        [partition * comm_rank..];
                    let len = cmp::min(send.len(), partition);
                    &send[..len]
                })
                .for_each(|buffer_send| {
                    let shared = &mut buffer_shared[partition * comm_rank..];
                    let len = cmp::min(shared.len(), partition);
                    shared[..len]
                        .iter_mut()
                        .zip(buffer_send)
                        .for_each(|(shared, send)| shared.sum_mut(send));
                });
        });
    }

    barrier.wait(comm.rank(), comm.size());

    metrics::time!(metrics::timers::COPY, {
        buffer_receive.copy_from_slice(buffer_shared);
    });
}

fn align(value: usize) -> usize {
    (value + crate::PAGE_SIZE - 1) & !(crate::PAGE_SIZE - 1)
}
