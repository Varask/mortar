
use anyhow::{bail, Context, Result};
use clap::Parser;
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

/// PCHIP slope computation (Fritsch-Carlson) for monotone cubic Hermite interpolation.
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

    // Endpoints
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

    // Interior slopes
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

/// Evaluate PCHIP at xq (x in strictly increasing, within range)
fn pchip_eval(x: &[f64], y: &[f64], d: &[f64], xq: f64) -> Result<f64> {
    let n = x.len();
    if xq < x[0] || xq > x[n - 1] {
        bail!("Query out of bounds");
    }

    // Find interval i such that x[i] <= xq <= x[i+1]
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

    // Cubic Hermite basis
    let h00 = (1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t);
    let h10 = t * (1.0 - t) * (1.0 - t);
    let h01 = t * t * (3.0 - 2.0 * t);
    let h11 = t * t * (t - 1.0);

    Ok(h00 * y[i] + h10 * h * d[i] + h01 * y[i + 1] + h11 * h * d[i + 1])
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
