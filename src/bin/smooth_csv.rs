use anyhow::{bail, Context, Result};
use clap::Parser;
use mortar::pchip::{pchip_eval, pchip_slopes};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]


struct Args {
    /// Input CSV path
    input: PathBuf,
    /// Step in meters for resampling (default 1m)
    #[arg(long, default_value_t = 1)]
    step: i32,
    /// Output CSV path (default: <stem>_smoothed_<step>m.csv)
    #[arg(long)]
    out: Option<PathBuf>,
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

#[derive(Serialize)]
struct OutRow {
    range_m: i32,
    elev_mil: f64,
}

fn default_out_path(input: &Path, step: i32) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}_smoothed_{}m.csv", step))
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.step <= 0 {
        bail!("--step must be > 0");
    }

    // Read CSV
    let mut rdr = csv::Reader::from_path(&args.input)
        .with_context(|| format!("Failed to open {}", args.input.display()))?;

    let mut pts: Vec<(f64, f64)> = Vec::new();
    for rec in rdr.deserialize::<InRow>() {
        let r = rec?;
        if r.range_m.is_finite() && r.elev_mil.is_finite() {
            pts.push((r.range_m, r.elev_mil));
        }
    }

    if pts.len() < 2 {
        bail!("Not enough valid rows in {}", args.input.display());
    }

    // Sort and unique by range (keep last if duplicates)
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut x: Vec<f64> = Vec::new();
    let mut y: Vec<f64> = Vec::new();
    for (rx, ry) in pts {
        if x.last().copied() == Some(rx) {
            *y.last_mut().unwrap() = ry;
        } else {
            x.push(rx);
            y.push(ry);
        }
    }

    // Build slopes
    let d = pchip_slopes(&x, &y)?;

    let x_min = x[0].ceil() as i32;
    let x_max = x[x.len() - 1].floor() as i32;

    let mut out_rows: Vec<OutRow> = Vec::new();
    let mut r = x_min;
    while r <= x_max {
        let elev = pchip_eval(&x, &y, &d, r as f64)?;
        out_rows.push(OutRow {
            range_m: r,
            elev_mil: elev,
        });
        r += args.step;
    }

    let out_path = args.out.unwrap_or_else(|| default_out_path(&args.input, args.step));
    let mut wtr = csv::Writer::from_writer(
        File::create(&out_path).with_context(|| format!("Failed to create {}", out_path.display()))?,
    );
    for row in out_rows {
        wtr.serialize(row)?;
    }
    wtr.flush()?;

    println!("Saved: {}", out_path.display());
    Ok(())
}
