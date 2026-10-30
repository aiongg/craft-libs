//! [`SwatchPair`]: two overlapping colour wells, foreground over background or fill over stroke.

use craft_theme::{ControlSize, Srgba};
use egui::Painter;
use egui::accesskit::{Action, Role};
use egui::epaint::RectShape;
use egui::{
    Align, AsIdSalt, Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder,
    Vec2, pos2, vec2,
};

use super::popover::{self, ColorPopoverResponse};
use super::{ColorPicker, PickedColor};
use crate::controls::swatch::checkerboard;
use crate::controls::{Swatch, SwatchStyle, TooltipExt as _};
use crate::direction;
use crate::edit::EditResponse;
use crate::localized::LocaleNumbers;
use crate::primitives::{FocusRing, Interactive, Sensed};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// One of a [`SwatchPair`]'s wells. The primary one is at the start, top, the secondary one is
/// offset towards the end and down: foreground and background, or fill and stroke.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Well {
    #[default]
    Primary,
    Secondary,
}

impl Well {
    /// The other well.
    pub fn other(self) -> Well {
        match self {
            Well::Primary => Well::Secondary,
            Well::Secondary => Well::Primary,
        }
    }
}

/// What a [`SwatchPair`] shows for one well.
#[derive(Clone, Debug)]
pub struct PairWell {
    label: String,
    color: Srgba,
    ring: bool,
}

impl PairWell {
    /// A well named `label` (its accessible name and tooltip, translated: "Foreground colour")
    /// of the host's `color`.
    pub fn new(label: impl Into<String>, color: Srgba) -> Self {
        Self { label: label.into(), color, ring: false }
    }

    /// Draws the well as a ring (a stroke, as DesignCraft's stroke well) instead of a filled
    /// square. The middle shows what is behind it.
    pub fn ring(mut self, ring: bool) -> Self {
        self.ring = ring;
        self
    }
}

/// Sizes and colours a pair draws itself, from [`SwatchPair::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchPairStyle {
    /// Side of a well: `color_picker.pair_well`.
    pub well: f32,
    /// Offset of the secondary well from the primary one on both axes: `color_picker.pair_step`.
    pub step: f32,
    /// Width of the gap in the panel's colour around the front well: `color_picker.pair_gap`.
    pub gap: f32,
    /// Width of a ring well's band: `color_picker.pair_ring`.
    pub ring: f32,
}

/// Two overlapping colour wells, the active one in front, as PhotoCraft's foreground and
/// background chips and DesignCraft's fill and stroke proxy:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::{PairWell, SwatchPair, Well};
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let (mut active, mut fg, mut bg) = (Well::Primary, Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0));
/// let numbers = LocaleNumbers::from_tag("en");
/// let pair = SwatchPair::new("colours", "Colours", [PairWell::new("Foreground colour", fg), PairWell::new("Background colour", bg)], &numbers)
///     .active(active)
///     .swap_button("Swap colours")
///     .default_button("Default colours")
///     .show(ui);
/// active = pair.active;
/// if pair.swap {
///     std::mem::swap(&mut fg, &mut bg);
/// }
/// if let Some(p) = pair.edit.event.as_ref().and_then(|e| e.value()) {
///     *if pair.well == Well::Primary { &mut fg } else { &mut bg } = p.color;
/// }
/// # });
/// ```
///
/// - The host holds the active well ([`active`](SwatchPair::active)) and the colours. A click on
///   a well, or an arrow key while one has the focus, makes it active and returns it as
///   [`SwatchPairResponse::active`]; it is drawn in front, with a gap in the colour the pair sits
///   on round it ([`backdrop`](SwatchPair::backdrop)). The wells keep their places.
/// - A double-click, or Enter on the focused well, opens a [`ColorPopover`](super::ColorPopover)
///   popup under that well, which edits the well's colour as the popover does; a click outside
///   closes it. The response's [`edit`](SwatchPairResponse::edit) is the popup's, for
///   [`well`](SwatchPairResponse::well).
/// - Host-picker mode, as PhotoCraft's tool rail: [`popover(false)`](SwatchPair::popover) turns the
///   pair's own popup off, so a click, or Enter or Space on the focused well, reports
///   [`clicked`](SwatchPairResponse::clicked) and a double-click
///   [`double_clicked`](SwatchPairResponse::double_clicked) for the host to open its own picker.
///   [`click_activates(false)`](SwatchPair::click_activates) keeps the host's active well in front
///   whatever is clicked; the arrow keys then move the focus between the wells without changing
///   it, and the focused well is the Tab stop while the focus is in the pair.
/// - In a column that centres its items (the [`ToolRail`](crate::patterns::ToolRail) footer) the
///   pair centres itself in the column's width.
/// - The optional buttons ([`swap_button`](SwatchPair::swap_button),
///   [`default_button`](SwatchPair::default_button)) are icons in a row over the wells, the default
///   one at the start and the swap one at the end, as Photoshop's; a click reports
///   [`swap`](SwatchPairResponse::swap) or [`defaults`](SwatchPairResponse::defaults) for the
///   host to apply. The host binds the shortcuts (PhotoCraft's X and D) to the same actions.
/// - The wells are one Tab stop: the active well. Left and Up make the primary well active, Right
///   and Down the secondary one, and the focus follows (Left and Right swap right to left, as the
///   wells do). In a dialog's [`FieldTab`](crate::behavior::FieldTab) scope the pair is a
///   non-field control: Ctrl+Tab reaches it. The buttons are Tab stops of their own.
/// - AccessKit: a `Group` named `label` holding two `Button`s named by their wells' labels, with
///   the hex colour as value, selected for the active one, and the popup's `has_popup` and
///   expanded state only while the pair's own popover is on; then the buttons named by their
///   labels. A screen reader's Focus action on
///   the inactive well makes it active.
#[must_use = "show the pair with `show`"]
pub struct SwatchPair<'a> {
    id_salt: IdSalt,
    label: String,
    wells: [PairWell; 2],
    numbers: &'a LocaleNumbers,
    active: Well,
    swap: Option<String>,
    defaults: Option<String>,
    enabled: bool,
    backdrop: Option<Color32>,
    picker: Option<Box<dyn Fn(ColorPicker<'a>) -> ColorPicker<'a> + 'a>>,
    popover: bool,
    click_activates: bool,
}

/// What [`SwatchPair::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event and the actions"]
pub struct SwatchPairResponse {
    /// The pair's wells and buttons.
    pub response: Response,
    /// The active well after this frame: the host's, or the one the user chose.
    pub active: Well,
    /// The user made another well active this frame.
    pub activated: bool,
    /// The well clicked this frame: by the pointer, or Enter or Space on the focused well (with
    /// the pair's own popover on, Enter opens the popup instead). Reported whether or not the
    /// click made the well active.
    pub clicked: Option<Well>,
    /// The well double-clicked this frame.
    pub double_clicked: Option<Well>,
    /// The edit of the picker in the popup; the response is the group's. Without an event while
    /// no popup is open.
    pub edit: EditResponse<PickedColor>,
    /// The well `edit` belongs to: the one whose popup is open.
    pub well: Well,
    /// The swap button was pressed.
    pub swap: bool,
    /// The default button was pressed.
    pub defaults: bool,
    /// The picker's eyedropper button was pressed ([`ColorPopoverResponse::eyedropper_requested`]).
    pub eyedropper_requested: bool,
    /// The id of the picker in the open popup, for [`ColorPicker::sampled`].
    pub picker_id: Option<Id>,
    /// The well whose popup is open after this frame.
    pub open: Option<Well>,
}

impl<'a> SwatchPair<'a> {
    /// A pair named `label` (the group's accessible name) of `wells`, primary first, whose
    /// popups read and write their fields with `numbers`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, wells: [PairWell; 2], numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            wells,
            numbers,
            active: Well::Primary,
            swap: None,
            defaults: None,
            enabled: true,
            backdrop: None,
            picker: None,
            popover: true,
            click_activates: true,
        }
    }

    /// The active well, drawn in front; the primary one by default.
    pub fn active(mut self, active: Well) -> Self {
        self.active = active;
        self
    }

    /// Shows the swap button, named `label` (translated: "Swap colours").
    pub fn swap_button(mut self, label: impl Into<String>) -> Self {
        self.swap = Some(label.into());
        self
    }

    /// Shows the default button, named `label` (translated: "Default colours").
    pub fn default_button(mut self, label: impl Into<String>) -> Self {
        self.defaults = Some(label.into());
        self
    }

    /// The colour of the surface the pair sits on, which the gap round the front well shows
    /// (a toolbar's fill); `surface.panel` without it.
    pub fn backdrop(mut self, color: Color32) -> Self {
        self.backdrop = Some(color);
        self
    }

    /// Whether the pair takes input; a disabled one never opens its popups.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets up the picker in the popups as [`ColorPopover::picker`](super::ColorPopover::picker).
    pub fn picker(mut self, setup: impl Fn(ColorPicker<'a>) -> ColorPicker<'a> + 'a) -> Self {
        self.picker = Some(Box::new(setup));
        self
    }

    /// Whether a double-click or Enter opens the pair's own colour popup; on by default. Off, the
    /// host opens its own picker from [`SwatchPairResponse::clicked`] or
    /// [`double_clicked`](SwatchPairResponse::double_clicked), and Enter and Space on a well
    /// click it.
    pub fn popover(mut self, popover: bool) -> Self {
        self.popover = popover;
        self
    }

    /// Whether a click on a well makes it active; on by default. Off, the host's
    /// [`active`](SwatchPair::active) stays in front whatever is clicked, and the arrow keys move
    /// the focus between the wells without changing it.
    pub fn click_activates(mut self, click_activates: bool) -> Self {
        self.click_activates = click_activates;
        self
    }

    pub fn style(tokens: &UiTokens) -> SwatchPairStyle {
        let t = &tokens.component.color_picker;
        SwatchPairStyle { well: t.pair_well, step: t.pair_step, gap: t.pair_gap, ring: t.pair_ring }
    }

    pub fn show(self, ui: &mut Ui) -> SwatchPairResponse {
        let Self { id_salt, label, wells, numbers, active, swap, defaults, enabled, backdrop, picker, popover, click_activates } = self;
        let enabled = enabled && ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let tokens = Tokens::of(ui);
        let style = SwatchPair::style(&tokens);
        let chips = Vec2::splat(style.well + style.step);
        // The icon row over the chips, when there is an icon.
        let header = if swap.is_some() || defaults.is_some() { ICON + ICON_GAP } else { 0.0 };
        let block = vec2(chips.x, header + chips.y);
        let available = ui.available_rect_before_wrap();
        // In a column that centres its items (a tool rail's footer) the block is centred.
        let layout = ui.layout();
        let band = if layout.main_dir().is_vertical() && layout.horizontal_align() == Align::Center {
            Rect::from_center_size(pos2(available.center().x, available.min.y + block.y / 2.0), block)
        } else {
            Rect::from_min_size(available.min, vec2(available.width(), block.y))
        };
        let builder =
            UiBuilder::new().scope_id(id).max_rect(band).layout(direction::row(ui)).accessibility_role(Role::Group).accessibility_label(label);
        let inner = ui.scope_builder(builder, |ui| {
            let swatch = Swatch::style(&tokens, ControlSize::Md);
            let (area, _) = ui.allocate_exact_size(block, Sense::hover());
            let rects = well_rects(ui, &style, Rect::from_min_max(area.min + vec2(0.0, header), area.max));
            let icon_rects = icon_rects(ui, area);
            let ids = [Interactive::id_in(ui, (id, "primary")), Interactive::id_in(ui, (id, "secondary"))];
            let index = |w: Well| w as usize;

            let mut now = active;
            let focused = ids.iter().position(|&i| ui.memory(|m| m.has_focus(i)));
            let entered = enabled && popover && focused.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            // The well with the Tab stop: the active one, or while the arrows move the focus
            // without activating, the focused one.
            let mut stop = match focused {
                Some(i) if !click_activates => {
                    if i == 0 {
                        Well::Primary
                    } else {
                        Well::Secondary
                    }
                }
                _ => now,
            };
            if enabled && let Some(to) = roving_target(ui, &ids, focused) {
                stop = if to == 0 { Well::Primary } else { Well::Secondary };
                if click_activates {
                    now = stop;
                }
                ui.memory_mut(|m| m.request_focus(ids[index(stop)]));
            }

            // The inactive well senses first so that the active one, in front, takes the hits
            // where they overlap. Only one well is a Tab stop.
            let mut sensed = Vec::new();
            for well in [now.other(), now] {
                let spec = &wells[index(well)];
                let sense = if well == stop { Sense::click() } else { Sense::CLICK };
                let element = Interactive::with_id(ids[index(well)], Role::Button, spec.label.as_str())
                    .sense(sense)
                    .state_layer(false)
                    .focus_ring(false)
                    .selected(well == now)
                    .enabled(enabled)
                    .value(Swatch::hex(spec.color.to_rgba()));
                sensed.push((well, element.interact_at(ui, rects[index(well)])));
                // Not a Tab stop, but a screen reader may focus it: that makes it active.
                if well != stop && enabled {
                    ui.ctx().accesskit_node_builder(ids[index(well)], |node| node.add_action(Action::Focus));
                }
            }
            let mut toggled = entered.then_some(focused.map(|i| if i == 0 { Well::Primary } else { Well::Secondary })).flatten();
            let (mut clicked, mut double_clicked) = (None, None);
            for (well, s) in &sensed {
                let r = s.response();
                if enabled && r.clicked() {
                    clicked = Some(*well);
                    if click_activates {
                        now = *well;
                    }
                }
                // A double-click is reported on the second release, so the popup opens with the
                // button up and no later release over the well takes the focus back.
                if enabled && r.double_clicked() {
                    double_clicked = Some(*well);
                    if popover {
                        toggled = Some(*well);
                    }
                }
            }
            let activated = now != active;

            let button = |ui: &Ui, name: &str, label: &Option<String>, rect: Rect| {
                label.as_ref().map(|label| {
                    let element = Interactive::with_id(Interactive::id_in(ui, (id, name)), Role::Button, label.as_str())
                        .sense(Sense::click())
                        .state_layer(false)
                        .focus_ring(false)
                        .enabled(enabled);
                    element.interact_at(ui, rect)
                })
            };
            let defaults_sensed = button(ui, "defaults", &defaults, icon_rects[0]);
            let swap_sensed = button(ui, "swap", &swap, icon_rects[1]);
            let hovered = |s: &Option<Sensed>| s.as_ref().is_some_and(|s| s.response().hovered());
            let (defaults_hot, swap_hot) = (hovered(&defaults_sensed), hovered(&swap_sensed));
            let defaults_pressed = defaults_sensed.as_ref().is_some_and(|s| s.response().clicked());
            let swap_pressed = swap_sensed.as_ref().is_some_and(|s| s.response().clicked());

            let mut finished: Vec<(Well, Response)> = sensed.into_iter().map(|(well, s)| (well, s.finish(ui))).collect();
            finished.sort_by_key(|(well, _)| index(*well));
            let mut anchors: Vec<Response> = finished.into_iter().map(|(_, response)| response).collect();

            // Paint the back well, then the front one.
            let backdrop = backdrop.unwrap_or_else(|| tokens.color.surface.panel.to_egui());
            for well in [now.other(), now] {
                let spec = &wells[index(well)];
                let rect = rects[index(well)];
                paint_well(ui, &tokens, &swatch, &style, rect, spec, ((well == now).then_some(backdrop), enabled));
                anchors[index(well)] = anchors[index(well)].clone().tooltip(spec.label.as_str());
            }
            for well in [Well::Primary, Well::Secondary] {
                let anchor = &anchors[index(well)];
                FocusRing::paint_radius(ui, anchor.rect, swatch.radius, anchor);
            }

            let mut shown: Option<(Well, ColorPopoverResponse)> = None;
            for well in [Well::Primary, Well::Secondary].into_iter().filter(|_| popover) {
                let spec = &wells[index(well)];
                let setup = picker.as_deref().map(|setup| Box::new(setup) as Box<dyn FnOnce(ColorPicker<'a>) -> ColorPicker<'a> + '_>);
                let response =
                    popover::popup(ui, anchors[index(well)].clone(), toggled == Some(well), &spec.label, spec.color, numbers, enabled, setup);
                if response.open || response.edit.event.is_some() {
                    shown = Some((well, response));
                }
            }

            for (sensed, label, rect, hot, glyph) in [
                (defaults_sensed, &defaults, icon_rects[0], defaults_hot, Glyph::Defaults),
                (swap_sensed, &swap, icon_rects[1], swap_hot, Glyph::Swap),
            ] {
                let (Some(sensed), Some(label)) = (sensed, label) else { continue };
                let response = sensed.finish(ui).tooltip(label.as_str());
                paint_icon(ui, &tokens, &swatch, rect, (hot && enabled, enabled), glyph, backdrop);
                FocusRing::paint_radius(ui, rect.expand(2.0), swatch.radius, &response);
            }
            (now, activated, shown, swap_pressed, defaults_pressed, clicked, double_clicked)
        });
        let (now, activated, shown, swap, defaults, clicked, double_clicked) = inner.inner;
        let mut response = inner.response;
        if activated {
            response.mark_changed();
        }
        let (well, event, eyedropper_requested, picker_id, open) = match shown {
            Some((well, r)) => (well, r.edit.event, r.eyedropper_requested, r.picker_id, r.open.then_some(well)),
            None => (now, None, false, None, None),
        };
        SwatchPairResponse {
            edit: EditResponse::new(response.clone(), event),
            response,
            active: now,
            activated,
            clicked,
            double_clicked,
            well,
            swap,
            defaults,
            eyedropper_requested,
            picker_id,
            open,
        }
    }
}

/// The wells' rects in the block `area`: the primary at the start and top, the secondary one step
/// towards the end and down.
fn well_rects(ui: &Ui, style: &SwatchPairStyle, area: Rect) -> [Rect; 2] {
    let forward = direction::forward(ui);
    let side = Vec2::splat(style.well);
    let primary = if forward < 0.0 { pos2(area.max.x - style.well, area.min.y) } else { area.min };
    let primary = Rect::from_min_size(primary, side);
    [primary, primary.translate(vec2(style.step * forward, style.step))]
}

/// The well to make active from this frame's arrow keys while a well is focused (taking the keys),
/// or from a screen reader's Focus action on a well.
fn roving_target(ui: &Ui, ids: &[Id; 2], focused: Option<usize>) -> Option<usize> {
    let mut to = ids.iter().position(|&id| ui.input(|i| i.has_accesskit_action_request(id, Action::Focus)));
    if let Some(from) = focused {
        ui.memory_mut(|m| m.set_focus_lock_filter(ids[from], EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() }));
        let (back, forward) = if direction::is_rtl(ui) { (Key::ArrowRight, Key::ArrowLeft) } else { (Key::ArrowLeft, Key::ArrowRight) };
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, forward) || i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                to = Some(1);
            }
            if i.consume_key(Modifiers::NONE, back) || i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                to = Some(0);
            }
        });
    }
    to
}

/// Side of the icon boxes over the chips, and the gap between them and the chips.
const ICON: f32 = 13.0;
const ICON_GAP: f32 = 3.0;

#[derive(Clone, Copy)]
enum Glyph {
    Defaults,
    Swap,
}

/// The default button's box at the start of the icon row and the swap button's at the end.
fn icon_rects(ui: &Ui, area: Rect) -> [Rect; 2] {
    let side = Vec2::splat(ICON);
    let (left, right) = (Rect::from_min_size(area.min, side), Rect::from_min_size(pos2(area.max.x - ICON, area.min.y), side));
    if direction::is_rtl(ui) { [right, left] } else { [left, right] }
}

/// The corner radius of the chips and of the icons' hover boxes: the swatch's, at most 3.
fn chip_radius(swatch: &SwatchStyle) -> f32 {
    f32::from(swatch.radius.nw).min(3.0)
}

/// One glyph in its icon box, with a `hover` box behind it while hot. `state` is hot and enabled.
fn paint_icon(ui: &Ui, tokens: &UiTokens, swatch: &SwatchStyle, rect: Rect, (hot, enabled): (bool, bool), glyph: Glyph, chrome: Color32) {
    let painter = ui.painter();
    if hot {
        painter.rect_filled(rect.expand(2.0), chip_radius(swatch), tokens.color.state.hover.to_egui());
    }
    let ink = if hot { tokens.color.text.normal } else { tokens.color.text.muted }.to_egui();
    let ink = if enabled { ink } else { tokens.color.text.disabled.to_egui() };
    match glyph {
        Glyph::Swap => paint_swap(painter, rect.min, ink),
        Glyph::Defaults => {
            let small = Vec2::splat(7.0);
            let black = Rect::from_min_size(rect.min + vec2(1.0, 1.0), small);
            let white = Rect::from_min_size(rect.min + vec2(5.0, 5.0), small);
            painter.rect_filled(white, 1.0, Color32::WHITE);
            painter.rect_stroke(white, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            painter.rect_filled(black.expand(1.0), 1.5, chrome);
            painter.rect_filled(black, 1.0, Color32::BLACK);
            painter.rect_stroke(black, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
        }
    }
}

/// The swap glyph on a 13 by 13 box with its top left at `origin`: a quadratic shaft from (4, 4)
/// to (9, 9) that leaves horizontally and ends vertically, with a filled head at each end pointing
/// left and down.
fn paint_swap(painter: &Painter, origin: egui::Pos2, ink: Color32) {
    const START: egui::Pos2 = pos2(4.0, 4.0);
    const END: egui::Pos2 = pos2(9.0, 9.0);
    const CONTROL: egui::Pos2 = pos2(9.0, 4.0);
    const HEAD_LEN: f32 = 3.0;
    const HEAD_HALF: f32 = 2.8;
    let at = |p: egui::Pos2| origin + p.to_vec2();
    let curve: Vec<_> = (0..=10)
        .map(|i| {
            let t = i as f32 / 10.0;
            let (a, b) = (1.0 - t, t);
            at(pos2(a * a * START.x + 2.0 * a * b * CONTROL.x + b * b * END.x, a * a * START.y + 2.0 * a * b * CONTROL.y + b * b * END.y))
        })
        .collect();
    painter.add(Shape::line(curve, Stroke::new(1.6, ink)));
    for (point, out) in [(START, -(CONTROL - START).normalized()), (END, (END - CONTROL).normalized())] {
        let side = vec2(-out.y, out.x);
        let head = vec![at(point + out * HEAD_LEN), at(point + side * HEAD_HALF), at(point - side * HEAD_HALF)];
        painter.add(Shape::convex_polygon(head, ink, Stroke::NONE));
    }
}

/// The shapes of one well: for the front one the gap round it in the `gap` colour, then a filled
/// well framed by a light line inside a dark one, or a ring.
fn paint_well(
    ui: &Ui,
    tokens: &UiTokens,
    swatch: &SwatchStyle,
    style: &SwatchPairStyle,
    rect: Rect,
    spec: &PairWell,
    (gap, enabled): (Option<Color32>, bool),
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let radius = swatch.radius;
    if let Some(gap) = gap {
        let round = CornerRadius::same(radius.nw.min(3) + style.gap.round() as u8);
        painter.add(RectShape::filled(rect.expand(style.gap), round, gap));
    }
    let rgba = spec.color.to_rgba();
    if !spec.ring {
        let radius = CornerRadius::same(radius.nw.min(3));
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        if rgba.a() < 255 {
            painter.extend(checkerboard(rect, radius, swatch.checker, swatch.checker_cell));
        }
        painter.add(RectShape::filled(rect, radius, fade(rgba.to_egui())));
        // A light line inside a dark one, so any colour reads against the surface.
        painter.add(RectShape::stroke(
            rect,
            radius,
            Stroke::new(1.0, fade(tokens.color.text.normal.to_egui().gamma_multiply(0.9))),
            StrokeKind::Inside,
        ));
        painter.add(RectShape::stroke(rect, radius, Stroke::new(1.0, fade(tokens.component.field.border.rest.to_egui())), StrokeKind::Outside));
        return;
    }
    let fade = |c: egui::Color32| if enabled { c } else { c.gamma_multiply(0.5) };
    let hole = rect.shrink(style.ring);
    let hole_radius = CornerRadius {
        nw: radius.nw.saturating_sub(style.ring as u8),
        ne: radius.ne.saturating_sub(style.ring as u8),
        sw: radius.sw.saturating_sub(style.ring as u8),
        se: radius.se.saturating_sub(style.ring as u8),
    };
    let contrast = fade(swatch.inner_edge(Some(rgba)));
    let mut shapes: Vec<Shape> = vec![RectShape::stroke(rect, radius, Stroke::new(style.ring, fade(rgba.to_egui())), StrokeKind::Inside).into()];
    shapes.push(RectShape::stroke(rect.shrink(1.0), radius, Stroke::new(1.0, contrast), StrokeKind::Inside).into());
    shapes.push(RectShape::stroke(hole.expand(2.0), hole_radius, Stroke::new(1.0, contrast), StrokeKind::Inside).into());
    shapes.push(RectShape::stroke(rect, radius, Stroke::new(1.0, fade(swatch.edge)), StrokeKind::Inside).into());
    shapes.push(RectShape::stroke(hole, hole_radius, Stroke::new(1.0, fade(swatch.edge)), StrokeKind::Outside).into());
    painter.extend(shapes);
}
