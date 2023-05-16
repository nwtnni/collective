mod allreduce;
mod broadcast;
mod reduce;

use anyhow::anyhow;

#[derive(clap::Parser)]
pub enum Benchmark {
    Allreduce(allreduce::Allreduce),
    Broadcast(broadcast::Broadcast),
    Reduce(reduce::Reduce),
}

#[derive(clap::Args)]
pub struct Configuration {
    #[arg(short, long, default_value = "1000")]
    iterations: usize,

    #[arg(short, long, default_value = "200")]
    warmup: usize,

    #[arg(short, long, value_delimiter = ',')]
    sizes: Vec<usize>,
}

#[derive(Copy, Clone, clap::ValueEnum)]
pub enum Operation {
    Sum,
}

impl Benchmark {
    pub fn run(mut self, configuration: &Configuration) -> anyhow::Result<()> {
        let universe = mpi::initialize().ok_or_else(|| anyhow!("Failed to initialize MPI"))?;
        let world = universe.world();

        for size in &configuration.sizes {
            for iteration in 0..configuration.warmup + configuration.iterations {
                let duration = match &mut self {
                    Benchmark::Allreduce(allreduce) => allreduce.run(&world, *size),
                    Benchmark::Broadcast(broadcast) => broadcast.run(&world, *size),
                    Benchmark::Reduce(reduce) => reduce.run(&world, *size),
                };

                if iteration >= configuration.warmup {
                    println!("{duration}");
                }
            }
        }

        Ok(())
    }
}
