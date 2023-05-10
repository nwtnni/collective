#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi;

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

static _MPI_Bcast: Lazy<
    libloading::Symbol<
        'static,
        unsafe extern "C" fn(
            *const ffi::c_void,
            ffi::c_int,
            mpi::ffi::MPI_Datatype,
            ffi::c_int,
            mpi::ffi::MPI_Comm,
        ) -> ffi::c_int,
    >,
> = Lazy::new(|| unsafe { LIBMPI.get(b"MPI_Bcast\0").unwrap() });

static _MPI_Barrier: Lazy<
    libloading::Symbol<'static, unsafe extern "C" fn(mpi::ffi::MPI_Comm) -> ffi::c_int>,
> = Lazy::new(|| unsafe { LIBMPI.get(b"MPI_Barrier\0").unwrap() });

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
    buffer: *const ffi::c_void,
    count: ffi::c_int,
    datatype: mpi::ffi::MPI_Datatype,
    root: ffi::c_int,
    comm: mpi::ffi::MPI_Comm,
) -> ffi::c_int {
    println!(
        "Called MPI_Bcast with arguments: {buffer:?} {count:?} {datatype:?} {root:?} {comm:?}",
    );

    _MPI_Bcast(buffer, count, datatype, root, comm)
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Barrier(comm: mpi::ffi::MPI_Comm) -> ffi::c_int {
    println!("Called MPI_Barrier with arguments: {comm:?}");

    _MPI_Barrier(comm)
}
