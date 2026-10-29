//! [`PanelFooter`]: a bar of buttons under a panel body.

use craft_theme::ControlSize;
use craft_theme::Stroke as Hairline;
use egui::emath::GuiRounding as _;
use egui::{Color32, Rect, Response, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A panel footer's bar, rule and buttons, from [`PanelFooter::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelFooterStyle {
    /// Height of the bar under the rule: `panel_footer.height`.
    pub height: f32,
    /// Between the buttons: `panel_footer.gap`.
    pub gap: f32,
    /// Side of a footer button: `panel_footer.button`.
    pub button: f32,
    /// The rule over the bar: `line.divider`, one physical pixel.
    pub rule: Stroke,
    /// The panel body's margin, which the rule runs across and the bar runs over:
    /// `space.pad.panel`.
    pub margin: f32,
}

/// A panel footer as in Photoshop's Layers and History panels (PhotoCraft's `panel_footer`): a bar
/// of buttons under a rule that runs from edge to edge of the panel, the buttons centred in the
/// bar and laid out from the reading end.
///
/// ```
/// use craft_ui::composites::PanelFooter;
/// use craft_ui::icons::Icon;
/// # egui::__run_test_ui(|ui| {
/// PanelFooter::show(ui, |ui| {
///     if PanelFooter::button(ui, Icon::Trash, "Delete layer").clicked() { /* delete */ }
///     PanelFooter::button(ui, Icon::Plus, "New layer");
/// });
/// # });
/// ```
///
/// - Show it last in a panel body that is inside the body's margin (`space.pad.panel`). The footer
///   reserves the rule and the bar less that margin: the rule runs out across the margin to the
///   panel's edges and the bar runs on over the bottom margin, so it ends at the panel's bottom
///   edge. The clip is widened to the bar, so hover boxes are not cut.
/// - The first widget added is at the reading end; later ones follow towards the start. Buttons are
///   `panel_footer.button` squares ([`PanelFooter::button`]) `panel_footer.gap` apart.
/// - **Keyboard and AccessKit:** the host's widgets are Tab stops in the order added; the footer
///   adds no node.
pub struct PanelFooter;

impl PanelFooter {
    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> PanelFooterStyle {
        let f = &tokens.component.panel_footer;
        PanelFooterStyle {
            height: f.height,
            gap: f.gap,
            button: f.button,
            rule: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()),
            margin: tokens.space.pad.panel,
        }
    }

    /// The height the footer takes from the panel body: the rule and the bar down to the body's
    /// bottom margin.
    pub fn reserved_height(style: &PanelFooterStyle) -> f32 {
        (style.rule.width + style.height - style.margin).max(style.rule.width)
    }

    /// Shows the footer across the available width and adds the host's widgets to the bar.
    pub fn show<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let tokens = Tokens::of(ui);
        let style = PanelFooter::style(&tokens, ui.pixels_per_point());
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), PanelFooter::reserved_height(&style)));
        let edge = rect.x_range().expand(style.margin);
        let y = (rect.top() + style.rule.width / 2.0).round_to_pixel_center(ui.pixels_per_point());
        let bar = Rect::from_min_size(pos2(rect.left(), rect.top() + style.rule.width), vec2(rect.width(), style.height));
        let clip = ui.clip_rect().union(bar).union(Rect::from_x_y_ranges(edge, rect.y_range()));
        ui.painter().with_clip_rect(clip).hline(edge, y, style.rule);
        let mut bar_ui = ui.new_child(UiBuilder::new().max_rect(bar).layout(direction::trailing(ui)));
        bar_ui.set_clip_rect(clip);
        bar_ui.spacing_mut().item_spacing.x = style.gap;
        add(&mut bar_ui)
    }

    /// An icon button of the footer's button size, named `label` (its tooltip and accessible name).
    pub fn button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
        let side = Tokens::of(ui).component.panel_footer.button;
        let (_, square) = ui.allocate_space(vec2(side, side));
        IconButton::new(icon, label).size(ControlSize::Md).show_at(ui, square)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PanelFooter"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_number("width", 240.0, 60.0, 400.0);
    vec![Story { id: "composites/panel-footer/layers", level: Level::Composites, component: "PanelFooter", name: "Layers", args, show: show_layers }]
}

/// A panel `width` wide with a body and a footer of three buttons, inside the body margin.
pub(super) fn show_layers(ui: &mut Ui, args: &mut Args) {
    let width = args.number("width") as f32;
    let tokens = Tokens::of(ui);
    let margin = tokens.space.pad.panel;
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        egui::Frame::new().inner_margin(egui::Margin::same(margin as i8)).fill(Color32::TRANSPARENT).show(ui, |ui| {
            ui.set_height(80.0);
            ui.label("Layers");
            PanelFooter::show(ui, |ui| {
                PanelFooter::button(ui, Icon::Trash, "Delete layer");
                PanelFooter::button(ui, Icon::Plus, "New layer");
                PanelFooter::button(ui, Icon::Settings, "Layer settings");
            });
        });
    });
}
