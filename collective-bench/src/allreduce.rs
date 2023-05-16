use std::mem;
use std::time::Instant;

use anyhow::anyhow;
use anyhow::Context as _;
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
    pub fn run(
        &self,
        world: &SystemCommunicator,
        size: usize,
        validate: bool,
    ) -> anyhow::Result<u64> {
        assert_eq!(size % mem::size_of::<f32>(), 0);

        let local = (0..size / mem::size_of::<f32>())
            .map(|index| (world.rank() + index as i32) as f32)
            .collect::<Vec<_>>();

        let mut global = vec![0.0f32; size / mem::size_of::<f32>()];

        world.barrier();
        let start = Instant::now();
        world.all_reduce_into(&local, &mut global[..], SystemOperation::sum());
        let end = Instant::now();

        if validate {
            for (index, actual) in global.into_iter().enumerate() {
                let expected =
                    // Contribution from each rank
                    (((world.size() - 1) * world.size()) / 2)
                    // Contribution from each index
                    + (index as i32 * world.size());

                if (actual - expected as f32).abs() > 0.001 {
                    return Err(anyhow!(
                        "Expected value {:03} at index {}, but found {:03}",
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
