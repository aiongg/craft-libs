//! The egui calls `craft_ui::testing` needs: frames, raw input and options that apps reach only through the runtime.

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Shape, Ui, ViewportCommand};

/// What a frame sent out that trials read.
pub(crate) struct FrameResult {
    /// The frame sent a close command.
    pub closed: bool,
    /// The cursor the frame's output asked for.
    pub cursor: egui::CursorIcon,
    /// The wait egui asks for before the root viewport's next pass; `Duration::MAX` when it asks for none.
    pub repaint_delay: std::time::Duration,
    /// The AccessKit tree update of the frame.
    pub tree: Option<crate::runtime::egui::access::AccessTree>,
}

/// Runs one frame at `time` seconds with `events` and drops its texture changes.
pub(crate) fn run_frame(ctx: &Context, time: f64, events: Vec<Event>, ui: impl FnMut(&mut Ui)) -> FrameResult {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let input = RawInput { screen_rect: Some(screen), time: Some(time), events, ..RawInput::default() };
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    let closed = output.viewport_output.values().any(|viewport| viewport.commands.iter().any(|command| matches!(command, ViewportCommand::Close)));
    let repaint_delay = output.viewport_output.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |viewport| viewport.repaint_delay);
    FrameResult { closed, cursor: output.platform_output.cursor_icon, repaint_delay, tree: crate::runtime::egui::access::tree_of(&mut output) }
}

/// Runs a pass of the viewport `viewport` (a child of the root) with no events, while the root stays live.
pub(crate) fn run_viewport_pass(ctx: &Context, time: f64, viewport: egui::ViewportId, ui: impl FnMut(&mut Ui)) {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let mut input = RawInput { screen_rect: Some(screen), time: Some(time), viewport_id: viewport, ..RawInput::default() };
    for id in [egui::ViewportId::ROOT, viewport] {
        input.viewports.insert(id, egui::ViewportInfo { parent: (id != egui::ViewportId::ROOT).then_some(egui::ViewportId::ROOT), ..egui::ViewportInfo::default() });
    }
    let _ = ctx.run_ui(input, ui);
}

/// A key press event with `modifiers`.
pub(crate) fn key_press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// Gives egui back the option the runtime turns off at install: `Cmd` with `=` or `-` zooms the interface.
pub(crate) fn allow_zoom_keys(ctx: &Context) {
    ctx.options_mut(|options| options.zoom_with_keyboard = true);
}

/// egui's longest movement that still counts as a click.
pub(crate) fn max_click_dist(ctx: &Context) -> f32 {
    ctx.options(|options| options.input_options.max_click_dist)
}

/// Sets egui's longest press that still counts as a click.
pub(crate) fn set_max_click_duration(ctx: &Context, seconds: f64) {
    ctx.options_mut(|options| options.input_options.max_click_duration = seconds);
}

/// A fresh context with no plugin and AccessKit on.
pub(crate) fn bare_context() -> Context {
    let ctx = Context::default();
    ctx.enable_accesskit();
    ctx
}

/// A paragraph of `text` wrapped to the available width.
pub(crate) fn wrapped_text(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(text).wrap());
}

/// The default font definitions with the proportional family's first font moved to the end, a change that
/// needs a new layout of every text.
pub(crate) fn reordered_fonts() -> egui::FontDefinitions {
    let mut definitions = egui::FontDefinitions::default();
    if let Some(proportional) = definitions.families.get_mut(&egui::FontFamily::Proportional)
        && !proportional.is_empty()
    {
        proportional.rotate_left(1);
    }
    definitions
}

/// Runs one frame like `run_frame` and returns the shapes it painted, nested shapes flattened.
pub(crate) fn run_frame_shapes(ctx: &Context, time: f64, ui: impl FnMut(&mut Ui)) -> Vec<Shape> {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let input = RawInput { screen_rect: Some(screen), time: Some(time), ..RawInput::default() };
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    let mut flat = Vec::new();
    for clipped in output.shapes {
        flatten(clipped.shape, &mut flat);
    }
    flat
}

fn flatten(shape: Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.into_iter().for_each(|shape| flatten(shape, out)),
        other => out.push(other),
    }
}

/// Shows a focusable egui widget of `id` at the `Ui` cursor, as the sensing core of a component root does.
pub(crate) fn focusable_widget(ui: &mut Ui, id: egui::Id) -> Rect {
    let rect = Rect::from_min_size(ui.cursor().min, ui.spacing().interact_size);
    ui.advance_cursor_after_rect(rect);
    ui.interact(rect, id, egui::Sense::click());
    rect
}

/// The egui layer `ui` paints on.
pub(crate) fn layer_of(ui: &Ui) -> egui::LayerId {
    ui.layer_id()
}

/// The primary button going down or up at `pos`.
pub(crate) fn primary_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// The secondary button going down or up at `pos`.
pub(crate) fn secondary_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE }
}

/// The modifiers held from this event on.
pub(crate) fn modifiers_changed(modifiers: Modifiers) -> Event {
    Event::ModifiersChanged(modifiers)
}

/// The middle button going down or up at `pos`.
pub(crate) fn middle_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Middle, pressed, modifiers: Modifiers::NONE }
}

/// A wheel event of `dy` points with the zoom modifier held.
pub(crate) fn zoom_wheel_event(dy: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::Vec2::new(0.0, dy),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::COMMAND,
        source: egui::MouseWheelSource::Wheel,
    }
}

/// A key release event.
pub(crate) fn key_release(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers }
}

/// The actions the focus trials request.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum AccessActionKind {
    /// AccessKit `Click`.
    Click,
    /// AccessKit `Increment`.
    Increment,
    /// AccessKit `Expand`, an action with no intent of its own.
    Expand,
}

impl AccessActionKind {
    /// Whether `action` is this one.
    pub fn is(self, action: &craft_behavior::AccessRequest) -> bool {
        action.action == self.action()
    }

    fn action(self) -> accesskit::Action {
        match self {
            AccessActionKind::Click => accesskit::Action::Click,
            AccessActionKind::Increment => accesskit::Action::Increment,
            AccessActionKind::Expand => accesskit::Action::Expand,
        }
    }
}

/// An AccessKit action request for the node of `target`.
pub(crate) fn access_request(kind: AccessActionKind, target: egui::Id) -> Event {
    let action = kind.action();
    let target_node = accesskit::NodeId(crate::runtime::egui::access::node_key(target));
    Event::AccessKitActionRequest(accesskit::ActionRequest { action, target_tree: accesskit::TreeId::ROOT, target_node, data: None })
}

/// A globally unique id named by `name`.
pub(crate) fn named_id(name: &str) -> egui::Id {
    crate::runtime::egui::memory::unique(("focus-trial", name))
}

/// An allocating widget over `size` on `ui`: what app code may add over a canvas body.
pub(crate) fn allocate_blocker(ui: &mut Ui, size: egui::Vec2) {
    ui.allocate_response(size, egui::Sense::hover());
}

/// Whether the `Response` reports a click.
pub(crate) fn clicked(response: &egui::Response) -> bool {
    response.clicked()
}

/// Whether the `Response` reports a primary-button pointer click.
pub(crate) fn clicked_by_primary(response: &egui::Response) -> bool {
    response.clicked_by(egui::PointerButton::Primary)
}

/// The widget id egui's hit test chose as the click target at the latest pointer position.
pub(crate) fn click_target(ctx: &Context) -> Option<egui::Id> {
    crate::runtime::egui::input::hit_targets(ctx).0
}

/// A wheel event at the pointer, `dy` points of vertical scroll.
pub(crate) fn wheel_event(dy: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::Vec2::new(0.0, dy),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: egui::MouseWheelSource::Wheel,
    }
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

/// Whether a repaint was requested since the last pass.
pub(crate) fn repaint_requested(ctx: &Context) -> bool {
    ctx.has_requested_repaint()
}
