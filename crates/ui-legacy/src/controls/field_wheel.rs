//! The mouse wheel on number fields, as PhotoCraft's (`field_wheel.rs` there): one step a notch,
//! ten with Shift. Sliders with [`wheel_unfocused`](super::Slider::wheel_unfocused) and a
//! select's closed box with [`wheel_steps`](super::Select::wheel_steps) follow the rules for a
//! field without the focus.
//!
//! - While a field has the keyboard focus, the wheel steps it wherever the pointer is.
//!   [`FieldWheel`], a plugin [`theme::install`](crate::theme::install) adds, takes the wheel
//!   out of each pass's input before anything is laid out, so no panel scrolls with it.
//!   Alt+wheel and Ctrl/Cmd+wheel stay the view's, as does a plain sideways scroll.
//! - Over a field without the focus, the wheel steps it too, but only a gesture that starts over
//!   it: a panel being scrolled keeps scrolling when a field passes under the pointer. A gesture
//!   that leaves the field is the view's from then on, also when it comes back over the field.

use egui::plugin::Plugin;
use egui::{Context, Event, Id, MouseWheelUnit, Rect, Ui, Vec2};

/// Wheel events this close together (seconds) are one gesture.
const GESTURE_GAP: f64 = 0.3;

/// Who a wheel gesture belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Owner {
    /// It started in this pass and no field has taken it yet.
    New,
    Field(Id),
    /// It started where no field took it (a panel, the canvas).
    Elsewhere,
}

#[derive(Clone, Copy, Debug)]
struct WheelGesture {
    /// Input time of its last event.
    last: f64,
    owner: Owner,
}

/// The focused field that takes the wheel, and the pass that recorded it.
#[derive(Clone, Copy, Debug)]
struct Focused {
    id: Id,
    pass: u64,
}

/// Where this viewport's state is kept: a field in one viewport doesn't take another's wheel.
/// Read before locking the context's data, which these ids can't be made under.
struct Keys {
    gesture: Id,
    focused: Id,
    taken: Id,
}

fn keys(ctx: &Context) -> Keys {
    let base = Id::unique("craft_ui::field_wheel").with(ctx.viewport_id());
    Keys { gesture: base.with("gesture"), focused: base.with("focused"), taken: base.with("taken") }
}

/// A wheel event a field takes: plain or Shift, not the view's Alt or Ctrl/Cmd zoom, nor a plain
/// sideways scroll (a trackpad swipe pans the view; Shift+wheel can arrive sideways). An event
/// with no delta at all counts: macOS starts a trackpad gesture with one, and it is what lets a
/// field under the pointer claim the gesture.
fn for_field(event: &Event) -> bool {
    matches!(event, Event::MouseWheel { delta, modifiers, .. }
        if (delta.y != 0.0 || delta.x == 0.0 || modifiers.shift) && !modifiers.alt && !modifiers.ctrl && !modifiers.command && !modifiers.mac_cmd)
}

/// Adds [`FieldWheel`] to `ctx`, once.
pub(crate) fn install(ctx: &Context) {
    if ctx.plugin_opt::<FieldWheel>().is_none() {
        ctx.add_plugin(FieldWheel);
    }
}

/// Hands each pass's wheel to the focused number field, before anything is laid out, and
/// follows which gesture the wheel is in.
struct FieldWheel;

impl Plugin for FieldWheel {
    fn debug_name(&self) -> &'static str {
        "craft_ui::controls::field_wheel"
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        begin_pass(ui.ctx());
    }
}

fn begin_pass(ctx: &Context) {
    let k = keys(ctx);
    ctx.data_mut(|d| d.remove::<Vec<Event>>(k.taken));
    let pass = ctx.cumulative_pass_nr();
    let focused = ctx.data(|d| d.get_temp::<Focused>(k.focused)).filter(|f| f.pass + 1 >= pass && ctx.memory(|m| m.has_focus(f.id)));
    let (now, wheel) = ctx.input(|i| (i.time, i.events.iter().any(|e| matches!(e, Event::MouseWheel { .. }))));
    if let Some(Focused { id, .. }) = focused {
        let (taken, view_wheel): (Vec<Event>, bool) = ctx.input_mut(|i| {
            let (taken, rest) = std::mem::take(&mut i.events).into_iter().partition(for_field);
            i.events = rest;
            (taken, i.events.iter().any(|e| matches!(e, Event::MouseWheel { .. })))
        });
        if wheel {
            // The latest wheel is the view's when any of it was left to the view (Alt, Ctrl/Cmd).
            let owner = if view_wheel { Owner::Elsewhere } else { Owner::Field(id) };
            ctx.data_mut(|d| d.insert_temp(k.gesture, WheelGesture { last: now, owner }));
        }
        if !taken.is_empty() {
            ctx.data_mut(|d| d.insert_temp(k.taken, taken));
        }
        // egui spreads each notch over a few passes: the tail of the field's own notches scrolls
        // nothing, while the view's keeps all of its own.
        if ctx.data(|d| d.get_temp::<WheelGesture>(k.gesture)).is_some_and(|g| g.owner == Owner::Field(id)) {
            ctx.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
        return;
    }
    if !wheel {
        return;
    }
    let gesture = ctx.data(|d| d.get_temp::<WheelGesture>(k.gesture));
    let owner = match gesture {
        Some(g) if now - g.last <= GESTURE_GAP => match g.owner {
            // No field took it when it started.
            Owner::New => Owner::Elsewhere,
            owner => owner,
        },
        _ => Owner::New,
    };
    ctx.data_mut(|d| d.insert_temp(k.gesture, WheelGesture { last: now, owner }));
}

/// Wheel lines for the field `id` in `rect` in this pass (up positive, a Shift notch counting
/// `shift` lines). While the field is `focused`: the wheel [`FieldWheel`] took for it at the
/// start of the pass (from the pass after the one that first finds it focused here, which
/// records it). Without the focus: a gesture that started with the pointer over it. A field that
/// doesn't `act` (disabled, being dragged, showing text that is no number) takes none, and the
/// wheel over it is the view's from the next pass on. Number fields and sliders count ten lines a
/// Shift notch; a select's closed box, which steps through options, one.
pub(super) fn notches(ui: &Ui, id: Id, focused: bool, rect: Rect, acts: bool, shift: f32) -> f64 {
    if !acts {
        return 0.0;
    }
    let ctx = ui.ctx();
    let k = keys(ctx);
    let events: Vec<Event> = if focused {
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| d.insert_temp(k.focused, Focused { id, pass }));
        ctx.data_mut(|d| d.remove_temp::<Vec<Event>>(k.taken)).unwrap_or_default()
    } else {
        hovered(ui, id, rect)
    };
    let wheel: Vec<(MouseWheelUnit, Vec2, bool)> = events
        .iter()
        .filter_map(|e| match e {
            Event::MouseWheel { unit, delta, modifiers, .. } if for_field(e) => Some((*unit, *delta, modifiers.shift)),
            _ => None,
        })
        .collect();
    f64::from(count(ui, id.with("craft_ui::field_wheel::lines"), &wheel, shift))
}

/// This pass's wheel events for the field `id` without the focus, in `rect`: those of a gesture
/// that started with the pointer over it and hasn't left it.
fn hovered(ui: &Ui, id: Id, rect: Rect) -> Vec<Event> {
    let ctx = ui.ctx();
    let k = keys(ctx);
    let Some(mut g) = ctx.data(|d| d.get_temp::<WheelGesture>(k.gesture)) else { return Vec::new() };
    let (events, wheel): (Vec<Event>, bool) =
        ui.input(|i| (i.events.iter().filter(|e| for_field(e)).cloned().collect(), i.events.iter().any(|e| matches!(e, Event::MouseWheel { .. }))));
    let over = ui.rect_contains_pointer(rect);
    // Wheel the field doesn't take (a sideways swipe after a zero-delta start), or any wheel
    // with the pointer off the field, hands the rest of the gesture to the view: coming back
    // over the field in the same gesture doesn't step it.
    if g.owner == Owner::Field(id) && wheel && (events.is_empty() || !over) {
        g.owner = Owner::Elsewhere;
        ctx.data_mut(|d| d.insert_temp(k.gesture, g));
        return Vec::new();
    }
    if !over {
        return Vec::new();
    }
    match g.owner {
        Owner::New if !events.is_empty() => {
            g.owner = Owner::Field(id);
            ctx.data_mut(|d| d.insert_temp(k.gesture, g));
        }
        Owner::Field(owner) if owner == id => {}
        _ => return Vec::new(),
    }
    // Nothing under the field scrolls with the wheel that steps it, smoothed tail included.
    ctx.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
    events
}

/// Whole lines in `wheel` (unit, delta, Shift), with what is left of a line carried in `key` to
/// the next pass; a Shift notch counts `shift` lines. Points count by egui's line scroll speed.
fn count(ui: &Ui, key: Id, wheel: &[(MouseWheelUnit, Vec2, bool)], shift: f32) -> f32 {
    let mut carried: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(0.0);
    let per_line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    let per_line = if per_line.is_finite() && per_line > 0.0 { per_line } else { 40.0 };
    let mut lines = 0.0;
    for &(unit, delta, shifted) in wheel {
        // Some systems turn Shift+wheel into a sideways scroll.
        let d = if delta.y != 0.0 { delta.y } else { delta.x };
        carried += match unit {
            MouseWheelUnit::Point => d / per_line,
            MouseWheelUnit::Line | MouseWheelUnit::Page => d,
        };
        let whole = carried.trunc();
        carried -= whole;
        lines += if shifted { whole * shift } else { whole };
    }
    if !carried.is_finite() {
        carried = 0.0;
    }
    ui.data_mut(|d| d.insert_temp(key, carried));
    if lines.is_finite() { lines } else { 0.0 }
}
