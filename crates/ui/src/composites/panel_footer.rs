//! [`PanelFooter`]: a bar of buttons under a panel body.

use craft_theme::ControlSize;
use craft_theme::Stroke as Hairline;
use egui::accesskit::HasPopup;
use egui::emath::GuiRounding as _;
use egui::{Rect, Response, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::patterns::rail::flyout::paint_corner_triangle;
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

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
    /// The rule over the bar: `panel.footer.line`, one physical pixel.
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
            rule: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.component.panel.footer_line.to_egui()),
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
        PanelFooter::button_in(ui, icon, label, false)
    }

    /// A footer button that opens a menu (New Fill or Adjustment Layer, layer styles): the
    /// [`button`](PanelFooter::button) with a `rail.indicator` triangle at its bottom-end corner,
    /// as the rail's flyout buttons have, and a `Menu` popup in AccessKit. The host opens the menu
    /// from the response.
    pub fn menu_button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
        PanelFooter::button_in(ui, icon, label, true)
    }

    fn button_in(ui: &mut Ui, icon: Icon, label: &str, menu: bool) -> Response {
        let tokens = Tokens::of(ui);
        let side = tokens.component.panel_footer.button;
        let (_, square) = ui.allocate_space(vec2(side, side));
        let response = IconButton::new(icon, label).size(ControlSize::Md).show_at(ui, square);
        if menu {
            let rail = &tokens.component.rail;
            if ui.is_rect_visible(response.rect) {
                paint_corner_triangle(ui, response.rect, rail.indicator_inset, rail.indicator_size, rail.indicator_color.to_egui());
            }
            response.ctx.accesskit_node_builder(response.id, |node| node.set_has_popup(HasPopup::Menu));
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PanelFooter"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/panel-footer/layers", Level::Composites, "PanelFooter", "Layers", LayersArgs { width: 240.0 }, show_layers)
            .on(Surface::Own),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct LayersArgs {
    #[arg(range = 60.0..=400.0)]
    pub(super) width: f32,
}

/// The height of the story's panel.
const STORY_PANEL_HEIGHT: f32 = 140.0;

/// A panel `width` wide with a body and, at its bottom, a footer of four buttons, inside the body
/// margin. The menu button opens the menu of new fill and adjustment layers; the body names the
/// chosen item.
pub(super) fn show_layers(ui: &mut Ui, args: &mut LayersArgs) {
    let width = args.width.min(ui.available_width());
    let tokens = Tokens::of(ui);
    let margin = tokens.space.pad.panel;
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        egui::Frame::new().inner_margin(egui::Margin::same(margin as i8)).fill(tokens.color.surface.panel.to_egui()).show(ui, |ui| {
            ui.set_width(width - 2.0 * margin);
            ui.set_height(STORY_PANEL_HEIGHT - 2.0 * margin);
            let state_id = ui.scope_id().with("panel-footer-layers-chosen");
            let mut chosen = ui.data(|d| d.get_temp::<&'static str>(state_id));
            ui.label("Layers");
            if let Some(chosen) = chosen {
                ui.label(format!("Added: {chosen}"));
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                PanelFooter::show(ui, |ui| {
                    PanelFooter::button(ui, Icon::Trash, "Delete layer");
                    PanelFooter::button(ui, Icon::Plus, "New layer");
                    PanelFooter::button(ui, Icon::Settings, "Layer settings");
                    let menu_button = PanelFooter::menu_button(ui, Icon::Adjustments, "New fill or adjustment layer");
                    let entries = [
                        MenuEntry::Item(MenuItem::new("Solid Color...", "Solid Color")),
                        MenuEntry::Item(MenuItem::new("Gradient...", "Gradient")),
                        MenuEntry::Separator,
                        MenuEntry::Item(MenuItem::new("Levels...", "Levels")),
                    ];
                    let at = menu_button.clicked().then_some(OpenAt::Rect(menu_button.rect));
                    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, &menu_button);
                    if shown.chosen.is_some() {
                        chosen = shown.chosen;
                    }
                });
            });
            if let Some(chosen) = chosen {
                ui.data_mut(|d| d.insert_temp(state_id, chosen));
            }
        });
    });
}
