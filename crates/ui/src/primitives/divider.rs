//! [`Divider`]: one-physical-pixel separators.

use egui::emath::GuiRounding as _;
use egui::{Painter, Pos2, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::story::NoArgs;
use crate::story::{Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// A divider's line, from [`Divider::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DividerStyle {
    /// `line.divider`, one physical pixel wide.
    pub stroke: Stroke,
}

/// A separator line in `line.divider`, one physical pixel wide at every scale and on the pixel
/// grid. Where two containers meet, the host draws the edge once.
#[derive(Clone, Copy, Debug)]
pub struct Divider {
    vertical: bool,
    follow_content: bool,
}

impl Divider {
    /// A line across the available width.
    pub fn horizontal() -> Self {
        Self { vertical: false, follow_content: false }
    }

    /// A line down the available height, for rows.
    pub fn vertical() -> Self {
        Self { vertical: true, follow_content: false }
    }

    /// Takes no width in egui's sizing pass, so a container sized by its content is not widened by
    /// the line (as egui's `Separator` does).
    pub fn follow_content(mut self) -> Self {
        self.follow_content = true;
        self
    }

    /// The resolved divider style: the `line.divider` colour at a hairline width, which is one
    /// physical pixel at `pixels_per_point`.
    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> DividerStyle {
        DividerStyle { stroke: Stroke::new(craft_theme::Stroke::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()) }
    }

    /// Allocates the line's pixel and draws it; the layout's gaps separate it from its neighbours.
    pub fn show(self, ui: &mut Ui) -> Response {
        let ppp = ui.pixels_per_point();
        let width = craft_theme::Stroke::Hairline.width(ppp);
        let available = if self.follow_content && ui.is_sizing_pass() { vec2(0.0, 0.0) } else { ui.available_size_before_wrap() };
        let size = if self.vertical { vec2(width, available.y) } else { vec2(available.x, width) };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let (from, to) = if self.vertical { (rect.center_top(), rect.center_bottom()) } else { (rect.left_center(), rect.right_center()) };
        Divider::paint(ui.painter(), from, to, &Tokens::of(ui));
        response
    }

    /// Draws a line from `from` to `to`. A horizontal or vertical line is centred on a pixel row or
    /// column and its ends are on pixel edges.
    pub fn paint(painter: &Painter, from: Pos2, to: Pos2, tokens: &UiTokens) {
        let ppp = painter.pixels_per_point();
        let snap = |p: Pos2, along_x: bool, along_y: bool| {
            let x = if along_x { p.x.round_to_pixels(ppp) } else { p.x.round_to_pixel_center(ppp) };
            let y = if along_y { p.y.round_to_pixels(ppp) } else { p.y.round_to_pixel_center(ppp) };
            pos2(x, y)
        };
        let (horizontal, vertical) = (from.y == to.y, from.x == to.x);
        let points = [snap(from, horizontal, vertical), snap(to, horizontal, vertical)];
        painter.line_segment(points, Divider::style(tokens, ppp).stroke);
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Divider"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("primitives/divider/lines", Level::Primitives, "Divider", "Lines", NoArgs {}, show)]
}

fn show(ui: &mut Ui, _: &mut NoArgs) {
    use crate::localized::Labels as _;
    let words = crate::story::StoryLabels::of(ui);
    super::Stack::new(super::Gap::Stack).show(ui, |ui| {
        ui.add(super::Text::new("Above"));
        Divider::horizontal().show(ui);
        ui.add(super::Text::new("Below"));
        Divider::horizontal().show(ui);
        super::Row::new(super::Gap::Inline).show(ui, |ui| {
            ui.add(super::Text::new(words.start.as_str()));
            Divider::vertical().show(ui);
            ui.add(super::Text::new(words.middle.as_str()));
            Divider::vertical().show(ui);
            ui.add(super::Text::new(words.end.as_str()));
        });
    });
}
