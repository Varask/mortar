//! Interpolation PCHIP (Piecewise Cubic Hermite Interpolating Polynomial).
//!
//! Implémentation de l'algorithme de Fritsch-Carlson pour une interpolation
//! cubique monotone préservant la forme des données.

use anyhow::{bail, Result};

/// Calcule les pentes PCHIP (Fritsch-Carlson) pour une interpolation cubique monotone.
///
/// # Arguments
///
/// * `x` - Abscisses strictement croissantes
/// * `y` - Ordonnées correspondantes
///
/// # Erreurs
///
/// Retourne une erreur si moins de 2 points ou si `x` n'est pas strictement croissant.
pub fn pchip_slopes(x: &[f64], y: &[f64]) -> Result<Vec<f64>> {
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

/// Évalue l'interpolation PCHIP en un point donné.
///
/// # Arguments
///
/// * `x` - Abscisses strictement croissantes
/// * `y` - Ordonnées correspondantes
/// * `d` - Pentes calculées par [`pchip_slopes`]
/// * `xq` - Point d'évaluation
///
/// # Erreurs
///
/// Retourne une erreur si `xq` est hors des bornes de `x`.
pub fn pchip_eval(x: &[f64], y: &[f64], d: &[f64], xq: f64) -> Result<f64> {
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

    // Cubic Hermite basis
    let h00 = (1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t);
    let h10 = t * (1.0 - t) * (1.0 - t);
    let h01 = t * t * (3.0 - 2.0 * t);
    let h11 = t * t * (t - 1.0);

    Ok(h00 * y[i] + h10 * h * d[i] + h01 * y[i + 1] + h11 * h * d[i + 1])
}
