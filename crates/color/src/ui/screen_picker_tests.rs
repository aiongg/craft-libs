//! Behaviour tests of `screen_picker`: a fake capture in place of the screen.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use craft_platform::screen::ScreenImage;
use egui::{Context, Id, Key, PointerButton, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::screen_picker::{CaptureResult, ScreenColorPicker, Source, frame, lock, runtime};

struct Fake(Mutex<Receiver<CaptureResult>>);

impl Source for Fake {
    fn poll(&self) -> Option<CaptureResult> {
        self.0.lock().unwrap().try_recv().ok()
    }
}

/// Two screen pixels side by side: red, then azure.
fn capture() -> CaptureResult {
    Ok(vec![ScreenImage {
        origin: [0, 0],
        size: [2, 1],
        scale: 1.0,
        rgba: vec![255, 0, 0, 255, 0, 128, 255, 255].into(),
        icc_profile: None,
        name: "Fake".into(),
    }])
}

const LEFT: egui::Pos2 = egui::pos2(40.0, 50.0);

/// A harness whose single pass is the capture window's frame over the running pick's first
/// snapshot (a 200 x 100 window).
fn window() -> Harness<'static> {
    Harness::builder().with_size(vec2(200.0, 100.0)).build_ui(|ui| {
        let shared = runtime(ui.ctx());
        let snapshot = lock(&shared).images.first().cloned();
        if let Some(snapshot) = snapshot {
            frame(ui, &shared, &snapshot);
        }
    })
}

/// Starts a pick for `owner` on a fake capture that has already finished, and polls it in.
fn started(ctx: &Context, owner: Id, target: u64) -> Sender<CaptureResult> {
    let (tx, rx) = channel();
    tx.send(capture()).unwrap();
    ScreenColorPicker::open(ctx, owner);
    let rx = std::cell::Cell::new(Some(rx));
    ScreenColorPicker::tick_with(ctx, target, &|| Box::new(Fake(Mutex::new(rx.take().unwrap_or_else(|| channel().1)))));
    tx
}

fn click(h: &mut Harness<'_>, at: egui::Pos2) {
    h.hover_at(at);
    h.run();
    h.drag_at(at);
    h.run();
    h.drop_at(at);
    h.run();
}

/// A pick's result goes only to the widget that asked.
#[test]
fn the_result_goes_to_its_owner_only() {
    let mut h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let _tx = started(&ctx, owner, 1);
    assert!(ScreenColorPicker::showing(&ctx) && ScreenColorPicker::busy(&ctx));
    click(&mut h, LEFT);
    ScreenColorPicker::tick_with(&ctx, 1, &|| unreachable!());
    assert!(!ScreenColorPicker::busy(&ctx) && !ScreenColorPicker::showing(&ctx));
    assert_eq!(ScreenColorPicker::take(&ctx, Id::unique("other")), None);
    assert_eq!(ScreenColorPicker::take(&ctx, owner).map(|c| c.to_hex()), Some("#ff0000".into()));
    assert_eq!(ScreenColorPicker::take(&ctx, owner), None, "taken once");
}

/// Esc, a right-click and a changed editing target cancel with no colour.
#[test]
fn every_way_out_cancels() {
    let mut h = window();
    let ctx = h.ctx.clone();
    let owner = Id::unique("asker");
    let ends = |ctx: &Context| {
        ScreenColorPicker::tick_with(ctx, 1, &|| unreachable!());
        assert!(!ScreenColorPicker::busy(ctx) && !ScreenColorPicker::showing(ctx));
        assert_eq!(ScreenColorPicker::take(ctx, Id::unique("asker")), None);
    };

    let _tx = started(&ctx, owner, 1);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    ends(&ctx);

    let _tx = started(&ctx, owner, 1);
    h.hover_at(LEFT);
    h.run();
    let secondary = |pressed| egui::Event::PointerButton { pos: LEFT, button: PointerButton::Secondary, pressed, modifiers: Default::default() };
    h.event(secondary(true));
    h.run();
    h.event(secondary(false));
    h.run();
    ends(&ctx);

    let _tx = started(&ctx, owner, 1);
    ScreenColorPicker::tick_with(&ctx, 2, &|| unreachable!());
    assert!(!ScreenColorPicker::busy(&ctx) && !ScreenColorPicker::showing(&ctx));
    assert_eq!(ScreenColorPicker::take(&ctx, owner), None);
}

/// The button is disabled while a pick runs.
#[test]
fn the_button_is_disabled_while_busy() {
    let owner = Id::unique("asker");
    let mut h = Harness::new_ui(move |ui| {
        ScreenColorPicker::button(ui, owner);
    });
    h.run();
    assert!(!h.get_by_label("Pick screen color").accesskit_node().is_disabled());
    ScreenColorPicker::open(&h.ctx, owner);
    h.run();
    assert!(h.get_by_label("Pick screen color").accesskit_node().is_disabled());
}
