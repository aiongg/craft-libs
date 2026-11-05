//! Stories of the corner widgets. The story holds fake
//! shapes, applies the edits the widgets report and draws the rounded outlines itself, as an app's
//! engine does.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/corner-widgets/edit` | A fake rectangle, a star and a path's corner anchors; args: kinds, hide-above angle, individual |

use egui::{Pos2, Shape, Stroke, Ui};
use kurbo::{Point, Vec2};

use super::{CornerKind, CornerPoint, CornerTarget, CornerWidgets};
use crate::geometry::fillet::fillet;
use crate::ui::stroke::stories::EventKindLabels;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::RulerUnits;
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryLabels, lock, shared};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document_unfocusable};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["CornerWidgets"];

craft_ui::words! {
    /// The last-event words of the corner story, English by default.
    #[non_exhaustive]
    pub(crate) struct CornerStoryLabels {
        cycled: "cornerStory.cycled" = "Kind changed";
        activated: "cornerStory.activated" = "Activated";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/corner-widgets/edit",
            Level::Primitives,
            "CornerWidgets",
            "Edit",
            CornerArgs { view: ViewArgs::default(), kinds: true, hide_above: 177.0, individual: true },
            show_corners
        )
        .on(craft_ui::story::Surface::Canvas),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CornerArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// ⌥-click cycles Round, Inverted Round and Chamfer.
    kinds: bool,
    /// The widest inner angle, in degrees, whose corner shows a widget.
    #[arg(name = "hide above", range = 90.0..=180.0)]
    hide_above: f64,
    /// Whether the path's corners take their own radius (the rectangle always does, the star
    /// never).
    individual: bool,
}

/// A fake shape: a closed polygon, each vertex with a radius and a kind.
#[derive(Clone, Debug)]
struct FakeShape {
    key: u64,
    vertices: Vec<Point>,
    radii: Vec<f64>,
    kinds: Vec<CornerKind>,
    individual: bool,
}

impl FakeShape {
    fn new(key: u64, vertices: Vec<Point>, radius: f64, individual: bool) -> Self {
        let n = vertices.len();
        Self { key, vertices, radii: vec![radius; n], kinds: vec![CornerKind::Round; n], individual }
    }

    fn target(&self) -> CornerTarget {
        let n = self.vertices.len();
        let corners = (0..n)
            .map(|i| CornerPoint {
                vertex: self.vertices[i],
                prev: self.vertices[(i + n - 1) % n],
                next: self.vertices[(i + 1) % n],
                radius: self.radii[i],
                kind: self.kinds[i],
            })
            .collect();
        CornerTarget { key: self.key, corners, individual: self.individual }
    }

    /// The outline with every corner cut as its kind says, as screen points.
    fn outline(&self, view: &CanvasView) -> Vec<Pos2> {
        const ARC_STEPS: usize = 12;
        let n = self.vertices.len();
        let mut out = Vec::new();
        for i in 0..n {
            let vertex = self.vertices[i];
            let cut = fillet(self.vertices[(i + n - 1) % n], vertex, self.vertices[(i + 1) % n], self.radii[i]);
            let Some(f) = cut else {
                out.push(view.to_screen(vertex));
                continue;
            };
            match self.kinds[i] {
                CornerKind::Chamfer => out.extend([view.to_screen(f.from), view.to_screen(f.to)]),
                kind => {
                    let centre = if kind == CornerKind::InvertedRound { f.from + f.to.to_vec2() - f.centre.to_vec2() } else { f.centre };
                    let a = (f.from - centre).atan2();
                    let mut sweep = (f.to - centre).atan2() - a;
                    if sweep > std::f64::consts::PI {
                        sweep -= std::f64::consts::TAU;
                    } else if sweep < -std::f64::consts::PI {
                        sweep += std::f64::consts::TAU;
                    }
                    for s in 0..=ARC_STEPS {
                        let angle = a + sweep * s as f64 / ARC_STEPS as f64;
                        out.push(view.to_screen(centre + Vec2::from_angle(angle) * f.radius));
                    }
                }
            }
        }
        out
    }
}

/// What the story keeps: the shapes (topmost first), the shapes at the start of a drag, and the
/// last event.
#[derive(Clone, Debug)]
struct CornerState {
    shapes: Vec<FakeShape>,
    before: Option<Vec<FakeShape>>,
    last: Option<String>,
}

impl CornerState {
    fn new() -> Self {
        let at = |x: f64, y: f64| Point::new(DOC.x0 + DOC.width() * x, DOC.y0 + DOC.height() * y);
        let rect = vec![at(0.06, 0.18), at(0.34, 0.18), at(0.34, 0.5), at(0.06, 0.5)];
        let (centre, outer, inner) = (at(0.64, 0.34), DOC.height() * 0.28, DOC.height() * 0.13);
        let star = (0..10)
            .map(|i| {
                let radius = if i % 2 == 0 { outer } else { inner };
                centre + Vec2::from_angle(std::f64::consts::TAU * f64::from(i) / 10.0 - std::f64::consts::FRAC_PI_2) * radius
            })
            .collect();
        let path = vec![at(0.12, 0.92), at(0.2, 0.62), at(0.5, 0.58), at(0.82, 0.66), at(0.9, 0.92)];
        let radius = DOC.height() * 0.04;
        Self {
            shapes: vec![FakeShape::new(1, rect, radius, true), FakeShape::new(2, star, radius, false), FakeShape::new(3, path, radius, true)],
            before: None,
            last: None,
        }
    }
}

/// Shapes with rounded corners and the widgets over them: drag a widget to round a corner (⇧ and
/// the arrows work from the keyboard), ⌥ on a rectangle changes one corner, ⌥-click cycles the
/// kind, a double-click reports activation.
fn show_corners(ui: &mut Ui, args: &mut CornerArgs) {
    let words = StoryLabels::of(ui);
    let kept = shared(ui, "corners", CornerState::new);
    let mut state = lock(&kept);
    state.shapes[2].individual = args.individual;
    // The corner group is the Tab stop, so the canvas takes none.
    let canvas = document_unfocusable(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let (line, fill) = (tokens.color.accent.solid.to_egui(), tokens.color.accent.solid.to_egui().gamma_multiply(0.25));
    for shape in state.shapes.iter().rev() {
        painter.add(Shape::convex_polygon(shape.outline(&canvas.view), fill, Stroke::new(tokens.component.line_guide.width, line)));
    }
    let targets: Vec<CornerTarget> = state.shapes.iter().map(FakeShape::target).collect();
    let units = RulerUnits::pixels(LocaleNumbers::of(ui).as_ref().clone());
    let response =
        CornerWidgets::new("corner-story", &targets, &units).kinds(args.kinds).hide_above(args.hide_above).show(ui, &canvas.response, &canvas.view);
    if let Some(event) = &response.event {
        state.last = EventKindLabels::of(ui).kind(event);
    }
    match &response.event {
        Some(EditEvent::Begin { .. }) => state.before = Some(state.shapes.clone()),
        Some(EditEvent::Cancel { .. }) => {
            if let Some(before) = state.before.take() {
                state.shapes = before;
            }
        }
        Some(EditEvent::Commit { .. }) => state.before = None,
        _ => {}
    }
    if let Some(edit) = response.event.as_ref().and_then(|e| e.value()) {
        for radii in &edit.targets {
            if let Some(shape) = state.shapes.iter_mut().find(|s| s.key == radii.key) {
                shape.radii.clone_from(&radii.radii);
            }
        }
    }
    if let Some((corner, kind)) = response.cycle_kind
        && let Some(shape) = state.shapes.iter_mut().find(|s| s.key == corner.target)
    {
        shape.kinds[corner.corner] = kind;
        state.last = Some(CornerStoryLabels::of(ui).cycled);
    }
    if response.activate.is_some() {
        state.last = Some(CornerStoryLabels::of(ui).activated);
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            *state = CornerState::new();
        }
        let text = match &state.last {
            Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
            None => words.no_event.clone(),
        };
        ui.add(Text::new(text).content(Content::Muted));
    });
}
