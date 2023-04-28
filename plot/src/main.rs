use clap::Parser;
use plot::Ifstat;
use plot::Plot as _;

#[derive(Parser)]
enum Command {
    Ifstat(Ifstat),
}

fn main() -> anyhow::Result<()> {
    match Command::parse() {
        Command::Ifstat(ifstat) => ifstat.plot()?,
    }

    Ok(())
}
