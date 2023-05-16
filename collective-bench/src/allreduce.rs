use std::mem;

use clap::Parser;
use mpi::collective::SystemOperation;
use mpi::topology::SystemCommunicator;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;

#[derive(Parser)]
pub struct Allreduce {
    #[arg(short, long, value_enum)]
    operation: crate::Operation,
}

impl Allreduce {
    pub fn run(&self, world: &SystemCommunicator, size: usize) -> f64 {
        let local = (0..size / mem::size_of::<f32>())
            .map(|index| (world.rank() + index as i32) as f32)
            .collect::<Vec<_>>();

        let mut global = vec![0; size / mem::size_of::<f32>()];

        world.barrier();
        let start = mpi::time();
        world.all_reduce_into(&local, &mut global[..], SystemOperation::sum());
        let end = mpi::time();

        end - start
    }
}
