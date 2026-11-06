//! The wheel service (8.6): takers register their view rects, the begin hook splits the pass's wheel input
//! among them from last pass's registrations, and each taker reads its share once.

use crate::runtime::egui::{focus, input, pass};
use crate::runtime::frame::PerViewport;
use crate::runtime::pointer;
use craft_behavior::wheel::split;
use craft_behavior::{PointerEvent, Wheel, WheelPhase, WheelTaker, WheelUnit};
use egui::{Context, Id, Pos2, Rect, Vec2};

#[derive(Default)]
struct State {
    this: Vec<WheelTaker<Id>>,
    last: Vec<WheelTaker<Id>>,
    /// The last-pressed gated control while the pointer has stayed inside it since.
    armed: Option<Id>,
    shares: Vec<(Id, Wheel)>,
}

/// Registers a taker for next pass's split.
pub(crate) fn register(ctx: &Context, taker: WheelTaker<Id>) {
    PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| state.this.push(taker));
}

/// The innermost non-gated taker of last pass whose view meets `rect`: where a reveal request goes.
pub(crate) fn nearest_scroll_owner(ctx: &Context, rect: Rect) -> Option<Id> {
    PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| {
        state.last.iter().filter(|taker| !taker.gated && taker.rect.intersects(rect)).min_by(|a, b| a.rect.area().total_cmp(&b.rect.area())).map(|taker| taker.owner)
    })
}

/// The begin hook (H7): rotates the registrations and splits this pass's wheel input.
pub(crate) fn begin(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let pass_pointer = pointer::pass_pointer(ctx);
    let frame = pass_pointer.frame.clone();
    let wheel_in = input::wheel_read(ctx, pointer::WHEEL_LINE_POINTS, pointer::WHEEL_PAGE_POINTS);
    let focused = focus::focused(ctx);
    let hover = pass_pointer.hover.or(frame.as_ref().and_then(|frame| frame.pos));
    let pressed_on = pass_pointer.events.iter().any(|event| matches!(event, PointerEvent::Pressed { .. })).then_some(pass_pointer.press_targets);
    PerViewport::<State>::with(ctx, viewport, |state| {
        state.last = std::mem::take(&mut state.this);
        state.shares.clear();
        if let Some(targets) = pressed_on {
            state.armed = targets.click.or(targets.drag).filter(|id| state.last.iter().any(|taker| taker.gated && taker.owner == *id));
        }
        if let Some(armed) = state.armed {
            let inside = state.last.iter().find(|taker| taker.owner == armed).zip(hover).is_some_and(|(taker, at): (&WheelTaker<Id>, Pos2)| taker.rect.contains(at));
            if !inside {
                state.armed = None;
            }
        }
        let (Some(frame), Some(at)) = (frame, hover) else { return };
        let (scroll, unit, phase, mods) = wheel_in.unwrap_or((Vec2::ZERO, WheelUnit::Point, WheelPhase::Move, frame.mods));
        if scroll == Vec2::ZERO && frame.zoom == 1.0 {
            return;
        }
        let share = Wheel::new(scroll, mods, unit, phase).zoom(frame.zoom);
        state.shares = split(&state.last, at, share, focused, state.armed);
    });
}

/// The component's share of this pass's wheel, once.
pub(crate) fn take(ctx: &Context, id: Id) -> Option<Wheel> {
    PerViewport::<State>::with(ctx, pass::current_viewport(ctx), |state| {
        let at = state.shares.iter().position(|(owner, _)| *owner == id)?;
        Some(state.shares.remove(at).1)
    })
}
