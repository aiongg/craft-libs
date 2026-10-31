//! The colour area and strip in a headless egui context: the press, the drag gesture, Esc, the
//! keys, kept hue, AccessKit, the dialog Tab order and the focus ring.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba, kept_or_converted};
use egui::accesskit::{Action, Node, NodeId, Orientation, Role};
use egui::epaint::RectShape;
use egui::{Color32, Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, pos2, vec2};

use super::{AreaMode, ColorArea, ColorStrip};
use craft_ui::behavior::FieldTab;
use craft_ui::controls::TextField;
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf, install};

/// A host editing one colour in HSB with an area and a strip between two text fields, in a
/// dialog's [`FieldTab`] scope. It keeps the values last sent with the colour, as a picker does,
/// and puts the start back on a cancel when `revert` is on.
struct Rig {
    ctx: Context,
    time: f64,
    color: Srgba,
    kept: Option<ModelValues>,
    before: (Srgba, Option<ModelValues>),
    revert: bool,
    area: (Id, Rect, Option<EditEvent<ModelValues>>),
    strip: (Id, Rect, Option<EditEvent<ModelValues>>),
    fields: (Id, Id),
    focus_colour: Color32,
    nodes: Vec<(NodeId, Node)>,
    shapes: Vec<Shape>,
}

impl Rig {
    fn new(color: Srgba) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let none = (Id::NULL, Rect::NOTHING, None);
        let mut rig = Self {
            ctx,
            time: 0.0,
            color,
            kept: None,
            before: (color, None),
            revert: true,
            area: none.clone(),
            strip: none,
            fields: (Id::NULL, Id::NULL),
            focus_colour: Color32::TRANSPARENT,
            nodes: Vec::new(),
            shapes: Vec::new(),
        };
        rig.frame(vec![]);
        rig
    }

    fn values(&self) -> ModelValues {
        kept_or_converted(&HsbModel, self.color, self.kept.as_ref())
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            self.focus_colour = Tokens::of(ui).color.line.focus.to_egui();
            FieldTab::new("dialog").show(ui, |ui| {
                self.fields.0 = TextField::new("before", "Before", "a").show(ui).response.id;
                let values = self.values();
                ui.horizontal(|ui| {
                    let edit = ColorArea::new("area", "Saturation and brightness", AreaMode::Hue, values).show(ui);
                    self.area = (edit.response.id, edit.response.rect, edit.event);
                    let edit = ColorStrip::new("strip", "Hue", AreaMode::Hue, values).show(ui);
                    self.strip = (edit.response.id, edit.response.rect, edit.event);
                });
                self.fields.1 = TextField::new("after", "After", "b").show(ui).response.id;
            });
        });
        for event in [self.area.2.clone(), self.strip.2.clone()].into_iter().flatten() {
            self.apply(event);
        }
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        self.shapes = output.shapes.iter().map(|c| c.shape.clone()).collect();
        output.drop_without_applying_deltas();
    }

    fn apply(&mut self, event: EditEvent<ModelValues>) {
        match event {
            EditEvent::Begin { .. } => self.before = (self.color, self.kept),
            EditEvent::Cancel { .. } if self.revert => (self.color, self.kept) = self.before,
            EditEvent::Cancel { .. } => {}
            other => {
                if let Some(v) = other.value() {
                    self.color = HsbModel.to_srgb(&v.values);
                    self.kept = Some(*v);
                }
            }
        }
    }

    /// Where saturation `s` and brightness `b` are in the area.
    fn area_at(&self, s: f32, b: f32) -> Pos2 {
        let r = self.area.1;
        pos2(r.left() + s / 100.0 * r.width(), r.bottom() - b / 100.0 * r.height())
    }

    fn press(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        self.frame(vec![button(pos, false)]);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::ModifiersChanged(modifiers), Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    fn focus(&mut self, id: Id) {
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frame(vec![]);
    }

    fn focused(&self) -> Option<Id> {
        self.ctx.memory(|m| m.focused())
    }

    fn node(&self, id: Id) -> &Node {
        let id = id.accesskit_id();
        self.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| node).unwrap_or_else(|| panic!("no AccessKit node {id:?}"))
    }

    fn area_text(&self) -> String {
        self.node(self.area.0).value().unwrap_or_default().to_owned()
    }

    /// Strokes of `color` round `rect` (outer edge `outset` points out).
    fn rings(&self, rect: Rect, outset: f32, color: Color32) -> usize {
        fn collect(shape: &Shape, out: &mut Vec<RectShape>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
                Shape::Rect(r) => out.push(r.clone()),
                _ => {}
            }
        }
        let mut rects = Vec::new();
        self.shapes.iter().for_each(|s| collect(s, &mut rects));
        let around = rect.expand(outset);
        let near = |r: Rect| (r.min - around.min).length() + (r.max - around.max).length() < 0.01;
        rects.iter().filter(|r| r.stroke.color == color && r.stroke.width > 0.0 && near(r.rect.expand(r.stroke.width))).count()
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn hsb(h: f32, s: f32, b: f32) -> ModelValues {
    ModelValues::new("hsb", &[h, s, b])
}

fn begin(event: Option<EditEvent<ModelValues>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id, .. }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update(id: GestureId, value: ModelValues) -> Option<EditEvent<ModelValues>> {
    Some(EditEvent::update(id, value, Modifiers::NONE))
}

fn set(value: ModelValues, modifiers: Modifiers) -> Option<EditEvent<ModelValues>> {
    Some(EditEvent::set(value, modifiers))
}

#[test]
fn a_press_sets_the_colour_in_the_press_frame_and_a_drag_is_one_gesture() {
    let mut rig = Rig::new(HsbModel.to_srgb(&hsb(240.0, 50.0, 50.0).values));
    rig.press(rig.area_at(25.0, 75.0));
    let id = begin(rig.area.2.clone());
    assert_eq!(rig.area_text(), "Saturation 25 %, Brightness 75 %", "the press frame draws the value under the pointer");
    assert_eq!(rig.focused(), Some(rig.area.0), "a press focuses the area");
    rig.frame(vec![]);
    assert_eq!(rig.area.2, update(id, hsb(240.0, 25.0, 75.0)));
    rig.drag(rig.area_at(60.0, 30.0));
    assert_eq!(rig.area.2, update(id, hsb(240.0, 60.0, 30.0)));
    // Outside the area the value stays at the edge.
    rig.drag(rig.area_at(140.0, -20.0));
    assert_eq!(rig.area.2, update(id, hsb(240.0, 100.0, 0.0)));
    rig.release(rig.area_at(140.0, -20.0));
    assert_eq!(rig.area.2, Some(EditEvent::commit(id, hsb(240.0, 100.0, 0.0), true, Modifiers::NONE)));

    // A press and release in one frame is a Set.
    let at = rig.area_at(10.0, 90.0);
    rig.frame(vec![Event::PointerMoved(at), button(at, true), button(at, false)]);
    assert_eq!(rig.area.2, set(hsb(240.0, 10.0, 90.0), Modifiers::NONE));

    // The strip: the same on its own component.
    let strip = rig.strip.1;
    rig.press(pos2(strip.center().x, strip.bottom() - strip.height() / 4.0));
    let id = begin(rig.strip.2.clone());
    rig.frame(vec![]);
    assert_eq!(rig.strip.2, update(id, hsb(90.0, 10.0, 90.0)));
}

#[test]
fn esc_cancels_a_drag_and_the_area_shows_the_start_value() {
    let mut rig = Rig::new(HsbModel.to_srgb(&hsb(240.0, 50.0, 50.0).values));
    rig.revert = false;
    rig.press(rig.area_at(50.0, 50.0));
    let id = begin(rig.area.2.clone());
    rig.drag(rig.area_at(80.0, 20.0));
    assert_eq!(rig.area.2, update(id, hsb(240.0, 80.0, 20.0)));
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.area.2, Some(EditEvent::cancel(id, CancelReason::Escape)));
    rig.frame(vec![]);
    assert_eq!(rig.area_text(), "Saturation 50 %, Brightness 50 %", "the host still shows the drag; the area shows the start");
    rig.drag(rig.area_at(10.0, 10.0));
    rig.release(rig.area_at(10.0, 10.0));
    assert_eq!(rig.area.2, None, "the drag Esc ended goes no further");
}

#[test]
fn keys_are_sets_and_change_nothing_during_a_drag() {
    let mut rig = Rig::new(HsbModel.to_srgb(&hsb(240.0, 50.0, 50.0).values));
    rig.focus(rig.area.0);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.area.2, set(hsb(240.0, 51.0, 50.0), Modifiers::NONE));
    rig.key(Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(rig.area.2, set(hsb(240.0, 51.0, 60.0), Modifiers::SHIFT));
    rig.key(Key::PageDown, Modifiers::NONE);
    assert_eq!(rig.area.2, set(hsb(240.0, 51.0, 50.0), Modifiers::NONE));
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.area.2, set(hsb(240.0, 100.0, 50.0), Modifiers::NONE));

    rig.focus(rig.strip.0);
    rig.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(rig.strip.2, set(hsb(241.0, 100.0, 50.0), Modifiers::NONE));
    rig.key(Key::PageDown, Modifiers::NONE);
    assert_eq!(rig.strip.2, set(hsb(205.0, 100.0, 50.0), Modifiers::NONE));
    rig.key(Key::Home, Modifiers::NONE);
    assert_eq!(rig.strip.2, set(hsb(0.0, 100.0, 50.0), Modifiers::NONE));

    rig.press(rig.area_at(50.0, 50.0));
    let id = begin(rig.area.2.clone());
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.area.2, update(id, hsb(0.0, 50.0, 50.0)), "the press's value goes out, End changes nothing");
    rig.key(Key::PageUp, Modifiers::NONE);
    assert_eq!(rig.area.2, None);
}

#[test]
fn hue_survives_a_drag_to_grey_and_to_black() {
    let mut rig = Rig::new(HsbModel.to_srgb(&hsb(240.0, 50.0, 50.0).values));
    rig.press(rig.area_at(50.0, 50.0));
    rig.drag(rig.area_at(0.0, 50.0));
    assert!(rig.color.same_rgb8(Srgba::rgb(0.5, 0.5, 0.5)), "grey: {:?}", rig.color);
    assert_eq!(rig.values(), hsb(240.0, 0.0, 50.0), "a grey keeps its hue");
    rig.drag(rig.area_at(60.0, 0.0));
    assert!(rig.color.same_rgb8(Srgba::rgb(0.0, 0.0, 0.0)), "black: {:?}", rig.color);
    rig.release(rig.area_at(60.0, 0.0));
    rig.frame(vec![]);
    assert_eq!(rig.values(), hsb(240.0, 60.0, 0.0), "black keeps hue and saturation");
    assert_eq!(rig.node(rig.strip.0).numeric_value(), Some(240.0), "the strip still shows the hue");
}

#[test]
fn accesskit_names_the_area_by_its_components_and_the_strip_by_its_one() {
    let rig = Rig::new(HsbModel.to_srgb(&hsb(120.0, 50.0, 100.0).values));
    let area = rig.node(rig.area.0);
    assert_eq!((area.role(), area.label()), (Role::Slider, Some("Saturation and brightness")));
    assert_eq!(area.value(), Some("Saturation 50 %, Brightness 100 %"));
    assert_eq!(area.numeric_value(), None);
    assert!(area.supports_action(Action::Focus) && area.supports_action(Action::Click));

    let strip = rig.node(rig.strip.0);
    assert_eq!((strip.role(), strip.label(), strip.orientation()), (Role::Slider, Some("Hue"), Some(Orientation::Vertical)));
    assert_eq!((strip.numeric_value(), strip.min_numeric_value(), strip.max_numeric_value()), (Some(120.0), Some(0.0), Some(360.0)));
    assert_eq!(strip.numeric_value_step(), Some(1.0));
    assert!(strip.supports_action(Action::Increment) && strip.supports_action(Action::Decrement) && strip.supports_action(Action::SetValue));
}

#[test]
fn field_tab_skips_the_area_and_strip_and_ctrl_tab_reaches_them() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.focus(rig.fields.0);
    rig.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.focused(), Some(rig.fields.1), "Tab goes from field to field");
    rig.focus(rig.fields.0);
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.focused(), Some(rig.area.0), "Ctrl+Tab reaches the area");
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.focused(), Some(rig.strip.0), "and then the strip");
}

#[test]
fn the_focus_ring_is_painted_round_the_focused_part_outside_its_edge() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.focus(rig.fields.0);
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.focused(), Some(rig.area.0));
    // The edge is one point outside the rect, the ring one point further out and one wide.
    assert_eq!(rig.rings(rig.area.1, 3.0, rig.focus_colour), 1, "a ring round the area");
    assert_eq!(rig.rings(rig.strip.1, 3.0, rig.focus_colour), 0);
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.rings(rig.strip.1, 3.0, rig.focus_colour), 1, "a ring round the strip");
    assert_eq!(rig.rings(rig.area.1, 3.0, rig.focus_colour), 0);
}

#[test]
fn right_to_left_keeps_the_area_axes() {
    let mut rig = Rig::new(HsbModel.to_srgb(&hsb(240.0, 50.0, 50.0).values));
    craft_ui::direction::apply(&rig.ctx, craft_ui::direction::Direction::Rtl);
    rig.frame(vec![]);
    rig.press(rig.area_at(80.0, 30.0));
    let id = begin(rig.area.2.clone());
    rig.frame(vec![]);
    assert_eq!(rig.area.2, update(id, hsb(240.0, 80.0, 30.0)), "saturation still grows to the right");
}

/// The area's names follow its axes: across, then down.
#[test]
fn each_area_mode_is_named_by_its_two_components() {
    for mode in AreaMode::ALL {
        let channels = mode.model().channels();
        let [across, down] = mode.area();
        let expected = format!("{} and {}", channels[across].name, channels[down].name.to_lowercase());
        assert_eq!(mode.name(), expected, "{mode:?}");
    }
}
