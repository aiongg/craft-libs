//! The ICC binary parser: header, tag table, curves, lookup tables and text.

use super::*;

// ---------------------------------------------------------------- reader

pub(crate) struct Reader<'a> {
    pub(crate) b: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn slice(&self, off: usize, len: usize) -> Result<&'a [u8], CmsError> {
        off.checked_add(len).and_then(|end| self.b.get(off..end)).ok_or(CmsError::Truncated)
    }
    fn u8(&self, off: usize) -> Result<u8, CmsError> {
        self.b.get(off).copied().ok_or(CmsError::Truncated)
    }
    pub(crate) fn u16(&self, off: usize) -> Result<u16, CmsError> {
        let s = self.slice(off, 2)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }
    pub(crate) fn u32(&self, off: usize) -> Result<u32, CmsError> {
        let s = self.slice(off, 4)?;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn s15f16(&self, off: usize) -> Result<f64, CmsError> {
        Ok(self.u32(off)? as i32 as f64 / 65536.0)
    }
    fn xyz(&self, off: usize) -> Result<[f64; 3], CmsError> {
        Ok([self.s15f16(off)?, self.s15f16(off + 4)?, self.s15f16(off + 8)?])
    }
}

fn sig(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

/// Upper bound on CLUT nodes × outputs we accept, to reject hostile files.
const MAX_CLUT_VALUES: usize = crate::clut::MAX_VALUES;

pub(super) fn parse_profile(bytes: &[u8]) -> Result<Profile, CmsError> {
    let r = Reader { b: bytes };
    if bytes.len() < 132 {
        return Err(CmsError::Truncated);
    }
    if r.slice(36, 4)? != b"acsp" {
        return Err(CmsError::BadSignature);
    }
    let declared = r.u32(0)? as usize;
    // Some writers pad or truncate; trust the smaller of the two.
    let end = if declared >= 132 && declared <= bytes.len() { declared } else { bytes.len() };
    let r = Reader { b: &bytes[..end] };
    let version = (r.u8(8)?, r.u8(9)?);
    let class = ProfileClass::from_sig(r.u32(12)?);
    let color_space = ColorSpace::from_sig(r.u32(16)?);
    let pcs_sig = r.u32(20)?;
    let pcs = match &pcs_sig.to_be_bytes() {
        b"Lab " => Pcs::Lab,
        _ => Pcs::Xyz,
    };
    let rendering_intent = Intent::from_u32(r.u32(64)? & 0xFFFF).unwrap_or(Intent::Perceptual);
    let count = r.u32(128)? as usize;
    if count > 1024 {
        return Err(CmsError::Invalid(format!("{count} tags")));
    }
    let mut tags: Vec<(u32, usize, usize)> = Vec::with_capacity(count);
    for i in 0..count {
        let o = 132 + i * 12;
        let (s, off, len) = (r.u32(o)?, r.u32(o + 4)? as usize, r.u32(o + 8)? as usize);
        tags.push((s, off, len));
    }
    let find = |s: &[u8; 4]| tags.iter().find(|t| t.0 == sig(s)).map(|t| (t.1, t.2));
    let tag_data = |s: &[u8; 4]| -> Result<Option<&[u8]>, CmsError> {
        match find(s) {
            Some((off, len)) => Ok(Some(r.slice(off, len)?)),
            None => Ok(None),
        }
    };

    let xyz_tag = |s: &[u8; 4]| -> Result<Option<[f64; 3]>, CmsError> {
        match tag_data(s)? {
            Some(d) if d.len() >= 20 && &d[..4] == b"XYZ " => Ok(Some(Reader { b: d }.xyz(8)?)),
            Some(_) => Err(CmsError::Invalid(format!("tag {} is not XYZType", String::from_utf8_lossy(s)))),
            None => Ok(None),
        }
    };
    let curve_tag = |s: &[u8; 4]| -> Result<Option<Curve>, CmsError> {
        match tag_data(s)? {
            Some(d) => Ok(Some(parse_curve(d)?.0)),
            None => Ok(None),
        }
    };

    let white_point = xyz_tag(b"wtpt")?.unwrap_or(math::D50);
    let black_point = xyz_tag(b"bkpt").ok().flatten();
    let chad = match tag_data(b"chad")? {
        Some(d) if d.len() >= 44 && &d[..4] == b"sf32" => {
            let rd = Reader { b: d };
            let mut m = [[0.0; 3]; 3];
            for (i, row) in m.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    *v = rd.s15f16(8 + (i * 3 + j) * 4)?;
                }
            }
            Some(m)
        }
        _ => None,
    };
    let matrix = match (xyz_tag(b"rXYZ")?, xyz_tag(b"gXYZ")?, xyz_tag(b"bXYZ")?) {
        (Some(rr), Some(g), Some(b)) => Some([[rr[0], g[0], b[0]], [rr[1], g[1], b[1]], [rr[2], g[2], b[2]]]),
        _ => None,
    };
    let trc = match (curve_tag(b"rTRC")?, curve_tag(b"gTRC")?, curve_tag(b"bTRC")?) {
        (Some(a), Some(b), Some(c)) => Some([a, b, c]),
        _ => None,
    };
    let gray_trc = curve_tag(b"kTRC")?;
    let lut = |s: &[u8; 4], a2b: bool| -> Result<Option<Lut>, CmsError> {
        match tag_data(s)? {
            Some(d) => parse_lut(d, a2b).map(Some),
            None => Ok(None),
        }
    };
    let a2b = [lut(b"A2B0", true)?, lut(b"A2B1", true)?, lut(b"A2B2", true)?];
    let b2a = [lut(b"B2A0", false)?, lut(b"B2A1", false)?, lut(b"B2A2", false)?];
    let description = tag_data(b"desc")?.map(parse_text).unwrap_or_default();
    let copyright = tag_data(b"cprt")?.map(parse_text).unwrap_or_default();

    let p = Profile {
        version,
        class,
        color_space,
        pcs,
        rendering_intent,
        description,
        copyright,
        white_point,
        black_point,
        chad,
        matrix,
        trc,
        gray_trc,
        a2b,
        b2a,
        bytes: None,
        hash: Default::default(),
    };
    // Validate channel counts of LUTs against the header. A DeviceLink connects two device
    // spaces: its header PCS field holds the output colour space (ICC.1:2010 §7.2.7), so a
    // CMYK→CMYK link has a 4→4 AToB0.
    let dev = p.color_space.channels();
    let conn = if p.class == ProfileClass::DeviceLink { ColorSpace::from_sig(pcs_sig).channels() } else { 3 };
    for l in p.a2b.iter().flatten() {
        if dev != 0 && l.inputs != dev || conn != 0 && l.outputs != conn {
            return Err(CmsError::Invalid(format!("AToB tag has {}→{} channels for a {:?} profile", l.inputs, l.outputs, p.color_space)));
        }
    }
    for l in p.b2a.iter().flatten() {
        if conn != 0 && l.inputs != conn || dev != 0 && l.outputs != dev {
            return Err(CmsError::Invalid(format!("BToA tag has {}→{} channels for a {:?} profile", l.inputs, l.outputs, p.color_space)));
        }
    }
    Ok(p)
}

/// Parses a `curv` or `para` element; returns the curve and the element's byte length
/// (unpadded).
fn parse_curve(d: &[u8]) -> Result<(Curve, usize), CmsError> {
    let r = Reader { b: d };
    match r.slice(0, 4)? {
        b"curv" => {
            let n = r.u32(8)? as usize;
            // Bounded before it is used: `n * 2` is computed next, and a count straight from the
            // file overflows a 32-bit `usize` (wasm32) with overflow checks on, panicking before
            // this guard could reject it.
            if n > 1 << 20 {
                return Err(CmsError::Invalid("curve too long".into()));
            }
            let len = 12 + n * 2;
            match n {
                0 => Ok((Curve::Identity, len)),
                1 => Ok((Curve::Gamma(r.u16(12)? as f64 / 256.0), len)),
                _ => {
                    let s = r.slice(12, n * 2)?;
                    let t = s.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0).collect();
                    Ok((Curve::Table(t), len))
                }
            }
        }
        b"para" => {
            let kind = r.u16(8)?;
            if kind > 4 {
                return Err(CmsError::Unsupported(format!("parametric curve type {kind}")));
            }
            let n = Curve::param_count(kind);
            let mut p = [0.0f64; 7];
            for (i, v) in p.iter_mut().enumerate().take(n) {
                *v = r.s15f16(12 + i * 4)?;
            }
            let len = 12 + n * 4;
            let c = if kind == 0 { Curve::Gamma(p[0]) } else { Curve::Parametric { kind, p } };
            Ok((c, len))
        }
        other => Err(CmsError::Unsupported(format!("curve type {:?}", String::from_utf8_lossy(other)))),
    }
}

fn parse_curves_at(d: &[u8], off: usize, n: usize) -> Result<Vec<Curve>, CmsError> {
    let mut out = Vec::with_capacity(n);
    let mut o = off;
    for _ in 0..n {
        let sub = d.get(o..).ok_or(CmsError::Truncated)?;
        let (c, len) = parse_curve(sub)?;
        out.push(c);
        o += (len + 3) & !3;
    }
    Ok(out)
}

fn parse_lut(d: &[u8], a2b: bool) -> Result<Lut, CmsError> {
    let r = Reader { b: d };
    let ty = r.slice(0, 4)?;
    match ty {
        b"mft1" | b"mft2" => {
            let wide = ty == b"mft2";
            let (i, o, g) = (r.u8(8)? as usize, r.u8(9)? as usize, r.u8(10)? as usize);
            if !(1..=15).contains(&i) || !(1..=15).contains(&o) || g == 0 {
                return Err(CmsError::Invalid("bad lut dimensions".into()));
            }
            let mut m = [[0.0; 3]; 3];
            for (a, row) in m.iter_mut().enumerate() {
                for (b, v) in row.iter_mut().enumerate() {
                    *v = r.s15f16(12 + (a * 3 + b) * 4)?;
                }
            }
            let (n_in, n_out, mut off) = if wide { (r.u16(48)? as usize, r.u16(50)? as usize, 52) } else { (256, 256, 48) };
            if wide && (!(2..=4096).contains(&n_in) || !(2..=4096).contains(&n_out)) {
                return Err(CmsError::Invalid("bad lut16 table size".into()));
            }
            let bps = if wide { 2 } else { 1 };
            let read_tables = |off: &mut usize, count: usize, len: usize| -> Result<Vec<Curve>, CmsError> {
                let mut v = Vec::with_capacity(count);
                for _ in 0..count {
                    let s = r.slice(*off, len * bps)?;
                    let t: Vec<f32> = if wide {
                        s.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0).collect()
                    } else {
                        s.iter().map(|b| *b as f32 / 255.0).collect()
                    };
                    *off += len * bps;
                    v.push(Curve::Table(t));
                }
                Ok(v)
            };
            let in_curves = read_tables(&mut off, i, n_in)?;
            let nodes = (g as u64).checked_pow(i as u32).unwrap_or(u64::MAX);
            if nodes as u128 * o as u128 > MAX_CLUT_VALUES as u128 {
                return Err(CmsError::Invalid("CLUT too large".into()));
            }
            let total = nodes as usize * o;
            let s = r.slice(off, total * bps)?;
            let data: Vec<f32> = if wide {
                s.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0).collect()
            } else {
                s.iter().map(|b| *b as f32 / 255.0).collect()
            };
            off += total * bps;
            let out_curves = read_tables(&mut off, o, n_out)?;
            let mut stages = Vec::new();
            // The matrix only applies when the input is XYZ (BToA with an XYZ PCS).
            let ident = m.iter().enumerate().all(|(a, row)| row.iter().enumerate().all(|(b, v)| (v - if a == b { 1.0 } else { 0.0 }).abs() < 1e-6));
            if !a2b && i == 3 && !ident {
                // Only meaningful for an XYZ PCS; the caller decides (we cannot know the PCS here),
                // so keep it: for Lab PCS writers are required to store the identity.
                stages.push(Stage::matrix3(&m));
            }
            stages.push(Stage::Curves(in_curves));
            stages.push(Stage::Clut(Clut::new(vec![g; i], o, data)?));
            stages.push(Stage::Curves(out_curves));
            Ok(Lut { kind: if wide { LutKind::Lut16 } else { LutKind::Lut8 }, inputs: i, outputs: o, stages })
        }
        b"mAB " | b"mBA " => {
            let (i, o) = (r.u8(8)? as usize, r.u8(9)? as usize);
            if !(1..=15).contains(&i) || !(1..=15).contains(&o) {
                return Err(CmsError::Invalid("bad lutAB dimensions".into()));
            }
            let (ob, om, omc, oc, oa) = (r.u32(12)? as usize, r.u32(16)? as usize, r.u32(20)? as usize, r.u32(24)? as usize, r.u32(28)? as usize);
            let matrix = |off: usize| -> Result<Stage, CmsError> {
                let mut m = vec![0.0; 9];
                for (k, v) in m.iter_mut().enumerate() {
                    *v = r.s15f16(off + k * 4)?;
                }
                let offset = vec![r.s15f16(off + 36)?, r.s15f16(off + 40)?, r.s15f16(off + 44)?];
                Ok(Stage::Matrix { rows: 3, cols: 3, m, offset })
            };
            let clut = |off: usize, cin: usize, cout: usize| -> Result<Stage, CmsError> {
                let mut grid = Vec::with_capacity(cin);
                for k in 0..cin {
                    let g = r.u8(off + k)? as usize;
                    if g < 1 {
                        return Err(CmsError::Invalid("zero grid points".into()));
                    }
                    grid.push(g);
                }
                let prec = r.u8(off + 16)? as usize;
                let nodes: u128 = grid.iter().map(|g| *g as u128).product();
                if nodes * cout as u128 > MAX_CLUT_VALUES as u128 {
                    return Err(CmsError::Invalid("CLUT too large".into()));
                }
                let total = nodes as usize * cout;
                let data: Vec<f32> = match prec {
                    1 => r.slice(off + 20, total)?.iter().map(|b| *b as f32 / 255.0).collect(),
                    2 => r.slice(off + 20, total * 2)?.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0).collect(),
                    _ => return Err(CmsError::Invalid(format!("CLUT precision {prec}"))),
                };
                Ok(Stage::Clut(Clut::new(grid, cout, data)?))
            };
            let mut stages = Vec::new();
            if ty == b"mAB " {
                // A curves → CLUT → M curves → matrix → B curves.
                if oc != 0 {
                    if oa == 0 {
                        return Err(CmsError::Invalid("lutAtoB with CLUT but no A curves".into()));
                    }
                    stages.push(Stage::Curves(parse_curves_at(d, oa, i)?));
                    stages.push(clut(oc, i, o)?);
                } else if i != o {
                    return Err(CmsError::Invalid("lutAtoB without CLUT changes channel count".into()));
                }
                if om != 0 {
                    stages.push(Stage::Curves(parse_curves_at(d, omc, o)?));
                    stages.push(matrix(om)?);
                }
                if ob == 0 {
                    return Err(CmsError::Invalid("lutAtoB without B curves".into()));
                }
                stages.push(Stage::Curves(parse_curves_at(d, ob, o)?));
            } else {
                // B curves → matrix → M curves → CLUT → A curves.
                if ob == 0 {
                    return Err(CmsError::Invalid("lutBtoA without B curves".into()));
                }
                stages.push(Stage::Curves(parse_curves_at(d, ob, i)?));
                if om != 0 {
                    stages.push(matrix(om)?);
                    stages.push(Stage::Curves(parse_curves_at(d, omc, i)?));
                }
                if oc != 0 {
                    if oa == 0 {
                        return Err(CmsError::Invalid("lutBtoA with CLUT but no A curves".into()));
                    }
                    stages.push(clut(oc, i, o)?);
                    stages.push(Stage::Curves(parse_curves_at(d, oa, o)?));
                } else if i != o {
                    return Err(CmsError::Invalid("lutBtoA without CLUT changes channel count".into()));
                }
            }
            Ok(Lut { kind: LutKind::Ab, inputs: i, outputs: o, stages })
        }
        other => Err(CmsError::Unsupported(format!("LUT type {:?}", String::from_utf8_lossy(other)))),
    }
}

/// Text from `desc` (textDescriptionType), `mluc` (English or first record) or `text`.
fn parse_text(d: &[u8]) -> String {
    let r = Reader { b: d };
    let Ok(ty) = r.slice(0, 4) else { return String::new() };
    let ascii = |s: &[u8]| String::from_utf8_lossy(s.split(|b| *b == 0).next().unwrap_or(&[])).trim().to_string();
    match ty {
        b"desc" => {
            let n = r.u32(8).unwrap_or(0) as usize;
            r.slice(12, n).map(ascii).unwrap_or_default()
        }
        b"text" => ascii(d.get(8..).unwrap_or(&[])),
        b"mluc" => {
            let (Ok(n), Ok(size)) = (r.u32(8), r.u32(12)) else { return String::new() };
            let mut best: Option<(usize, usize)> = None;
            for k in 0..(n as usize).min(256) {
                // Checked: the record size comes from the file, and `16 + k * size` overflows a
                // 32-bit `usize` at k = 1 for a size near u32::MAX (wrapping would read another
                // record's bytes as the language, length and offset).
                let Some(o) = k.checked_mul(size as usize).and_then(|k| 16usize.checked_add(k)) else { break };
                // Complete record read: `slice` does its own checked math, and the fields come
                // from local offsets inside the 12-byte record, so no `o + 4` / `o + 8` is ever
                // computed on a file-derived offset. Reading the fields eagerly at `o`, `o + 4`,
                // `o + 8` evaluates every addition first: with `o` fitting but `o + 8` overflowing
                // a 32-bit `usize` (wasm32, overflow checks on), the addition panics before the
                // failed read could break the loop.
                let Ok(rec) = r.slice(o, 12) else { break };
                let rr = Reader { b: rec };
                let (Ok(lang), Ok(len), Ok(off)) = (rr.u16(0), rr.u32(4), rr.u32(8)) else { break };
                if best.is_none() || lang == u16::from_be_bytes(*b"en") {
                    best = Some((off as usize, len as usize));
                    if lang == u16::from_be_bytes(*b"en") {
                        break;
                    }
                }
            }
            let Some((off, len)) = best else { return String::new() };
            let Ok(s) = r.slice(off, len) else { return String::new() };
            let units: Vec<u16> = s.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units).trim_end_matches('\0').trim().to_string()
        }
        _ => String::new(),
    }
}
