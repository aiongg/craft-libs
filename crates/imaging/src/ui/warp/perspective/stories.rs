//! Stories of Perspective Warp (task ST-TR2): not registered. Registering them (task RG2) adds
//! `warp::perspective::stories` to the crate's `register!` list, `PerspectiveWarp` and
//! `PerspectiveBar` to `COMPONENTS` and [`PerspectiveStoryLabels`] to `WORDS`, and regenerates the
//! registry dump.
//!
//! The layer is `MockImage` on the shared stage of the transform preview stories
//! (`transform_preview::stories::Document`), drawn by the real `TransformPreview` through
//! `PreviewMap::function` over a fake map: each plane's own homography from its source corners to
//! its target corners. Every case keeps its own session and applies the edits the overlay or the
//! bar reports. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/perspective-warp/planes` | Layout with two linked planes; Warp with the picture following; a picked corner with keyboard focus (the story asks for the group's focus once; the ring shows once the keyboard is in use) |
//! | `composites/perspective-warp/bar` | Layout (count "2 planes", Remove Planes) and Warp (Straighten, Level, Both); the plane count is an arg |

use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::quad::Homography;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::typed_story;
use egui::{IdSalt, Ui};

use super::{PerspectiveBar, PerspectiveBarOutput, PerspectiveEdit, PerspectiveSession, PerspectiveWarp, Plane, PlaneMode};
use crate::ui::transform_preview::stories::{Document, draw};
use crate::ui::transform_preview::{PreviewMap, PreviewQuality};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PerspectiveWarp", "PerspectiveBar"];

craft_ui::words! {
    /// The captions of the Perspective Warp stories, English by default.
    #[non_exhaustive]
    pub(crate) struct PerspectiveStoryLabels {
        layout: "perspectiveStory.layout" = "Layout, two linked planes";
        warp: "perspectiveStory.warp" = "Warp, the picture follows";
        picked: "perspectiveStory.picked" = "A picked corner";
        bar_layout: "perspectiveStory.barLayout" = "Layout";
        bar_warp: "perspectiveStory.barWarp" = "Warp";
    }
}

/// The two stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/perspective-warp/planes",
            Level::Primitives,
            "PerspectiveWarp",
            "Planes",
            PlanesArgs { width: Document::DEFAULT_SIZE[0], height: Document::DEFAULT_SIZE[1], pull: 8.0 },
            show_planes
        )
        .on(Surface::Canvas),
        typed_story!("composites/perspective-warp/bar", Level::Composites, "PerspectiveBar", "Bar", BarArgs { planes: 2 }, show_bar),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PlanesArgs {
    /// The document's width, in pixels.
    #[arg(range = 200.0..=4000.0)]
    width: f64,
    /// The document's height, in pixels.
    #[arg(range = 200.0..=4000.0)]
    height: f64,
    /// How far the warp pulls the corners, in percent of the layer's size.
    #[arg(range = 0.0..=30.0)]
    pull: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// The number of planes the bar's session holds.
    #[arg(range = 0..=4)]
    planes: u32,
}

/// What a case starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Layout,
    Warp,
    Picked,
}

/// A case's session, kept between passes.
#[derive(Clone)]
struct PlaneCell {
    kind: Kind,
    layer: Rect,
    /// The planes' count and the bits of the pull the session was built with.
    planes: u32,
    pull: u64,
    session: PerspectiveSession,
    /// The group's focus was asked for.
    focused: bool,
}

/// `count` planes side by side across `layer`, neighbours sharing an edge, each target `pull`
/// percent of the layer's size from its source (a corner shared by planes is pulled once).
fn planes_of(layer: Rect, count: u32, pull: f64) -> Vec<Plane> {
    let n = f64::from(count.max(1));
    let (dx, dy) = (layer.width() * pull / 100.0, layer.height() * pull / 100.0);
    // Every corner moves by a function of where it is, so linked corners stay linked.
    let pulled = |p: Point| {
        let (fx, fy) = ((p.x - layer.x0) / layer.width() - 0.5, (p.y - layer.y0) / layer.height() - 0.5);
        Point::new(p.x - fx * 2.0 * dx * (0.5 - fy), p.y + dy * (0.5 - fy))
    };
    (0..count)
        .map(|k| {
            let (x0, x1) = (layer.x0 + layer.width() * f64::from(k) / n, layer.x0 + layer.width() * f64::from(k + 1) / n);
            let source = [Point::new(x0, layer.y0), Point::new(x1, layer.y0), Point::new(x1, layer.y1), Point::new(x0, layer.y1)];
            Plane { source, target: source.map(pulled) }
        })
        .collect()
}

impl PlaneCell {
    fn new(kind: Kind, layer: Rect, planes: u32, pull: f64) -> Self {
        let mut session = PerspectiveSession::new(layer);
        let warped = planes_of(layer, planes, if kind == Kind::Layout { 0.0 } else { pull });
        session.replace(warped);
        if kind != Kind::Layout && planes > 0 {
            session.apply(PerspectiveEdit::Mode(PlaneMode::Warp), 0.0);
        }
        Self { kind, layer, planes, pull: pull.to_bits(), session, focused: false }
    }

    fn load(ui: &Ui, salt: &'static str, kind: Kind, layer: Rect, planes: u32, pull: f64) -> Self {
        let id = ui.make_persistent_id(("perspective-story", salt));
        ui.data(|d| d.get_temp::<PlaneCell>(id))
            .filter(|cell| cell.kind == kind && cell.layer == layer && cell.planes == planes && cell.pull == pull.to_bits())
            .unwrap_or_else(|| Self::new(kind, layer, planes, pull))
    }

    fn store(self, ui: &Ui, salt: &'static str) {
        let id = ui.make_persistent_id(("perspective-story", salt));
        ui.data_mut(|d| d.insert_temp(id, self));
    }

    fn reset(&mut self, pull: f64) {
        *self = Self::new(self.kind, self.layer, self.planes, pull);
    }

    fn apply(&mut self, edits: Vec<PerspectiveEdit>, now: f64) {
        for edit in edits {
            self.session.apply(edit, now);
        }
        self.session.settle(now, false);
    }
}

/// Where the fake map takes a point of the layer: through the plane whose source holds it, from
/// its source corners to its target corners; a point on no plane stays.
fn warped_by(planes: &[Plane]) -> impl Fn(Point) -> Point {
    let inverses: Vec<(Homography, Plane)> =
        planes.iter().filter_map(|plane| Homography::unit_to_quad(plane.source).and_then(|h| h.inverse()).map(|h| (h, *plane))).collect();
    move |p| {
        let inside = |v: f64| (-1e-9..=1.0 + 1e-9).contains(&v);
        inverses
            .iter()
            .find_map(|(to_unit, plane)| {
                let unit = to_unit.apply(p);
                (inside(unit.x) && inside(unit.y)).then(|| plane.point(unit.x, unit.y, true))
            })
            .unwrap_or(p)
    }
}

fn show_planes(ui: &mut Ui, args: &mut PlanesArgs) {
    let labels = PerspectiveStoryLabels::of(ui);
    let doc = Document::new(args.width, args.height);
    let textures = doc.textures(ui);
    let textures = textures.as_ref();
    let layer = doc.layer();
    let cases = [(Kind::Layout, "layout", &labels.layout), (Kind::Warp, "warp", &labels.warp), (Kind::Picked, "picked", &labels.picked)];
    ui.columns(cases.len(), |columns| {
        for (column, (kind, salt, text)) in columns.iter_mut().zip(cases) {
            column.add(Text::new(text).content(Content::Muted));
            let mut cell = PlaneCell::load(column, salt, kind, layer, 2, args.pull);
            let width = column.available_width();
            let (response, view) = doc.interactive_canvas(column, ("perspective-story", salt), width);
            {
                let map = warped_by(cell.session.planes());
                draw(column, &view, &doc, textures, PreviewMap::function(layer, PreviewQuality::PLANE_CELLS, &map), 1.0, false);
            }
            if kind == Kind::Picked && !cell.focused {
                let group = column.make_persistent_id(IdSalt::new(salt));
                column.memory_mut(|m| m.request_focus(group));
                cell.focused = true;
            }
            let now = column.input(|i| i.time);
            let reported = PerspectiveWarp::new(salt, &cell.session).show(column, &response, &view);
            if reported.discard {
                cell.reset(args.pull);
            }
            cell.apply(reported.edits, now);
            cell.store(column, salt);
        }
    });
}

fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let labels = PerspectiveStoryLabels::of(ui);
    let layer = Document::new(Document::DEFAULT_SIZE[0], Document::DEFAULT_SIZE[1]).layer();
    let cases = [(Kind::Layout, "bar-layout", &labels.bar_layout), (Kind::Warp, "bar-warp", &labels.bar_warp)];
    for (kind, salt, text) in cases {
        ui.add(Text::new(text).content(Content::Muted));
        let mut cell = PlaneCell::load(ui, salt, kind, layer, args.planes, BAR_PULL);
        let mut out = PerspectiveBarOutput::default();
        OptionsBar::new(salt).label(text.as_str()).show(ui, |slots| {
            slots.body(|ui| out = PerspectiveBar::new("bar", &cell.session).show(ui));
        });
        if out.discard {
            cell.reset(BAR_PULL);
        }
        let now = ui.input(|i| i.time);
        cell.apply(out.edits, now);
        cell.store(ui, salt);
    }
}

/// How far the bar story's warped planes are pulled, in percent of the layer's size.
const BAR_PULL: f64 = 8.0;
