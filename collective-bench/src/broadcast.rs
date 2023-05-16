use mpi::topology::SystemCommunicator;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

#[derive(clap::Parser)]
pub struct Broadcast;

impl Broadcast {
    pub fn run(&self, world: &SystemCommunicator, size: usize) -> f64 {
        let root = world.process_at_rank(0);

        let mut buffer = vec![0u8; size];

        if world.rank() == root.rank() {
            buffer
                .iter_mut()
                .enumerate()
                .for_each(|(index, element)| *element = index as u8);
        }

        world.barrier();
        let start = mpi::time();
        root.broadcast_into(&mut buffer[..]);
        let end = mpi::time();

        end - start
    }
}
