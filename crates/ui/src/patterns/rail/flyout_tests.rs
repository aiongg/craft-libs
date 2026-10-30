//! `FlyoutButton` tests: the pure gesture rules, then the button and its flyout driven by input.

use egui::accesskit::{HasPopup, Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::gesture::{drag_open, long_press, long_press_wait, reclick_closes};
use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::theme::{Base, Tokens, TokensOf as _, install};

#[test]
fn a_press_drags_open_only_from_inside_to_outside_while_down() {
    let button = Rect::from_min_size(pos2(10.0, 10.0), vec2(30.0, 30.0));
    let (inside, outside) = (Some(pos2(20.0, 20.0)), Some(pos2(60.0, 20.0)));
    assert!(drag_open(button, inside, outside, true));
    assert!(!drag_open(button, inside, inside, true), "still on the button");
    assert!(!drag_open(button, outside, outside, true), "the press began elsewhere");
    assert!(!drag_open(button, None, outside, true), "no press");
    assert!(!drag_open(button, inside, None, true), "no pointer");
    assert!(!drag_open(button, inside, outside, false), "the button is up");
}

#[test]
fn a_long_press_is_0_35_seconds() {
    assert!(!long_press(None));
    assert!(!long_press(Some(0.34)));
    assert!(long_press(Some(0.35)));
    assert!(long_press(Some(1.0)));
    let wait = long_press_wait(Some(0.34)).map(|d| d.as_secs_f64());
    assert!(wait.is_some_and(|s| (s - 0.01).abs() < 1e-9), "{wait:?}");
    assert_eq!(long_press_wait(Some(0.35)), None);
    assert_eq!(long_press_wait(None), None);
}

#[test]
fn a_click_on_the_open_flyouts_button_closes_it_but_not_the_release_of_the_opening_press() {
    assert!(reclick_closes(true, false, true, false));
    assert!(reclick_closes(true, true, false, false), "a secondary click closes it too");
    assert!(!reclick_closes(true, false, true, true), "the release that ends the opening press");
    assert!(reclick_closes(true, true, false, true), "a secondary click is never that release");
    assert!(!reclick_closes(false, true, true, false), "nothing is open");
}

type Out = FlyoutResponse<&'static str>;

/// The host side: one button for three tools, and what it reported.
struct App {
    items: Vec<FlyoutItem<&'static str>>,
    current: &'static str,
    out: Option<Out>,
    clicks: usize,
    doubles: usize,
    chosen: Vec<&'static str>,
    time: f64,
    top: f32,
    style: Option<FlyoutStyle>,
}

impl App {
    fn new(count: usize) -> Self {
        let all = vec![
            FlyoutItem::new("brush", Icon::Brush, "Brush").shortcut("B"),
            FlyoutItem::new("stamp", Icon::Stamp, "Stamp").shortcut("N"),
            FlyoutItem::new("eraser", Icon::Eraser, "Eraser"),
        ];
        Self {
            items: all.into_iter().take(count).collect(),
            current: "brush",
            out: None,
            clicks: 0,
            doubles: 0,
            chosen: Vec::new(),
            time: 0.0,
            top: 20.0,
            style: None,
        }
    }

    fn frame(&mut self, ui: &mut Ui) {
        ui.add_space(self.top);
        self.style = Some(FlyoutButton::<&str>::style(&Tokens::of(ui)));
        // A rail sits at the reading start edge, which a plain `Ui` does not mirror.
        let align = if direction::is_rtl(ui) { egui::Align::Max } else { egui::Align::Min };
        let out = ui
            .with_layout(egui::Layout::top_down(align), |ui| {
                FlyoutButton::new("slot", Icon::Brush, "Slot").items(&self.items).current(&self.current).selected(true).show(ui)
            })
            .inner;
        self.clicks += usize::from(out.clicked);
        self.doubles += usize::from(out.double_clicked);
        if let Some(tool) = out.chosen {
            self.chosen.push(tool);
            self.current = tool;
        }
        self.out = Some(out);
        let _ = ui.button("Next");
    }

    #[allow(clippy::unwrap_used)]
    fn out(&self) -> &Out {
        self.out.as_ref().unwrap()
    }
}

fn harness_in(dir: Direction, count: usize) -> Harness<'static, App> {
    let builder = HarnessBuilder::default().with_size(vec2(600.0, 400.0)).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        direction::apply(ctx, dir);
    });
    let mut h = builder.build_ui_state(|ui, app: &mut App| app.frame(ui), App::new(count));
    h.input_mut().focused = true;
    h.run_steps(2);
    h
}

fn harness(count: usize) -> Harness<'static, App> {
    harness_in(Direction::Ltr, count)
}

/// Runs one frame `dt` seconds after the last, with `events`.
fn step(h: &mut Harness<'_, App>, dt: f64, events: Vec<Event>) {
    h.state_mut().time += dt;
    let time = h.state().time;
    h.input_mut().time = Some(time);
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE }
}

fn press(h: &mut Harness<'_, App>, at: Pos2, b: PointerButton) {
    step(h, 0.016, vec![Event::PointerMoved(at), button(at, b, true)]);
}

fn release(h: &mut Harness<'_, App>, at: Pos2, b: PointerButton) {
    step(h, 0.016, vec![button(at, b, false)]);
    step(h, 0.016, vec![]);
}

fn click_at(h: &mut Harness<'_, App>, at: Pos2) {
    press(h, at, PointerButton::Primary);
    release(h, at, PointerButton::Primary);
}

fn center(h: &Harness<'_, App>) -> Pos2 {
    h.state().out().response.rect.center()
}

#[allow(clippy::expect_used)]
fn row(h: &Harness<'_, App>, label: &str) -> Pos2 {
    h.query_by_label(label).expect("a row").rect().center()
}

fn key(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
    step(h, 0.016, vec![event(true)]);
    step(h, 0.016, vec![event(false)]);
}

fn focus(h: &mut Harness<'_, App>) {
    let id = h.state().out().response.id;
    h.ctx.memory_mut(|m| m.request_focus(id));
    step(h, 0.016, vec![]);
}

fn is_open(h: &Harness<'_, App>) -> bool {
    h.state().out().open
}

#[test]
fn a_secondary_click_opens_and_a_click_on_a_row_chooses_and_closes() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    assert!(is_open(&h));
    assert_eq!(h.state().clicks, 0, "a secondary click selects nothing");
    let stamp = row(&h, "Stamp");
    assert!(stamp.x > h.state().out().response.rect.right(), "the list opens towards the canvas");
    click_at(&mut h, stamp);
    assert_eq!(h.state().chosen, ["stamp"]);
    assert!(!is_open(&h));
}

#[test]
fn the_list_mirrors_in_a_right_to_left_interface() {
    let mut h = harness_in(Direction::Rtl, 3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    assert!(is_open(&h));
    let button = h.state().out().response.rect;
    assert!(row(&h, "Stamp").x < button.left(), "{:?} {button:?}", row(&h, "Stamp"));
}

#[test]
fn holding_the_button_for_0_35_seconds_opens_and_selects_and_its_release_keeps_it_open() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Primary);
    step(&mut h, 0.34, vec![]);
    assert!(!is_open(&h), "held 0.34 s");
    step(&mut h, 0.02, vec![]);
    assert!(is_open(&h), "held 0.36 s");
    step(&mut h, 0.01, vec![]);
    assert!(is_open(&h));
    assert_eq!(h.state().clicks, 1, "the shown item is selected once");
    release(&mut h, at, PointerButton::Primary);
    assert!(is_open(&h), "the release that ends the opening press does not close it");
    assert_eq!(h.state().clicks, 1);
    click_at(&mut h, at);
    assert!(!is_open(&h), "a click on the open flyout's button closes it");
}

#[test]
fn pressing_and_dragging_away_opens_and_releasing_on_a_row_chooses_it() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Primary);
    step(&mut h, 0.016, vec![Event::PointerMoved(at + vec2(60.0, 0.0))]);
    assert!(is_open(&h));
    step(&mut h, 0.016, vec![]);
    let eraser = row(&h, "Eraser");
    step(&mut h, 0.016, vec![Event::PointerMoved(eraser)]);
    release(&mut h, eraser, PointerButton::Primary);
    assert_eq!(h.state().chosen, ["eraser"]);
    assert_eq!(h.state().clicks, 0, "a drag selects nothing by itself");
    assert!(!is_open(&h));
}

#[test]
fn a_press_outside_or_escape_closes_and_escape_returns_the_focus() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    assert!(is_open(&h));
    // A drag that starts on the canvas is no click, but its press closes the flyout.
    press(&mut h, pos2(400.0, 300.0), PointerButton::Primary);
    step(&mut h, 0.016, vec![Event::PointerMoved(pos2(470.0, 340.0))]);
    release(&mut h, pos2(470.0, 340.0), PointerButton::Primary);
    assert!(!is_open(&h));
    assert!(h.state().chosen.is_empty());

    focus(&mut h);
    key(&mut h, Key::Menu, Modifiers::NONE);
    assert!(is_open(&h));
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!is_open(&h));
    assert!(h.state().out().response.has_focus(), "focus returns to the button");
}

#[test]
fn the_menu_key_and_shift_f10_open_arrows_and_enter_choose_and_the_focus_returns() {
    for (k, modifiers) in [(Key::Menu, Modifiers::NONE), (Key::F10, Modifiers::SHIFT)] {
        let mut h = harness(3);
        focus(&mut h);
        key(&mut h, k, modifiers);
        assert!(is_open(&h), "{k:?}");
        key(&mut h, Key::ArrowDown, Modifiers::NONE);
        key(&mut h, Key::ArrowDown, Modifiers::NONE);
        key(&mut h, Key::Enter, Modifiers::NONE);
        assert_eq!(h.state().chosen, ["eraser"], "{k:?}");
        assert!(!is_open(&h));
        assert!(h.state().out().response.has_focus());
        assert_eq!(h.state().clicks, 0, "the Enter that chose a row selected nothing else");
    }
}

#[test]
fn tab_closes_the_flyout() {
    let mut h = harness(3);
    focus(&mut h);
    key(&mut h, Key::Menu, Modifiers::NONE);
    assert!(is_open(&h));
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert!(!is_open(&h));
    assert!(h.get_by_label("Next").accesskit_node().is_focused(), "the focus moved on");
}

#[test]
fn a_press_on_the_canvas_dragged_onto_a_row_chooses_nothing() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    let eraser = row(&h, "Eraser");
    press(&mut h, pos2(400.0, 300.0), PointerButton::Primary);
    step(&mut h, 0.016, vec![Event::PointerMoved(eraser)]);
    release(&mut h, eraser, PointerButton::Primary);
    assert!(h.state().chosen.is_empty());
    assert!(!is_open(&h));
}

#[test]
fn only_the_primary_button_long_presses() {
    let mut h = harness(3);
    let at = center(&h);
    press(&mut h, at, PointerButton::Middle);
    step(&mut h, 0.4, vec![]);
    assert!(!is_open(&h));
    assert_eq!(h.state().clicks, 0);
    release(&mut h, at, PointerButton::Middle);
    assert!(!is_open(&h));
}

#[test]
fn enter_on_the_button_selects_the_shown_item_without_opening() {
    let mut h = harness(3);
    focus(&mut h);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().clicks, 1);
    assert!(!is_open(&h));
}

#[test]
fn a_double_click_is_reported() {
    let mut h = harness(3);
    let at = center(&h);
    click_at(&mut h, at);
    assert_eq!(h.state().doubles, 0);
    click_at(&mut h, at);
    assert_eq!(h.state().doubles, 1);
    assert!(!is_open(&h));
}

#[test]
fn a_single_item_button_is_plain() {
    let mut h = harness(1);
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    assert!(!is_open(&h));
    press(&mut h, at, PointerButton::Primary);
    step(&mut h, 0.5, vec![]);
    assert!(!is_open(&h), "no long press");
    release(&mut h, at, PointerButton::Primary);
    assert_eq!(indicators(&h), 0);
    let node = h.get_by_label("Slot");
    assert_eq!(node.accesskit_node().has_popup(), None);

    assert_eq!(indicators(&harness(3)), 1);
}

/// Filled polygons in the indicator's colour.
fn indicators(h: &Harness<'_, App>) -> usize {
    let color = h.state().style.map(|s| s.indicator_color).unwrap_or_default();
    h.output().shapes.iter().filter(|s| matches!(&s.shape, egui::Shape::Path(path) if path.fill == color && path.points.len() == 3)).count()
}

#[test]
fn the_indicator_sits_at_the_corner_facing_the_canvas() {
    for (dir, right) in [(Direction::Ltr, true), (Direction::Rtl, false)] {
        let h = harness_in(dir, 3);
        let rect = h.state().out().response.rect;
        let color = h.state().style.map(|s| s.indicator_color).unwrap_or_default();
        let triangle = h.output().shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Path(path) if path.fill == color && path.points.len() == 3 => Some(path.points.clone()),
            _ => None,
        });
        let points = triangle.unwrap_or_default();
        assert_eq!(points.len(), 3, "{dir:?}");
        let x = points.iter().map(|p| p.x).sum::<f32>() / 3.0;
        assert_eq!(x > rect.center().x, right, "{dir:?}");
        assert!(points.iter().all(|p| p.y > rect.center().y), "bottom corner");
    }
}

#[test]
fn accesskit_names_the_popup_and_the_radio_rows() {
    let mut h = harness(3);
    let button = h.get_by_role_and_label(Role::Button, "Slot");
    assert_eq!(button.accesskit_node().has_popup(), Some(HasPopup::Menu));
    assert_eq!(button.accesskit_node().data().is_expanded(), Some(false));
    let at = center(&h);
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    assert_eq!(h.get_by_role_and_label(Role::Button, "Slot").accesskit_node().data().is_expanded(), Some(true));
    assert!(h.get_by_role_and_label(Role::Menu, "Slot").accesskit_node().data().label().is_some(), "the flyout is a named menu");
    let brush = h.get_by_label("Brush");
    assert_eq!(brush.accesskit_node().role(), Role::MenuItemRadio);
    assert_eq!(brush.accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(h.get_by_label("Stamp").accesskit_node().toggled(), Some(Toggled::False));
}

#[test]
fn no_tooltip_shows_over_an_open_flyout() {
    let mut h = harness(3);
    let at = center(&h);
    step(&mut h, 0.016, vec![Event::PointerMoved(at)]);
    step(&mut h, 1.0, vec![]);
    assert_eq!(h.get_all_by_label("Slot").count(), 2, "the button and its tooltip");
    press(&mut h, at, PointerButton::Secondary);
    release(&mut h, at, PointerButton::Secondary);
    step(&mut h, 1.0, vec![]);
    assert!(is_open(&h));
    assert_eq!(h.get_all_by_role_and_label(Role::Button, "Slot").count(), 1, "no tooltip over an open flyout");
    assert_eq!(h.get_all_by_label("Slot").count(), 2, "the button and the named menu");
}

#[test]
fn no_tooltip_shows_while_the_button_is_held() {
    let mut h = harness(3);
    let at = center(&h);
    step(&mut h, 0.016, vec![Event::PointerMoved(at)]);
    step(&mut h, 0.1, vec![]);
    assert_eq!(h.get_all_by_label("Slot").count(), 2, "a tooltip shows at rest");
    press(&mut h, at, PointerButton::Primary);
    step(&mut h, 0.1, vec![]);
    assert!(!is_open(&h));
    assert_eq!(h.get_all_by_label("Slot").count(), 1, "held, before the long press");
}

#[test]
fn near_the_bottom_the_list_moves_up_and_stays_beside_the_button() {
    for (dir, ltr) in [(Direction::Ltr, true), (Direction::Rtl, false)] {
        let mut h = harness_in(dir, 3);
        h.state_mut().top = 350.0;
        h.run_steps(2);
        let button = h.state().out().response.rect;
        let gap = h.state().style.map_or(0.0, |s| s.gap);
        let at = button.center();
        press(&mut h, at, PointerButton::Secondary);
        release(&mut h, at, PointerButton::Secondary);
        step(&mut h, 0.016, vec![]);
        assert!(is_open(&h), "{dir:?}");
        let id = h.state().out().response.id.with("popup");
        let list = h.ctx.memory(|m| m.area_rect(id)).unwrap_or(Rect::NOTHING);
        assert!(list.top() < button.top(), "{dir:?}: {list:?} {button:?}");
        assert!(list.bottom() <= 400.0 + 0.5, "{dir:?}: inside the screen");
        let edge = if ltr { list.left() - button.right() } else { button.left() - list.right() };
        assert!((edge - gap).abs() < 0.5, "{dir:?}: {edge} from the button, want {gap}");
    }
}
