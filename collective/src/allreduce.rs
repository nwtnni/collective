use std::cmp;
use std::ffi;
use std::mem;

use mpi::traits::Communicator as _;

use crate::barrier::Barrier;
use crate::datatype::MpiType;
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
    let comm = crate::Communicator(comm);

    if f32::matches(datatype) {
        allreduce_sum::<f32>(buffer_send, buffer_receive, count, comm);
    } else if i32::matches(datatype) {
        allreduce_sum::<i32>(buffer_send, buffer_receive, count, comm);
    } else if i8::matches(datatype) {
        allreduce_sum::<i8>(buffer_send, buffer_receive, count, comm);
    }

    mpi::ffi::MPI_SUCCESS as ffi::c_int
}

unsafe fn allreduce_sum<T: MpiType + Copy>(
    buffer_send: *const ffi::c_void,
    buffer_receive: *mut ffi::c_void,
    count: ffi::c_int,
    comm: crate::Communicator,
) {
    let buffer_send = std::slice::from_raw_parts(buffer_send as *const T, count as usize);
    let buffer_receive = std::slice::from_raw_parts_mut(buffer_receive as *mut T, count as usize);

    // | Barrier (128B)      |
    // | Region 0 Lock (64B) |
    // | Region 1 Lock (64B) |
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

        // Zero memory
        if comm.rank() == 0 {
            locks.fill(0);
            remainder[..data_size].fill(0);
        }

        let (prefix, data, suffix) = remainder[..data_size].align_to_mut::<T>();

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

    barrier.wait(comm.size());

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

        buffer_shared[offset..][..count]
            .iter_mut()
            .zip(&buffer_send[offset..][..count])
            .for_each(|(shared, send)| shared.sum_mut(send));

        locks[region].unlock();
    }

    // Wait for all processes to finish writes
    barrier.wait(comm.size());

    buffer_receive.copy_from_slice(buffer_shared);
}
