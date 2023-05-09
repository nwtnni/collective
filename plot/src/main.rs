use clap::Parser;
use plot::Ifstat;
use plot::Osu;
use plot::Plot as _;

#[derive(Parser)]
enum Command {
    Ifstat(Ifstat),
    Osu(Osu),
}

fn main() -> anyhow::Result<()> {
    match Command::parse() {
        Command::Ifstat(ifstat) => ifstat.plot()?,
        Command::Osu(osu) => osu.plot()?,
    }

    Ok(())
}
