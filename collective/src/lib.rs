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

const CACHE_LINE: usize = 64;

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

            shared[CACHE_LINE..][..count as usize].copy_from_slice(local);

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
            local.copy_from_slice(&shared[CACHE_LINE..][..count as usize]);

            // Update broadcaster
            epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    EPOCH.store(epoch_after, Ordering::Release);
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
