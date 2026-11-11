//! Reconstruction: inverse DCT, upsampling and colour conversion to pixels.

use super::parse::*;
use super::*;

/// `T[x][u] = C(u)/2 · cos((2x+1)uπ/16)`: one dimension of the inverse DCT (A.3.3).
pub(super) fn idct_basis() -> [[f32; 8]; 8] {
    let mut t = [[0f32; 8]; 8];
    for (x, row) in t.iter_mut().enumerate() {
        for (u, v) in row.iter_mut().enumerate() {
            let c = if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
            *v = c / 2.0 * (((2 * x + 1) * u) as f32 * std::f32::consts::PI / 16.0).cos();
        }
    }
    t
}

/// The component's samples (padded to whole blocks, row stride `bw * 8`), level-shifted and
/// clamped to `0..=max`.
pub(super) fn reconstruct(comp: &Component, q: &[u16; 64], basis: &[[f32; 8]; 8], max: f32) -> Vec<f32> {
    let stride = comp.bw * 8;
    let mut plane = vec![0f32; stride * comp.bh * 8];
    let shift = (max + 1.0) / 2.0;
    for (n, blk) in comp.coefs.iter().enumerate() {
        let (bx, by) = (n % comp.bw, n / comp.bw);
        if blk[1..].iter().all(|&c| c == 0) {
            // Only DC: every sample is T[y][0] · T[x][0] · F[0][0], the same products the full
            // transform below sums (its other terms are zero), so the result is identical.
            let t = basis[0][0];
            let v = (t * (t * (blk[0] as f32 * f32::from(q[0]))) + shift).clamp(0.0, max);
            for y in 0..8 {
                let row = (by * 8 + y) * stride + bx * 8;
                if let Some(out) = plane.get_mut(row..row + 8) {
                    out.fill(v);
                }
            }
            continue;
        }
        let mut f = [0f32; 64];
        for (k, &c) in blk.iter().enumerate() {
            f[ZIGZAG[k]] = c as f32 * f32::from(q[k]);
        }
        // Rows: r[v][x] = Σu T[x][u] F[v][u]; then columns: s[y][x] = Σv T[y][v] r[v][x].
        let mut r = [0f32; 64];
        for v in 0..8 {
            for x in 0..8 {
                r[v * 8 + x] = (0..8).map(|u| basis[x][u] * f[v * 8 + u]).sum();
            }
        }
        for (y, ty) in basis.iter().enumerate() {
            let row = (by * 8 + y) * stride + bx * 8;
            if let Some(out) = plane.get_mut(row..row + 8) {
                for (x, o) in out.iter_mut().enumerate() {
                    let s: f32 = ty.iter().enumerate().map(|(v, t)| t * r[v * 8 + x]).sum();
                    *o = (s + shift).clamp(0.0, max);
                }
            }
        }
    }
    plane
}

/// For each output coordinate: the two neighboring component samples and the weight of the
/// second (centered linear interpolation of a subsampled component).
pub(super) fn upsample_map(out: usize, samples: usize, factor: usize, fmax: usize) -> Vec<(usize, usize, f32)> {
    let last = samples.saturating_sub(1);
    (0..out)
        .map(|x| {
            let s = ((x as f32 + 0.5) * factor as f32 / fmax as f32 - 0.5).max(0.0);
            let x0 = (s.floor() as usize).min(last);
            ((x0), (x0 + 1).min(last), (s - x0 as f32).clamp(0.0, 1.0))
        })
        .collect()
}

/// Turn the coefficients into interleaved output samples.
pub(super) fn render(frame: Frame, tables: &Tables, adobe_transform: Option<u8>) -> Result<(u8, Samples), Error> {
    let max = if frame.precision > 8 { 4095.0 } else { 255.0 };
    let basis = idct_basis();
    let (w, h) = (frame.width, frame.height);
    let mut planes = Vec::with_capacity(frame.comps.len());
    for comp in &frame.comps {
        // A component no scan reached has only zero coefficients; any defined table will do.
        let q = comp.quant.or_else(|| tables.quant.get(comp.tq).copied().flatten());
        let q = q.ok_or_else(|| err("component uses an undefined quantization table"))?;
        let plane = reconstruct(comp, &q, &basis, max);
        let xs = upsample_map(w, comp.cw, comp.h, frame.hmax);
        let ys = upsample_map(h, comp.ch, comp.v, frame.vmax);
        planes.push((plane, comp.bw * 8, xs, ys));
    }
    let nc = planes.len();
    let sixteen = frame.precision > 8;
    let rgb_ids = frame.comps.iter().map(|c| c.id).eq(*b"RGB");
    let ycc = match nc {
        3 => adobe_transform != Some(0) && !rgb_ids,
        4 => adobe_transform == Some(2),
        _ => false,
    };
    let (mut wide, mut narrow) = (Vec::new(), Vec::new());
    if sixteen {
        wide.reserve_exact(w * h * nc);
    } else {
        narrow.reserve_exact(w * h * nc);
    }
    let mut px = [0f32; 4];
    let center = (max + 1.0) / 2.0;
    for y in 0..h {
        for x in 0..w {
            for (c, (plane, stride, xs, ys)) in planes.iter().enumerate() {
                let (Some(&(x0, x1, fx)), Some(&(y0, y1, fy))) = (xs.get(x), ys.get(y)) else {
                    return Err(err("sample map out of range"));
                };
                let at = |xx: usize, yy: usize| plane.get(yy * stride + xx).copied().unwrap_or(0.0);
                let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * fx;
                let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * fx;
                if let Some(p) = px.get_mut(c) {
                    *p = top + (bottom - top) * fy;
                }
            }
            if ycc {
                // YCbCr → RGB (JFIF / T.871); for YCCK the result is the inverted CMY of Adobe's
                // convention, i.e. the ink amounts.
                let (yy, cb, cr) = (px[0], px[1] - center, px[2] - center);
                px[0] = yy + 1.402 * cr;
                px[1] = yy - 0.344_136 * cb - 0.714_136 * cr;
                px[2] = yy + 1.772 * cb;
                if nc == 4 {
                    px[3] = max - px[3];
                }
            } else if nc == 4 {
                // Adobe-style CMYK is stored inverted (max = no ink).
                for p in &mut px {
                    *p = max - *p;
                }
            }
            for &p in px.iter().take(nc) {
                let p = p.clamp(0.0, max);
                if sixteen {
                    wide.push((p * 65535.0 / max).round() as u16);
                } else {
                    narrow.push(p.round() as u8);
                }
            }
        }
    }
    let samples = if sixteen { Samples::U16(wide) } else { Samples::U8(narrow) };
    Ok((nc as u8, samples))
}
