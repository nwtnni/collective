mod allreduce;
mod broadcast;

use std::io;
use std::io::BufRead as _;
use std::io::Write as _;

use anyhow::anyhow;
use anyhow::Context;
use hdrhistogram::Histogram;
use mpi::traits::Communicator as _;

#[derive(clap::Parser)]
pub enum Benchmark {
    Allreduce(allreduce::Allreduce),
    Broadcast(broadcast::Broadcast),
    Summarize,
}

#[derive(clap::Args)]
pub struct Configuration {
    #[arg(short, long, default_value = "1000")]
    iterations: usize,

    #[arg(short, long, default_value = "200")]
    warmup: usize,

    #[arg(short, long, value_delimiter = ',')]
    sizes: Vec<usize>,

    #[arg(short, long)]
    validate: bool,
}

#[derive(Copy, Clone, clap::ValueEnum)]
pub enum Operation {
    Sum,
}

impl Benchmark {
    pub fn run(self, configuration: &Configuration) -> anyhow::Result<()> {
        if matches!(self, Benchmark::Summarize) {
            return Self::summarize();
        }

        let universe = mpi::initialize().ok_or_else(|| anyhow!("Failed to initialize MPI"))?;
        let world = universe.world();
        let mut stdout = io::stdout().lock();

        for size in &configuration.sizes {
            if world.rank() == 0 {
                write!(stdout, "{size}")?;
            }

            for iteration in 0..configuration.warmup + configuration.iterations {
                let duration = match &self {
                    Benchmark::Allreduce(allreduce) => {
                        allreduce.run(&world, *size, configuration.validate)?
                    }
                    Benchmark::Broadcast(broadcast) => {
                        broadcast.run(&world, *size, configuration.validate)?
                    }
                    Benchmark::Summarize { .. } => unreachable!(),
                };

                if iteration >= configuration.warmup && world.rank() == 0 {
                    write!(stdout, ",{}", duration)?;
                }
            }

            if world.rank() == 0 {
                writeln!(stdout)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    fn summarize() -> anyhow::Result<()> {
        let stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();
        let mut histogram = Histogram::<u64>::new(3).context("Failed to construct histogram")?;

        writeln!(
            stdout,
            "size,count,mean,stdev,0,25,50,75,90,99,99.9,99.99,100",
        )?;

        for line in stdin.lines() {
            histogram.clear();

            let line = line.context("Expected UTF-8")?;
            let Some((size, durations)) = line.trim().split_once(',') else {
                return Err(anyhow!("Expected format size,duration-0,duration-1,..."));
            };

            for duration in durations.split(',') {
                let duration = duration
                    .parse::<u64>()
                    .context("Expected u64 (nanosecond) durations")?;

                histogram.record(duration)?;
            }

            write!(
                stdout,
                "{},{},{},{}",
                size,
                histogram.len(),
                histogram.mean(),
                histogram.stdev(),
            )?;

            for percentile in [0.0, 25.0, 50.0, 75.0, 90.0, 99.0, 99.9, 99.99, 100.0] {
                write!(stdout, ",{}", histogram.value_at_percentile(percentile))?;
            }

            writeln!(stdout)?;
        }

        Ok(())
    }
}
