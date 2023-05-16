use std::mem;
use std::time::Instant;

use mpi::collective::SystemOperation;
use mpi::topology::SystemCommunicator;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

#[derive(clap::Parser)]
pub struct Reduce;

impl Reduce {
    pub fn run(&self, world: &SystemCommunicator, size: usize) -> u64 {
        let root = world.process_at_rank(0);

        let local = (0..size / mem::size_of::<f32>())
            .map(|index| (world.rank() + index as i32) as f32)
            .collect::<Vec<_>>();
        let mut global = vec![0; size / mem::size_of::<f32>()];

        world.barrier();
        let start = Instant::now();
        if world.rank() == root.rank() {
            root.reduce_into_root(&local, &mut global, SystemOperation::sum());
        } else {
            root.reduce_into(&local, SystemOperation::sum());
        }
        let end = Instant::now();

        end.duration_since(start)
            .as_nanos()
            .try_into()
            .expect("Duration larger than 64 bits")
    }
}
