use std::env;

use anyhow::anyhow;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

fn main() -> anyhow::Result<()> {
    let Some(bytes) = env::args().nth(1).and_then(|argument| argument.parse::<usize>().ok()) else {
        return Err(anyhow!("Usage: broadcast <BYTES>"));
    };

    let universe = mpi::initialize().expect("Failed to initialize MPI");
    let world = universe.world();
    let root = world.process_at_rank(0);

    let mut buffer = vec![0u8; bytes];

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

    if world.rank() == root.rank() {
        println!("MPI_Bcast duration = {}", end - start);
    }

    Ok(())
}
