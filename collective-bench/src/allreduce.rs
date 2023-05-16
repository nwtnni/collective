use std::mem;
use std::time::Instant;

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
    pub fn run(&self, world: &SystemCommunicator, size: usize) -> u64 {
        let local = (0..size / mem::size_of::<f32>())
            .map(|index| (world.rank() + index as i32) as f32)
            .collect::<Vec<_>>();

        let mut global = vec![0; size / mem::size_of::<f32>()];

        world.barrier();
        let start = Instant::now();
        world.all_reduce_into(&local, &mut global[..], SystemOperation::sum());
        let end = Instant::now();

        end.duration_since(start)
            .as_nanos()
            .try_into()
            .expect("Duration larger than 64 bits")
    }
}
