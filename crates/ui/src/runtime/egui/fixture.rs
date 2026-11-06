//! The egui calls `craft_ui::testing` needs: frames, raw input and options that apps reach only through the runtime.

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Ui, ViewportCommand};

/// Runs one frame at `time` seconds with `events` and drops its texture changes. Returns whether the
/// frame sent a close command.
pub(crate) fn run_frame(ctx: &Context, time: f64, events: Vec<Event>, ui: impl FnMut(&mut Ui)) -> bool {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let input = RawInput { screen_rect: Some(screen), time: Some(time), events, ..RawInput::default() };
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    output.viewport_output.values().any(|viewport| viewport.commands.iter().any(|command| matches!(command, ViewportCommand::Close)))
}

/// A key press event with `modifiers`.
pub(crate) fn key_press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// Gives egui back the option the runtime turns off at install: `Cmd` with `=` or `-` zooms the interface.
pub(crate) fn allow_zoom_keys(ctx: &Context) {
    ctx.options_mut(|options| options.zoom_with_keyboard = true);
}

/// Sets egui's longest press that still counts as a click.
pub(crate) fn set_max_click_duration(ctx: &Context, seconds: f64) {
    ctx.options_mut(|options| options.input_options.max_click_duration = seconds);
}

/// A fresh context with no plugin.
pub(crate) fn bare_context() -> Context {
    Context::default()
}
