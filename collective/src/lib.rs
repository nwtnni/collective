#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

mod allreduce;
mod barrier;
mod broadcast;
mod mutex;

use std::env;
use std::ffi;
use std::fs;
use std::mem;
use std::os::unix::fs::OpenOptionsExt as _;

use anyhow::anyhow;
use anyhow::Context as _;
use memmap2::MmapMut;
use once_cell::sync::Lazy;

const CACHE_LINE_SIZE: usize = 64;
const PAGE_SIZE: usize = 4096;

static PCI_FILE: Lazy<fs::File> = Lazy::new(|| initialize_file().unwrap());
static PCI_MAP: Lazy<std::sync::Mutex<MmapMut>> =
    Lazy::new(|| initialize_map().map(std::sync::Mutex::new).unwrap());

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

    // Forward to actual `MPI_Init_thread`
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

    _MPI_Init_thread(argc, argv, required, provided)
}

fn initialize_file() -> anyhow::Result<fs::File> {
    let path = env::var("COLLECTIVE_PCI_PATH")
        .context("Missing COLLECTIVE_PCI_PATH environment variable")?;
    let path = path.trim();

    let o_direct = match env::var("COLLECTIVE_O_DIRECT") {
        Ok(_) => libc::O_DIRECT,
        Err(_) => 0,
    };

    let o_sync = match env::var("COLLECTIVE_O_SYNC") {
        Ok(_) => libc::O_SYNC,
        Err(_) => 0,
    };

    fs::File::options()
        .read(true)
        .write(true)
        .custom_flags(o_direct | o_sync)
        .open(path)
        .with_context(|| anyhow!("Failed to read {}", path))
}

fn initialize_map() -> anyhow::Result<MmapMut> {
    unsafe { MmapMut::map_mut(&*PCI_FILE).context("Failed to mmap PCI file") }
}
