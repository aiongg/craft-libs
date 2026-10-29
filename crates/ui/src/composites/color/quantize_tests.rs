//! `quantize` on the colour area and strip: the reported colour has every red, green and blue
//! channel on a level whatever the mode, a key steps to the next different colour, and the field
//! draws flat cells of the snapped colours.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{AreaMode, ColorArea, ColorStrip};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

const LEVELS: usize = 6;

/// A quantised strip and area of one mode editing one colour.
struct Rig {
    ctx: Context,
    time: f64,
    mode: AreaMode,
    values: ModelValues,
    meshes: Vec<egui::Mesh>,
    area: (Id, Rect, Option<EditEvent<ModelValues>>),
    strip: (Id, Rect, Option<EditEvent<ModelValues>>),
}

impl Rig {
    fn new(mode: AreaMode) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let none = (Id::NULL, Rect::NOTHING, None);
        let values = mode.model().from_srgb(Srgba::rgb(0.2, 0.2, 0.2));
        let mut rig = Self { ctx, time: 0.0, mode, values, meshes: Vec::new(), area: none.clone(), strip: none };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let (values, mode) = (self.values, self.mode);
        let output = ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| {
                let edit = ColorArea::new("area", "Area", mode, values).size(vec2(120.0, 120.0)).quantize(LEVELS).show(ui);
                self.area = (edit.response.id, edit.response.rect, edit.event);
                let edit = ColorStrip::new("strip", "Strip", mode, values).size(vec2(20.0, 120.0)).quantize(LEVELS).show(ui);
                self.strip = (edit.response.id, edit.response.rect, edit.event);
            });
        });
        for event in [self.area.2.clone(), self.strip.2.clone()].into_iter().flatten() {
            if let Some(v) = event.value() {
                self.values = *v;
            }
        }
        self.meshes = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Mesh(mesh) => Some((**mesh).clone()),
                _ => None,
            })
            .collect();
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

    /// The last event's values.
    fn event_values(event: &Option<EditEvent<ModelValues>>) -> ModelValues {
        *event.as_ref().and_then(|e| e.value()).unwrap_or_else(|| panic!("no value in {event:?}"))
    }

    /// The red, green and blue of the values the host holds, as 8-bit levels.
    fn held(&self) -> [u8; 3] {
        let [r, g, b, _] = self.mode.model().to_srgb(&self.values.values).to_rgba().0;
        [r, g, b]
    }

    /// The red, green and blue the last event reported, as 8-bit levels.
    fn reported(&self, event: &Option<EditEvent<ModelValues>>) -> [u8; 3] {
        let [r, g, b, _] = self.mode.model().to_srgb(&Self::event_values(event).values).to_rgba().0;
        [r, g, b]
    }
}

const WEB: [u8; 6] = [0, 51, 102, 153, 204, 255];

fn assert_web(got: [u8; 3]) {
    assert!(got.iter().all(|c| WEB.contains(c)), "{got:?} is not a web colour");
}

#[test]
fn quantize_snaps_the_pointer_to_the_nearest_colour_and_a_key_steps_to_the_next_one() {
    let mut rig = Rig::new(AreaMode::Red);
    let area = rig.area.1;
    // Blue 65 % of the way (153 after snapping from 165.75), green 85 % up (204 from 216.75).
    rig.tap(pos2(area.left() + 0.65 * 120.0, area.top() + 0.15 * 120.0));
    assert_eq!(rig.reported(&rig.area.2), [51, 204, 153]);
    // Anywhere near the same colour is the same colour: nothing changes.
    rig.tap(pos2(area.left() + 0.65 * 120.0 + 4.0, area.top() + 0.15 * 120.0 + 3.0));
    assert!(rig.area.2.is_none(), "{:?}", rig.area.2);
    // The corners are the first and last levels.
    rig.tap(pos2(area.right() - 0.5, area.bottom() - 0.5));
    assert_eq!(rig.reported(&rig.area.2), [51, 0, 255]);

    let strip = rig.strip.1;
    rig.tap(pos2(strip.center().x, strip.bottom() - 0.45 * 120.0));
    assert_eq!(rig.reported(&rig.strip.2), [102, 0, 255]);
    rig.tap(pos2(strip.center().x, strip.top() + 1.0));
    assert_eq!(rig.reported(&rig.strip.2), [255, 0, 255]);

    // A key moves to the next different colour, not by the component's own step.
    rig.key(rig.strip.0, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.reported(&rig.strip.2), [204, 0, 255]);
    rig.key(rig.area.0, Key::ArrowUp, Modifiers::NONE);
    assert_eq!(rig.reported(&rig.area.2), [204, 51, 255]);
    rig.key(rig.area.0, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.reported(&rig.area.2), [204, 51, 204]);
}

#[test]
fn quantize_in_hsb_mode_reports_web_colours_in_hsb_values() {
    let mut rig = Rig::new(AreaMode::Hue);
    let (area, strip) = (rig.area.1, rig.strip.1);
    for (fx, fy) in [(0.1, 0.9), (0.37, 0.52), (0.73, 0.21), (1.0, 0.0), (0.0, 1.0)] {
        rig.tap(pos2(area.left() + fx * area.width(), area.top() + fy * area.height()));
        assert_web(rig.held());
        assert_eq!(rig.values, HsbModel.from_srgb(HsbModel.to_srgb(&rig.values.values)), "the values are the snapped colour's own");
    }
    for fy in [0.05, 0.33, 0.5, 0.8, 0.95] {
        rig.tap(pos2(strip.center().x, strip.top() + fy * strip.height()));
        assert_web(rig.held());
    }
    // A key still gets to another colour although one hue step is far below a level.
    rig.tap(pos2(area.left() + 0.9 * area.width(), area.top() + 0.1 * area.height()));
    rig.tap(pos2(strip.center().x, strip.top() + 0.5 * strip.height()));
    let before = rig.held();
    rig.key(rig.strip.0, Key::ArrowUp, Modifiers::NONE);
    let after = rig.held();
    assert_web(after);
    assert_ne!(before, after);
}

#[test]
fn the_quantised_field_and_strip_draw_flat_web_colour_cells_without_a_repeated_hue_end() {
    let rig = Rig::new(AreaMode::Hue);
    let cells = |count: usize| rig.meshes.iter().filter(|m| m.vertices.len() == 4 * count).collect::<Vec<_>>();
    let field = cells(64 * 64);
    let strip = cells(64);
    assert_eq!((field.len(), strip.len()), (1, 1), "a 64 by 64 field and 64 rows");
    for mesh in field.iter().chain(&strip) {
        for quad in mesh.vertices.chunks(4) {
            assert!(quad.iter().all(|v| v.color == quad[0].color), "a cell has one colour");
            assert_web([quad[0].color.r(), quad[0].color.g(), quad[0].color.b()]);
        }
    }
    // Hue runs from the top (near 360) to the bottom (near 0): the cells sample the middle of each
    // row, so neither end is a cell of its own.
    let rows: Vec<_> = strip[0].vertices.chunks(4).map(|q| q[0].pos.y).collect();
    assert!(rows.windows(2).all(|w| w[0] < w[1]), "one cell per row, top to bottom");
}
