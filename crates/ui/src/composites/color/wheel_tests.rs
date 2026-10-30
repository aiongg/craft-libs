//! The colour wheel in a headless egui context: geometry, the press and drag on each part, keys,
//! the edit lifecycle, kept hue, AccessKit, the dialog Tab order and right to left.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba, kept_or_converted};
use egui::accesskit::{Action, Node, NodeId, Role};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::wheel::{WheelGeometry, hue_at, hue_direction, triangle_corners, triangle_point, triangle_sv};
use super::{ColorWheel, ColorWheelStyle};
use crate::behavior::FieldTab;
use crate::controls::TextField;
use crate::edit::{CancelReason, EditEvent, GestureId};
use crate::theme::{Base, Tokens, TokensOf, install};

/// A host editing one colour in HSB with a wheel between two text fields in a dialog's
/// [`FieldTab`] scope, keeping the values last sent as a picker does.
struct Rig {
    ctx: Context,
    time: f64,
    color: Srgba,
    kept: Option<ModelValues>,
    before: (Srgba, Option<ModelValues>),
    event: Option<EditEvent<ModelValues>>,
    geometry: WheelGeometry,
    ids: (Id, Id),
    fields: (Id, Id),
    style: ColorWheelStyle,
    nodes: Vec<(NodeId, Node)>,
    /// How far the layer sits from the screen's origin; `geometry` stays in layer coordinates.
    offset: egui::Vec2,
}

impl Rig {
    fn new(values: ModelValues) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            color: HsbModel.to_srgb(&values.values),
            kept: Some(values),
            before: (Srgba::rgb(0.0, 0.0, 0.0), None),
            event: None,
            geometry: WheelGeometry { center: Pos2::ZERO, inner: 0.0, outer: 0.0, triangle: 0.0 },
            ids: (Id::NULL, Id::NULL),
            fields: (Id::NULL, Id::NULL),
            style: default_style(),
            nodes: Vec::new(),
            offset: egui::Vec2::ZERO,
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
        let offset = self.offset;
        let output = ctx.run_ui(input, |ui| {
            ui.ctx().set_transform_layer(ui.layer_id(), egui::emath::TSTransform::from_translation(offset));
            let style = ColorWheel::style(&Tokens::of(ui));
            self.style = style;
            FieldTab::new("dialog").show(ui, |ui| {
                self.fields.0 = TextField::new("before", "Before", "a").show(ui).response.id;
                let wheel = ColorWheel::new("wheel", "Color wheel", self.values()).show(ui);
                self.event = wheel.edit.event;
                self.geometry = wheel.geometry;
                self.ids = (wheel.ring, wheel.triangle);
                self.fields.1 = TextField::new("after", "After", "b").show(ui).response.id;
            });
        });
        match self.event.clone() {
            Some(EditEvent::Begin { .. }) => self.before = (self.color, self.kept),
            Some(EditEvent::Cancel { .. }) => (self.color, self.kept) = self.before,
            Some(other) => {
                if let Some(v) = other.value() {
                    self.color = HsbModel.to_srgb(&v.values);
                    self.kept = Some(*v);
                }
            }
            None => {}
        }
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
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

    fn click(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
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

    /// On the ring at `hue` degrees.
    fn on_ring(&self, hue: f32) -> Pos2 {
        self.geometry.ring_knob(hue)
    }

    /// In the triangle at saturation `s` and brightness `b` (percents) at `hue`.
    fn in_triangle(&self, hue: f32, s: f32, b: f32) -> Pos2 {
        triangle_point(self.geometry.corners(hue, 0.0), s / 100.0, b / 100.0)
    }
}

fn default_style() -> ColorWheelStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    let output = ctx.run_ui(RawInput::default(), |ui| style = Some(ColorWheel::style(&Tokens::of(ui))));
    output.drop_without_applying_deltas();
    style.unwrap_or_else(|| panic!("no style"))
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn hsb(h: f32, s: f32, b: f32) -> ModelValues {
    ModelValues::new("hsb", &[h, s, b])
}

fn begin(event: Option<EditEvent<ModelValues>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update(id: GestureId, value: ModelValues) -> Option<EditEvent<ModelValues>> {
    Some(EditEvent::Update { id, value, modifiers: Modifiers::NONE })
}

fn set(value: ModelValues) -> Option<EditEvent<ModelValues>> {
    Some(EditEvent::Set { value, modifiers: Modifiers::NONE })
}

#[test]
fn geometry_follows_the_area_and_its_points_round_trip() {
    let rig = Rig::new(hsb(0.0, 100.0, 100.0));
    let style = rig.style;
    for (side, ring) in [(100.0, style.ring_min), (1000.0, style.ring_max)] {
        let g = ColorWheel::geometry(&style, Rect::from_min_size(pos2(10.0, 20.0), vec2(side, side)));
        assert_eq!(g.center, pos2(10.0 + side / 2.0, 20.0 + side / 2.0));
        assert_eq!(g.outer, side / 2.0 - style.outer_inset);
        assert_eq!(g.outer - g.inner, ring, "the ring width clamps at {ring} on a {side} area");
        assert_eq!(g.triangle, g.inner - style.triangle_inset);
    }
    let center = pos2(50.0, 50.0);
    for hue in [0.0, 1.0, 90.0, 179.0, 270.0, 359.0] {
        assert_eq!(hue_at(center, center + hue_direction(hue) * 30.0), Some(hue));
    }
    assert_eq!(hue_at(center, pos2(f32::NAN, 3.0)), None);
    let corners = triangle_corners(center, 40.0, 200.0);
    for (s, b) in [(0.0, 0.0), (1.0, 1.0), (0.25, 0.8), (0.0, 1.0)] {
        let (rs, rb) = triangle_sv(corners, triangle_point(corners, s, b)).unwrap_or_default();
        assert!((rs - s).abs() < 1e-4 && (rb - b).abs() < 1e-4, "({s}, {b}) came back as ({rs}, {rb})");
    }
    assert_eq!(triangle_sv(corners, pos2(f32::NAN, 0.0)), None);
    assert_eq!(triangle_sv(triangle_corners(center, 0.0, 0.0), center), None, "a degenerate triangle picks nothing");
    let outside = triangle_sv(corners, center + vec2(500.0, 0.0)).unwrap_or_default();
    assert!((0.0..=1.0).contains(&outside.0) && (0.0..=1.0).contains(&outside.1));
}

#[test]
fn a_press_on_the_ring_turns_the_hue_and_the_part_stays_for_the_whole_drag() {
    let mut rig = Rig::new(hsb(240.0, 50.0, 60.0));
    rig.press(rig.on_ring(90.0));
    let id = begin(rig.event.clone());
    assert_eq!(rig.focused(), Some(rig.ids.0), "a press focuses the ring");
    assert_eq!(rig.node(rig.ids.0).numeric_value(), Some(90.0), "the press frame draws the value");
    rig.frame(vec![]);
    assert_eq!(rig.event, update(id, hsb(90.0, 50.0, 60.0)), "only the hue changes");
    // The pointer leaves the ring for the triangle and the hue still follows its angle.
    rig.drag(rig.geometry.center + hue_direction(180.0) * (rig.geometry.inner * 0.5));
    assert_eq!(rig.event, update(id, hsb(180.0, 50.0, 60.0)));
    rig.release(rig.geometry.center);
    assert!(matches!(rig.event, Some(EditEvent::Commit { changed: true, .. })), "{:?}", rig.event);
}

#[test]
fn a_press_in_the_triangle_picks_saturation_and_brightness_and_keeps_the_hue() {
    let mut rig = Rig::new(hsb(240.0, 50.0, 60.0));
    rig.press(rig.in_triangle(240.0, 25.0, 75.0));
    let id = begin(rig.event.clone());
    assert_eq!(rig.focused(), Some(rig.ids.1), "a press focuses the triangle");
    rig.frame(vec![]);
    assert_eq!(rig.event, update(id, hsb(240.0, 25.0, 75.0)));
    // Dragging out over the ring keeps editing the triangle, at its edge.
    rig.drag(rig.on_ring(240.0));
    assert_eq!(rig.event.as_ref().and_then(|e| e.value()).map(|v| v.values[0]), Some(240.0));
    // The black corner: the hue survives a drag to black.
    rig.drag(rig.geometry.corners(240.0, 0.0)[2]);
    rig.release(rig.geometry.corners(240.0, 0.0)[2]);
    rig.frame(vec![]);
    assert!(rig.color.same_rgb8(Srgba::rgb(0.0, 0.0, 0.0)), "{:?}", rig.color);
    assert_eq!(rig.values().values[0], 240.0, "black keeps its hue");

    // A press and release in one frame is a Set.
    rig.click(rig.in_triangle(240.0, 10.0, 90.0));
    assert_eq!(rig.event, set(hsb(240.0, 10.0, 90.0)));
    rig.click(rig.on_ring(30.0));
    assert_eq!(rig.event, set(hsb(30.0, 10.0, 90.0)));
}

#[test]
fn a_press_beside_the_wheel_starts_nothing_and_right_to_left_keeps_red_at_three_oclock() {
    let mut rig = Rig::new(hsb(240.0, 50.0, 60.0));
    let beside = rig.geometry.center + vec2(rig.geometry.outer + rig.style.grab_outer + 3.0, 0.0);
    rig.press(beside);
    assert_eq!(rig.event, None);
    rig.drag(rig.on_ring(10.0));
    assert_eq!(rig.event, None, "a drag that began beside the wheel picks nothing");
    rig.release(rig.on_ring(10.0));
    assert_eq!(rig.event, None);

    crate::direction::apply(&rig.ctx, crate::direction::Direction::Rtl);
    rig.frame(vec![]);
    rig.click(rig.geometry.center + vec2(rig.geometry.outer - 3.0, 0.0));
    assert_eq!(rig.event, set(hsb(0.0, 50.0, 60.0)));
}

#[test]
fn a_drag_is_one_gesture_and_esc_cancels_it() {
    let mut rig = Rig::new(hsb(240.0, 50.0, 60.0));
    rig.press(rig.on_ring(100.0));
    let id = begin(rig.event.clone());
    rig.drag(rig.on_ring(120.0));
    assert_eq!(rig.event, update(id, hsb(120.0, 50.0, 60.0)));
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.event, Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
    rig.drag(rig.on_ring(10.0));
    rig.release(rig.on_ring(10.0));
    assert_eq!(rig.event, None, "the drag Esc ended goes no further");
}

#[test]
fn ring_keys_wrap_and_triangle_keys_are_sets() {
    let mut rig = Rig::new(hsb(359.0, 50.0, 60.0));
    rig.focus(rig.ids.0);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(0.0, 50.0, 60.0)), "359 wraps to 0");
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(359.0, 50.0, 60.0)));
    rig.key(Key::PageUp, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(35.0, 50.0, 60.0)));
    rig.key(Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(rig.event, Some(EditEvent::Set { value: hsb(45.0, 50.0, 60.0), modifiers: Modifiers::SHIFT }));
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(359.0, 50.0, 60.0)));
    rig.key(Key::Home, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(0.0, 50.0, 60.0)));

    rig.focus(rig.ids.1);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(0.0, 51.0, 60.0)));
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(0.0, 51.0, 59.0)));
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.event, set(hsb(0.0, 100.0, 59.0)));

    rig.press(rig.in_triangle(0.0, 50.0, 50.0));
    let id = begin(rig.event.clone());
    rig.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.event, update(id, hsb(0.0, 50.0, 50.0)), "keys change nothing during a drag");
}

#[test]
fn accesskit_names_the_group_and_the_two_parts() {
    let rig = Rig::new(hsb(120.0, 40.0, 70.0));
    let ring = rig.node(rig.ids.0);
    assert_eq!((ring.role(), ring.label()), (Role::Slider, Some("Hue")));
    assert_eq!(
        (ring.numeric_value(), ring.min_numeric_value(), ring.max_numeric_value(), ring.numeric_value_step()),
        (Some(120.0), Some(0.0), Some(360.0), Some(1.0))
    );
    assert!(ring.supports_action(Action::Increment) && ring.supports_action(Action::Decrement) && ring.supports_action(Action::SetValue));
    let triangle = rig.node(rig.ids.1);
    assert_eq!((triangle.role(), triangle.label()), (Role::Slider, Some("Saturation and brightness")));
    assert_eq!(triangle.value(), Some("Saturation 40 %, Brightness 70 %"));
    assert_eq!(triangle.numeric_value(), None);
    assert!(triangle.supports_action(Action::Focus) && triangle.supports_action(Action::Click));
    let group = rig.nodes.iter().find(|(_, n)| n.label() == Some("Color wheel")).map(|(_, n)| n);
    assert_eq!(group.map(|n| n.role()), Some(Role::Group));
}

#[test]
fn the_ring_then_the_triangle_are_two_tab_stops_and_a_field_scope_skips_them() {
    let mut rig = Rig::new(hsb(10.0, 50.0, 50.0));
    rig.focus(rig.fields.0);
    rig.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.focused(), Some(rig.fields.1), "Tab goes from field to field");
    rig.focus(rig.fields.0);
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.focused(), Some(rig.ids.0), "Ctrl+Tab reaches the ring");
    rig.key(Key::Tab, Modifiers::CTRL);
    assert_eq!(rig.focused(), Some(rig.ids.1), "and then the triangle");
}

/// A wheel between number fields: with `press_waits` a press during a field's typing run lets the
/// run commit first and edits in the next pass; without it the press edits at once.
fn press_during_typing(waits: bool) -> (Option<EditEvent<ModelValues>>, Option<EditEvent<ModelValues>>) {
    use crate::controls::NumberField;
    use crate::localized::LocaleNumbers;
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let (mut time, mut geometry, mut event, mut field) = (0.0, WheelGeometry { center: Pos2::ZERO, inner: 0.0, outer: 0.0, triangle: 0.0 }, None, Id::NULL);
    let mut frame = |events: Vec<Event>, event: &mut Option<EditEvent<ModelValues>>, geometry: &mut WheelGeometry, field: &mut Id| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let output = ctx.run_ui(input, |ui| {
            *field = NumberField::new("hue", "Hue", 10.0, &numbers).part(true).show(ui).response.id;
            let wheel = ColorWheel::new("wheel", "Color wheel", hsb(240.0, 50.0, 60.0)).press_waits(waits).show(ui);
            *event = wheel.edit.event;
            *geometry = wheel.geometry;
        });
        output.drop_without_applying_deltas();
    };
    frame(vec![], &mut event, &mut geometry, &mut field);
    ctx.memory_mut(|m| m.request_focus(field));
    frame(vec![], &mut event, &mut geometry, &mut field);
    frame(vec![Event::Text("12".to_owned())], &mut event, &mut geometry, &mut field);
    frame(vec![], &mut event, &mut geometry, &mut field);
    let at = geometry.ring_knob(90.0);
    frame(vec![Event::PointerMoved(at), button(at, true), button(at, false)], &mut event, &mut geometry, &mut field);
    let pressed = event.clone();
    frame(vec![], &mut event, &mut geometry, &mut field);
    (pressed, event)
}

#[test]
fn a_press_waits_for_a_typing_run_only_when_the_host_says_so() {
    let (pressed, _) = press_during_typing(false);
    assert!(pressed.is_some(), "standalone: the press edits at once");
    let (pressed, next) = press_during_typing(true);
    assert_eq!(pressed, None, "in a panel: the press waits for the run's commit");
    assert!(next.is_some(), "and edits in the next pass: {next:?}");
}

#[test]
fn a_drag_keeps_the_part_it_pressed_in_a_moved_layer() {
    let mut rig = Rig::new(hsb(240.0, 50.0, 60.0));
    rig.offset = vec2(0.0, 120.0);
    rig.frame(vec![]);
    rig.frame(vec![]);
    let screen = |rig: &Rig, p: Pos2| p + rig.offset;
    let at = screen(&rig, rig.on_ring(90.0));
    rig.press(at);
    let id = begin(rig.event.clone());
    rig.frame(vec![]);
    let to = screen(&rig, rig.on_ring(150.0));
    rig.drag(to);
    assert_eq!(rig.event, update(id, hsb(150.0, 50.0, 60.0)), "the drag turns the hue it pressed on");
}

/// A host's widget under the wheel's row, beside the wheel, takes the click.
#[test]
fn a_click_beside_the_wheel_reaches_the_widget_under_it() {
    use egui::{Sense, Ui};
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let (mut time, mut geometry, mut clicked) = (0.0, WheelGeometry { center: Pos2::ZERO, inner: 0.0, outer: 0.0, triangle: 0.0 }, false);
    let grab = default_style().grab_outer;
    let mut frame = |events: Vec<Event>, geometry: &mut WheelGeometry, clicked: &mut bool| {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let (g, beside) = (*geometry, Rect::from_center_size(geometry.center + vec2(geometry.outer + grab + 20.0, 0.0), vec2(10.0, 10.0)));
        let output = ctx.run_ui(input, |ui: &mut Ui| {
            let under = ui.interact_opt(beside, ui.scope_id().with("under"), Sense::click(), Default::default());
            *clicked |= g.outer > 0.0 && under.clicked();
            let wheel = ColorWheel::new("wheel", "Color wheel", hsb(240.0, 50.0, 60.0)).show(ui);
            *geometry = wheel.geometry;
        });
        output.drop_without_applying_deltas();
    };
    frame(vec![], &mut geometry, &mut clicked);
    frame(vec![], &mut geometry, &mut clicked);
    let at = geometry.center + vec2(geometry.outer + grab + 20.0, 0.0);
    frame(vec![Event::PointerMoved(at)], &mut geometry, &mut clicked);
    frame(vec![button(at, true)], &mut geometry, &mut clicked);
    frame(vec![button(at, false)], &mut geometry, &mut clicked);
    assert!(clicked, "the wheel does not sense presses beside itself");
}
