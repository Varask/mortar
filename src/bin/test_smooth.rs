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

// ---- PCHIP (copie identique de ton smooth_csv.rs) ----

fn pchip_slopes(x: &[f64], y: &[f64]) -> Result<Vec<f64>> {
    let n = x.len();
    if n < 2 {
        bail!("Need at least 2 points");
    }

    let mut h = vec![0.0; n - 1];
    let mut delta = vec![0.0; n - 1];

    for i in 0..(n - 1) {
        h[i] = x[i + 1] - x[i];
        if h[i] <= 0.0 {
            bail!("x must be strictly increasing");
        }
        delta[i] = (y[i + 1] - y[i]) / h[i];
    }

    let mut d = vec![0.0; n];

    if n == 2 {
        d[0] = delta[0];
        d[1] = delta[0];
        return Ok(d);
    }

    // d0
    {
        let h0 = h[0];
        let h1 = h[1];
        let del0 = delta[0];
        let del1 = delta[1];
        let mut d0 = ((2.0 * h0 + h1) * del0 - h0 * del1) / (h0 + h1);
        if d0.signum() != del0.signum() {
            d0 = 0.0;
        } else if (del0.signum() != del1.signum()) && (d0.abs() > 3.0 * del0.abs()) {
            d0 = 3.0 * del0;
        }
        d[0] = d0;
    }

    // dn-1
    {
        let hn2 = h[n - 2];
        let hn3 = h[n - 3];
        let deln2 = delta[n - 2];
        let deln3 = delta[n - 3];
        let mut dn = ((2.0 * hn2 + hn3) * deln2 - hn2 * deln3) / (hn2 + hn3);
        if dn.signum() != deln2.signum() {
            dn = 0.0;
        } else if (deln2.signum() != deln3.signum()) && (dn.abs() > 3.0 * deln2.abs()) {
            dn = 3.0 * deln2;
        }
        d[n - 1] = dn;
    }

    // interior
    for i in 1..(n - 1) {
        let del_prev = delta[i - 1];
        let del_next = delta[i];
        if del_prev == 0.0 || del_next == 0.0 || del_prev.signum() != del_next.signum() {
            d[i] = 0.0;
        } else {
            let w1 = 2.0 * h[i] + h[i - 1];
            let w2 = h[i] + 2.0 * h[i - 1];
            d[i] = (w1 + w2) / (w1 / del_prev + w2 / del_next);
        }
    }

    Ok(d)
}

fn pchip_eval(x: &[f64], y: &[f64], d: &[f64], xq: f64) -> Result<f64> {
    let n = x.len();
    if xq < x[0] || xq > x[n - 1] {
        bail!("Query out of bounds");
    }

    let i = match x.binary_search_by(|v| v.partial_cmp(&xq).unwrap()) {
        Ok(idx) => {
            if idx == n - 1 {
                return Ok(y[n - 1]);
            }
            idx
        }
        Err(ins) => ins - 1,
    };

    let h = x[i + 1] - x[i];
    let t = (xq - x[i]) / h;

    let h00 = (1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t);
    let h10 = t * (1.0 - t) * (1.0 - t);
    let h01 = t * t * (3.0 - 2.0 * t);
    let h11 = t * t * (t - 1.0);

    Ok(h00 * y[i] + h10 * h * d[i] + h01 * y[i + 1] + h11 * h * d[i + 1])
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.step <= 0 {
        bail!("--step must be > 0");
    }

    // read
    let mut rdr = Reader::from_path(&args.input)
        .with_context(|| format!("open {}", args.input.display()))?;

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

    println!("\n--- SPLINED step={}m (first {}) ---", args.step, args.print_n);
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
    chart.draw_series(
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
