pub(crate) trait MpiType {
    fn matches(datatype: mpi::ffi::MPI_Datatype) -> bool;

    fn sum_mut(&mut self, other: &Self);
}

impl MpiType for f32 {
    fn matches(datatype: mpi::ffi::MPI_Datatype) -> bool {
        unsafe { datatype == mpi::ffi::RSMPI_FLOAT }
    }

    fn sum_mut(&mut self, other: &Self) {
        *self += other;
    }
}

impl MpiType for i8 {
    fn matches(datatype: mpi::ffi::MPI_Datatype) -> bool {
        unsafe { datatype == mpi::ffi::RSMPI_INT8_T }
    }

    fn sum_mut(&mut self, other: &Self) {
        *self += other;
    }
}

impl MpiType for i32 {
    fn matches(datatype: mpi::ffi::MPI_Datatype) -> bool {
        unsafe { datatype == mpi::ffi::RSMPI_INT32_T }
    }

    fn sum_mut(&mut self, other: &Self) {
        *self += other;
    }
}
