//! `FullWindowCanvas` in a `FullWindowDialog` harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{
    Color32, ColorImage, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, Painter, PointerButton, Pos2, TouchPhase, Vec2, pos2, vec2,
};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Size;

use super::canvas::{Tinted, overlay_view};
use super::{FullWindowCanvas, FullWindowView, RasterOverlay};
use crate::canvas::{
    CanvasTool, CanvasView, Checkerboard, HandTool, NavigationOptions, PixelRect, RasterSource, ToolAction, ToolContext, ToolPointer, ZoomTool,
};
use crate::dialog::{DialogState, FullWindowDialog};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);
const IMAGE: [u32; 2] = [200, 100];

/// A solid image, optionally at a reduced size (an overlay's proxy).
struct Solid {
    size: [u32; 2],
    colour: Color32,
}

impl RasterSource for Solid {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _region: PixelRect, _factor: u32, out: &mut ColorImage) {
        out.pixels.fill(self.colour);
    }
}

/// A tool that counts its presses.
#[derive(Default)]
struct Pen {
    presses: usize,
}

impl CanvasTool for Pen {
    fn press(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.presses += 1;
        ToolAction::Claimed
    }

    fn drag(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {}

    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}
}

struct App {
    state: DialogState,
    view: FullWindowView,
    image: Solid,
    overlay: Option<Solid>,
    navigation: NavigationOptions,
    pen: Pen,
    hand: HandTool,
    zoom: ZoomTool,
    /// Which tool the canvas runs: 0 none, 1 the pen, 2 the Hand, 3 the Zoom tool.
    tool: u8,
    text: String,
    focus_text: bool,
    /// The view the canvas returned in the last pass.
    shown: Option<CanvasView>,
    /// A zoom key left over for code below the dialog.
    leaked: bool,
}

impl App {
    fn new() -> Self {
        let navigation = NavigationOptions { animated_zoom: false, scrubby_zoom: false, ..NavigationOptions::default() };
        let doc = Size::new(f64::from(IMAGE[0]), f64::from(IMAGE[1]));
        Self {
            state: DialogState::new(),
            view: FullWindowView::new(),
            image: Solid { size: IMAGE, colour: Color32::from_rgb(40, 120, 200) },
            overlay: None,
            navigation,
            pen: Pen::default(),
            hand: HandTool::new(doc, navigation),
            zoom: ZoomTool::new(doc, navigation),
            tool: 0,
            text: String::new(),
            focus_text: false,
            shown: None,
            leaked: false,
        }
    }

    fn view(&self) -> CanvasView {
        self.shown.expect("a pass ran")
    }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut h = HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = ui.ctx().clone();
                let App { state, view, image, overlay, navigation, pen, hand, zoom, tool, text, focus_text, shown, .. } = app;
                FullWindowDialog::new("work").show(&ctx, state, &mut (), |ui, (), _fx| {
                    let edit = ui.add(egui::TextEdit::singleline(text));
                    edit.widget_info(|| egui::WidgetInfo::labeled(Role::TextInput, true, "Name"));
                    if std::mem::take(focus_text) {
                        edit.request_focus();
                    }
                    let mut canvas = FullWindowCanvas::new("view", view).source(image).checkerboard(Checkerboard::default()).navigation(*navigation);
                    if let Some(overlay) = overlay {
                        canvas = canvas.overlay(RasterOverlay::new(overlay, 4).tint(Color32::RED));
                    }
                    canvas = match *tool {
                        1 => canvas.tool(pen),
                        2 => canvas.tool(hand),
                        3 => canvas.tool(zoom),
                        _ => canvas,
                    };
                    *shown = Some(canvas.show(ui).view);
                });
                app.leaked = ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Num0));
            },
            app,
        );
    h.run();
    h
}

fn doc_at(h: &Harness<'_, App>, screen: Pos2) -> kurbo::Point {
    h.state().view().to_doc(screen)
}

fn pointer(h: &mut Harness<'_, App>, to: Pos2) {
    h.event(Event::PointerMoved(to));
    h.run();
}

fn button(h: &mut Harness<'_, App>, button: PointerButton, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event_modifiers(Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
    h.run();
}

fn drag(h: &mut Harness<'_, App>, button_: PointerButton, from: Pos2, to: Pos2, modifiers: Modifiers) {
    pointer(h, from);
    button(h, button_, from, true, modifiers);
    pointer(h, from + (to - from) * 0.5);
    pointer(h, to);
    button(h, button_, to, false, modifiers);
}

fn centre(h: &Harness<'_, App>) -> Pos2 {
    h.state().view().rect.center()
}

// The wheel zooms about the pointer by the user's `NavigationOptions` (Alt + wheel by default,
// the wheel with Zoom with Scroll Wheel), not `exp(scroll / 200)`.
#[test]
fn wheel_zooms_about_the_pointer_by_navigation_options() {
    let mut h = harness(App::new());
    let fitted = h.state().view().zoom;
    assert!(h.state().view.is_fit());
    let over = centre(&h) + vec2(60.0, 20.0);
    pointer(&mut h, over);

    // Plain wheel scrolls; it does not zoom.
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: MouseWheelSource::Unknown,
    });
    h.run();
    assert_eq!(h.state().view().zoom, fitted);
    let under = doc_at(&h, over);

    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::ALT,
        source: MouseWheelSource::Unknown,
    });
    h.run();
    let zoomed = h.state().view();
    assert!(zoomed.zoom > fitted, "Alt + wheel zooms in");
    assert!(!h.state().view.is_fit(), "a wheel zoom ends the fit");
    let still = doc_at(&h, over);
    assert!((still.x - under.x).abs() < 0.01 && (still.y - under.y).abs() < 0.01, "the point under the pointer stays: {under:?} {still:?}");

    // With Zoom with Scroll Wheel the plain wheel zooms.
    let mut app = App::new();
    app.navigation.zoom_with_wheel = true;
    let mut h = harness(app);
    let fitted = h.state().view().zoom;
    let middle = centre(&h);
    pointer(&mut h, middle);
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: MouseWheelSource::Unknown,
    });
    h.run();
    assert!(h.state().view().zoom > fitted);
}

// Space held and the middle button pan whatever the tool; the Hand tool pans.
#[test]
fn space_and_middle_button_pan_and_hand_tool_pans() {
    let mut app = App::new();
    app.tool = 1;
    let mut h = harness(app);
    let start = h.state().view().center;
    let from = centre(&h);
    let to = from + vec2(-30.0, 0.0);

    drag(&mut h, PointerButton::Primary, from, to, Modifiers::NONE);
    assert_eq!((h.state().pen.presses, h.state().view().center), (1, start), "the pen draws with a plain drag");

    pointer(&mut h, from);
    h.key_down(Key::Space);
    h.run();
    drag(&mut h, PointerButton::Primary, from, to, Modifiers::NONE);
    h.key_up(Key::Space);
    h.run();
    let panned = h.state().view().center;
    assert_eq!(h.state().pen.presses, 1, "Space makes the drag the Hand's");
    assert!(panned.x > start.x + 1.0, "the image followed the pointer left: {start:?} {panned:?}");

    drag(&mut h, PointerButton::Middle, from, from + vec2(0.0, 20.0), Modifiers::NONE);
    let middle = h.state().view().center;
    assert!(middle.y < panned.y - 1.0, "the middle button pans: {panned:?} {middle:?}");
    assert_eq!(h.state().pen.presses, 1);

    let mut app = App::new();
    app.tool = 2;
    let mut h = harness(app);
    let start = h.state().view().center;
    let from = centre(&h);
    drag(&mut h, PointerButton::Primary, from, from + vec2(25.0, 0.0), Modifiers::NONE);
    assert!(h.state().view().center.x < start.x - 1.0, "the Hand tool pans");
    assert!(!h.state().view.is_fit());
}

// ⌘+, ⌘−, ⌘0 and ⌘1 change the workspace's view and never reach a canvas drawn behind the
// dialog; a focused text field keeps them.
#[test]
fn zoom_keys_act_on_the_workspace_view_not_the_canvas_behind() {
    let mut h = harness(App::new());
    let fitted = h.state().view.zoom();

    h.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    h.run();
    assert_eq!(h.state().view.zoom(), 1.0);
    assert!(!h.state().view.is_fit());

    h.key_press_modifiers(Modifiers::COMMAND, Key::Minus);
    h.run();
    assert!(h.state().view.zoom() < 1.0);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Plus);
    h.run();
    assert_eq!(h.state().view.zoom(), 1.0, "in and out are inverse steps from a rung");

    h.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    h.run();
    assert!(h.state().view.is_fit() && h.state().view.zoom() == fitted);
    assert!(!h.state().leaked, "the key was consumed before code below the dialog saw it");

    let mut app = App::new();
    app.focus_text = true;
    let mut h = harness(app);
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    h.run();
    assert!(h.state().view.is_fit(), "a text field keeps the chord");
}

// The Zoom tool: a click zooms in about the point, Alt + click out, a drag zooms to the rect.
#[test]
fn zoom_tool_click_alt_click_and_marquee() {
    let mut app = App::new();
    app.tool = 3;
    let mut h = harness(app);
    let fitted = h.state().view().zoom;
    let point = centre(&h) + vec2(-80.0, 30.0);
    let under = doc_at(&h, point);

    button(&mut h, PointerButton::Primary, point, true, Modifiers::NONE);
    button(&mut h, PointerButton::Primary, point, false, Modifiers::NONE);
    let zoomed = h.state().view().zoom;
    assert!(zoomed > fitted, "a click zooms in");
    let still = doc_at(&h, point);
    assert!((still.x - under.x).abs() < 0.01 && (still.y - under.y).abs() < 0.01, "about the clicked point");

    button(&mut h, PointerButton::Primary, point, true, Modifiers::ALT);
    button(&mut h, PointerButton::Primary, point, false, Modifiers::ALT);
    assert!(h.state().view().zoom < zoomed, "Alt + click zooms out");

    // A drag draws a rectangle and zooms to it.
    let before = h.state().view().zoom;
    let (a, b) = (centre(&h) + vec2(-40.0, -20.0), centre(&h) + vec2(40.0, 20.0));
    let target = doc_at(&h, centre(&h));
    drag(&mut h, PointerButton::Primary, a, b, Modifiers::NONE);
    let rect = h.state().view();
    assert!(rect.zoom > before * 2.0, "the rectangle fills the canvas: {before} -> {}", rect.zoom);
    assert!((rect.center.x - target.x).abs() < 1.0 && (rect.center.y - target.y).abs() < 1.0, "centred on the rectangle");
    assert!(!h.state().view.is_fit());
}

// An overlay at texel 4 covers the same screen rect as the image at every zoom, tinted.
#[test]
fn overlay_source_draws_at_its_texel_scale() {
    for zoom in [0.5, 1.0, 3.0] {
        let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 480.0)), kurbo::Point::new(70.0, 30.0), zoom);
        let image = view.quad(kurbo::Rect::new(0.0, 0.0, 200.0, 100.0));
        let overlay = overlay_view(&view, 4).quad(kurbo::Rect::new(0.0, 0.0, 50.0, 25.0));
        for (a, b) in image.iter().zip(&overlay) {
            assert!((*a - *b).length() < 1e-3, "zoom {zoom}: {a:?} {b:?}");
        }
    }

    let proxy = Solid { size: [50, 25], colour: Color32::from_rgb(200, 100, 50) };
    let tinted = Tinted { source: &proxy, tint: Color32::from_rgb(255, 0, 128) };
    let mut out = ColorImage::filled([2, 1], Color32::TRANSPARENT);
    tinted.read(PixelRect::of_size([2, 1]), 1, &mut out);
    assert_eq!(out.pixels[0], Color32::from_rgb(200, 0, 25), "texels are multiplied by the tint");
    let plain = Tinted { source: &proxy, tint: Color32::WHITE };
    plain.read(PixelRect::of_size([2, 1]), 1, &mut out);
    assert_eq!(out.pixels[0], proxy.colour, "white leaves them as they are");

    // And the canvas draws it without disturbing the view.
    let mut app = App::new();
    app.overlay = Some(Solid { size: [50, 25], colour: Color32::WHITE });
    let mut h = harness(app);
    h.run();
    assert!(h.state().view.is_fit());
}

// The canvas is one Tab stop, a Canvas named "Image" with the zoom as its value.
#[test]
fn canvas_is_one_tab_stop_named_image_with_the_zoom() {
    let mut app = App::new();
    app.focus_text = true;
    let mut h = harness(app);
    h.run();
    let zoom = h.state().view.zoom();
    let canvas = h.get_by_role_and_label(Role::Canvas, "Image");
    let expected = LocaleNumbers::from_tag("en").percent(zoom);
    assert_eq!(canvas.accesskit_node().value().as_deref(), Some(expected.as_str()));

    h.key_press(Key::Tab);
    h.run();
    assert!(h.get_by_role_and_label(Role::Canvas, "Image").accesskit_node().is_focused(), "Tab from the text field lands on the canvas");
    h.key_press(Key::Tab);
    h.run();
    assert!(!h.get_by_role_and_label(Role::Canvas, "Image").accesskit_node().is_focused(), "the canvas holds no further stops");
}
