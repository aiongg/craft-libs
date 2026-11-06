//! The egui calls `craft_ui::testing` needs: frames, raw input and options that apps reach only through the runtime.

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Ui, ViewportCommand};

/// What a frame sent out that trials read.
pub(crate) struct FrameResult {
    /// The frame sent a close command.
    pub closed: bool,
    /// The cursor the frame's output asked for.
    pub cursor: egui::CursorIcon,
}

/// Runs one frame at `time` seconds with `events` and drops its texture changes.
pub(crate) fn run_frame(ctx: &Context, time: f64, events: Vec<Event>, ui: impl FnMut(&mut Ui)) -> FrameResult {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let input = RawInput { screen_rect: Some(screen), time: Some(time), events, ..RawInput::default() };
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    let closed = output.viewport_output.values().any(|viewport| viewport.commands.iter().any(|command| matches!(command, ViewportCommand::Close)));
    FrameResult { closed, cursor: output.platform_output.cursor_icon }
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

/// An allocating widget over `size` on `ui`: what app code may add over a canvas body.
pub(crate) fn allocate_blocker(ui: &mut Ui, size: egui::Vec2) {
    ui.allocate_response(size, egui::Sense::hover());
}

/// Whether the `Response` reports a click.
pub(crate) fn clicked(response: &egui::Response) -> bool {
    response.clicked()
}

/// The widget id egui's hit test chose as the click target at the latest pointer position.
pub(crate) fn click_target(ctx: &Context) -> Option<egui::Id> {
    crate::runtime::egui::input::hit_targets(ctx).0
}

/// A wheel event at the pointer, `dy` points of vertical scroll.
pub(crate) fn wheel_event(dy: f32) -> Event {
    Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::Vec2::new(0.0, dy), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE, source: egui::MouseWheelSource::Wheel }
}

/// The primary button going down or up at `pos`.
pub(crate) fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// The pointer moving to `pos`.
pub(crate) fn pointer_moved(pos: Pos2) -> Event {
    Event::PointerMoved(pos)
}

/// The pointer leaving the window.
pub(crate) fn pointer_gone() -> Event {
    Event::PointerGone
}

/// The window losing focus.
pub(crate) fn window_blurred() -> Event {
    Event::WindowFocused(false)
}

/// Gives key focus to the widget `id`.
pub(crate) fn focus(ctx: &Context, id: egui::Id) {
    ctx.memory_mut(|memory| memory.request_focus(id));
}
