use std::env;
use std::mem;

use anyhow::anyhow;
use mpi::collective::SystemOperation;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

fn main() -> anyhow::Result<()> {
    let Some(bytes) = env::args().nth(1).and_then(|argument| argument.parse::<usize>().ok()) else {
        return Err(anyhow!("Usage: reduce <BYTES>"));
    };

    assert!(bytes % mem::size_of::<f32>() == 0);

    let universe = mpi::initialize().expect("Failed to initialize MPI");
    let world = universe.world();
    let root = world.process_at_rank(0);

    let local = (0..bytes / mem::size_of::<f32>())
        .map(|index| (world.rank() + index as i32) as f32)
        .collect::<Vec<_>>();
    let mut global = vec![0; bytes / mem::size_of::<f32>()];

    let mut time = 0.0f64;

    world.barrier();
    time -= mpi::time();
    if world.rank() == root.rank() {
        root.reduce_into_root(&local, &mut global, SystemOperation::sum());
    } else {
        root.reduce_into(&local, SystemOperation::sum());
    }
    time += mpi::time();

    if world.rank() == 0 {
        println!("MPI_Reduce duration = {time}");
    }

    Ok(())
}
