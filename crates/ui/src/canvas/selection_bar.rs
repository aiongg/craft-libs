//! [`SelectionBar`]: a small floating toolbar over a selection on the canvas (Photoshop's
//! Contextual Task Bar; WordCraft's mini toolbar over selected text).

use egui::accesskit::Role;
use egui::{Area, AsIdSalt, CursorIcon, Id, Key, Order, Pos2, Rect, Sense, Ui, Vec2, WidgetInfo, pos2, vec2};

use crate::controls::{ControlSize, IconButton};
use crate::direction;
use crate::focus::{Region, RegionFocus};
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::{Gap, Surface, SurfaceKind};
use crate::theme::{ToEgui, Tokens, TokensOf};

const GRIP_DOT_DIAMETER: f32 = 1.0;
const GRIP_PITCH: f32 = 4.0;
/// The bar's width, in `size.control.md`, until the first pass has measured it.
const FIRST_WIDTH_IN_CONTROLS: f32 = 14.0;

/// The bar's measures, from the semantic tokens.
#[derive(Clone, Copy)]
pub(super) struct BarMetrics {
    height: f32,
    gap_above: f32,
    gap_below: f32,
    inset: f32,
    first_width: f32,
    grip_width: f32,
    grip_height: f32,
    grip_dot: f32,
    grip_pitch: f32,
}

impl BarMetrics {
    pub(super) fn of(tokens: &craft_theme::Tokens) -> Self {
        Self {
            height: tokens.size.bar.options,
            gap_above: tokens.space.gap.stack,
            gap_below: tokens.space.pad.panel,
            inset: tokens.space.gap.stack,
            first_width: tokens.size.control.md * FIRST_WIDTH_IN_CONTROLS,
            grip_width: tokens.size.icon.sm,
            grip_height: tokens.size.icon.md,
            grip_dot: GRIP_DOT_DIAMETER,
            grip_pitch: GRIP_PITCH,
        }
    }
}

/// The position the user dragged the bar to, relative to the canvas's top left.
#[derive(Clone, Copy, Default)]
struct Pin(Pos2);

/// The ⋯ menu's commands.
#[derive(Clone, Copy, PartialEq)]
enum More {
    Hide,
    Reset,
}

/// `at` moved as little as needed for a bar `width` wide to sit inside `canvas`.
fn inside(at: Pos2, width: f32, canvas: Rect, bar: &BarMetrics) -> Pos2 {
    let BarMetrics { height, inset, .. } = *bar;
    let x = at.x.min(canvas.max.x - width - inset).max(canvas.min.x + inset);
    let y = at.y.min(canvas.max.y - height - inset).max(canvas.min.y + inset);
    pos2(x, y)
}

/// The bar's top left corner next to `selection`: below it, or above it when `above`; on the
/// other side when that does not fit, and at the canvas's bottom when neither does.
pub(super) fn place(canvas: Rect, selection: Rect, width: f32, above: bool, bar: &BarMetrics) -> Pos2 {
    let BarMetrics { height, gap_above, gap_below, inset, .. } = *bar;
    let x = (selection.center().x - width / 2.0).clamp(canvas.min.x + inset, (canvas.max.x - width - inset).max(canvas.min.x + inset));
    let gap = if above { gap_above } else { gap_below };
    let over = selection.min.y - height - gap;
    let under = selection.max.y + gap;
    let fits_over = over >= canvas.min.y + inset;
    let fits_under = under + height <= canvas.max.y - inset;
    let y = match (above, fits_over, fits_under) {
        (false, _, true) | (true, false, true) => under,
        (false, true, false) | (true, true, _) => over,
        (_, false, false) => (canvas.max.y - inset - height).max(canvas.min.y + inset),
    };
    pos2(x, y)
}

/// The arrow keys pressed this pass as a direction (right and down positive), consumed.
fn grip_arrows(ctx: &egui::Context) -> Vec2 {
    ctx.input_mut(|i| {
        let mut step = |key| f32::from(i.consume_key(egui::Modifiers::NONE, key));
        vec2(step(Key::ArrowRight) - step(Key::ArrowLeft), step(Key::ArrowDown) - step(Key::ArrowUp))
    })
}

/// What [`SelectionBar::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectionBarResponse<R> {
    /// The body closure's result; `None` while the bar is hidden.
    pub inner: Option<R>,
    /// Where the bar was drawn.
    pub rect: Option<Rect>,
    /// The user dragged the grip: the bar stays at this screen position (relative to the
    /// canvas) until [`SelectionBar::unpin`]. The host may store it.
    pub pinned_at: Option<Pos2>,
    /// The user hid the bar from its ⋯ menu (Hide Bar); the host turns its preference off.
    pub hide_requested: bool,
    /// Reset Bar Position in the ⋯ menu: the bar follows the selection again; the host clears the
    /// position it stored.
    pub position_reset: bool,
}

/// A floating bar of the host's controls above a selection's screen rect.
///
/// ```no_run
/// use craft_ui::canvas::SelectionBar;
/// # fn demo(ui: &mut egui::Ui, canvas: egui::Rect, selection: Option<egui::Rect>) {
/// SelectionBar::new("selection-bar").show(ui, canvas, selection, |ui| {
///     let _ = ui.button("Select Subject");
/// });
/// # }
/// ```
///
/// - **Placement:** Photoshop's Contextual Task Bar: centred under `selection`, 14 pt below it;
///   above it (14 pt) when it would pass the canvas's bottom edge less 8 pt; at the canvas's
///   bottom less 8 pt when neither fits; x clamped 8 pt inside the canvas. Its width is the
///   content's, measured on the previous frame (420 pt before the first). [`SelectionBar::above`]
///   puts it 8 pt above the selection instead, flipping below when tight (Word's mini toolbar).
///   Hidden with no selection, and while a pointer button is down outside any egui area (a drag
///   on the canvas) unless the bar's grip is being dragged. A pinned bar stays where the user
///   dragged it, kept inside the canvas.
/// - **Look:** a popup surface with the toolbar's controls at the small control size, a grip at the
///   start and a ⋯ menu at the end (Hide Bar, Reset Bar Position). Themes change colours only.
/// - **Keyboard:** F6 lists it as a region while it shows ([`crate::focus::RegionFocus`], region
///   `selectionBar`); Esc clears the focus, which leaves the keys to the canvas.
/// - **AccessKit:** a `Toolbar` named by [`SelectionBarLabels::name`] ("Selection").
#[must_use = "show the bar with `show`"]
pub struct SelectionBar {
    id: Id,
    above: bool,
    pinned: Option<Pos2>,
    labels: Option<SelectionBarLabels>,
}

impl SelectionBar {
    /// A bar keyed `id_salt`, below the selection, with the default labels.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id: Id::unique(id_salt), above: false, pinned: None, labels: None }
    }

    /// Translated labels; default the installed words.
    pub fn labels(mut self, labels: SelectionBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Places the bar 8 pt above the selection, flipping below when tight.
    // wordcraft: crates/ui-egui/src/mini_toolbar.rs:12 `VGAP` (8 pt).
    pub fn above(mut self, above: bool) -> Self {
        self.above = above;
        self
    }

    /// Keeps the bar at a stored position (relative to the canvas rect's top left).
    pub fn pinned(mut self, at: Option<Pos2>) -> Self {
        self.pinned = at;
        self
    }

    /// Forgets the pinned position; the bar follows the selection again.
    pub fn unpin(ui: &Ui, id_salt: impl AsIdSalt) {
        let pin = Id::unique(id_salt).with("pin");
        ui.ctx().data_mut(|d| d.remove_temp::<Pin>(pin));
    }

    /// Draws the bar over `canvas` above `selection` (screen rects) with `add`'s controls.
    // designcraft: crates/ui-egui/src/taskbar.rs:18 `show` (placement :34-46, hidden while a
    // button is down off egui :22, `HEIGHT` 36 pt :11); wordcraft: crates/ui-egui/src/mini_toolbar.rs:166
    // `show`.
    pub fn show<R>(self, ui: &mut Ui, canvas: Rect, selection: Option<Rect>, add: impl FnOnce(&mut Ui) -> R) -> SelectionBarResponse<R> {
        let ctx = ui.ctx().clone();
        let hidden = SelectionBarResponse { inner: None, rect: None, pinned_at: None, hide_requested: false, position_reset: false };
        let grip_id = self.id.with("grip");
        let pin_id = self.id.with("pin");
        let dragging = ctx.dragged_id() == Some(grip_id);
        // Two separate context calls: `is_pointer_over_egui` inside an `input` closure deadlocks.
        let pressing_canvas = ctx.input(|i| i.pointer.any_down()) && !ctx.is_pointer_over_egui();
        let Some(selection) = selection else { return hidden };
        if pressing_canvas && !dragging {
            return hidden;
        }

        let labels = self.labels.clone().unwrap_or_else(|| SelectionBarLabels::of_ctx(&ctx));
        let tokens = Tokens::of(ui);
        let bar = BarMetrics::of(&tokens);
        let width = ctx.memory(|m| m.area_rect(self.id)).map_or(bar.first_width, |r| r.width());
        let pinned = ctx.data(|d| d.get_temp::<Pin>(pin_id)).map(|p| p.0).or(self.pinned);
        let at = match pinned {
            Some(p) => inside(canvas.min + p.to_vec2(), width, canvas, &bar),
            None => place(canvas, selection, width, self.above, &bar),
        };

        let mut pinned_at = None;
        let mut hide_requested = false;
        let mut position_reset = false;
        let shown = Area::new(self.id).order(Order::Middle).fixed_pos(at).role(Role::Toolbar).accessible_name(labels.name.clone()).show(&ctx, |ui| {
            crate::theme::apply_popup_style(ui);
            Surface::new(SurfaceKind::Popup)
                .show(ui, |ui| {
                    ui.set_height(bar.height);
                    ui.spacing_mut().item_spacing.x = Gap::Inline.points(&tokens);
                    ui.with_layout(direction::row(ui), |ui| {
                        ui.add_space(tokens.space.pad.control);
                        let (cell, _) = ui.allocate_exact_size(vec2(bar.grip_width, bar.grip_height), Sense::hover());
                        let grip = ui.interact(cell, grip_id, Sense::drag());
                        grip.widget_info(|| WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), &labels.grip));
                        let ink = if grip.hovered() || grip.dragged() { tokens.color.text.normal } else { tokens.color.text.muted };
                        for column in [-0.5, 0.5] {
                            for row in [-1.0, 0.0, 1.0] {
                                let centre = cell.center() + vec2(column, row) * bar.grip_pitch;
                                ui.painter().circle_filled(centre, bar.grip_dot, ink.to_egui());
                            }
                        }
                        // With the focus on the grip the arrow keys move the bar by half its height.
                        let keys = if grip.has_focus() {
                            let arrows = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() };
                            ui.ctx().memory_mut(|m| m.set_focus_lock_filter(grip_id, arrows));
                            grip_arrows(ui.ctx()) * (bar.height / 2.0)
                        } else {
                            Vec2::ZERO
                        };
                        if grip.dragged() || keys != Vec2::ZERO {
                            if grip.dragged() {
                                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                            }
                            let to = (inside(at + grip.drag_delta() + keys, width, canvas, &bar) - canvas.min).to_pos2();
                            ui.ctx().data_mut(|d| d.insert_temp(pin_id, Pin(to)));
                            pinned_at = Some(to);
                        } else if grip.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::Grab);
                        }

                        let inner = add(ui);

                        let more = ui.add(IconButton::new(Icon::More, labels.more.clone()).size(ControlSize::Sm).id_salt("more"));
                        let entries: Vec<MenuEntry<More>> = vec![
                            MenuItem::new(labels.hide.clone(), More::Hide).into(),
                            MenuItem::new(labels.reset_position.clone(), More::Reset).enabled(pinned.is_some()).into(),
                        ];
                        match PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &more).chosen {
                            Some(More::Hide) => hide_requested = true,
                            Some(More::Reset) => position_reset = true,
                            None => {}
                        }
                        inner
                    })
                    .inner
                })
                .inner
        });
        if position_reset {
            ctx.data_mut(|d| d.remove_temp::<Pin>(pin_id));
            pinned_at = None;
        }
        let rect = shown.response.rect;
        RegionFocus::new(&ctx).region(&ctx, Region::new("selectionBar", labels.name), rect, None);
        SelectionBarResponse { inner: Some(shown.inner), rect: Some(rect), pinned_at, hide_requested, position_reset }
    }
}

crate::words! {
    /// The selection bar's labels, English by default.
    #[non_exhaustive]
    pub struct SelectionBarLabels {
        name: "selectionBar.name" = "Selection";
        grip: "selectionBar.grip" = "Move Bar";
        more: "selectionBar.more" = "More";
        hide: "selectionBar.hide" = "Hide Bar";
        reset_position: "selectionBar.resetPosition" = "Reset Bar Position";
    }
}
