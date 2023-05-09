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
pub struct Osu {
    #[arg(long, default_value = "0")]
    min_size: u64,
    #[arg(long, default_value = "1048676")]
    max_size: u64,
    #[arg(short, long)]
    benchmark: String,
    #[arg(short, long)]
    algorithm: String,
    #[arg(short, long, default_value = "out.svg")]
    output: PathBuf,
    directories: Vec<PathBuf>,
}

impl Plot for Osu {
    fn plot(self) -> anyhow::Result<()> {
        let name = format!("osu-{}-{}.txt", self.benchmark, self.algorithm);
        let versions = self
            .directories
            .iter()
            .map(|directory| -> anyhow::Result<_> {
                let path = directory.join(&name);
                let data = parse_file(&path, self.min_size, self.max_size)
                    .with_context(|| anyhow!("Failed to parse file: {}", path.display()))?;
                Ok((directory.file_name().unwrap().to_string_lossy(), data))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let root = SVGBackend::new(&self.output, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        let max_latency = versions
            .values()
            .flatten()
            .map(|(_, latency)| *latency)
            .reduce(|a, b| a.max(b))
            .unwrap();

        let mut context = ChartBuilder::on(&root)
            .set_left_and_bottom_label_area_size(50)
            .margin(50)
            .caption(
                format!(
                    "Latency (us) vs. Size (bytes) for {} ({})",
                    self.benchmark, self.algorithm,
                ),
                ("sans-serif", 20),
            )
            .build_cartesian_2d(
                (0..self.max_size).with_key_points(
                    versions
                        .values()
                        .next()
                        .unwrap()
                        .iter()
                        .map(|(size, _)| *size)
                        .collect(),
                ),
                0.0..max_latency,
            )?;

        context
            .configure_mesh()
            .x_desc("Size (bytes)")
            .y_desc("Latency (us)")
            .draw()?;

        let mut order = versions.keys().cloned().collect::<Vec<_>>();
        order.sort();

        for (index, version) in order.iter().enumerate() {
            let data = &versions[version];
            let color = Palette99::pick(index).to_rgba();

            context
                .draw_series(LineSeries::new(
                    data.iter().map(|(size, latency)| (*size, *latency)),
                    color,
                ))?
                .label(version.clone())
                .legend(move |(x, y)| Cross::new((x + 10, y), 5, color.filled()));

            context.draw_series(
                data.iter()
                    .map(|(size, latency)| Circle::new((*size, *latency), 2, color)),
            )?;
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

fn parse_file(path: &Path, min_size: u64, max_size: u64) -> anyhow::Result<Vec<(u64, f64)>> {
    fs::read_to_string(path)?
        .trim()
        .split('\n')
        .skip(3)
        .map(|line| -> anyhow::Result<_> {
            let mut iter = line.split_whitespace();
            let size = iter.next().unwrap().parse::<u64>()?;
            let latency = iter.next().unwrap().parse::<f64>()?;
            Ok((size, latency))
        })
        .filter(|result| match result {
            Ok((size, _)) => *size >= min_size && *size <= max_size,
            Err(_) => true,
        })
        .collect::<Result<_, _>>()
}
