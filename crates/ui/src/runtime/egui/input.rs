//! Raw input in the input hook, key events converted to craft-keymap's key types, `InputOptions`.

use craft_behavior::one_press::split_after_first_press;
use craft_behavior::{PointerEvent, PointerKind, PointerSample, WheelPhase, WheelUnit};
use craft_host::target::Os;
use craft_keymap::Mods;
use egui::{Context, Event, Id, MouseWheelUnit, Pos2, TouchPhase, Vec2, ViewportId};

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

/// H2: a click is a release within `slop`, whatever its duration, so egui's click and the press lifecycle agree.
pub(crate) fn set_click_options(ctx: &Context, slop: f32) {
    let (dist, duration) = ctx.options(|options| (options.input_options.max_click_dist, options.input_options.max_click_duration));
    if dist != slop || duration != f64::INFINITY {
        ctx.options_mut(|options| {
            options.input_options.max_click_dist = slop;
            options.input_options.max_click_duration = f64::INFINITY;
        });
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

/// The operating system the key table picks rows for.
pub(crate) fn os(ctx: &Context) -> Os {
    use egui::os::OperatingSystem;
    match ctx.os() {
        OperatingSystem::Mac | OperatingSystem::IOS => Os::Mac,
        OperatingSystem::Windows => Os::Windows,
        OperatingSystem::Nix | OperatingSystem::Android | OperatingSystem::Unknown => Os::Linux,
    }
}

/// The modifiers egui reports, read for `os`: off macOS, Ctrl is the command key.
pub(crate) fn mods(modifiers: &egui::Modifiers, os: Os) -> craft_keymap::Mods {
    if os == Os::Mac {
        craft_keymap::Mods { cmd: modifiers.mac_cmd || modifiers.command, ctrl: modifiers.ctrl, alt: modifiers.alt, shift: modifiers.shift }
    } else {
        craft_keymap::Mods { cmd: modifiers.ctrl || modifiers.command, ctrl: false, alt: modifiers.alt, shift: modifiers.shift }
    }
}

macro_rules! key_conversion {
    ($($ours:ident = $theirs:ident),* ; $($same:ident),* $(,)?) => {
        /// craft-keymap's key for an egui key; `None` for egui's clipboard and browser keys.
        pub(crate) fn key_of(key: egui::Key) -> Option<craft_keymap::Key> {
            match key {
                $(egui::Key::$theirs => Some(craft_keymap::Key::$ours),)*
                $(egui::Key::$same => Some(craft_keymap::Key::$same),)*
                _ => None,
            }
        }
    };
}

key_conversion! {
    Up = ArrowUp, Down = ArrowDown, Left = ArrowLeft, Right = ArrowRight;
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
    F21, F22, F23, F24, F25, F26, F27, F28, F29, F30, F31, F32, F33, F34, F35,
    Escape, Tab, Menu, Backspace, Enter, Space, Insert, Delete, Home, End, PageUp, PageDown,
    Minus, Equals, Plus, OpenBracket, CloseBracket, Backslash, Semicolon, Quote, Backtick,
    Comma, Period, Slash, IntlBackslash, Colon, Pipe, Questionmark, Exclamationmark, OpenCurlyBracket, CloseCurlyBracket,
}

/// One key event as the key stage reads it.
pub(crate) enum KeyEvent {
    /// A key press, repeats included: the key as typed and the modifiers.
    Press { key: craft_keymap::Key, mods: craft_keymap::Mods },
    /// Typed text.
    Text(String),
    /// egui's own clipboard events.
    Clip(craft_behavior::EditCommand),
    /// A pointer press, which ends keyboard modality.
    PointerPress,
    /// An AccessKit action request.
    Access(Action),
    /// The held modifiers changed.
    Modifiers(craft_keymap::Mods),
    /// Anything else.
    Other,
}

/// The key stage's reading of `event`.
pub(crate) fn key_event(event: &Event, os: Os) -> KeyEvent {
    match event {
        Event::Key { key, pressed: true, modifiers, .. } => match key_of(*key) {
            Some(typed) => KeyEvent::Press { key: typed, mods: mods(modifiers, os) },
            None => KeyEvent::Other,
        },
        Event::Text(text) => KeyEvent::Text(text.clone()),
        Event::ModifiersChanged(modifiers) => KeyEvent::Modifiers(mods(modifiers, os)),
        Event::Copy => KeyEvent::Clip(craft_behavior::EditCommand::Copy),
        Event::Cut => KeyEvent::Clip(craft_behavior::EditCommand::Cut),
        Event::Paste(_) => KeyEvent::Clip(craft_behavior::EditCommand::Paste),
        Event::PointerButton { pressed: true, .. } => KeyEvent::PointerPress,
        Event::AccessKitActionRequest(request) => KeyEvent::Access(Action(request.clone())),
        _ => KeyEvent::Other,
    }
}

/// An AccessKit action request, read without naming AccessKit.
pub(crate) struct Action(accesskit::ActionRequest);

/// What an action request asks for.
pub(crate) enum ActionKind {
    Focus,
    Click,
    Increment,
    Decrement,
    ShowContextMenu,
    SetValue(f64),
    Other,
}

impl Action {
    /// The node the request targets, as `node_key` numbers nodes.
    pub(crate) fn node(&self) -> u64 {
        self.0.target_node.0
    }

    pub(crate) fn kind(&self) -> ActionKind {
        match (self.0.action, &self.0.data) {
            (accesskit::Action::Focus, _) => ActionKind::Focus,
            (accesskit::Action::Click, _) => ActionKind::Click,
            (accesskit::Action::Increment, _) => ActionKind::Increment,
            (accesskit::Action::Decrement, _) => ActionKind::Decrement,
            (accesskit::Action::ShowContextMenu, _) => ActionKind::ShowContextMenu,
            (accesskit::Action::SetValue, Some(accesskit::ActionData::NumericValue(value))) => ActionKind::SetValue(*value),
            _ => ActionKind::Other,
        }
    }

    /// The request as an `Intent::Access` payload for the sub-node `part`.
    pub(crate) fn request(&self, part: Option<craft_behavior::PartRef>) -> craft_behavior::AccessRequest {
        craft_behavior::AccessRequest::new(self.0.action, part, self.0.data.clone())
    }
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
    let os = os(ctx);
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
            mods: mods(&input.modifiers, os),
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
    let os = os(ctx);
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
                read = Some((sum, unit, phase, mods(modifiers, os)));
            }
        }
        read
    })
}
