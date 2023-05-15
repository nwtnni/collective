#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

mod barrier;
mod broadcast;
mod mutex;

use std::cmp;
use std::env;
use std::ffi;
use std::fs;
use std::mem;
use std::os::unix::fs::OpenOptionsExt as _;

use anyhow::anyhow;
use anyhow::Context as _;
use memmap2::MmapMut;
use mpi::traits::Communicator as _;
use once_cell::sync::Lazy;

use barrier::Barrier;
use mutex::Mutex;

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
static PCI_MAP: Lazy<std::sync::Mutex<MmapMut>> =
    Lazy::new(|| initialize_map().map(std::sync::Mutex::new).unwrap());

const CACHE_LINE_SIZE: usize = 64;
const PAGE_SIZE: usize = 4096;

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
pub unsafe extern "C" fn MPI_Allreduce(
    buffer_send: *const ffi::c_void,
    buffer_receive: *mut ffi::c_void,
    count: ffi::c_int,
    _: mpi::ffi::MPI_Datatype,
    _: mpi::ffi::MPI_Op,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    let comm = Communicator(comm);
    let buffer_send = std::slice::from_raw_parts(buffer_send as *const f32, count as usize);
    let buffer_receive = std::slice::from_raw_parts_mut(buffer_receive as *mut f32, count as usize);

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
    let region_size = PAGE_SIZE;
    let data_size = count as usize * mem::size_of::<f32>();
    let region_count = (data_size + region_size - 1) / region_size;
    let region_offset = comm.rank() as usize * (region_count / comm.size() as usize);

    let mut pci_map = PCI_MAP.lock().unwrap();

    // Partition shared memory into disjoint areas
    let (barrier, locks, buffer_shared) = {
        let (barrier, remainder) = pci_map.split_at_mut(Barrier::SIZE);
        let (locks, remainder) = remainder.split_at_mut(Mutex::SIZE * region_count);
        let (prefix, data, suffix) = remainder[..data_size].align_to_mut::<f32>();

        assert!(prefix.is_empty());
        assert!(suffix.is_empty());

        // Zero memory
        if comm.rank() == 0 {
            locks.fill(0);
            data.fill(0.0);
        }

        let barrier = Barrier::new(barrier.as_ptr());
        let locks = (0..region_count)
            .map(|region| region * region_size)
            .map(|offset| locks[offset..].as_ptr())
            .map(|address| unsafe { Mutex::new(address) })
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
        let offset = region * region_size / mem::size_of::<f32>();
        let count = cmp::min(
            region_size / mem::size_of::<f32>(),
            buffer_shared[offset..].len(),
        );

        locks[region].lock();

        buffer_shared[offset..][..count]
            .iter_mut()
            .zip(&buffer_send[offset..][..count])
            .for_each(|(shared, send)| *shared += send);

        locks[region].unlock();
    }

    // Wait for all processes to finish writes
    barrier.wait(comm.size());

    buffer_receive.copy_from_slice(buffer_shared);
    mpi::ffi::MPI_SUCCESS as ffi::c_int
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
