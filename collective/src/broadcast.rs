use std::ffi;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use mpi::traits::Communicator as _;

#[no_mangle]
pub unsafe extern "C" fn MPI_Bcast(
    buffer: *mut ffi::c_void,
    count: ffi::c_int,
    _: mpi::ffi::MPI_Datatype,
    root: ffi::c_int,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    let comm = crate::Communicator(comm);

    if comm.size() == 1 {
        return mpi::ffi::MPI_SUCCESS as ffi::c_int;
    }

    static EPOCH: AtomicU64 = AtomicU64::new(0);

    let epoch_before = EPOCH.load(Ordering::Acquire);
    let epoch_after = epoch_before + comm.size() as u64;

    if comm.rank() == root {
        unsafe {
            let mut shared = crate::PCI_MAP.lock().unwrap();

            let local = std::slice::from_raw_parts(buffer as *const u8, count as usize);

            shared[crate::CACHE_LINE_SIZE..][..count as usize].copy_from_slice(local);

            // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2090-2092
            let epoch = &*shared.as_ptr().cast::<AtomicU64>();

            // Kick off broadcast
            epoch.fetch_add(1, Ordering::AcqRel);

            // Spin waiting for everyone to read
            while epoch.load(Ordering::Acquire) < epoch_after {}
        }
    } else {
        unsafe {
            let shared = crate::PCI_MAP.lock().unwrap();

            // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2090-2092
            let epoch = &*shared.as_ptr().cast::<AtomicU64>();

            // Spin until broadcast starts
            while epoch.load(Ordering::Acquire) == epoch_before {}

            let local = std::slice::from_raw_parts_mut(buffer as *mut u8, count as usize);
            local.copy_from_slice(&shared[crate::CACHE_LINE_SIZE..][..count as usize]);

            // Update broadcaster
            epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    EPOCH.store(epoch_after, Ordering::Release);
    mpi::ffi::MPI_SUCCESS as ffi::c_int
}
