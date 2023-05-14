#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::env;
use std::ffi;
use std::fs;
use std::mem;
use std::os::unix::fs::OpenOptionsExt as _;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use anyhow::anyhow;
use anyhow::Context as _;
use memmap2::MmapMut;
use mpi::traits::Communicator as _;
use once_cell::sync::Lazy;

static _MPI_Init_thread: Lazy<
    unsafe extern "C" fn(
        *const ffi::c_int,
        *const *const *const ffi::c_char,
        ffi::c_int,
        *const ffi::c_int,
    ),
> = Lazy::new(|| unsafe {
    mem::transmute(libc::dlsym(
        libc::RTLD_NEXT,
        ffi::CStr::from_bytes_with_nul(b"MPI_Init_thread\0")
            .unwrap()
            .as_ptr(),
    ))
});

static PCI_FILE: Lazy<fs::File> = Lazy::new(|| initialize_file().unwrap());
static PCI_MAP: Lazy<Mutex<MmapMut>> = Lazy::new(|| initialize_map().map(Mutex::new).unwrap());

const CACHE_LINE_SIZE: usize = 64;
const PAGE_SIZE: usize = 4096;

static EPOCH: AtomicU64 = AtomicU64::new(0);

struct Communicator(mpi::ffi::MPI_Comm);

unsafe impl mpi::traits::AsRaw for Communicator {
    type Raw = mpi::ffi::MPI_Comm;
    fn as_raw(&self) -> Self::Raw {
        self.0
    }
}

impl mpi::traits::Communicator for Communicator {}

#[no_mangle]
pub unsafe extern "C" fn MPI_Init_thread(
    argc: *const ffi::c_int,
    argv: *const *const *const ffi::c_char,
    required: ffi::c_int,
    provided: *const ffi::c_int,
) {
    Lazy::force(&PCI_FILE);
    Lazy::force(&PCI_MAP);
    _MPI_Init_thread(argc, argv, required, provided)
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Bcast(
    buffer: *mut ffi::c_void,
    count: ffi::c_int,
    _: mpi::ffi::MPI_Datatype,
    root: ffi::c_int,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    let comm = Communicator(comm);

    if comm.size() == 1 {
        return mpi::ffi::MPI_SUCCESS as ffi::c_int;
    }

    let epoch_before = EPOCH.load(Ordering::Acquire);
    let epoch_after = epoch_before + comm.size() as u64;

    if comm.rank() == root {
        unsafe {
            let mut shared = PCI_MAP.lock().unwrap();

            let local = std::slice::from_raw_parts(buffer as *const u8, count as usize);

            shared[CACHE_LINE_SIZE..][..count as usize].copy_from_slice(local);

            // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2090-2092
            let epoch = &*shared.as_ptr().cast::<AtomicU64>();

            // Kick off broadcast
            epoch.fetch_add(1, Ordering::AcqRel);

            // Spin waiting for everyone to read
            while epoch.load(Ordering::Acquire) < epoch_after {}
        }
    } else {
        unsafe {
            let shared = PCI_MAP.lock().unwrap();

            // https://doc.rust-lang.org/src/core/sync/atomic.rs.html#2090-2092
            let epoch = &*shared.as_ptr().cast::<AtomicU64>();

            // Spin until broadcast starts
            while epoch.load(Ordering::Acquire) == epoch_before {}

            let local = std::slice::from_raw_parts_mut(buffer as *mut u8, count as usize);
            local.copy_from_slice(&shared[CACHE_LINE_SIZE..][..count as usize]);

            // Update broadcaster
            epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    EPOCH.store(epoch_after, Ordering::Release);
    mpi::ffi::MPI_SUCCESS as ffi::c_int
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Allreduce(
    buffer_send: *const ffi::c_void,
    buffer_receive: *mut ffi::c_void,
    count: ffi::c_int,
    datatype: mpi::ffi::MPI_Datatype,
    op: mpi::ffi::MPI_Op,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    assert_eq!(datatype, mpi::ffi::RSMPI_FLOAT);
    assert_eq!(op, mpi::ffi::RSMPI_SUM);

    let buffer_send = std::slice::from_raw_parts(buffer_send as *const f32, count as usize);
    let buffer_receive = std::slice::from_raw_parts_mut(buffer_receive as *mut f32, count as usize);
    let comm = Communicator(comm);

    // | Region 0 Lock (64B)   |
    // | Region 1 Lock (64B)   |
    // | ...                   |
    // | Broadcast Ready (64B) |
    // | Broadcast Start (64B) |
    // | ...                   |
    // | Region 0 (4KiB)       | <- P0
    // | ...                   |
    // | Region 8 (4KiB)       | <- P1
    // | ...                   |
    // | Region 16 (4KiB)      | <- P2
    // | ...                   |
    // | Region 24 (4KiB)      | <- P3
    // | ...                   |
    let region_size = PAGE_SIZE;
    let total_size = count as usize * mem::size_of::<f32>();
    let region_count = (total_size + region_size - 1) / region_size;
    let region_offset = comm.rank() as usize * (region_count / comm.size() as usize);
    let header_size = region_count * CACHE_LINE_SIZE;

    let mut pci_map = PCI_MAP.lock().unwrap();

    // Partition shared memory into disjoint areas
    let (region_locks, broadcast_ready, broadcast_start, buffer_shared) = {
        let (header, buffer) = pci_map.split_at_mut(header_size);
        let (regions, broadcasts) = header.split_at(region_count * CACHE_LINE_SIZE);
        let (ready, start) = broadcasts.split_at(CACHE_LINE_SIZE);
        let ready = &*ready.as_ptr().cast::<AtomicU64>();
        let start = &*start.as_ptr().cast::<AtomicU64>();
        (regions, ready, start, buffer)
    };

    let (prefix, buffer_shared, suffix) = buffer_shared[..total_size].align_to_mut::<f32>();
    assert!(prefix.is_empty());
    assert!(suffix.is_empty());

    // Start at different offsets
    for region in (0..region_count)
        .cycle()
        .skip(region_offset)
        .take(region_count)
    {
        let address = region * region_size / mem::size_of::<f32>();
        let size = region_size / mem::size_of::<f32>();

        let region_lock = Lock::new(region_locks[region * CACHE_LINE_SIZE..].as_ptr());
        region_lock.lock();

        buffer_shared[address..][..size]
            .iter_mut()
            .zip(&buffer_send[address..][..size])
            .for_each(|(shared, send)| *shared += send);

        region_lock.unlock();
    }

    // Wait for all processes to finish writes
    if broadcast_ready.fetch_add(1, Ordering::AcqRel) == comm.size() as u64 - 1 {
        broadcast_start.store(1, Ordering::Release);
    }

    while broadcast_start.load(Ordering::Acquire) == 0 {}

    buffer_receive.copy_from_slice(buffer_shared);
    mpi::ffi::MPI_SUCCESS as ffi::c_int
}

struct Lock<'pci>(&'pci AtomicU64);

impl<'pci> Lock<'pci> {
    const UNLOCKED: u64 = 0;
    const LOCKED: u64 = 1;

    unsafe fn new(address: *const u8) -> Self {
        Self(&*address.cast::<AtomicU64>())
    }

    fn lock(&self) {
        while self.0.load(Ordering::Acquire) == Self::LOCKED
            || self
                .0
                .compare_exchange(
                    Self::UNLOCKED,
                    Self::LOCKED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {}
    }

    fn unlock(&self) {
        self.0.store(Self::UNLOCKED, Ordering::Release);
    }
}

fn initialize_file() -> anyhow::Result<fs::File> {
    let path = env::var("COLLECTIVE_PCI_PATH")
        .context("Missing COLLECTIVE_PCI_PATH environment variable")?;
    let path = path.trim();

    fs::File::options()
        .read(true)
        .write(true)
        .custom_flags(libc::O_DIRECT)
        .custom_flags(libc::O_SYNC)
        .open(path)
        .with_context(|| anyhow!("Failed to read {}", path))
}

fn initialize_map() -> anyhow::Result<MmapMut> {
    unsafe { MmapMut::map_mut(&*PCI_FILE).context("Failed to mmap PCI file") }
}
