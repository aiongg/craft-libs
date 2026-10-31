//! The colour ramp in a headless egui context: the colour function, the pointer, the keyboard
//! cursor, the boxes, the cache and AccessKit.

use std::cell::Cell;

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{ColorRamp, RampKind, RampPick};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn near(a: Srgba, b: Srgba) -> bool {
    [(a.r, b.r), (a.g, b.g), (a.b, b.b), (a.a, b.a)].iter().all(|(x, y)| (x - y).abs() < 1e-3)
}

struct Rig {
    ctx: Context,
    time: f64,
    kind: RampKind,
    ends: [Srgba; 2],
    web_safe: bool,
    revision: u64,
    gamut_calls: Cell<usize>,
    events: Vec<EditEvent<RampPick>>,
    cycles: usize,
    menu_at: Option<Pos2>,
    bar: Rect,
    bar_id: egui::Id,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(kind: RampKind) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            kind,
            ends: [RED, BLUE],
            web_safe: false,
            revision: 0,
            gamut_calls: Cell::new(0),
            events: Vec::new(),
            cycles: 0,
            menu_at: None,
            bar: Rect::NOTHING,
            bar_id: egui::Id::NULL,
            nodes: Vec::new(),
        };
        rig.settle();
        rig
    }

    fn frame_with(&mut self, modifiers: Modifiers, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
            time: Some(self.time),
            events: std::iter::once(Event::ModifiersChanged(modifiers)).chain(events).collect(),
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let mut shown = None;
        let gamut = |c: Srgba| {
            self.gamut_calls.set(self.gamut_calls.get() + 1);
            c
        };
        let output = ctx.run_ui(input, |ui| {
            let ramp = ColorRamp::new("ramp", "Color ramp", self.kind)
                .ends(self.ends[0], self.ends[1])
                .web_safe(self.web_safe)
                .gamut(&gamut)
                .gamut_revision(self.revision)
                .kind_name("RGB Spectrum")
                .show(ui);
            shown = Some(ramp);
        });
        let Some(shown) = shown else { return };
        self.events.extend(shown.edit.event);
        self.cycles += usize::from(shown.cycle);
        self.menu_at = shown.menu_at.or(self.menu_at);
        self.bar = shown.bar.rect;
        self.bar_id = shown.bar.id;
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_with(Modifiers::NONE, events);
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
    }

    /// The point at fractions (`fx`, `fy`) of the bar.
    fn at(&self, fx: f32, fy: f32) -> Pos2 {
        pos2(self.bar.left() + self.bar.width() * fx, self.bar.top() + self.bar.height() * fy)
    }

    /// Takes the events so far.
    fn take(&mut self) -> Vec<EditEvent<RampPick>> {
        std::mem::take(&mut self.events)
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame_with(modifiers, vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![]);
    }

    fn focus_bar(&mut self) {
        let at = self.at(0.5, 0.5);
        self.time += 1.0;
        self.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)]);
        self.frame(vec![button(at, false, Modifiers::NONE)]);
        self.settle();
        self.take();
    }

    fn pick(&self, fx: f32, fy: f32) -> Srgba {
        ColorRamp::color_at(self.kind, self.web_safe, None, self.ends, fx, fy)
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.role() == role && n.label() == Some(name))
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }

    fn bounds(&self, role: Role, name: &str) -> Rect {
        let b = self.node(role, name).bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn value_of(event: &EditEvent<RampPick>) -> RampPick {
    *event.value().expect("an event with a value")
}

#[test]
fn the_colour_function_is_photoshops_ramp() {
    let c = |kind, x, y| ColorRamp::color_at(kind, false, None, [RED, BLUE], x, y);
    assert!(near(c(RampKind::Spectrum, 0.5, 0.0), Srgba::rgb(1.0, 1.0, 1.0)), "white at the top");
    assert!(near(c(RampKind::Spectrum, 0.0, 0.5), RED), "red in the middle row at the left");
    assert!(near(c(RampKind::Spectrum, 1.0 / 3.0, 0.5), Srgba::rgb(0.0, 1.0, 0.0)), "green a third across");
    assert!(near(c(RampKind::Spectrum, 0.7, 1.0), Srgba::rgb(0.0, 0.0, 0.0)), "black at the bottom");
    assert!(near(c(RampKind::Grayscale, 0.0, 0.3), Srgba::rgb(1.0, 1.0, 1.0)), "grey runs white to black");
    assert!(near(c(RampKind::Grayscale, 1.0, 0.3), Srgba::rgb(0.0, 0.0, 0.0)));
    assert!(near(c(RampKind::Between, 0.0, 0.5), RED) && near(c(RampKind::Between, 1.0, 0.5), BLUE), "between: the ends");
    assert!(near(c(RampKind::Between, 0.5, 0.5), Srgba::rgb(0.5, 0.0, 0.5)));

    for x in 0..=20 {
        let web = ColorRamp::color_at(RampKind::Spectrum, true, None, [RED, BLUE], x as f32 / 20.0, 0.37);
        assert!([web.r, web.g, web.b].iter().all(|v| ((v * 5.0).round() - v * 5.0).abs() < 1e-4), "{web:?} is a web colour");
    }
    let wild = ColorRamp::color_at(RampKind::Spectrum, false, None, [RED, BLUE], f32::NAN, f32::INFINITY);
    assert!(near(wild, Srgba::rgb(1.0, 1.0, 1.0)), "non-finite input is clamped into the bar: {wild:?}");
}

#[test]
fn a_press_picks_in_the_press_frame_and_a_drag_is_one_gesture() {
    let mut rig = Rig::new(RampKind::Spectrum);
    let at = rig.at(0.25, 0.5);
    rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)]);
    let started = rig.take();
    assert!(matches!(started.first(), Some(EditEvent::Begin { .. })), "{started:?}");
    let press = started.iter().filter_map(EditEvent::value).next().map(|p| p.color);
    assert!(press.is_some_and(|c| near(c, rig.pick(0.25, 0.5))), "the press picks the colour under the pointer: {press:?}");

    // The pointer leaves the bar: the pick stays at its edge.
    let away = pos2(rig.bar.right() + 50.0, rig.bar.center().y);
    rig.frame(vec![Event::PointerMoved(away)]);
    rig.frame(vec![button(away, false, Modifiers::NONE)]);
    let events = rig.take();
    let Some(EditEvent::Commit { value, .. }) = events.last() else { panic!("{events:?}") };
    assert!(near(value.color, rig.pick(1.0, 0.5)), "clamped at the right edge: {value:?}");
    assert!(!events.iter().any(|e| matches!(e, EditEvent::Begin { .. })), "one gesture");
}

#[test]
fn a_press_and_release_in_one_frame_is_a_set_and_alt_marks_it() {
    let mut rig = Rig::new(RampKind::Spectrum);
    rig.time += 1.0;
    let at = rig.at(0.5, 0.25);
    rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    let events = rig.take();
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(near(value.color, rig.pick(0.5, 0.25)) && !value.other);

    rig.time += 1.0;
    let alt = Modifiers::ALT;
    rig.frame_with(alt, vec![Event::PointerMoved(at), button(at, true, alt), button(at, false, alt)]);
    let events = rig.take();
    assert!(events.iter().filter_map(EditEvent::value).all(|p| p.other) && !events.is_empty(), "{events:?}");
}

#[test]
fn shift_click_cycles_and_picks_nothing() {
    let mut rig = Rig::new(RampKind::Spectrum);
    rig.time += 1.0;
    let at = rig.at(0.4, 0.5);
    let shift = Modifiers::SHIFT;
    rig.frame_with(shift, vec![Event::PointerMoved(at), button(at, true, shift)]);
    rig.frame_with(shift, vec![button(at, false, shift)]);
    assert_eq!(rig.cycles, 1);
    assert!(rig.take().is_empty(), "nothing is picked");

    rig.time += 1.0;
    rig.frame_with(shift, vec![Event::PointerMoved(at), button(at, true, shift), button(at, false, shift)]);
    assert_eq!(rig.cycles, 2, "also as one frame");
    assert!(rig.take().is_empty());
}

#[test]
fn a_web_safe_press_picks_the_cell_it_draws() {
    let mut rig = Rig::new(RampKind::Spectrum);
    rig.web_safe = true;
    rig.settle();
    rig.time += 1.0;
    let at = rig.at(0.2, 0.3);
    rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    let events = rig.take();
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    let (cx, cy) = (((0.2_f32 * 48.0).floor() + 0.5) / 48.0, ((0.3_f32 * 8.0).floor() + 0.5) / 8.0);
    assert!(near(value.color, ColorRamp::color_at(RampKind::Spectrum, true, None, rig.ends, cx, cy)));
}

#[test]
fn a_between_ramp_keeps_the_ends_it_had_at_the_press() {
    let mut rig = Rig::new(RampKind::Between);
    let before = rig.ends;
    let at = rig.at(0.5, 0.5);
    rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)]);
    // Every pick changes the host's ends: the ramp must not slide under the pointer.
    rig.ends = [Srgba::rgb(0.0, 1.0, 0.0), Srgba::rgb(0.0, 1.0, 0.0)];
    let to = rig.at(0.75, 0.5);
    rig.frame(vec![Event::PointerMoved(to)]);
    rig.frame(vec![Event::PointerMoved(to)]);
    let last = rig.events.iter().rev().find_map(EditEvent::value).map(|p| p.color);
    assert!(last.is_some_and(|c| near(c, ColorRamp::color_at(RampKind::Between, false, None, before, 0.75, 0.5))), "{last:?}");
}

#[test]
fn the_keyboard_cursor_moves_and_enter_picks_its_cell() {
    let mut rig = Rig::new(RampKind::Spectrum);
    rig.focus_bar();
    let ends = rig.ends;
    let cell =
        |col: usize, row: usize| ColorRamp::color_at(RampKind::Spectrum, false, None, ends, (col as f32 + 0.5) / 48.0, (row as f32 + 0.5) / 8.0);

    // The press put the cursor under the pointer, in the middle of the bar.
    rig.key(Key::Home, Modifiers::NONE);
    rig.key(Key::PageUp, Modifiers::NONE);
    assert!(rig.take().is_empty(), "moving the cursor picks nothing");
    rig.key(Key::Enter, Modifiers::NONE);
    let events = rig.take();
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(near(value.color, cell(0, 0)) && !value.other, "Home and Page Up: the top left cell");

    rig.key(Key::ArrowRight, Modifiers::SHIFT);
    rig.key(Key::ArrowDown, Modifiers::NONE);
    rig.key(Key::Space, Modifiers::NONE);
    assert!(near(value_of(&rig.take()[0]).color, cell(10, 1)), "Shift moves ten cells");

    rig.key(Key::End, Modifiers::NONE);
    rig.key(Key::PageDown, Modifiers::NONE);
    rig.key(Key::Enter, Modifiers::ALT);
    let picked = value_of(&rig.take()[0]);
    assert!(near(picked.color, cell(47, 7)) && picked.other, "End and Page Down; Alt+Enter marks `other`");
}

#[test]
fn the_boxes_set_white_and_black() {
    let mut rig = Rig::new(RampKind::Spectrum);
    for (name, expected) in [("White", Srgba::rgb(1.0, 1.0, 1.0)), ("Black", Srgba::rgb(0.0, 0.0, 0.0))] {
        let at = rig.bounds(Role::Button, name).center();
        rig.time += 1.0;
        rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)]);
        rig.frame(vec![button(at, false, Modifiers::NONE)]);
        let events = rig.take();
        let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
        assert!(near(value.color, expected) && !value.other, "{name}");
    }
}

#[test]
fn the_colours_are_cached_until_something_they_depend_on_changes() {
    let mut rig = Rig::new(RampKind::Spectrum);
    assert_eq!(rig.gamut_calls.get(), 49 * 9, "the first frame maps every vertex once");
    rig.settle();
    assert_eq!(rig.gamut_calls.get(), 49 * 9, "later frames reuse them");
    rig.revision += 1;
    rig.settle();
    assert_eq!(rig.gamut_calls.get(), 2 * 49 * 9, "a new gamut revision recomputes them");
    rig.web_safe = true;
    rig.settle();
    assert_eq!(rig.gamut_calls.get(), 2 * 49 * 9 + 48 * 8, "so does the web flag");
}

#[test]
fn accesskit_names_the_group_with_its_kind_and_the_boxes() {
    let rig = Rig::new(RampKind::Spectrum);
    let group = rig.node(Role::Group, "Color ramp");
    assert_eq!(group.value(), Some("RGB Spectrum"));
    rig.node(Role::Button, "White");
    rig.node(Role::Button, "Black");
}

#[test]
fn a_right_click_on_the_bar_asks_the_host_for_its_menu() {
    let mut rig = Rig::new(RampKind::Spectrum);
    let secondary = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    let at = rig.at(0.3, 0.5);
    rig.time += 1.0;
    rig.frame(vec![Event::PointerMoved(at), secondary(at, true)]);
    rig.frame(vec![secondary(at, false)]);
    assert_eq!(rig.menu_at, Some(at));
    assert!(rig.take().is_empty(), "a right-click picks nothing");

    // One frame, and a press that began outside the bar, which is no click on it.
    rig.menu_at = None;
    rig.time += 1.0;
    rig.frame(vec![Event::PointerMoved(at), secondary(at, true), secondary(at, false)]);
    assert_eq!(rig.menu_at, Some(at), "press and release in one frame");
    rig.menu_at = None;
    let off = pos2(rig.bar.left() - 5.0, rig.bar.center().y);
    rig.time += 1.0;
    rig.frame(vec![Event::PointerMoved(off), secondary(off, true)]);
    rig.frame(vec![Event::PointerMoved(at), secondary(at, false)]);
    assert_eq!(rig.menu_at, None);
}

#[test]
fn a_between_ramp_draws_the_ends_it_had_at_the_press_while_dragged() {
    let mut rig = Rig::new(RampKind::Between);
    let left = |rig: &Rig| {
        let key = rig.bar_id.with("craft_ui::ramp_cache");
        let cached = rig.ctx.data(|d| d.get_temp::<(u64, std::sync::Arc<Vec<egui::Color32>>)>(key));
        cached.and_then(|(_, colors)| colors.first().copied())
    };
    assert_eq!(left(&rig), Some(egui::Color32::from(RED)));
    let at = rig.at(0.5, 0.5);
    rig.frame(vec![Event::PointerMoved(at), button(at, true, Modifiers::NONE)]);
    rig.ends = [Srgba::rgb(0.0, 1.0, 0.0), Srgba::rgb(0.0, 1.0, 0.0)];
    let to = rig.at(0.75, 0.5);
    rig.frame(vec![Event::PointerMoved(to)]);
    rig.frame(vec![Event::PointerMoved(to)]);
    assert_eq!(left(&rig), Some(egui::Color32::from(RED)), "the bar keeps the press's colours under the pointer");
    rig.frame(vec![button(to, false, Modifiers::NONE)]);
    rig.settle();
    assert_eq!(left(&rig), Some(egui::Color32::from(Srgba::rgb(0.0, 1.0, 0.0))), "after the release it follows the host");
}

/// A ramp beside a number field, with `press_waits`: Alt is held at the press during the field's
/// typing run and released before the run's commit lets the press run.
#[test]
fn a_waiting_press_keeps_the_modifiers_it_arrived_with() {
    use craft_ui::controls::NumberField;
    use craft_ui::localized::LocaleNumbers;
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let mut time = 0.0;
    let mut picks = Vec::new();
    let mut bar = Rect::NOTHING;
    let mut field = egui::Id::NULL;
    let mut frame = |events: Vec<Event>, picks: &mut Vec<RampPick>, bar: &mut Rect, field: &mut egui::Id| {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let output = ctx.run_ui(input, |ui| {
            *field = NumberField::new("hue", "Hue", 10.0, &numbers).part(true).show(ui).response.id;
            let ramp = ColorRamp::new("ramp", "Color ramp", RampKind::Spectrum).press_waits(true).show(ui);
            picks.extend(ramp.edit.event.as_ref().and_then(|e| e.value()));
            *bar = ramp.bar.rect;
        });
        output.drop_without_applying_deltas();
    };
    frame(vec![], &mut picks, &mut bar, &mut field);
    ctx.memory_mut(|m| m.request_focus(field));
    frame(vec![], &mut picks, &mut bar, &mut field);
    frame(vec![Event::Text("12".to_owned())], &mut picks, &mut bar, &mut field);
    frame(vec![], &mut picks, &mut bar, &mut field);
    let at = bar.center();
    let alt = Modifiers::ALT;
    frame(
        vec![Event::ModifiersChanged(alt), Event::PointerMoved(at), button(at, true, alt), button(at, false, alt)],
        &mut picks,
        &mut bar,
        &mut field,
    );
    assert!(picks.is_empty(), "the press waits");
    frame(vec![Event::ModifiersChanged(Modifiers::NONE)], &mut picks, &mut bar, &mut field);
    assert!(!picks.is_empty() && picks.iter().all(|p| p.other), "the pick is Alt's: {picks:?}");
}

#[cfg(feature = "serde")]
#[test]
fn a_ramp_kind_reads_and_writes_its_name() {
    for (kind, name) in [
        (RampKind::Spectrum, "spectrum"),
        (RampKind::CmykSpectrum, "cmykSpectrum"),
        (RampKind::Grayscale, "grayscale"),
        (RampKind::Between, "between"),
    ] {
        assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<RampKind>(&format!("\"{name}\"")).unwrap(), kind);
    }
}

#[test]
fn the_cmyk_spectrum_goes_through_the_gamut_and_is_otherwise_the_spectrum() {
    let muted = |c: Srgba| Srgba::rgb(c.r * 0.5, c.g * 0.5, c.b * 0.5);
    let plain = ColorRamp::color_at(RampKind::CmykSpectrum, false, None, [RED, BLUE], 0.0, 0.5);
    assert!(near(plain, ColorRamp::color_at(RampKind::Spectrum, false, None, [RED, BLUE], 0.0, 0.5)));
    let mapped = ColorRamp::color_at(RampKind::CmykSpectrum, false, Some(&muted), [RED, BLUE], 0.0, 0.5);
    assert!(near(mapped, Srgba::rgb(0.5, 0.0, 0.0)), "{mapped:?}");
}
