use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Context as _;
use clap::Parser;
use plotters::prelude::*;

use crate::Plot;

#[derive(Parser)]
pub struct Ifstat {
    #[arg(short, long)]
    sum: bool,

    #[arg(short, long)]
    throughput: bool,

    #[arg(short, long, required = true)]
    direction: Direction,

    #[arg(short, long, default_value = "out.png")]
    output: PathBuf,

    logs: Vec<PathBuf>,
}

impl Plot for Ifstat {
    fn plot(self) -> anyhow::Result<()> {
        let logs = self
            .logs
            .iter()
            .map(|path| -> anyhow::Result<_> {
                Ok((
                    parse_path(path)?,
                    parse_file(path, self.throughput)
                        .with_context(|| anyhow!("Failed to read {}", path.display()))?,
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let root = BitMapBackend::new(&self.output, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        let (x_max, y_max) =
            logs.values()
                .flatten()
                .fold((f64::MIN, f64::MIN), |(x_max, y_max), (x, y)| {
                    (
                        if x_max > *x { x_max } else { *x },
                        if y_max > *y { y_max } else { *y },
                    )
                });

        let mut context = ChartBuilder::on(&root)
            .set_left_and_bottom_label_area_size(50)
            .caption("ifstat", ("sans-serif", 20))
            .build_cartesian_2d(0.0..x_max + 10.0, 0.0..y_max + 10.0)?;

        context.configure_mesh().draw()?;
        context.draw_series(LineSeries::new(logs[&0].iter().copied(), GREEN))?;

        Ok(())
    }
}

fn parse_file(path: &Path, throughput: bool) -> anyhow::Result<Vec<(f64, f64)>> {
    fs::read_to_string(path)?
        .trim()
        .split('\n')
        .map(|line| -> anyhow::Result<_> {
            let mut iter = line.split_whitespace().skip(1);
            let time = iter.next().unwrap().parse::<f64>()?;
            let value = iter
                .nth(if throughput { 2 } else { 0 })
                .unwrap()
                .parse::<f64>()?;
            Ok((time, value))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn parse_path(path: &Path) -> anyhow::Result<usize> {
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("Expected path with file name, but got {}", path.display()))?;

    let name = name
        .to_str()
        .ok_or_else(|| anyhow!("Expected Unicode path name, but got {}", path.display()))?;

    name.split_once('-')
        .and_then(|(index, _)| index.parse::<usize>().ok())
        .ok_or_else(|| {
            anyhow!(
                "Expected file name in <INDEX>-<NAME> format, but got {}",
                path.display()
            )
        })
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Direction {
    Receive,
    Transmit,
}

impl FromStr for Direction {
    type Err = anyhow::Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "receive" | "r" | "in" | "i" => Ok(Direction::Receive),
            "transmit" | "t" | "out" | "o" => Ok(Direction::Transmit),
            unknown => Err(anyhow!(
                "Expected one of {{[r]eceive, [t]ransmit, [i]n, [o]ut}}, but got {}",
                unknown
            )),
        }
    }
}
