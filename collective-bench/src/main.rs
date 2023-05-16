use clap::Parser;

use collective_bench::Benchmark;
use collective_bench::Configuration;

#[derive(Parser)]
struct Command {
    #[command(flatten)]
    configuration: Configuration,

    #[command(subcommand)]
    benchmark: Benchmark,
}

fn main() -> anyhow::Result<()> {
    let command = Command::parse();
    command.benchmark.run(&command.configuration)
}
