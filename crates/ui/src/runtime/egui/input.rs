//! Raw input in the input hook, key events converted to craft-keymap's key types, `InputOptions`.

use craft_behavior::one_press::split_after_first_press;
use egui::{Context, Event, RawInput, ViewportId};

/// What the one-press rule needs between frames, for one viewport.
#[derive(Default)]
pub(crate) struct HeldInput {
    events: Vec<Event>,
    frame: Option<u64>,
}

/// H0: this pass carries at most one pointer press, as its last pointer event. Events after the first press
/// wait in `held` and lead the next frame's events. A repeated pass of the same frame, a discard pass,
/// receives no input events, so it gets none of the held ones. Returns whether events were held, so the hook
/// asks for a repaint.
pub(crate) fn hold_back_presses(input: &mut RawInput, frame: u64, held: &mut HeldInput) -> bool {
    if held.frame == Some(frame) {
        return false;
    }
    held.frame = Some(frame);
    let mut events = std::mem::take(&mut held.events);
    events.append(&mut input.events);
    let (now, later) = split_after_first_press(events, |event| matches!(event, Event::PointerButton { pressed: true, .. }));
    input.events = now;
    held.events = later;
    !held.events.is_empty()
}

/// Asks for an immediate repaint of `viewport`, from the input hook where the viewport is not yet current.
pub(crate) fn request_repaint_of(ctx: &Context, viewport: ViewportId) {
    ctx.request_repaint_of(viewport);
}

/// H2: a click is a release within slop, whatever its duration, so egui's click and the press lifecycle agree.
pub(crate) fn set_click_options(ctx: &Context) {
    if ctx.options(|options| options.input_options.max_click_duration) != f64::INFINITY {
        ctx.options_mut(|options| options.input_options.max_click_duration = f64::INFINITY);
    }
}

/// The options the runtime owns at install: interface scale and quit are commands, so egui's own shortcuts go.
pub(crate) fn clear_shortcuts(ctx: &Context) {
    ctx.options_mut(|options| {
        options.zoom_with_keyboard = false;
        options.quit_shortcuts.clear();
    });
}

/// Stops the pass's egui drag, so hover reaches other widgets again.
pub(crate) fn stop_dragging(ctx: &Context) {
    ctx.stop_dragging();
}
