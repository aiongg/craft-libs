//! Two windows on craft-ui-rt over the winit platform, with a placeholder renderer that records
//! frames and draws nothing: the setup the renderer tasks extend. Each window shows a button that
//! opens a popup layer and a text field that asks the platform for an IME session while focused.
//! Printed: window creation, AT activation (per window), IME requests, text and popup events.
//!
//! `cargo run -p craft-ui-platform-winit --example two_windows` (CRAFT_DEMO_EXIT_AFTER=<seconds>
//! closes both windows after that long, for headless runs).

use std::rc::Rc;
use std::time::{Duration, Instant};

use craft_theme::{Appearance, Contrast, Mode, TextSize, builtins, resolve};
use craft_ui_platform_winit::{WinitPlatform, run};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{
    Align, BuildCx, Constraints, Driver, Element, Event, EventCx, Gesture, Handled, ImePurpose, Layer, LayerSpec, LayoutCx, OutsidePress, PaintCx,
    Placement, Platform, Rect, Rgba, Runtime, Sense, Show, Side, State, Vec2, WindowCommand, WindowSpec,
};

fn tokens() -> craft_ui_rt::Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

/// A clickable box; its output is true once per click.
struct Button {
    size: Vec2,
}

impl Element for Button {
    type Output = bool;
    type Local = ();

    fn props_key(&self) -> u64 {
        1
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        let hovered = cx.interaction().hovered;
        cx.fill_rect(bounds, Rgba([50 + 40 * u8::from(hovered), 60, 90, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => {
                if matches!(p.gesture, Gesture::Up { click: true, .. }) {
                    *cx.output::<bool>() = true;
                }
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// A one-line text field: focusable, asks for an IME session while focused and logs its input.
struct Field {
    window: &'static str,
    at_printed: Rc<std::cell::Cell<bool>>,
}

impl Element for Field {
    type Output = ();
    type Local = String;

    fn props_key(&self) -> u64 {
        2
    }

    fn init(&self, _: &mut BuildCx<'_>) -> String {
        String::new()
    }

    fn layout(&self, _: &mut String, _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(Vec2::new(220.0, 28.0))
    }

    fn paint(&self, _: &String, cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([30, 30, 34, 255]));
        cx.hit(bounds, Sense::HOVER | Sense::CLICK, 0);
    }

    fn event(&self, text: &mut String, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => {
                if matches!(p.gesture, Gesture::Down { .. }) {
                    cx.focus();
                }
                Handled::Yes
            }
            Event::Text(t) | Event::ImeCommit(t) => {
                text.push_str(t);
                println!("[{}] field text: {text:?}", self.window);
                Handled::Yes
            }
            Event::ImePreedit { text: t, .. } => {
                println!("[{}] ime preedit: {t:?}", self.window);
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, text: &String, _: &mut craft_ui_rt::AccessCx<'_>) -> Option<accesskit::Node> {
        if !self.at_printed.replace(true) {
            println!("[{}] AT activated: access tree built", self.window);
        }
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        node.set_value(text.clone());
        Some(node)
    }

    fn focusable(&self) -> bool {
        true
    }

    fn text_input(&self) -> Option<ImePurpose> {
        Some(ImePurpose::Normal)
    }

    fn ime_cursor(&self, _: &String) -> Option<Rect> {
        Some(Rect::from_min_size(craft_ui_rt::Pos2::new(4.0, 4.0), Vec2::new(2.0, 20.0)))
    }
}

struct Popup;

impl Element for Popup {
    type Output = bool;
    type Local = ();

    fn props_key(&self) -> u64 {
        3
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        Vec2::new(160.0, 80.0)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.fill_rect(bounds, Rgba([70, 70, 80, 255]));
        cx.hit(bounds, Sense::CLICK, 0);
    }

    fn event(&self, _: &mut (), cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        if let Event::Dismiss(_) = ev {
            *cx.output::<bool>() = true;
            return Handled::Yes;
        }
        Handled::No
    }
}

struct Root {
    window: &'static str,
    open: State<bool>,
    at_printed: Rc<std::cell::Cell<bool>>,
}

impl Element for Root {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let clicked = Button { size: Vec2::new(120.0, 28.0) }.show(cx);
        let anchor = cx.last_child().expect("the button");
        Field { window: self.window, at_printed: self.at_printed.clone() }.show(cx);
        if clicked {
            let now = self.open.get(cx);
            println!("[{}] popup {}", self.window, if now { "closed" } else { "opened" });
            self.open.set(cx, !now);
        }
        if self.open.get(cx) {
            let spec = LayerSpec::new(Placement::Anchor { node: anchor, side: Side::Below, align: Align::Min, gap: 2.0 })
                .outside_press(OutsidePress::DismissAndConsume);
            if cx.layer(Layer::Popup, "popup", spec, Popup) {
                println!("[{}] popup dismissed by an outside press", self.window);
                self.open.set(cx, false);
            }
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let mut y = 8.0;
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            let s = cx.layout_child(child, c.loosen());
            cx.place_child(child, Vec2::new(8.0, y));
            y += s.y + 8.0;
        }
        c.max
    }
}

fn open_window(d: &mut Driver<WinitPlatform, RecordingRenderer>, name: &'static str, position: craft_ui_rt::Pos2) {
    let open = d.runtime.store_mut().create(false);
    let spec = WindowSpec { size: Some(Vec2::new(360.0, 220.0)), position: Some(position), ..WindowSpec::new(name) };
    let id = d.open_window(&spec, Root { window: name, open, at_printed: Rc::default() });
    println!("[{name}] opened as {id:?}");
}

fn main() {
    let exit_after = std::env::var("CRAFT_DEMO_EXIT_AFTER").ok().and_then(|v| v.parse::<f64>().ok());
    let started = Instant::now();
    let result = run(
        move |platform: WinitPlatform| async move {
            let mut d = Driver::new(Runtime::new(tokens()), platform, RecordingRenderer::default());
            open_window(&mut d, "left", craft_ui_rt::Pos2::new(60.0, 60.0));
            open_window(&mut d, "right", craft_ui_rt::Pos2::new(480.0, 60.0));
            if let Some(secs) = exit_after {
                d.platform.request_frame_at(secs);
            }
            d
        },
        move |d, _report| {
            for window in d.runtime.take_close_requests() {
                println!("{window:?}: close requested, closing");
                d.platform.window(window, WindowCommand::Close);
            }
            if exit_after.is_some_and(|s| started.elapsed() >= Duration::from_secs_f64(s)) {
                let monitors = d.platform.monitors().len();
                println!("exit timer reached ({monitors} monitor(s)); closing every window");
                for window in d.platform.surfaces().windows() {
                    d.platform.window(window, WindowCommand::Close);
                }
            }
        },
    );
    if let Err(e) = result {
        eprintln!("event loop error: {e}");
        std::process::exit(1);
    }
    println!("clean exit");
}
