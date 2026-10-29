//! [`Spinner`]: the indeterminate ring.

use std::f32::consts::TAU;

use egui::accesskit::Role;
use egui::{Rect, Response, Sense, Shape, Stroke, Ui, Widget, WidgetInfo, vec2};

use crate::icons::IconSize;
use crate::primitives::{Content, Gap, Icon, IconStyle, Row, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// Seconds for one turn.
const TURN: f64 = 1.0;
/// The ring covers this fraction of the circle.
const ARC: f32 = 0.75;
/// Segments of the ring's polyline.
const SEGMENTS: usize = 24;
/// The radius of the circles the icon sets draw, in view-box units of 24.
const RADIUS: f32 = 10.0 / 24.0;

/// A three-quarter ring that turns once a second:
/// `ui.add(Spinner::new().size(IconSize::Lg).label("Loading"))`.
///
/// It is the control height (`size.control.md`) unless [`size`](Spinner::size) gives an icon
/// size, and draws in the icon stroke width and a [`Content`] colour ([`Content::Normal`] by
/// default, as PhotoCraft's spinners draw in the strong text colour). With reduced motion it is
/// still. It is
/// in the AccessKit tree as a busy progress indicator when [`label`](Spinner::label) names it;
/// one inside a button or another control that reports busy is decoration.
#[derive(Clone, Debug)]
pub struct Spinner {
    /// `None` is the control height.
    size: Option<IconSize>,
    content: Content,
    label: Option<String>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self { size: None, content: Content::Normal, label: None }
    }

    /// The side of an icon of `size` instead of the control height.
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }

    /// The accessible name; without it the spinner is not in the AccessKit tree.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The side, stroke width and colour of the ring: those of an icon of `size` in `content`; for
    /// no size the control height with the stroke of a large icon.
    pub fn style(tokens: &UiTokens, size: Option<IconSize>, content: Content) -> IconStyle {
        match size {
            Some(size) => Icon::style(tokens, size, content),
            None => IconStyle { points: tokens.size.control.md, ..Icon::style(tokens, IconSize::Lg, content) },
        }
    }

    /// The ring's points at `turn` radians in `rect`.
    pub fn points(rect: Rect, turn: f32) -> Vec<egui::Pos2> {
        let radius = rect.width().min(rect.height()) * RADIUS;
        (0..=SEGMENTS)
            .map(|i| {
                let angle = turn + i as f32 / SEGMENTS as f32 * ARC * TAU;
                rect.center() + radius * vec2(angle.cos(), angle.sin())
            })
            .collect()
    }

    /// Paints the ring in `rect`, turning with the clock unless motion is reduced, and asks for
    /// the next frame only while it moves and is visible.
    pub fn paint(ui: &Ui, rect: Rect, stroke: f32, color: egui::Color32) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let still = Tokens::of(ui).appearance.reduced_motion;
        let turn = if still { 0.0 } else { (ui.input(|i| i.time) % TURN / TURN) as f32 * TAU };
        ui.painter().add(Shape::line(Spinner::points(rect, turn), Stroke::new(stroke, color)));
        if !still {
            ui.ctx().request_repaint();
        }
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let style = Spinner::style(&Tokens::of(ui), self.size, self.content);
        let (rect, response) = ui.allocate_exact_size(vec2(style.points, style.points), Sense::hover());
        Spinner::paint(ui, rect, style.stroke, style.color);
        if let Some(label) = &self.label {
            response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), label));
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_busy());
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Spinner"];

const CONTENT_NAMES: &[&str] = &["normal", "muted", "accent"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "controls/spinner/sizes",
        level: Level::Controls,
        component: "Spinner",
        name: "Sizes",
        args: || Args::new().with_choice("content", CONTENT_NAMES, 0),
        show: show_sizes,
    }]
}

/// One spinner per icon size and the default control height, each named for a screen reader.
fn show_sizes(ui: &mut Ui, args: &mut Args) {
    let content = Content::from_name(args.choice("content")).unwrap_or(Content::Normal);
    let sizes = IconSize::ALL.map(Some).into_iter().chain([None]);
    Row::new(Gap::Section).show(ui, |ui| {
        for (size, name) in sizes.zip(["Sm", "Md", "Lg", "Control"]) {
            ui.add(Text::new(name).content(Content::Muted));
            let spinner = Spinner::new().content(content).label(format!("Loading {name}"));
            ui.add(match size {
                Some(size) => spinner.size(size),
                None => spinner,
            });
        }
    });
}
