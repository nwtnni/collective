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

    /// Minimum of time window to plot
    #[arg(long)]
    x_min: Option<f64>,

    /// Maximum of time window to plot
    #[arg(long)]
    x_max: Option<f64>,

    /// Write plot to disk.
    #[arg(short, long, default_value = "out.svg")]
    output: PathBuf,

    /// `ifstat` log files to parse. Requires file name in `<INDEX>-<NAME>` format.
    logs: Vec<PathBuf>,
}

impl Plot for Ifstat {
    fn plot(self) -> anyhow::Result<()> {
        let (_, name) = parse_path(&self.logs[0])?;

        let logs = self
            .logs
            .iter()
            .map(|path| -> anyhow::Result<_> {
                Ok((
                    parse_path(path)?.0,
                    parse_file(path, self.throughput)
                        .with_context(|| anyhow!("Failed to read {}", path.display()))?,
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let root = SVGBackend::new(&self.output, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        let (receive_max, transmit_max) = logs.values().flatten().fold(
            (f64::MIN, f64::MIN),
            |(receive_max, transmit_max), (_, receive, transmit)| {
                (
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

        let x_min = self.x_min.unwrap_or_else(|| {
            logs.values()
                .map(|log| {
                    constrict(
                        log.iter()
                            .map(|(time, receive, transmit)| (*time, receive.max(*transmit))),
                    )
                })
                .min_by(f64::total_cmp)
                .expect("[INTERNAL ERROR]: `logs` is nonempty")
        });

        let x_max = self.x_max.unwrap_or_else(|| {
            logs.values()
                .map(|log| {
                    constrict(
                        log.iter()
                            .rev()
                            .map(|(time, receive, transmit)| (*time, receive.min(*transmit))),
                    )
                })
                .max_by(f64::total_cmp)
                .expect("[INTERNAL ERROR]: `logs` is nonempty")
        });

        let y_max = match (self.receive, self.transmit) {
            (true, true) => receive_max.max(transmit_max),
            (true, false) => receive_max,
            (false, true) => transmit_max,
            (false, false) => {
                unreachable!("[INTERNAL ERROR]: clap requires one of --receive or --transmit")
            }
        };

        let mut context = ChartBuilder::on(&root)
            .set_left_and_bottom_label_area_size(50)
            .caption(
                format!("Network data transfer for {name} benchmark"),
                ("sans-serif", 20),
            )
            .build_cartesian_2d(0.0..x_max - x_min, 0.0..y_max)?;

        context
            .configure_mesh()
            .x_desc("Time Elapsed (sec)")
            .y_desc("Data Transferred (GiB)")
            .draw()?;

        let mut order = logs.keys().copied().collect::<Vec<_>>();
        order.sort();

        if self.receive {
            for index in &order {
                let log = &logs[index];
                let color = Palette99::pick(*index).to_rgba();

                context
                    .draw_series(LineSeries::new(
                        log.iter()
                            .map(|(time, receive, _)| (*time - x_min, *receive)),
                        color,
                    ))?
                    .label(format!("Received (Node {index})"))
                    .legend(move |(x, y)| Circle::new((x + 10, y), 5, color.filled()));

                context.draw_series(log.iter().map(|(time, receive, _)| {
                    Circle::new((*time - x_min, *receive), 2, color.filled())
                }))?;
            }
        }

        if self.transmit {
            for index in &order {
                let log = &logs[index];
                let color = Palette99::pick(*index).to_rgba();

                context
                    .draw_series(LineSeries::new(
                        log.iter()
                            .map(|(time, _, transmit)| (*time - x_min, *transmit)),
                        color,
                    ))?
                    .label(format!("Transmitted (Node {index})"))
                    .legend(move |(x, y)| Cross::new((x + 10, y), 5, color.filled()));

                context.draw_series(
                    log.iter().map(|(time, _, transmit)| {
                        Cross::new((*time - x_min, *transmit), 2, color)
                    }),
                )?;
            }
        }

        context
            .configure_series_labels()
            .position(SeriesLabelPosition::LowerRight)
            .background_style(WHITE)
            .border_style(BLACK)
            .draw()?;

        Ok(())
    }
}

fn constrict<I: IntoIterator<Item = (f64, f64)>>(data: I) -> f64 {
    let mut data = data.into_iter();
    let (mut xi, yi) = data.next().unwrap();
    for (x, y) in data {
        if (yi - y).abs() < 1e-9 {
            break;
        } else {
            xi = x;
        }
    }
    xi
}

fn parse_file(path: &Path, throughput: bool) -> anyhow::Result<Vec<(f64, f64, f64)>> {
    fs::read_to_string(path)?
        .trim()
        .split('\n')
        .map(|line| -> anyhow::Result<_> {
            let mut iter = line.split_whitespace().skip(2);
            let time = iter.next().unwrap().parse::<f64>()? / 1e9;
            let (receive, transmit) = if throughput { (2, 2) } else { (0, 2) };
            let receive = iter.nth(receive).unwrap().parse::<f64>()? / 1e9;
            let transmit = iter.nth(transmit).unwrap().parse::<f64>()? / 1e9;
            Ok((time, receive, transmit))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn parse_path(path: &Path) -> anyhow::Result<(usize, &str)> {
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("Expected path with file name, but got {}", path.display()))?;

    let name = name
        .to_str()
        .ok_or_else(|| anyhow!("Expected Unicode path name, but got {}", path.display()))?;

    name.split_once('-')
        .and_then(|(index, name)| (index.parse::<usize>().ok().map(|index| (index, name))))
        .ok_or_else(|| {
            anyhow!(
                "Expected file name in <INDEX>-<NAME> format, but got {}",
                path.display()
            )
        })
}
