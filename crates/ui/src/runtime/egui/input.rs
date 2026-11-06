//! Raw input in the input hook, key events converted to craft-keymap's key types, `InputOptions`.

use craft_behavior::one_press::split_after_first_press;
use craft_behavior::{Mods, PointerEvent, PointerKind, PointerSample, WheelPhase, WheelUnit};
use egui::{Context, Event, Id, Modifiers, MouseWheelUnit, Pos2, TouchPhase, Vec2, ViewportId};

pub use egui::RawInput;

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

/// The modifiers as craft-keymap's: `cmd` follows egui's `command`, and `ctrl` is the Control key where it is
/// distinct from it (macOS).
pub(crate) fn mods_of(modifiers: Modifiers) -> Mods {
    let ctrl_is_command = modifiers.command && !modifiers.mac_cmd;
    Mods::new(modifiers.command, modifiers.ctrl && !ctrl_is_command, modifiers.alt, modifiers.shift)
}

/// What the running pass's pointer input says, converted once.
pub(crate) struct PointerRead {
    /// The primary-button events in arrival order. `clicks` of each press is 1: the caller counts.
    pub events: Vec<PointerEvent>,
    /// The latest pointer position when the pass began, `None` when the pointer is not in the window.
    pub pos: Option<Pos2>,
    /// Where the pointer hovers: `None` while it is gone.
    pub hover: Option<Pos2>,
    /// The pointer movement of the pass.
    pub delta: Vec2,
    /// Whether the primary button is down.
    pub primary_down: bool,
    /// Whether the primary button went down in this pass.
    pub primary_pressed: bool,
    /// The modifiers held.
    pub mods: Mods,
    /// The pass's time in seconds.
    pub time: f64,
    pub kind: PointerKind,
    pub sample: Option<PointerSample>,
    /// The window lost focus in this pass.
    pub focus_lost: bool,
    /// The zoom factor of the pass: pinch and Ctrl or Cmd with the wheel.
    pub zoom: f32,
    /// The two-finger gesture: translation, zoom, rotation and centre.
    pub multi_touch: Option<(Vec2, f32, f32, Pos2)>,
    /// egui's longest movement that still makes a click.
    pub slop: f32,
}

/// Converts the running pass's pointer input.
pub(crate) fn pointer_read(ctx: &Context) -> PointerRead {
    let slop = ctx.options(|options| options.input_options.max_click_dist);
    ctx.input(|input| {
        let mut events = Vec::new();
        let mut focus_lost = false;
        let mut kind = PointerKind::Mouse;
        let mut sample = None;
        for event in &input.raw.events {
            match event {
                Event::PointerMoved(pos) => events.push(PointerEvent::Moved(*pos)),
                Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, .. } => {
                    events.push(PointerEvent::pressed(*pos, input.time, 1));
                }
                Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, .. } => {
                    events.push(PointerEvent::released(*pos));
                }
                Event::WindowFocused(false) => focus_lost = true,
                Event::Touch { phase, force, .. } => {
                    kind = PointerKind::Touch;
                    if *phase != TouchPhase::End {
                        sample = force.map(PointerSample::new);
                    }
                }
                _ => {}
            }
        }
        let multi_touch = input.multi_touch().map(|touch| (touch.translation_delta, touch.zoom_delta, touch.rotation_delta, touch.center_pos));
        PointerRead {
            events,
            pos: input.pointer.latest_pos(),
            hover: input.pointer.hover_pos(),
            delta: input.pointer.delta(),
            primary_down: input.pointer.primary_down(),
            primary_pressed: input.pointer.primary_pressed(),
            mods: mods_of(input.modifiers),
            time: input.time,
            kind,
            sample,
            focus_lost,
            zoom: input.zoom_delta(),
            multi_touch,
            slop,
        }
    })
}

/// The ids of egui's click and drag widgets in the hit test at the latest pointer position, which is the
/// press's own hit test when the pass carries a press as its last pointer event (H0).
pub(crate) fn hit_targets(ctx: &Context) -> (Option<Id>, Option<Id>) {
    ctx.viewport(|viewport| (viewport.hits.click.map(|widget| widget.id), viewport.hits.drag.map(|widget| widget.id)))
}

/// The wheel input of the pass: the summed scroll in points (`line` and `page` points per line and page), the
/// last event's unit, phase and modifiers. `None` when no wheel event arrived.
pub(crate) fn wheel_read(ctx: &Context, line: f32, page: f32) -> Option<(Vec2, WheelUnit, WheelPhase, Mods)> {
    ctx.input(|input| {
        let mut read: Option<(Vec2, WheelUnit, WheelPhase, Mods)> = None;
        for event in &input.raw.events {
            if let Event::MouseWheel { unit, delta, modifiers, phase, .. } = event {
                let (unit, points) = match unit {
                    MouseWheelUnit::Point => (WheelUnit::Point, 1.0),
                    MouseWheelUnit::Line => (WheelUnit::Line, line),
                    MouseWheelUnit::Page => (WheelUnit::Page, page),
                };
                let phase = match phase {
                    TouchPhase::Start => WheelPhase::Start,
                    TouchPhase::Move => WheelPhase::Move,
                    TouchPhase::End => WheelPhase::End,
                    TouchPhase::Cancel => WheelPhase::Cancel,
                };
                let sum = read.map_or(Vec2::ZERO, |(sum, ..)| sum) + *delta * points;
                read = Some((sum, unit, phase, mods_of(*modifiers)));
            }
        }
        read
    })
}
