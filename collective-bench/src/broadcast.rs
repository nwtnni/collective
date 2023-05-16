use std::time::Instant;

use anyhow::anyhow;
use anyhow::Context as _;
use mpi::topology::SystemCommunicator;
use mpi::traits::Communicator as _;
use mpi::traits::CommunicatorCollectives as _;
use mpi::traits::Root as _;

#[derive(clap::Parser)]
pub struct Broadcast;

impl Broadcast {
    pub fn run(
        &self,
        world: &SystemCommunicator,
        size: usize,
        validate: bool,
    ) -> anyhow::Result<u64> {
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

        if validate {
            for (index, actual) in buffer.into_iter().enumerate() {
                let expected = index as u8;
                if actual != expected {
                    return Err(anyhow!(
                        "Expected value {} at index {}, but found {}",
                        expected,
                        index,
                        actual,
                    ));
                }
            }
        }

        end.duration_since(start)
            .as_nanos()
            .try_into()
            .context("Duration larger than 64 bits")
    }
}
