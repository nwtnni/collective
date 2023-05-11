#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi;
use std::fs;
use std::os::unix::fs::OpenOptionsExt as _;
use std::sync::Mutex;
use std::thread;
use std::time;

use anyhow::anyhow;
use anyhow::Context as _;
use memmap2::MmapMut;
use mpi::traits::Communicator as _;
use once_cell::sync::Lazy;

static LIBMPI: Lazy<libloading::Library> =
    Lazy::new(|| unsafe { libloading::Library::new("libmpi.so").unwrap() });

static _MPI_Init_thread: Lazy<
    libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            *const ffi::c_int,
            *const *const *const ffi::c_char,
            ffi::c_int,
            *const ffi::c_int,
        ),
    >,
> = Lazy::new(|| unsafe { LIBMPI.get(b"MPI_Init_thread\0").unwrap() });

static _MPI_Barrier: Lazy<
    libloading::Symbol<'static, unsafe extern "C" fn(mpi::ffi::MPI_Comm) -> ffi::c_int>,
> = Lazy::new(|| unsafe { LIBMPI.get(b"MPI_Barrier\0").unwrap() });

static PCI_FILE: Lazy<fs::File> = Lazy::new(|| initialize_file().unwrap());
static PCI_MAP: Lazy<Mutex<MmapMut>> = Lazy::new(|| initialize_map().map(Mutex::new).unwrap());

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
    println!(
        "Called MPI_Init_thread with arguments: {:?} {:?} {:?} {:?}",
        argc, argv, required, provided,
    );

    _MPI_Init_thread(argc, argv, required, provided)
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Bcast(
    buffer: *mut ffi::c_void,
    count: ffi::c_int,
    datatype: mpi::ffi::MPI_Datatype,
    root: ffi::c_int,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    println!(
        "Called MPI_Bcast with arguments: {buffer:?} {count:?} {datatype:?} {root:?} {comm:?}",
    );

    let comm = Communicator(comm);

    if comm.size() == 1 {
        return mpi::ffi::MPI_SUCCESS as ffi::c_int;
    }

    if comm.rank() == root {
        println!("Called MPI_Bcast from root: {}", comm.rank());

        unsafe {
            let slice = std::slice::from_raw_parts(buffer as *const u8, count as usize);
            PCI_MAP.lock().unwrap().copy_from_slice(slice);
        }
    } else {
        println!("Called MPI_Bcast from peer: {}", comm.rank());

        unsafe {
            // FIXME: implement proper synchronization through shared memory
            thread::sleep(time::Duration::from_secs(5));

            let slice = std::slice::from_raw_parts_mut(buffer as *mut u8, count as usize);
            slice.copy_from_slice(&PCI_MAP.lock().unwrap());
        }
    }

    mpi::ffi::MPI_SUCCESS as ffi::c_int
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Barrier(comm: mpi::ffi::MPI_Comm) -> ffi::c_int {
    println!("Called MPI_Barrier with arguments: {comm:?}");

    _MPI_Barrier(comm)
}

fn initialize_file() -> anyhow::Result<fs::File> {
    let path = fs::read_to_string("./pci.txt").context("Failed to read ./pci.txt")?;
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
