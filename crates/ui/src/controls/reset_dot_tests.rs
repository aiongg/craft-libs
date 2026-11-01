//! `ResetDot` in a harness.

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::ResetDot;
use crate::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    changed: bool,
    clicked: bool,
    rect: Rect,
}

impl Rig {
    fn new(changed: bool) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, changed, clicked: false, rect: Rect::NOTHING };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let response = ResetDot::new("dot", self.changed, "On").show(ui);
            self.rect = response.rect;
            self.clicked = response.clicked();
        });
        output.drop_without_applying_deltas();
    }

    fn click(&mut self) -> bool {
        let at = self.rect.center();
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.clicked
    }

    /// Tab from nothing: whether the dot took the focus.
    fn tabbed_to(&mut self) -> bool {
        self.frame(vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        self.frame(vec![]);
        self.ctx.memory(|m| m.focused().is_some())
    }
}

// A changed dot reports `clicked()`; one at its default does not and is not a Tab stop.
#[test]
fn a_changed_dot_resets_on_a_click() {
    assert!(Rig::new(true).click(), "a changed dot is clicked");
    assert!(!Rig::new(false).click(), "a dot at its default does nothing");
    assert!(Rig::new(true).tabbed_to());
    assert!(!Rig::new(false).tabbed_to());
}
