//! [`Popover`]: a non-modal popup anchored to a control or a point.
//!
//! It holds any content, owns the popups opened from inside it (selects, menus, colour popovers,
//! other popovers), closes by rules the host picks, and can have a resize grip with a remembered
//! size.
//!
//! - [`Popover`]: the builder: anchor, size, close rules, [`show`](Popover::show).
//! - [`PopoverAnchor`]: a control or a point.
//! - [`PopoverSize`]: fit, fixed or resizable.
//! - [`PopoverClose`]: which rules close it.
//! - [`ResizeGrip`]: the corner grip, also usable alone.
//!
//! ```
//! use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
//! # egui::__run_test_ui(|ui| { let mut open = false;
//! let chip = ui.button("Brush");
//! if chip.clicked() {
//!     open = !open;
//! }
//! Popover::new("brush-picker", PopoverAnchor::Control(chip.clone()))
//!     .size(PopoverSize::Resizable { default: egui::vec2(300.0, 420.0), min: egui::vec2(230.0, 170.0) })
//!     .close(PopoverClose::default().enter(true))
//!     .toggle(chip.id)
//!     .label("Brush Preset picker")
//!     .show(ui, &mut open, |ui, _pop| {
//!         ui.label("Presets");
//!     });
//! # });
//! ```

mod anchor;
mod grip;
mod size;
mod stories;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;

use std::cell::Cell;

use egui::accesskit::{HasPopup, Role};
use egui::{
    Align2, Area, AsIdSalt, Context, FocusDirection, Frame, Id, IdSalt, Key, LayerId, Margin, Modifiers, Order, Rect, Sense, Shape, Ui, Vec2, pos2,
};

pub use anchor::PopoverAnchor;
pub use grip::{GripResponse, ResizeGrip};
pub use size::PopoverSize;
pub(crate) use stories::{COMPONENTS, stories};

use crate::direction;
use crate::popup_list::{self, Drawing};
use crate::primitives::{Surface, SurfaceKind};
use crate::theme::{Tokens, TokensOf, apply_popup_style};

/// Whether a popover or any other popup is open in this viewport. Single-key shortcuts and Tab
/// and Esc readers outside the popups leave their keys alone while it is.
pub fn any_open(ctx: &Context) -> bool {
    popup_list::any_open(ctx)
}

/// Why a popover closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CloseReason {
    /// A press outside the popover and its children.
    Outside,
    /// Esc, with no gesture or child open.
    Escape,
    /// Enter, with [`PopoverClose::enter`] on.
    Enter,
    /// The keyboard focus left the popover, its children and its anchor.
    FocusLeft,
    /// The contents called [`PopoverCtx::close`].
    Content,
    /// Another popover of the same [group](Popover::group) opened.
    Group,
    /// The popover it was opened from took a press or closed.
    Parent,
}

/// Which rules close a popover. Closing from the contents ([`PopoverCtx::close`]) is always
/// available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopoverClose {
    outside: bool,
    escape: bool,
    enter: bool,
    focus_left: bool,
}

impl Default for PopoverClose {
    /// A press outside, Esc, and the focus leaving; not Enter.
    fn default() -> Self {
        Self { outside: true, escape: true, enter: false, focus_left: true }
    }
}

impl PopoverClose {
    /// Close on a press outside the popover and all its children.
    pub fn outside(mut self, on: bool) -> Self {
        self.outside = on;
        self
    }

    /// Close on Esc while no gesture or child popup takes it.
    pub fn escape(mut self, on: bool) -> Self {
        self.escape = on;
        self
    }

    /// Close on Enter, for pickers that commit by closing.
    pub fn enter(mut self, on: bool) -> Self {
        self.enter = on;
        self
    }

    /// Close when the keyboard focus leaves the popover, its children and its anchor.
    pub fn focus_left(mut self, on: bool) -> Self {
        self.focus_left = on;
        self
    }
}

/// What the contents of a popover can ask of it.
pub struct PopoverCtx {
    id: Id,
    content_size: Vec2,
    close: Cell<bool>,
    initial: Cell<Option<Id>>,
}

impl PopoverCtx {
    /// Closes the popover after this pass (a double-click on a preset, a menu item).
    pub fn close(&self) {
        self.close.set(true);
    }

    /// The popover's id.
    pub fn id(&self) -> Id {
        self.id
    }

    /// The size of the content area: what a resizable popover's contents fill. For
    /// [`PopoverSize::Fit`] it is the size of the contents last pass.
    pub fn content_size(&self) -> Vec2 {
        self.content_size
    }

    /// The control that takes the keyboard focus when the popover opens (a colour popover's hex
    /// field). Without one, the first control in the popover does.
    pub fn initial_focus(&self, id: Id) {
        self.initial.set(Some(id));
    }
}

/// What [`Popover::show`] returns while the popover is open.
#[derive(Debug)]
#[non_exhaustive]
pub struct PopoverResponse<R> {
    /// What the contents returned.
    pub inner: R,
    /// The popover's rect: the card with its padding.
    pub rect: Rect,
    /// Why the popover closed in this pass, if it did; the host's `open` is then false.
    pub closed_by: Option<CloseReason>,
    /// The size of the content area.
    pub size: Vec2,
}

/// A non-modal popup anchored to a control or a point. See the [module](self) for an example.
///
/// - **Opening**: the host sets `open`. The press that opens it is never outside. Focus moves
///   into it: to [`PopoverCtx::initial_focus`], else the first control.
/// - **Children**: every popup opened from the contents is a child. A press in a child is not
///   outside; the popover stays open while the focus is in a child; Esc closes the innermost child
///   first; a press on the popover's own surface (a slider, the grip) closes the children; a close
///   of the popover closes them.
/// - **Closing**: [`PopoverClose`]. After Esc or Enter the focus goes back to a control anchor
///   unless it moved to another control.
/// - **Edits**: the popover makes none. Controls inside edit live, one step per gesture; a host
///   that wants a whole visit as one step wraps the contents in [`edit::scope`](crate::edit::scope).
/// - **AccessKit**: a control anchor has `has_popup` (dialog) and `expanded`; the popover is a
///   non-modal `Dialog` named `label`; the grip has no node.
#[must_use = "show the popover with `show`"]
pub struct Popover {
    id_salt: IdSalt,
    anchor: PopoverAnchor,
    size: PopoverSize,
    close: PopoverClose,
    toggle: Option<Id>,
    group: Option<Id>,
    label: String,
}

const STATE: &str = "craft-popover-state";
const GROUP: &str = "craft-popover-group";
const GROUP_CLOSE: &str = "craft-popover-group-close";

/// The popover's state between passes.
#[derive(Clone, Copy, Default)]
struct State {
    /// The frame it opened in.
    opened: u64,
    /// The focus was in the popover's family or on its anchor at the end of the last pass.
    had_focus: bool,
    /// Attempts to move the focus into it.
    focus_tries: u8,
    /// The side of the anchor it shows on: true for above.
    above: Option<bool>,
}

impl Popover {
    /// A popover named by `id_salt`, hanging from `anchor`.
    pub fn new(id_salt: impl AsIdSalt, anchor: PopoverAnchor) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            anchor,
            size: PopoverSize::Fit,
            close: PopoverClose::default(),
            toggle: None,
            group: None,
            label: String::new(),
        }
    }

    pub fn size(mut self, size: PopoverSize) -> Self {
        self.size = size;
        self
    }

    pub fn close(mut self, close: PopoverClose) -> Self {
        self.close = close;
        self
    }

    /// The control whose click toggles the popover: a press on it is not outside.
    pub fn toggle(mut self, id: Id) -> Self {
        self.toggle = Some(id);
        self
    }

    /// Opening a popover closes the open one of the same group (one picker at a time).
    pub fn group(mut self, id: Id) -> Self {
        self.group = Some(id);
        self
    }

    /// The accessible name of the popover, translated.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Shows the popover while `*open` and returns what `add` returned; `None` while closed. Call
    /// it every pass the popover should exist: it keeps nothing in the host. The popover clears
    /// `*open` when it closes itself.
    pub fn show<R>(self, ui: &mut Ui, open: &mut bool, add: impl FnOnce(&mut Ui, &PopoverCtx) -> R) -> Option<PopoverResponse<R>> {
        let Self { id_salt, anchor, size, close, toggle, group, label } = self;
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let state_id = id.with(STATE);
        let control = match &anchor {
            PopoverAnchor::Control(response) => Some(response.clone()),
            PopoverAnchor::Point(_) => None,
        };
        if control.as_ref().is_some_and(|r| !r.enabled()) {
            *open = false;
        }
        if let Some(control) = &control {
            ctx.accesskit_node_builder(control.id, |node| {
                node.set_has_popup(HasPopup::Dialog);
                node.set_expanded(*open);
            });
        }
        if !*open {
            shut(&ctx, id, state_id);
            return None;
        }

        let frame_nr = ctx.cumulative_frame_nr();
        let stored = ctx.data(|d| d.get_temp::<State>(state_id));
        let fresh = stored.is_none();
        let mut state = stored.unwrap_or(State { opened: frame_nr, ..State::default() });
        if let (true, Some(group)) = (fresh, group) {
            join_group(&ctx, group, id, frame_nr);
        }

        let tokens = Tokens::of(ui);
        let parts = tokens.component.popover;
        let screen = ctx.content_rect();
        let rtl = direction::is_rtl(ui);
        let shown = ctx.read_response(id).is_some() && !fresh;
        let family = popup_list::family(&ctx, id);
        let had_children = family.len() > 1;

        // Rules that close it before its contents run.
        let mut closing = take_group_close(&ctx, id, frame_nr);
        if popup_list::take_parent_close(&ctx, id) && !fresh {
            closing = closing.or(Some(CloseReason::Parent));
        }
        let press = read_press(&ctx, id, &family, toggle, close.outside && frame_nr != state.opened);
        closing = closing.or(press.closing);
        let owners: Vec<Id> = control.iter().map(|c| c.id).chain(toggle).collect();
        let inside = popup_list::focus_inside(&ctx, &owners, &family);
        let pressed = popup_list::pressed_into(&ctx, &family);
        let had = std::mem::replace(&mut state.had_focus, inside && !pressed) && shown;
        // Only a move of the focus to a widget elsewhere counts; egui clearing it (a press on
        // nothing, Esc) is left to the press and Esc rules.
        let moved_away = !inside && ctx.memory(|m| m.focused()).is_some();
        if close.focus_left && had && moved_away && !pressed {
            closing = closing.or(Some(CloseReason::FocusLeft));
        }

        // Size of the content area.
        let margin = parts.screen_margin;
        let room = (screen.size() - Vec2::splat(2.0 * margin)).max(Vec2::ZERO);
        let content_size = match size {
            PopoverSize::Fit => ctx.data(|d| d.get_temp::<Vec2>(id.with("fit-size"))).unwrap_or(Vec2::ZERO),
            PopoverSize::Fixed(v) => v,
            PopoverSize::Resizable { default, min } => size::read(&ctx, id, default, min, room),
        };

        // Where it goes. The pivot keeps the corner at the anchor whatever the size.
        let last = shown.then(|| ctx.memory(|m| m.area_rect(id))).flatten();
        let (pos, pivot) = placement(&anchor, rtl, screen, last, &mut state.above);

        let surface = Surface::style(&tokens, SurfaceKind::Popup);
        let inner_margin = Margin::same((parts.padding + popup_list::edge_width(&surface)).ceil() as i8);
        let pop = PopoverCtx { id, content_size, close: Cell::new(false), initial: Cell::new(None) };
        let drawing = Drawing::enter(&ctx, id);
        let area = Area::new(id)
            .order(Order::Foreground)
            .fixed_pos(pos)
            .pivot(pivot)
            .constrain_to(screen)
            .sense(Sense::CLICK)
            .role(Role::Dialog)
            .accessible_name(label);
        let drawn = area.show(&ctx, |ui| {
            apply_popup_style(ui);
            let entry = (shown && state.focus_tries < 3 && !focus_in(&ctx, id)).then(|| {
                state.focus_tries += 1;
                begin_focus_entry(&ctx, id)
            });
            let slot = ui.painter().add(Shape::Noop);
            let framed = Frame::NONE.inner_margin(inner_margin).show(ui, |ui| {
                ui.set_max_size(room);
                match size {
                    PopoverSize::Fit => {}
                    PopoverSize::Fixed(v) => {
                        ui.set_width(v.x);
                        ui.set_height(v.y);
                    }
                    PopoverSize::Resizable { .. } => {
                        ui.set_width(content_size.x);
                        ui.set_min_height(content_size.y);
                    }
                }
                let inner = add(ui, &pop);
                (inner, ui.min_rect().size())
            });
            if let Some(previous) = entry
                && !end_focus_entry(&ctx, id, previous)
            {
                state.focus_tries = u8::MAX;
            }
            let rect = framed.response.rect;
            ui.painter().set(slot, Shape::Vec(surface.shapes(rect, ui.pixels_per_point())));
            let (inner, used) = framed.inner;
            let resized = match size {
                PopoverSize::Resizable { min, .. } => grip_resize(ui, id, rect, content_size, min, room),
                _ => None,
            };
            (inner, rect, used, resized)
        });
        drop(drawing);
        let (inner, rect, used, resized) = drawn.inner;
        if matches!(size, PopoverSize::Fit) {
            ctx.data_mut(|d| d.insert_temp(id.with("fit-size"), used));
        }

        // Rules that close it after its contents ran.
        let mut reason = closing;
        if pop.close.get() {
            reason = reason.or(Some(CloseReason::Content));
        }
        if !had_children && !fresh {
            if close.escape && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                reason = reason.or(Some(CloseReason::Escape));
            }
            if close.enter && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                reason = reason.or(Some(CloseReason::Enter));
            }
        }
        if let Some(focus) = pop.initial.get()
            && state.focus_tries < 3
            && shown
            && !focus_in(&ctx, id)
        {
            ctx.memory_mut(|m| m.request_focus(focus));
        }

        let layer = LayerId::new(Order::Foreground, id);
        for child in family.iter().skip(1) {
            ctx.set_sublayer(layer, LayerId::new(Order::Foreground, *child));
        }

        let size_now = resized.unwrap_or(match size {
            PopoverSize::Fit => used,
            _ => content_size,
        });
        match reason {
            Some(reason) => {
                *open = false;
                if matches!(reason, CloseReason::Escape | CloseReason::Enter)
                    && let Some(control) = &control
                    && focus_free_or_inside(&ctx, &family)
                {
                    ctx.memory_mut(|m| m.request_focus(control.id));
                }
                shut(&ctx, id, state_id);
            }
            None => {
                ctx.data_mut(|d| d.insert_temp(state_id, state));
                popup_list::record_open(&ctx, id, true);
            }
        }
        Some(PopoverResponse { inner, rect, closed_by: reason, size: size_now })
    }
}

/// Registers `id` as the open popover of `group` and asks the one it replaces to close.
fn join_group(ctx: &Context, group: Id, id: Id, frame_nr: u64) {
    let previous = ctx.data_mut(|d| d.get_temp::<Id>(group.with(GROUP)).filter(|&p| p != id));
    ctx.data_mut(|d| d.insert_temp(group.with(GROUP), id));
    if let Some(previous) = previous {
        ctx.data_mut(|d| d.insert_temp(previous.with(GROUP_CLOSE), frame_nr));
    }
}

/// [`CloseReason::Group`] when another popover of the group opened in this frame or the last.
fn take_group_close(ctx: &Context, id: Id, frame_nr: u64) -> Option<CloseReason> {
    let asked = ctx.data_mut(|d| d.remove_temp::<u64>(id.with(GROUP_CLOSE)));
    asked.filter(|&frame| frame_nr <= frame + 1).map(|_| CloseReason::Group)
}

/// What a press this pass means for the popover `id`.
struct Press {
    closing: Option<CloseReason>,
}

/// Reads the press of this pass: a press on the popover's own surface closes its children; a
/// press outside the popover, its children and the toggle control closes it when `outside`.
fn read_press(ctx: &Context, id: Id, family: &[Id], toggle: Option<Id>, outside: bool) -> Press {
    let Some(pos) = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()) else {
        return Press { closing: None };
    };
    let hit = ctx.layer_id_at(pos);
    let in_own = hit.is_some_and(|l| l.id == id);
    // A plain egui popup is any foreground layer that no craft-ui popup owns; a parent popover
    // owns its layer, so a press on it is outside its child.
    let foreign = hit.is_some_and(|l| l.id != id && l.order == Order::Foreground && !family.contains(&l.id) && !popup_list::is_open_popup(ctx, l.id));
    let in_child = foreign || hit.is_some_and(|l| l.id != id && family.contains(&l.id));
    let on_toggle = toggle.and_then(|t| ctx.read_response(t)).is_some_and(|r| r.rect.contains(pos));
    if in_own && !popup_list::child_anchors(ctx, id).iter().any(|r| r.contains(pos)) {
        popup_list::close_children(ctx, id);
    }
    let closing = (outside && !in_own && !in_child && !on_toggle).then_some(CloseReason::Outside);
    Press { closing }
}

/// Where the popover goes: the position and the pivot that keeps its corner at the anchor
/// whatever its size. A control anchor keeps the side (`above`) it first chose.
fn placement(anchor: &PopoverAnchor, rtl: bool, screen: Rect, last: Option<Rect>, above: &mut Option<bool>) -> (egui::Pos2, Align2) {
    match anchor {
        PopoverAnchor::Control(r) => {
            let a = r.rect;
            *above = above.or_else(|| {
                last.map(|l| {
                    let (below, over) = (screen.bottom() - a.bottom(), a.top() - screen.top());
                    below < l.height() && over > below
                })
            });
            let x = if rtl { a.right() } else { a.left() };
            match (above.unwrap_or(false), rtl) {
                (false, false) => (pos2(x, a.bottom()), Align2::LEFT_TOP),
                (false, true) => (pos2(x, a.bottom()), Align2::RIGHT_TOP),
                (true, false) => (pos2(x, a.top()), Align2::LEFT_BOTTOM),
                (true, true) => (pos2(x, a.top()), Align2::RIGHT_BOTTOM),
            }
        }
        PopoverAnchor::Point(p) => (*p, if rtl { Align2::RIGHT_TOP } else { Align2::LEFT_TOP }),
    }
}

/// Shows the resize grip over `rect` and returns the new content size while it is dragged.
fn grip_resize(ui: &mut Ui, id: Id, rect: Rect, content_size: Vec2, min: Vec2, room: Vec2) -> Option<Vec2> {
    let ctx = ui.ctx().clone();
    let grip = ResizeGrip::new(id.with("grip")).show(ui, rect);
    let start_id = id.with("grip-start");
    if grip.started {
        ctx.data_mut(|d| d.insert_temp(start_id, content_size));
    }
    let (growth, start) = (grip.growth?, ctx.data(|d| d.get_temp::<Vec2>(start_id))?);
    let grown = size::clamp(start + growth, min, room);
    size::write(&ctx, id, grown);
    Some(grown)
}

/// Forgets the state of the popover `id` and closes its children, once, after it was open.
fn shut(ctx: &Context, id: Id, state_id: Id) {
    if ctx.data_mut(|d| d.remove_temp::<State>(state_id)).is_some() {
        popup_list::close_children(ctx, id);
        popup_list::record_open(ctx, id, false);
    }
}

/// Whether the keyboard focus is on a widget in the popover's layer, as of the last pass.
fn focus_in(ctx: &Context, id: Id) -> bool {
    ctx.memory(|m| m.focused()).and_then(|f| ctx.read_response(f)).is_some_and(|r| r.layer_id.id == id)
}

/// Starts handing the keyboard focus to the first control drawn after this call: a stand-in has
/// the focus and egui passes it on to the next widget that wants it. Returns the focus to restore
/// when there is no such control in the popover; pair it with [`end_focus_entry`] after the
/// contents.
fn begin_focus_entry(ctx: &Context, id: Id) -> Option<Id> {
    let stand_in = id.with("focus-entry");
    ctx.memory_mut(|m| {
        let previous = m.focused();
        m.request_focus(stand_in);
        m.move_focus(FocusDirection::Next);
        m.interested_in_focus(stand_in, LayerId::new(Order::Foreground, id));
        previous
    })
}

/// Ends [`begin_focus_entry`] after the contents ran. A marker control after them catches the
/// hand-off when no control in the popover took it; the focus then goes back to where it was.
/// Returns whether the focus is in the popover.
fn end_focus_entry(ctx: &Context, id: Id, previous: Option<Id>) -> bool {
    let tail = id.with("focus-exit");
    ctx.memory_mut(|m| {
        m.interested_in_focus(tail, LayerId::new(Order::Foreground, id));
        m.move_focus(FocusDirection::None);
        let caught = m.focused() == Some(tail);
        if caught {
            match previous {
                Some(previous) => m.request_focus(previous),
                None => m.surrender_focus(tail),
            }
        }
        !caught
    })
}

/// Whether no control has the focus, or the focused one is in the popover or a child: a close
/// that leaves the focus there gives it back to the anchor.
fn focus_free_or_inside(ctx: &Context, family: &[Id]) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else { return true };
    ctx.read_response(focused).is_none_or(|r| family.contains(&r.layer_id.id))
}
