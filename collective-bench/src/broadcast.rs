use std::time::Instant;

use mpi::topology::SystemCommunicator;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

#[derive(clap::Parser)]
pub struct Broadcast;

impl Broadcast {
    pub fn run(&self, world: &SystemCommunicator, size: usize) -> u64 {
        let root = world.process_at_rank(0);

        let mut buffer = vec![0u8; size];

        if world.rank() == root.rank() {
            buffer
                .iter_mut()
                .enumerate()
                .for_each(|(index, element)| *element = index as u8);
        }

        world.barrier();
        let start = Instant::now();
        root.broadcast_into(&mut buffer[..]);
        let end = Instant::now();

        end.duration_since(start)
            .as_nanos()
            .try_into()
            .expect("Duration larger than 64 bits")
    }
}
