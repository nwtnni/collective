#![allow(clippy::missing_safety_doc)]

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
