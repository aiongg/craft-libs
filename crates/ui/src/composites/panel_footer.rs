//! [`PanelFooter`]: a bar of buttons under a panel body.

use craft_theme::Stroke as Hairline;
use craft_theme::{ControlSize, Tone};
use egui::accesskit::HasPopup;
use egui::emath::GuiRounding as _;
use std::any::Any;
use std::sync::Arc;

use egui::{DragAndDrop, Rect, Response, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::behavior::set_drop_hint;
use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::list::{DropMark, paint_drop_mark};
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::patterns::rail::flyout::paint_corner_triangle;
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// A footer button's label, kept under its response id for the drop hint.
#[derive(Clone)]
struct ButtonLabel(String);

/// A host's button on a panel footer, for panels that take buttons from the app (a media bin's
/// Automate to Sequence, Find and New Item; a preset browser's own actions). The panel shows it
/// with [`PanelFooter::drop_button`] when it takes drops, else with [`PanelFooter::button`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FooterAction {
    /// The id the panel reports for a click or a drop on it.
    pub id: &'static str,
    /// The glyph.
    pub icon: Icon,
    /// The tooltip and accessible name, translated.
    pub label: String,
    /// Whether it takes clicks. A disabled action that takes drops still takes an accepted drop
    /// (see [`PanelFooter::drop_button`]).
    pub enabled: bool,
    /// Whether the panel's drags may drop on it (FilmCraft's New Item makes a sequence from a
    /// clip dropped on it).
    pub accepts_drops: bool,
}

impl FooterAction {
    /// An enabled action that takes no drops.
    pub fn new(id: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        // filmcraft: crates/ui-egui/src/panels/project.rs:555 (`footer` buttons).
        Self { id, icon, label: label.into(), enabled: true, accepts_drops: false }
    }

    /// Enables or disables it.
    #[must_use]
    pub fn enabled(self, on: bool) -> Self {
        Self { enabled: on, ..self }
    }

    /// Lets the panel's drags drop on it.
    #[must_use]
    pub fn accepts_drops(self, on: bool) -> Self {
        Self { accepts_drops: on, ..self }
    }
}

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
/// - **Drops.** A button takes things dragged onto it with
///   [`drop_target`](PanelFooter::drop_target): a layer row onto New Layer, a channel onto New.
///   The button shows the list's drop outline while it accepts. A disabled button takes no drop.
///   A button that acts on a dragged row even while it is disabled for a click (Delete with
///   nothing selected, History's trash) is a [`drop_button`](PanelFooter::drop_button): it shows
///   enabled while a drag it accepts is over it.
/// - **Pressed buttons.** A button that shows a mode or a state the host owns (Actions' Button
///   Mode and Record) is a [`pressed_button`](PanelFooter::pressed_button).
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
        PanelFooter::button_in(ui, icon, label, None)
    }

    /// A footer button that shows the host's state, `pressed` (Actions' Button Mode while the
    /// panel is in Button Mode, Record while recording): the
    /// [`selected`](crate::controls::IconButton::selected) look, and AccessKit reports it toggled.
    /// `tone` tints the icon in that tone's status colour for an alert state (Record while
    /// recording, [`Tone::Bad`]); [`Tone::Neutral`] keeps the button's own colour. A click is
    /// reported in the response; the host changes its state, or ignores the click (Record while
    /// recording).
    pub fn pressed_button(ui: &mut Ui, icon: Icon, label: &str, pressed: bool, tone: Tone) -> Response {
        // pc: crates/ui-egui/src/panels/actions.rs:1110-1117 (Record: red dot while recording, still hovers).
        let side = Tokens::of(ui).component.panel_footer.button;
        let (_, square) = ui.allocate_space(vec2(side, side));
        let response = IconButton::new(icon, label).size(ControlSize::Md).selected(pressed).tone(tone).show_at(ui, square);
        ui.data_mut(|d| d.insert_temp(response.id, ButtonLabel(label.to_owned())));
        response
    }

    /// A footer button that takes drops of egui drag payloads of type `P` and acts on a dragged
    /// row even while it is disabled for a click: Layers' and Channels' Delete with nothing
    /// selected, History's trash. Returns the button's response and the payload when it is
    /// released over the button and `accepts` takes it.
    ///
    /// ```ignore
    /// let (trash, dropped) = PanelFooter::drop_button::<ListDrag<StepId>>(ui, Icon::Trash, "Delete", can_delete, |drag| drag.list == list);
    /// if let Some(drag) = dropped {
    ///     events.push(HistoryEvent::DeleteStep(drag.grabbed.clone()));
    /// }
    /// ```
    ///
    /// - While the pointer is over the button's square with a `P` payload that `accepts` takes,
    ///   the button is drawn enabled, whatever `enabled` says, and shows the drop outline and
    ///   hint as with [`drop_target`](PanelFooter::drop_target). The payload is tested before the
    ///   button is drawn, so its look and its AccessKit state follow the drag in the same pass.
    /// - Otherwise it is enabled as `enabled` says; a refused payload shows nothing and is never
    ///   returned. `accepts` runs once per pass.
    /// - Pass the enabled state here rather than showing the button in a disabled `Ui`, which
    ///   would keep it disabled under an accepted drag.
    pub fn drop_button<P: Any + Send + Sync>(
        ui: &mut Ui,
        icon: Icon,
        label: &str,
        enabled: bool,
        accepts: impl FnOnce(&P) -> bool,
    ) -> (Response, Option<Arc<P>>) {
        // pc: crates/ui-egui/src/panels/history.rs:118-143 (the trash is enabled for an acceptable drag);
        // lead call 17: enabled only while the accepted drag is over it.
        let side = Tokens::of(ui).component.panel_footer.button;
        let (_, square) = ui.allocate_space(vec2(side, side));
        let accepted = DragAndDrop::payload::<P>(ui.ctx()).filter(|_| ui.rect_contains_pointer(square)).filter(|payload| accepts(payload));
        let response = IconButton::new(icon, label).size(ControlSize::Md).enabled(enabled || accepted.is_some()).show_at(ui, square);
        ui.data_mut(|d| d.insert_temp(response.id, ButtonLabel(label.to_owned())));
        let dropped = accepted.and_then(|_| PanelFooter::take_or_mark::<P>(ui, &response));
        (response, dropped)
    }

    /// A footer button that opens a menu (New Fill or Adjustment Layer, layer styles): the
    /// [`button`](PanelFooter::button) with a `rail.indicator` triangle at its bottom-end corner,
    /// as the rail's flyout buttons have, and a `Menu` popup in AccessKit. The host opens the menu
    /// from the response and passes in `open` whether its menu is open (the last pass's
    /// [`MenuResponse::open`](crate::menu::MenuResponse::open)), which AccessKit reports as the
    /// button's expanded state.
    pub fn menu_button(ui: &mut Ui, icon: Icon, label: &str, open: bool) -> Response {
        PanelFooter::button_in(ui, icon, label, Some(open))
    }

    /// Makes `button` a drop target for egui drag payloads of type `P` (a list's
    /// [`ListDrag`](crate::behavior::ListDrag), a layer's mask), and returns the payload when it is
    /// released over the button and `accepts` takes it. Call it after the button, in the pass the
    /// button is shown.
    ///
    /// ```ignore
    /// let delete = PanelFooter::button(ui, Icon::Trash, "Delete layer");
    /// if let Some(drag) = PanelFooter::drop_target::<ListDrag<LayerId>>(ui, &delete, |drag| !drag.items.is_empty()) {
    ///     events.push(LayersEvent::FooterDrop { button: LayersFooterButton::Delete, rows: drag.items.clone() });
    /// }
    /// ```
    ///
    /// - While the pointer is over the button (`Response::dnd_hover_payload`, which works during
    ///   another widget's drag) with a `P` payload that `accepts` takes, the button shows the list's
    ///   drop outline ([`paint_drop_mark`](crate::list::paint_drop_mark) with
    ///   [`DropMark::Into`](crate::list::DropMark::Into) over the button's rect) and sets this pass's
    ///   [drop hint](crate::behavior::set_drop_hint) to the button's label, the text given to
    ///   [`button`](PanelFooter::button) or [`menu_button`](PanelFooter::menu_button). A payload
    ///   `accepts` refuses shows nothing and is never returned. `accepts` runs once per pass.
    /// - A disabled button takes nothing: no outline, no hint, no payload, and `accepts` doesn't
    ///   run. A button that takes drops while disabled for a click is a
    ///   [`drop_button`](PanelFooter::drop_button).
    /// - On the release over the button (`Response::dnd_release_payload`) it takes the payload and
    ///   returns it; a release elsewhere leaves the payload for the widget under the pointer.
    /// - A list's own drag released over the footer ends in that list as
    ///   `ListEvent::Reorder(Cancel(Dismissed))`, and the list leaves its payload for the widget
    ///   under the pointer, so the footer gets it in the same pass whether it is drawn after the
    ///   list or before it (egui clears a payload only at the end of the release pass). A list that
    ///   drags only to its footer refuses every row target with
    ///   [`Reorder::accepts`](crate::behavior::Reorder::accepts).
    pub fn drop_target<P: Any + Send + Sync>(ui: &Ui, button: &Response, accepts: impl FnOnce(&P) -> bool) -> Option<Arc<P>> {
        // pc: crates/ui-egui/src/panels/layer_drag.rs:40 `dropped_on` (outline while over, true on release);
        // the label for the hint is kept by `button_in` in egui temp data under the response id.
        if !button.enabled() {
            return None;
        }
        let hovered = button.dnd_hover_payload::<P>()?;
        if !accepts(&hovered) {
            return None;
        }
        PanelFooter::take_or_mark::<P>(ui, button)
    }

    /// For an accepted payload over `button`: takes it on the release, else paints the drop
    /// outline and sets the drop hint.
    fn take_or_mark<P: Any + Send + Sync>(ui: &Ui, button: &Response) -> Option<Arc<P>> {
        if ui.input(|i| i.pointer.any_released()) {
            return DragAndDrop::take_payload::<P>(ui.ctx());
        }
        paint_drop_mark(ui, button.rect, DropMark::Into);
        if let Some(label) = ui.data(|d| d.get_temp::<ButtonLabel>(button.id)) {
            set_drop_hint(ui.ctx(), label.0);
        }
        None
    }

    fn button_in(ui: &mut Ui, icon: Icon, label: &str, menu: Option<bool>) -> Response {
        let tokens = Tokens::of(ui);
        let side = tokens.component.panel_footer.button;
        let (_, square) = ui.allocate_space(vec2(side, side));
        let response = IconButton::new(icon, label).size(ControlSize::Md).show_at(ui, square);
        ui.data_mut(|d| d.insert_temp(response.id, ButtonLabel(label.to_owned())));
        if let Some(open) = menu {
            let rail = &tokens.component.rail;
            if ui.is_rect_visible(response.rect) {
                paint_corner_triangle(ui, response.rect, rail.indicator_inset, rail.indicator_size, rail.indicator_color.to_egui());
            }
            response.ctx.accesskit_node_builder(response.id, |node| {
                node.set_has_popup(HasPopup::Menu);
                node.set_expanded(open);
            });
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
            let open_id = state_id.with("open");
            let was_open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
            ui.label("Layers");
            if let Some(chosen) = chosen {
                ui.label(format!("Added: {chosen}"));
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                PanelFooter::show(ui, |ui| {
                    PanelFooter::button(ui, Icon::Trash, "Delete layer");
                    PanelFooter::button(ui, Icon::Plus, "New layer");
                    PanelFooter::button(ui, Icon::Settings, "Layer settings");
                    let menu_button = PanelFooter::menu_button(ui, Icon::Adjustments, "New fill or adjustment layer", was_open);
                    let entries = [
                        MenuEntry::Item(MenuItem::new("Solid Color...", "Solid Color")),
                        MenuEntry::Item(MenuItem::new("Gradient...", "Gradient")),
                        MenuEntry::Separator,
                        MenuEntry::Item(MenuItem::new("Levels...", "Levels")),
                    ];
                    let at = menu_button.clicked().then_some(OpenAt::Rect(menu_button.rect));
                    let shown = ContextMenu::new(&entries).own_gestures(false).open_at(at).show(ui, &menu_button);
                    ui.data_mut(|d| d.insert_temp(open_id, shown.open));
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
