#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]

use std::ffi;

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

    let library = libloading::Library::new("libmpi.so").unwrap();
    let mpi_init_thread = library
        .get::<unsafe extern "C" fn(
            *const ffi::c_int,
            *const *const *const ffi::c_char,
            ffi::c_int,
            *const ffi::c_int,
        )>(b"MPI_Init_thread\0")
        .unwrap();

    mpi_init_thread(argc, argv, required, provided)
}

type MPI_Datatype = ffi::c_int;
type MPI_Comm = ffi::c_int;

#[no_mangle]
pub unsafe extern "C" fn MPI_Bcast(
    buffer: *const ffi::c_void,
    count: ffi::c_int,
    datatype: MPI_Datatype,
    root: ffi::c_int,
    comm: MPI_Comm,
) -> ffi::c_int {
    println!(
        "Called MPI_Bcast with arguments: {buffer:?} {count:?} {datatype:?} {root:?} {comm:?}",
    );

    let library = libloading::Library::new("libmpi.so").unwrap();
    let mpi_bcast = library
        .get::<unsafe extern "C" fn(
            *const ffi::c_void,
            ffi::c_int,
            MPI_Datatype,
            ffi::c_int,
            MPI_Comm,
        ) -> ffi::c_int>(b"MPI_Bcast\0")
        .unwrap();

    mpi_bcast(buffer, count, datatype, root, comm)
}

#[no_mangle]
pub unsafe extern "C" fn MPI_Barrier(comm: MPI_Comm) -> ffi::c_int {
    println!("Called MPI_Barrier with arguments: {comm:?}");

    let library = libloading::Library::new("libmpi.so").unwrap();
    let mpi_barrier = library
        .get::<unsafe extern "C" fn(MPI_Comm) -> ffi::c_int>(b"MPI_Barrier\0")
        .unwrap();

    mpi_barrier(comm)
}
