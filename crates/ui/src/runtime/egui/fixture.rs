//! The egui calls `craft_ui::testing` needs: frames, raw input and options that apps reach only through the runtime.

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Shape, Ui, ViewportCommand};

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

/// Senses `rect` under `id`, as a placed child's one widget does.
pub(crate) fn placeholder(ui: &mut Ui, id: egui::Id, rect: Rect) -> egui::Response {
    ui.interact(rect, id, egui::Sense::hover())
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

/// A pointer press or release of the primary button at `pos`.
pub(crate) fn primary_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
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
