//! [`Badge`]: a small label in a tone.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::text::LayoutJob;
use egui::{Color32, CornerRadius, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Widget, WidgetInfo, vec2};

pub use craft_theme::{BadgeStyle, BadgeTone};

use crate::primitives::{Gap, Row, Stack};
use crate::story::{Args, Level, Story, variant_cluster};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};

/// The largest count a count badge shows; more reads `99+`.
pub const MAX_COUNT: u32 = 99;

/// A badge's size and colours, from [`Badge::look`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BadgeLook {
    /// `badge.height`.
    pub height: f32,
    /// At the start and end of the label: `badge.pad_x`.
    pub pad_x: f32,
    /// `badge.radius`.
    pub radius: CornerRadius,
    /// `badge.label`.
    pub label: TypeRole,
    /// `badge.<tone>.<style>.fill`.
    pub fill: Color32,
    /// `badge.<tone>.<style>.text`.
    pub text: Color32,
    /// `badge.<tone>.<style>.border`; one point wide.
    pub border: Color32,
}

/// A small label: `ui.add(Badge::new("Beta").tone(BadgeTone::Accent))`, or a count with
/// [`Badge::count`], which reads `99+` above [`MAX_COUNT`].
///
/// Soft badges (the default) tint the tone's background, text and border; solid ones fill with the
/// tone's solid colour. A badge has no input and no Tab stop; AccessKit sees a label with its
/// text. Tone and style are never the only carrier of meaning: the text says it.
#[derive(Clone, Debug)]
pub struct Badge {
    text: String,
    tone: BadgeTone,
    style: BadgeStyle,
}

impl Badge {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), tone: BadgeTone::Neutral, style: BadgeStyle::Soft }
    }

    /// A count: `99+` above [`MAX_COUNT`].
    pub fn count(n: u32) -> Self {
        Self::new(if n > MAX_COUNT { format!("{MAX_COUNT}+") } else { n.to_string() })
    }

    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn style(mut self, style: BadgeStyle) -> Self {
        self.style = style;
        self
    }

    pub fn look(tokens: &UiTokens, tone: BadgeTone, style: BadgeStyle) -> BadgeLook {
        let b = &tokens.component.badge;
        let colors = b.colors(tone, style);
        BadgeLook {
            height: b.height,
            pad_x: b.pad_x,
            radius: corner_radius(b.radius),
            label: b.label,
            fill: colors.fill.to_egui(),
            text: colors.text.to_egui(),
            border: colors.border.to_egui(),
        }
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let look = Badge::look(&tokens, self.tone, self.style);
        let galley = ui.painter().layout_job(LayoutJob::single_section(self.text.clone(), role_format(&tokens, look.label, look.text)));
        // At least as wide as tall, so a single digit is a circle.
        let size = vec2((galley.size().x + 2.0 * look.pad_x).max(look.height), look.height);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, ui.is_enabled(), &self.text));
        if ui.is_rect_visible(rect) {
            let rect = rect.round_to_pixels(ui.pixels_per_point());
            ui.painter().add(Shape::Rect(RectShape::new(rect, look.radius, look.fill, Stroke::new(1.0, look.border), StrokeKind::Inside)));
            let pos = Rect::from_center_size(rect.center(), galley.size()).min;
            ui.painter().galley(pos.round_to_pixels(ui.pixels_per_point()), galley, look.text);
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Badge"];

const SAMPLES: [&str; 3] = ["Beta", "Synced", "Needs review"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new("controls/badge/tones", Level::Controls, "Badge", "Tones", Args::new, show_tones),
        Story::new("controls/badge/counts", Level::Controls, "Badge", "Counts", || Args::new().with_number("count", 7.0, 0.0, 200.0), show_counts),
    ]
}

/// Every tone in each style.
fn show_tones(ui: &mut Ui, _: &mut Args) {
    Stack::new(Gap::Stack).show(ui, |ui| {
        for style in BadgeStyle::ALL {
            variant_cluster(ui, style.name(), |ui| {
                for (tone, text) in BadgeTone::ALL.into_iter().zip(SAMPLES.into_iter().cycle()) {
                    ui.add(Badge::new(format!("{text} {}", tone.name())).tone(tone).style(style));
                }
            });
        }
    });
}

/// Counts below, at and above the cap, and the count of the args.
fn show_counts(ui: &mut Ui, args: &mut Args) {
    let chosen = args.number("count") as u32;
    Row::new(Gap::Inline).show(ui, |ui| {
        for n in [chosen, 1, 9, 10, 99, 100, 1234] {
            ui.add(Badge::count(n));
        }
        ui.add(Badge::count(chosen).tone(BadgeTone::Bad).style(BadgeStyle::Solid));
    });
}
