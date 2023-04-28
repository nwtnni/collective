use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context as _;
use clap::Parser;
use plotters::prelude::*;

use crate::Plot;

#[derive(Parser)]
pub struct Ifstat {
    /// Sum statistics over all machines.
    #[arg(short, long)]
    sum: bool,

    /// Plot bytes/sec transferred instead of bytes transferred.
    #[arg(long)]
    throughput: bool,

    /// Plot inbound statistics.
    #[arg(short, long, required_unless_present = "transmit")]
    receive: bool,

    /// Plot outbound statistics.
    #[arg(short, long, required_unless_present = "receive")]
    transmit: bool,

    /// Write plot to disk.
    #[arg(short, long, default_value = "out.png")]
    output: PathBuf,

    /// `ifstat` log files to parse. Requires file name in `<INDEX>-<NAME>` format.
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

        let (x_max, receive_max, transmit_max) = logs.values().flatten().fold(
            (f64::MIN, f64::MIN, f64::MIN),
            |(x_max, receive_max, transmit_max), (x, receive, transmit)| {
                (
                    if x_max > *x { x_max } else { *x },
                    if receive_max > *receive {
                        receive_max
                    } else {
                        *receive
                    },
                    if transmit_max > *transmit {
                        transmit_max
                    } else {
                        *transmit
                    },
                )
            },
        );

        let y_max = match (self.receive, self.transmit) {
            (true, true) if receive_max > transmit_max => receive_max,
            (true, true) => transmit_max,
            (true, false) => receive_max,
            (false, true) => transmit_max,
            (false, false) => {
                unreachable!("[INTERNAL ERROR]: clap requires one of --receive or --transmit")
            }
        };

        let mut context = ChartBuilder::on(&root)
            .set_left_and_bottom_label_area_size(50)
            .caption("ifstat", ("sans-serif", 20))
            .build_cartesian_2d(0.0..x_max + 10.0, 0.0..y_max + 10.0)?;

        context.configure_mesh().draw()?;

        if self.receive {
            context.draw_series(LineSeries::new(
                logs[&0].iter().map(|(time, receive, _)| (*time, *receive)),
                GREEN,
            ))?;
        }

        if self.transmit {
            context.draw_series(LineSeries::new(
                logs[&0]
                    .iter()
                    .map(|(time, _, transmit)| (*time, *transmit)),
                GREEN,
            ))?;
        }

        Ok(())
    }
}

fn parse_file(path: &Path, throughput: bool) -> anyhow::Result<Vec<(f64, f64, f64)>> {
    fs::read_to_string(path)?
        .trim()
        .split('\n')
        .map(|line| -> anyhow::Result<_> {
            let mut iter = line.split_whitespace().skip(1);
            let time = iter.next().unwrap().parse::<f64>()?;
            let (receive, transmit) = if throughput { (2, 2) } else { (0, 2) };
            let receive = iter.nth(receive).unwrap().parse::<f64>()?;
            let transmit = iter.nth(transmit).unwrap().parse::<f64>()?;
            Ok((time, receive, transmit))
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
