// src/bin/test_smooth.rs
//
// Test + affichage: points "discrets" vs "splined" (PCHIP) + export PNG.
//
// Usage:
//   cargo run --bin test_smooth -- data/HE/M821_HE_4R.csv --step 25 --out compare.png
//
// (step=1 si tu veux au metre, mais pour le print c’est long)

use anyhow::{bail, Context, Result};
use clap::Parser;
use csv::Reader;
use mortar::pchip::{pchip_eval, pchip_slopes};
use plotters::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    input: PathBuf,
    #[arg(long, default_value_t = 1)]
    step: i32,
    #[arg(long, default_value = "compare.png")]
    out: String,
    #[arg(long, default_value_t = 20)]
    print_n: usize,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct InRow {
    range_m: f64,
    elev_mil: f64,
    #[serde(default)]
    time_flight_s: Option<f64>,
    #[serde(default)]
    delta_elev_per_100m_mil: Option<f64>,
    #[serde(default)]
    time_flight_per_100m_s: Option<f64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.step <= 0 {
        bail!("--step must be > 0");
    }

    // read
    let mut rdr =
        Reader::from_path(&args.input).with_context(|| format!("open {}", args.input.display()))?;

    let mut pts: Vec<(f64, f64)> = vec![];
    for rec in rdr.deserialize::<InRow>() {
        let r = rec?;
        if r.range_m.is_finite() && r.elev_mil.is_finite() {
            pts.push((r.range_m, r.elev_mil));
        }
    }
    if pts.len() < 2 {
        bail!("Not enough points");
    }
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // unique by x
    let mut x: Vec<f64> = vec![];
    let mut y: Vec<f64> = vec![];
    for (rx, ry) in pts {
        if x.last().copied() == Some(rx) {
            *y.last_mut().unwrap() = ry;
        } else {
            x.push(rx);
            y.push(ry);
        }
    }

    let d = pchip_slopes(&x, &y)?;

    let x_min = x[0].ceil() as i32;
    let x_max = x[x.len() - 1].floor() as i32;

    let mut spline: Vec<(i32, f64)> = vec![];
    let mut r = x_min;
    while r <= x_max {
        let elev = pchip_eval(&x, &y, &d, r as f64)?;
        spline.push((r, elev));
        r += args.step;
    }

    // ---- PRINT (discret + splined) ----
    println!("--- DISCRETE (first {}) ---", args.print_n);
    for (i, (rx, ry)) in x.iter().zip(y.iter()).take(args.print_n).enumerate() {
        println!("{:>2}: range={:>6.0}m  elev={:>8.2} mil", i, rx, ry);
    }

    println!(
        "\n--- SPLINED step={}m (first {}) ---",
        args.step, args.print_n
    );
    for (i, (rx, ry)) in spline.iter().take(args.print_n).enumerate() {
        println!("{:>2}: range={:>6}m  elev={:>8.2} mil", i, rx, ry);
    }

    // ---- PLOT PNG ----
    let root = BitMapBackend::new(&args.out, (1200, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    // y axis inverted (mortar logic)
    let (xmin, xmax) = (x_min as f64, x_max as f64);
    let mut ymin = y.iter().cloned().fold(f64::INFINITY, f64::min);
    let mut ymax = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    // include spline range too
    for (_, v) in &spline {
        ymin = ymin.min(*v);
        ymax = ymax.max(*v);
    }

    let mut chart = ChartBuilder::on(&root)
        .caption("Discrete vs PCHIP spline", ("sans-serif", 30))
        .margin(15)
        .x_label_area_size(45)
        .y_label_area_size(70)
        .build_cartesian_2d(xmin..xmax, ymax..ymin)?; // inverted Y

    chart
        .configure_mesh()
        .x_desc("Range (m)")
        .y_desc("Elevation (mil)")
        .draw()?;

    // Discrete points
    chart
        .draw_series(
            x.iter()
                .zip(y.iter())
                .map(|(rx, ry)| Circle::new((*rx, *ry), 4, BLACK.filled())),
        )?
        .label("discrete")
        .legend(|(x, y)| Circle::new((x, y), 4, BLACK.filled()));

    // Spline line
    chart
        .draw_series(LineSeries::new(
            spline.iter().map(|(rx, ry)| (*rx as f64, *ry)),
            &RED,
        ))?
        .label("pchip spline")
        .legend(|(x, y)| PathElement::new(vec![(x - 10, y), (x + 10, y)], &RED));

    chart
        .configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.9))
        .draw()?;

    println!("\nSaved plot: {}", args.out);
    Ok(())
}
