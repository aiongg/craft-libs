//! Offscreen render comparison against a reference run.
//!
//! Rule (a), [`Rule::Identical`]: every pixel equal. Rule (b), [`Rule::Tolerance`]: every access
//! node's bounds within 0.5 pt of the reference's; outside text boxes at most 0.5 % of pixels
//! differ, by at most 8 in any channel; text compares by its box only. The text boxes are the
//! reference's: the bounds of its text access nodes ([`TEXT_ROLES`]) and the text runs it painted
//! ([`Capture::text_runs`]), each grown by 1 px.

use craft_ui_rt::{Pos2, ReadBack, Rect, Renderer, RgbaImage, WindowId};

use super::observe::WindowSnapshot;
use super::rig::Rig;

/// Access roles whose bounds are text boxes.
pub const TEXT_ROLES: &[&str] = &["Label", "TextInput", "MultilineTextInput", "SearchInput"];

/// Which comparison a variant is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Every pixel equal.
    Identical,
    /// Rule (b) above.
    Tolerance,
}

/// One shot: the pixels, the window's snapshot and the text runs it painted.
#[derive(Clone, Debug)]
pub struct Capture {
    pub image: RgbaImage,
    pub snapshot: WindowSnapshot,
    /// `[min_x, min_y, max_x, max_y]` in logical points, window coordinates, for text without an
    /// access node. A renderer or the app reports them; empty where none are known.
    pub text_runs: Vec<[f32; 4]>,
    pub scale: f32,
}

/// Pixel comparison counts outside the ignored boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PixelStats {
    pub total: usize,
    /// Pixels that differ in any channel.
    pub differing: usize,
    pub max_delta: u8,
}

impl PixelStats {
    pub fn percent(&self) -> f64 {
        100.0 * self.differing as f64 / self.total.max(1) as f64
    }
}

fn pixel(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.size.width + x) * 4) as usize;
    [img.pixels[i], img.pixels[i + 1], img.pixels[i + 2], img.pixels[i + 3]]
}

fn ignored(boxes: &[Rect], x: u32, y: u32) -> bool {
    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
    boxes.iter().any(|r| r.contains(p))
}

/// Compares outside `ignore` (physical-pixel boxes). `None` when the sizes differ.
pub fn compare_pixels(a: &RgbaImage, b: &RgbaImage, ignore: &[Rect]) -> Option<PixelStats> {
    if a.size != b.size {
        return None;
    }
    let mut st = PixelStats::default();
    for y in 0..a.size.height {
        for x in 0..a.size.width {
            if ignored(ignore, x, y) {
                continue;
            }
            st.total += 1;
            let (p, q) = (pixel(a, x, y), pixel(b, x, y));
            let d = (0..4).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0);
            if d > 0 {
                st.differing += 1;
                st.max_delta = st.max_delta.max(d);
            }
        }
    }
    Some(st)
}

/// The text boxes of [`Rule::Tolerance`] in physical pixels, grown by 1 px.
pub fn text_boxes(reference: &Capture) -> Vec<Rect> {
    let nodes = reference.snapshot.access.iter().filter(|n| TEXT_ROLES.contains(&n.role.as_str())).map(|n| n.bounds);
    let s = reference.scale;
    nodes
        .chain(reference.text_runs.iter().copied())
        .map(|b| Rect::from_min_max(Pos2::new(b[0] * s - 1.0, b[1] * s - 1.0), Pos2::new(b[2] * s + 1.0, b[3] * s + 1.0)))
        .collect()
}

/// The largest bounds difference over nodes matched by (role, name), the k-th such node of `a`
/// with the k-th of `b`, in points; `None` when a node is missing or the counts differ.
pub fn max_bounds_delta(a: &WindowSnapshot, b: &WindowSnapshot) -> Option<f32> {
    let mut worst = 0.0f32;
    for (i, n) in a.access.iter().enumerate() {
        let same = |m: &&super::observe::AccessNode| m.role == n.role && m.name == n.name;
        let k = a.access[..i].iter().filter(same).count();
        let m = b.access.iter().filter(same).nth(k)?;
        for j in 0..4 {
            worst = worst.max((n.bounds[j] - m.bounds[j]).abs());
        }
    }
    (a.access.len() == b.access.len()).then_some(worst)
}

/// The verdict of one comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelResult {
    /// `None` when the images differ in size.
    pub stats: Option<PixelStats>,
    pub bounds_delta: Option<f32>,
    pub pass: bool,
}

/// Holds `actual` to `rule` against `reference`.
pub fn check_pixels(reference: &Capture, actual: &Capture, rule: Rule) -> PixelResult {
    let ignore = if rule == Rule::Tolerance { text_boxes(reference) } else { Vec::new() };
    let stats = compare_pixels(&reference.image, &actual.image, &ignore);
    let bounds_delta = max_bounds_delta(&reference.snapshot, &actual.snapshot);
    let pass = match (&stats, rule) {
        (Some(s), Rule::Identical) => s.differing == 0,
        (Some(s), Rule::Tolerance) => s.percent() <= 0.5 && s.max_delta <= 8 && bounds_delta.is_some_and(|d| d <= 0.5),
        (None, _) => false,
    };
    PixelResult { stats, bounds_delta, pass }
}

/// Black where equal, red where different, grey where different inside an ignored box, for a
/// comparison sheet.
pub fn diff_mask(a: &RgbaImage, b: &RgbaImage, ignore: &[Rect]) -> RgbaImage {
    let size = craft_ui_rt::SizeU::new(a.size.width.min(b.size.width), a.size.height.min(b.size.height));
    let mut m = RgbaImage::new(size);
    for y in 0..size.height {
        for x in 0..size.width {
            let i = ((y * size.width + x) * 4) as usize;
            let px: [u8; 4] = match (pixel(a, x, y) != pixel(b, x, y), ignored(ignore, x, y)) {
                (false, _) => [0, 0, 0, 255],
                (true, false) => [255, 0, 0, 255],
                (true, true) => [96, 96, 96, 255],
            };
            m.pixels[i..i + 4].copy_from_slice(&px);
        }
    }
    m
}

/// Reads back `window`'s last rendered frame through the renderer, stepping frames while the
/// read is pending (at most `tries`). The image is in physical pixels.
pub fn capture<R: Renderer>(rig: &mut Rig<R>, window: WindowId, tries: usize) -> Option<Capture> {
    let id = rig.driver.renderer.request_read_back(window, None);
    for _ in 0..=tries {
        match rig.driver.renderer.poll_read_back(id) {
            ReadBack::Ready(image) => {
                let scale = rig.driver.platform.window_state(window)?.scale;
                let snapshot = rig.snapshot().window(window)?.clone();
                return Some(Capture { image, snapshot, text_runs: Vec::new(), scale });
            }
            ReadBack::Failed => return None,
            ReadBack::Pending => {
                rig.step();
            }
        }
    }
    None
}
