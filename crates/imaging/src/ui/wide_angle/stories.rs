//! Stories of the Adaptive Wide Angle workspace (task ST-WS1): not registered. Registering them
//! (task RG2) adds `wide_angle::stories` to the crate's `register!` list and `WideAngleWorkspace`
//! to `COMPONENTS`, and regenerates the registry dump.
//!
//! The stories use `Barrel`: `MockImage` seen through a barrel distortion is the source, so the
//! straight lines of the scene curve, and `MockImage` itself is the corrected preview. The
//! geometry curves a constraint's line the same way (`arc`) and maps a source point to the
//! corrected picture by undoing it (`corrected`). The workspace covers the gallery window, as it
//! does in an app; the `open` arg (or the Overview tile's button) shows it. The constraint menu
//! opens on a right-click of a constraint. The story has no strings of its own: the workspace's
//! words and the Photo word of the story registry.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/wide-angle/constraints` | Free, Horizontal and Vertical constraints on the source; the same over the corrected preview (arg) |

use std::sync::{Arc, Mutex};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, claim, dialog_closed, dialog_shown};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui};

use super::{ConstraintOrientation, WideAngleConstraint, WideAngleGeometry, WideAngleOpen, WideAngleSession, WideAngleSource, WideAngleWorkspace};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["WideAngleWorkspace"];

/// The constraints, as shares of the picture's width and height: a Horizontal one near the top, a
/// Vertical one near the left edge and a Free one across the lower part. They sit near the edges,
/// where the distortion bends a line most.
const CONSTRAINTS: [([f64; 2], [f64; 2], ConstraintOrientation); 3] = [
    ([0.12, 0.14], [0.88, 0.14], ConstraintOrientation::Horizontal),
    ([0.10, 0.20], [0.10, 0.86], ConstraintOrientation::Vertical),
    ([0.28, 0.88], [0.84, 0.74], ConstraintOrientation::Free),
];
/// Newton-free inversion of the distortion: fixed-point rounds.
const INVERSE_ROUNDS: usize = 12;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/wide-angle/constraints",
            Level::Patterns,
            "WideAngleWorkspace",
            "Constraints",
            ConstraintsArgs { open: true, preview: PreviewArg::Source, curvature: 30.0 },
            show_constraints
        )
        .on(Surface::Own),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PreviewArg {
    #[choice(name = "Source")]
    Source,
    #[choice(name = "Corrected")]
    Corrected,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ConstraintsArgs {
    /// Whether the workspace is open.
    open: bool,
    /// What the canvas shows: the source with the curved constraint lines, or the corrected
    /// preview with the straightened ones (the Preview check box turns it off again).
    preview: PreviewArg,
    /// How strongly the lens bends the lines, in percent.
    #[arg(range = 0.0..=100.0)]
    curvature: f64,
}

// --- the fake lens ---

/// A barrel distortion about the picture's middle, in units of half the width: a point at radius
/// `r` of the straight picture lies at `r / (1 + k r²)` in the source.
#[derive(Clone, Copy)]
struct Lens {
    size: [u32; 2],
    k: f64,
}

impl Lens {
    fn centre(&self) -> (f64, f64) {
        (f64::from(self.size[0]) / 2.0, f64::from(self.size[1]) / 2.0)
    }

    /// Where the straight picture's point lands in the source.
    fn distort(&self, p: Point) -> Point {
        let (cx, cy) = self.centre();
        let (x, y) = ((p.x - cx) / cx, (p.y - cy) / cx);
        let scale = 1.0 / (1.0 + self.k * (x * x + y * y));
        Point::new(cx + x * scale * cx, cy + y * scale * cx)
    }

    /// The straight picture's point that lands on the source's `p`.
    fn undistort(&self, p: Point) -> Point {
        let (cx, cy) = self.centre();
        let (x, y) = ((p.x - cx) / cx, (p.y - cy) / cx);
        let (mut u, mut v) = (x, y);
        for _ in 0..INVERSE_ROUNDS {
            let scale = 1.0 + self.k * (u * u + v * v);
            (u, v) = (x * scale, y * scale);
        }
        Point::new(cx + u * cx, cy + v * cx)
    }
}

/// The app's geometry: the lens.
struct Geometry(Lens);

impl WideAngleGeometry for Geometry {
    fn arc(&self, _session: &WideAngleSession, a: [f64; 2], b: [f64; 2], segments: usize) -> Vec<Point> {
        // The line is straight in the scene: undo the lens at its ends, join them, bend each point.
        let (a, b) = (self.0.undistort(Point::new(a[0], a[1])), self.0.undistort(Point::new(b[0], b[1])));
        (0..=segments)
            .map(|i| {
                let t = i as f64 / segments.max(1) as f64;
                self.0.distort(a.lerp(b, t))
            })
            .collect()
    }

    fn corrected(&self, _session: &WideAngleSession, at: Point) -> Option<Point> {
        Some(self.0.undistort(at))
    }
}

/// `MockImage` as the lens sees it.
struct Barrel {
    image: MockImage,
    lens: Lens,
}

impl RasterSource for Barrel {
    fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    fn revision(&self) -> u64 {
        // One revision per strength in percent.
        self.image.revision() + (self.lens.k * 100.0).round() as u64
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let factor = f64::from(factor.max(1));
        let [width, height] = self.image.size().map(f64::from);
        for row in 0..rows {
            let y = f64::from(region.y0) + (row as f64 + 0.5) * factor;
            for column in 0..columns {
                let x = f64::from(region.x0) + (column as f64 + 0.5) * factor;
                let from = self.lens.undistort(Point::new(x, y));
                let [r, g, b, a] = self.image.pixel(from.x.clamp(0.0, width - 1.0), from.y.clamp(0.0, height - 1.0));
                out.pixels[row * columns + column] = Color32::from_rgba_unmultiplied(r, g, b, a);
            }
        }
    }
}

/// What the workspace shows: the source, and the corrected picture once the story says it is ready.
struct StorySource {
    source: Barrel,
    corrected: Option<MockImage>,
}

impl WideAngleSource for StorySource {
    fn source(&self) -> &dyn RasterSource {
        &self.source
    }

    fn corrected(&self) -> Option<&dyn RasterSource> {
        self.corrected.as_ref().map(|image| image as &dyn RasterSource)
    }
}

// --- the kept workspace ---

struct Kept {
    workspace: WideAngleWorkspace,
}

type Slot = Arc<Mutex<Option<Kept>>>;

/// The start: Auto, and the three constraints in source pixels.
fn start(size: [u32; 2]) -> WideAngleSession {
    let [width, height] = size.map(f64::from);
    let constraints = CONSTRAINTS
        .iter()
        .map(|&(a, b, orientation)| WideAngleConstraint { a: [a[0] * width, a[1] * height], b: [b[0] * width, b[1] * height], orientation })
        .collect();
    WideAngleSession { constraints, ..WideAngleSession::default() }
}

fn show_constraints(ui: &mut Ui, args: &mut ConstraintsArgs) {
    let id = ui.make_persistent_id("craft_imaging::wide_angle::story");
    let slot: Slot = ui.data_mut(|d| d.get_temp_mut_or_insert_with::<Slot>(id, Slot::default).clone());
    let Ok(mut kept) = slot.lock() else { return };
    if !(dialog_shown(ui, args.open) && claim(ui.ctx(), "wide_angle", ui.scope_id())) {
        *kept = None;
        return;
    }
    let image = MockImage::default();
    if kept.is_none() {
        let subject = StoryLabels::of(ui).photo;
        *kept = Some(Kept { workspace: WideAngleWorkspace::new(WideAngleOpen::new(image.size(), subject).session(start(image.size()))) });
    }
    let Some(kept) = kept.as_mut() else { return };
    let lens = Lens { size: image.size(), k: args.curvature / 100.0 };
    let source = StorySource { source: Barrel { image, lens }, corrected: (args.preview == PreviewArg::Corrected).then_some(image) };
    let out = kept.workspace.show(ui.ctx(), &Geometry(lens), &source);
    // The fake solve is instant: every ticket is answered in the pass that asked it.
    if let Some(ticket) = out.preview_wanted {
        kept.workspace.preview_ready(ticket);
    }
    if out.outcome.is_some_and(Outcome::closes) {
        dialog_closed(ui, &mut args.open);
    }
}
