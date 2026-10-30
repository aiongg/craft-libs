//! [`Spinner`]: the indeterminate ring.

use std::f32::consts::TAU;

use egui::accesskit::Role;
use egui::{Rect, Response, Sense, Shape, Stroke, Ui, Widget, WidgetInfo, vec2};

use crate::icons::IconSize;
use crate::primitives::{Content, Icon, IconStyle};
use crate::story::{Level, Story, StoryArgs, StoryChoice, variant_cluster, variant_list};
use crate::theme::{Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// The widest the arc gets, in degrees (egui's stock spinner).
const SWEEP: f32 = 240.0;

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

    /// The points of the arc from `start` to `end` radians on a ring of `radius` around the centre
    /// of `rect` (egui's stock spinner: one point per point of radius, 8 to 128, the end excluded).
    pub fn points(rect: Rect, radius: f32, start: f32, end: f32) -> Vec<egui::Pos2> {
        let n = (radius.round() as u32).clamp(8, 128);
        (0..n)
            .map(|i| {
                let angle = start + (end - start) * i as f32 / n as f32;
                rect.center() + radius * vec2(angle.cos(), angle.sin())
            })
            .collect()
    }

    /// Paints the ring in `rect` in `color`, as egui's stock spinner does: it turns once a second
    /// while its end sweeps `240°` times the sine of the time. With reduced motion it is the arc
    /// at its widest, still. It asks for the next frame only while it moves and is visible.
    pub fn paint(ui: &Ui, rect: Rect, color: egui::Color32) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let tokens = Tokens::of(ui);
        let (still, spinner) = (tokens.appearance.reduced_motion, &tokens.component.spinner);
        let radius = rect.width().min(rect.height()) * 0.5 - spinner.inset;
        let (start, end) = if still {
            (0.0, SWEEP.to_radians())
        } else {
            let time = ui.input(|i| i.time);
            let start = time * f64::from(TAU);
            (start as f32, (start + f64::from(SWEEP.to_radians()) * time.sin()) as f32)
        };
        ui.painter().add(Shape::line(Spinner::points(rect, radius, start, end), Stroke::new(spinner.stroke, color)));
        if !still {
            ui.ctx().request_repaint();
        }
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let style = Spinner::style(&Tokens::of(ui), self.size, self.content);
        let (rect, response) = ui.allocate_exact_size(vec2(style.points, style.points), Sense::hover());
        Spinner::paint(ui, rect, style.color);
        if let Some(label) = &self.label {
            response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), label));
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_busy());
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Spinner"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ContentArg {
    Normal,
    Muted,
    Accent,
}

impl ContentArg {
    fn content(self) -> Content {
        match self {
            ContentArg::Normal => Content::Normal,
            ContentArg::Muted => Content::Muted,
            ContentArg::Accent => Content::Accent,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SizesArgs {
    content: ContentArg,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("controls/spinner/sizes", Level::Controls, "Spinner", "Sizes", SizesArgs { content: ContentArg::Normal }, show_sizes)]
}

/// One spinner per icon size and the default control height, each named for a screen reader.
fn show_sizes(ui: &mut Ui, args: &mut SizesArgs) {
    let content = args.content.content();
    let sizes = IconSize::ALL.map(Some).into_iter().chain([None]);
    variant_list(ui, "spinner-sizes", |form| {
        for (size, name) in sizes.zip(["Sm", "Md", "Lg", "Control"]) {
            variant_cluster(form, name, |ui| {
                let spinner = Spinner::new().content(content).label(format!("Loading {name}"));
                ui.add(match size {
                    Some(size) => spinner.size(size),
                    None => spinner,
                });
            });
        }
    });
}
