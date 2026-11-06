//! Raw input in the input hook, key events converted to craft-keymap's key types, `InputOptions`.

use craft_behavior::one_press::split_after_first_press;
use egui::{Context, Event, ViewportId};

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

/// The operating system the key table picks rows for.
pub(crate) fn os(ctx: &Context) -> craft_behavior::Os {
    use egui::os::OperatingSystem;
    match ctx.os() {
        OperatingSystem::Mac | OperatingSystem::IOS => craft_behavior::Os::Mac,
        OperatingSystem::Windows => craft_behavior::Os::Windows,
        OperatingSystem::Nix | OperatingSystem::Android | OperatingSystem::Unknown => craft_behavior::Os::Linux,
    }
}

/// The modifiers egui reports, read for `os`: off macOS, Ctrl is the command key.
pub(crate) fn mods(modifiers: &egui::Modifiers, os: craft_behavior::Os) -> craft_keymap::Mods {
    if os == craft_behavior::Os::Mac {
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
pub(crate) fn key_event(event: &Event, os: craft_behavior::Os) -> KeyEvent {
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
