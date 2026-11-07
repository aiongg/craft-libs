//! The egui calls `craft_ui::testing` needs: frames, raw input and options that apps reach only through the runtime.

use egui::{Context, Event, Key, Modifiers, Pos2, Rect, Shape, Ui, ViewportCommand};
use std::sync::Arc;

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
    /// What the frame's `PlatformOutput.ime` said.
    pub ime: Option<ImeSeen>,
    /// The texts the frame put on the clipboard.
    pub copied: Vec<String>,
    /// The text runs the frame painted.
    pub texts: Vec<PaintedText>,
    /// The visual bounds of every shape the frame painted, within its clip rect. A shape with no extent (egui reports an empty galley's
    /// bounds as NaN) has none.
    pub bounds: Vec<Rect>,
}

/// A text run a frame painted.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PaintedText {
    /// Where the run lies, in points.
    pub rect: Rect,
    /// The layout cut text.
    pub elided: bool,
    /// The run's full text.
    pub text: String,
}

/// The IME output of a frame: the owner widget, whether it asks the OS to interrupt a composition, the caret rect.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImeSeen {
    /// The widget that owns the IME.
    pub owner: egui::Id,
    /// The OS composition is to be interrupted.
    pub interrupt: bool,
    /// Where the candidate window opens.
    pub caret: Rect,
}

/// The files the OS hovers over or drops on a frame's window, by name; a hovered file also has the media type the OS gave.
#[derive(Clone, Debug, Default)]
pub(crate) struct OsFiles {
    pub hovered: Vec<(String, String)>,
    pub dropped: Vec<String>,
}

/// A dropped file that exists only as a path.
#[derive(Debug)]
struct DroppedPath(std::path::PathBuf);

impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}

/// Runs one frame at `time` seconds with `events` and `files`, and drops its texture changes.
pub(crate) fn run_frame(ctx: &Context, time: f64, events: Vec<Event>, files: OsFiles, ui: impl FnMut(&mut Ui)) -> FrameResult {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let hovered_files = files.hovered.iter().map(|(name, mime)| egui::HoveredFile { path: Some(name.into()), mime: mime.clone() }).collect();
    let dropped_files = files.dropped.iter().map(|name| Arc::new(DroppedPath(name.into())) as egui::DroppedFileHandle).collect();
    let input = RawInput { screen_rect: Some(screen), time: Some(time), events, hovered_files, dropped_files, ..RawInput::default() };
    let mut output = ctx.run_ui(input, ui);
    output.textures_delta.clear();
    let mut flat = Vec::new();
    for clipped in &output.shapes {
        let mut shapes = Vec::new();
        flatten_ref(&clipped.shape, &mut shapes);
        flat.extend(shapes.into_iter().map(|shape| (shape, clipped.clip_rect)));
    }
    // A shape shows only inside its clip rect.
    let bounds = flat
        .iter()
        .map(|(shape, clip)| shape.visual_bounding_rect().intersect(*clip))
        .filter(|rect| rect.is_finite() && rect.min.x <= rect.max.x && rect.min.y <= rect.max.y)
        .collect();
    let texts = flat
        .iter()
        .filter_map(|(shape, _)| match shape {
            Shape::Text(text) => Some(PaintedText {
                rect: text.galley.rect.translate(text.pos.to_vec2()),
                elided: text.galley.elided,
                text: text.galley.job.text.clone(),
            }),
            _ => None,
        })
        .collect();
    let closed = output.viewport_output.values().any(|viewport| viewport.commands.iter().any(|command| matches!(command, ViewportCommand::Close)));
    let repaint_delay = output.viewport_output.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |viewport| viewport.repaint_delay);
    let copied = output
        .platform_output
        .commands
        .iter()
        .filter_map(|command| if let egui::OutputCommand::CopyText(text) = command { Some(text.clone()) } else { None })
        .collect();
    FrameResult {
        closed,
        copied,
        texts,
        bounds,
        cursor: output.platform_output.cursor_icon,
        repaint_delay,
        tree: crate::runtime::egui::access::tree_of(&mut output),
        ime: output.platform_output.ime.as_ref().map(|ime| ImeSeen {
            owner: ime.owner,
            interrupt: ime.should_interrupt_composition,
            caret: ime.cursor_rect,
        }),
    }
}

/// Runs a pass of the viewport `viewport` (a child of the root) with no events, while the root stays live.
pub(crate) fn run_viewport_pass(ctx: &Context, time: f64, viewport: egui::ViewportId, ui: impl FnMut(&mut Ui)) {
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    let mut input = RawInput { screen_rect: Some(screen), time: Some(time), viewport_id: viewport, ..RawInput::default() };
    for id in [egui::ViewportId::ROOT, viewport] {
        input.viewports.insert(
            id,
            egui::ViewportInfo { parent: (id != egui::ViewportId::ROOT).then_some(egui::ViewportId::ROOT), ..egui::ViewportInfo::default() },
        );
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
    static FONTS: std::sync::OnceLock<egui::FontDefinitions> = std::sync::OnceLock::new();
    let ctx = Context::default();
    ctx.set_fonts(FONTS.get_or_init(egui::FontDefinitions::default).clone());
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

fn flatten_ref<'a>(shape: &'a Shape, out: &mut Vec<&'a Shape>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| flatten_ref(shape, out)),
        other => out.push(other),
    }
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
    /// AccessKit `Focus`.
    Focus,
    /// AccessKit `Click`.
    Click,
    /// AccessKit `Increment`.
    Increment,
    /// AccessKit `Expand`, an action with no intent of its own.
    Expand,
    /// AccessKit `ScrollDown`.
    ScrollDown,
}

impl AccessActionKind {
    /// Whether `action` is this one.
    pub fn is(self, action: &craft_behavior::AccessRequest) -> bool {
        action.action == self.action()
    }

    fn action(self) -> accesskit::Action {
        match self {
            AccessActionKind::Focus => accesskit::Action::Focus,
            AccessActionKind::Click => accesskit::Action::Click,
            AccessActionKind::Increment => accesskit::Action::Increment,
            AccessActionKind::Expand => accesskit::Action::Expand,
            AccessActionKind::ScrollDown => accesskit::Action::ScrollDown,
        }
    }
}

/// An AccessKit action request for the node of `target`.
pub(crate) fn access_request(kind: AccessActionKind, target: egui::Id) -> Event {
    let action = kind.action();
    let target_node = accesskit::NodeId(crate::runtime::egui::access::node_key(target));
    Event::AccessKitActionRequest(accesskit::ActionRequest { action, target_tree: accesskit::TreeId::ROOT, target_node, data: None })
}

/// An AccessKit action request for the node `target`.
pub(crate) fn access_request_node(kind: AccessActionKind, target: crate::runtime::egui::access::NodeKey) -> Event {
    let target_node = accesskit::NodeId(target.raw());
    Event::AccessKitActionRequest(accesskit::ActionRequest { action: kind.action(), target_tree: accesskit::TreeId::ROOT, target_node, data: None })
}

/// An AccessKit `SetValue` request carrying `value` for the node of `target`.
pub(crate) fn access_set_value(target: egui::Id, value: f64) -> Event {
    let target_node = accesskit::NodeId(crate::runtime::egui::access::node_key(target));
    Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::SetValue,
        target_tree: accesskit::TreeId::ROOT,
        target_node,
        data: Some(accesskit::ActionData::NumericValue(value)),
    })
}

/// Tells the context which operating system it runs on.
pub(crate) fn set_os(ctx: &Context, os: craft_host::target::Os) {
    ctx.set_os(match os {
        craft_host::target::Os::Mac => egui::os::OperatingSystem::Mac,
        craft_host::target::Os::Windows => egui::os::OperatingSystem::Windows,
        _ => egui::os::OperatingSystem::Nix,
    });
}

/// An IME preedit event of `text` with no cursor range.
pub fn ime_preedit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Preedit { text: text.into(), active_range_chars: None })
}

pub use egui::{FullOutput, RawInput};

/// The id of the widget egui is dragging.
pub fn dragged_id(ctx: &Context) -> Option<egui::Id> {
    ctx.dragged_id()
}

/// Adds a plugin to `ctx` that records, for each input hook call, the viewport of the raw input and the
/// viewport that is current on the context.
pub fn record_input_hooks(ctx: &Context, log: std::sync::Arc<std::sync::Mutex<Vec<(egui::ViewportId, egui::ViewportId)>>>) {
    struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<(egui::ViewportId, egui::ViewportId)>>>);
    impl egui::Plugin for Recorder {
        fn debug_name(&self) -> &'static str {
            "craft_ui.fixture.input_hooks"
        }

        fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
            if let Ok(mut log) = self.0.lock() {
                log.push((input.viewport_id, ctx.viewport_id()));
            }
        }
    }
    ctx.add_plugin(Recorder(log));
}

/// The id of the widget egui has focused.
pub(crate) fn focused(ctx: &Context) -> Option<egui::Id> {
    ctx.memory(|memory| memory.focused())
}

/// The area `ui` has allocated so far.
pub(crate) fn used_rect(ui: &Ui) -> Rect {
    ui.min_rect()
}

/// Typed text.
pub(crate) fn text_event(text: &str) -> Event {
    Event::Text(text.to_owned())
}

/// An AccessKit `SetValue` request carrying `text` for the node of `target`.
pub fn set_value_request(target: egui::Id, text: &str) -> Event {
    let target_node = accesskit::NodeId(crate::runtime::egui::access::node_key(target));
    Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::SetValue,
        target_tree: accesskit::TreeId::ROOT,
        target_node,
        data: Some(accesskit::ActionData::Value(text.into())),
    })
}

/// An AccessKit `SetTextSelection` request for the node of `target`, from character `anchor.1` of the run node
/// `anchor.0` to character `focus.1` of `focus.0`.
pub fn text_selection_request(target: egui::Id, anchor: (egui::Id, usize), focus: (egui::Id, usize)) -> Event {
    let target_node = accesskit::NodeId(crate::runtime::egui::access::node_key(target));
    let position = |(node, character_index): (egui::Id, usize)| accesskit::TextPosition { node: node.accesskit_id(), character_index };
    Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::SetTextSelection,
        target_tree: accesskit::TreeId::ROOT,
        target_node,
        data: Some(accesskit::ActionData::SetTextSelection(accesskit::TextSelection { anchor: position(anchor), focus: position(focus) })),
    })
}

/// A globally unique id named by `name`.
pub(crate) fn named_id(name: &str) -> egui::Id {
    crate::runtime::egui::memory::unique(("focus-trial", name))
}

/// An allocating widget over `size` on `ui`: what app code may add over a canvas body.
pub(crate) fn allocate_blocker(ui: &mut Ui, size: egui::Vec2) {
    ui.allocate_response(size, egui::Sense::hover());
}

/// A clickable egui widget placed by hand in `rect`: what a component's own widgets never are.
pub(crate) fn raw_widget(ui: &mut Ui, rect: egui::Rect) {
    ui.interact(rect, crate::runtime::egui::memory::unique("raw_widget"), egui::Sense::click());
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

/// A wheel event at the pointer, `dy` lines of vertical scroll.
pub(crate) fn wheel_lines_event(dy: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::Vec2::new(0.0, dy),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: egui::MouseWheelSource::Wheel,
    }
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
