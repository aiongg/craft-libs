//! `bands` on the colour area and strip: the pointer snaps to the cell under it, a key steps one
//! band, and only band values are reported.

use craft_theme::{ColorModel, ModelValues, RgbModel, Srgba};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{AreaMode, ColorArea, ColorStrip};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

const BANDS: usize = 6;

/// A banded red strip and blue-by-green area editing one RGB colour.
struct Rig {
    ctx: Context,
    time: f64,
    values: ModelValues,
    area: (Id, Rect, Option<EditEvent<ModelValues>>),
    strip: (Id, Rect, Option<EditEvent<ModelValues>>),
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let none = (Id::NULL, Rect::NOTHING, None);
        let values = RgbModel.from_srgb(Srgba::rgb(0.2, 0.2, 0.2));
        let mut rig = Self { ctx, time: 0.0, values, area: none.clone(), strip: none };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let values = self.values;
        let output = ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| {
                let edit = ColorArea::new("area", "Blue and green", AreaMode::Red, values).size(vec2(120.0, 120.0)).bands(BANDS).show(ui);
                self.area = (edit.response.id, edit.response.rect, edit.event);
                let edit = ColorStrip::new("strip", "Red", AreaMode::Red, values).size(vec2(20.0, 120.0)).bands(BANDS).show(ui);
                self.strip = (edit.response.id, edit.response.rect, edit.event);
            });
        });
        for event in [self.area.2.clone(), self.strip.2.clone()].into_iter().flatten() {
            if let Some(v) = event.value() {
                self.values = *v;
            }
        }
        output.drop_without_applying_deltas();
    }

    /// A press and release in one frame at `pos`.
    fn tap(&mut self, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerMoved(pos), button(true), button(false)]);
    }

    fn key(&mut self, id: Id, key: Key, modifiers: Modifiers) {
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frame(vec![]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    /// The red, green and blue the last event reported.
    fn reported(event: &Option<EditEvent<ModelValues>>) -> [f32; 3] {
        let v = event.as_ref().and_then(|e| e.value()).unwrap_or_else(|| panic!("no value in {event:?}"));
        [v.values[0], v.values[1], v.values[2]]
    }
}

fn assert_levels(got: [f32; 3], want: [u8; 3]) {
    for (g, w) in got.into_iter().zip(want) {
        assert!((g - f32::from(w)).abs() < 1e-3, "{got:?} is not {want:?}");
    }
}

#[test]
fn bands_snap_the_pointer_and_report_only_band_values() {
    let mut rig = Rig::new();
    let area = rig.area.1;
    // Cell 20 points wide and high: the fourth column (blue 153), the second row (green 204).
    rig.tap(pos2(area.left() + 3.0 * 20.0 + 3.0, area.top() + 20.0 + 17.0));
    assert_levels(Rig::reported(&rig.area.2), [51, 204, 153]);
    // Anywhere in the cell is the same band.
    rig.tap(pos2(area.left() + 3.0, area.top() + 3.0));
    assert_levels(Rig::reported(&rig.area.2), [51, 255, 0]);
    rig.tap(pos2(area.left() + 3.0 * 20.0 + 17.0, area.top() + 20.0 + 2.0));
    assert_levels(Rig::reported(&rig.area.2), [51, 204, 153]);
    // The ends are the first and last bands.
    rig.tap(pos2(area.right() - 0.5, area.bottom() - 0.5));
    assert_levels(Rig::reported(&rig.area.2), [51, 0, 255]);

    let strip = rig.strip.1;
    rig.tap(pos2(strip.center().x, strip.bottom() - 2.0 * 20.0 - 4.0));
    assert_levels(Rig::reported(&rig.strip.2), [102, 0, 255]);
    rig.tap(pos2(strip.center().x, strip.top() + 1.0));
    assert_levels(Rig::reported(&rig.strip.2), [255, 0, 255]);

    // A key steps one band.
    rig.key(rig.strip.0, Key::ArrowDown, Modifiers::NONE);
    assert_levels(Rig::reported(&rig.strip.2), [204, 0, 255]);
    rig.key(rig.area.0, Key::ArrowUp, Modifiers::NONE);
    assert_levels(Rig::reported(&rig.area.2), [204, 51, 255]);
    rig.key(rig.area.0, Key::ArrowLeft, Modifiers::NONE);
    assert_levels(Rig::reported(&rig.area.2), [204, 51, 204]);
}
