//! Tab strip tests: one Tab stop and the focus keys, clicks on tabs, their × and the empty strip,
//! context-menu requests, the overflow menu by pointer and keyboard, AccessKit, the wheel, a
//! locked strip's drags, and where the insertion marker goes.

use egui::accesskit::{Action, ActionRequest, HasPopup, Orientation, Role};
use egui::{Color32, Context, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, TouchPhase, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::strip::marker_at;
use super::*;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

/// A host with its tabs and the shown one, applying `Activate`, and what its strip reported.
struct Host {
    kind: TabKind,
    tabs: Vec<Tab<u32>>,
    selected: u32,
    width: f32,
    reorderable: bool,
    wheel: bool,
    events: Vec<TabEvent<u32>>,
    shown: Vec<(u32, Rect)>,
    strip: Rect,
    background_dragged: bool,
}

impl Host {
    fn new(count: u32, selected: u32) -> Self {
        Host {
            kind: TabKind::Document,
            tabs: (0..count).map(|i| Tab::new(i, format!("Doc {i}")).closable(true)).collect(),
            selected,
            width: 440.0,
            reorderable: true,
            wheel: false,
            events: Vec::new(),
            shown: Vec::new(),
            strip: Rect::NOTHING,
            background_dragged: false,
        }
    }

    fn take(&mut self) -> Vec<TabEvent<u32>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let _ = ui.button("Before");
    let out = ui
        .allocate_ui(vec2(host.width, 0.0), |ui| {
            TabStrip::new("strip", "Documents", host.kind)
                .selected(&host.selected)
                .reorderable(host.reorderable)
                .wheel_selects(host.wheel)
                .show(ui, &host.tabs)
        })
        .inner;
    let _ = ui.button("After");
    for event in &out.events {
        if let TabEvent::Activate(key) = event {
            host.selected = *key;
        }
    }
    host.events.extend(out.events);
    host.shown = out.tabs;
    host.strip = out.response.rect;
    host.background_dragged |= out.background.dragged();
}

fn harness_in(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([480.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn harness(host: Host) -> Harness<'static, Host> {
    harness_in(host, Direction::Ltr)
}

fn strip<'h>(harness: &'h Harness<'_, Host>) -> Node<'h> {
    harness.get_by_role(Role::TabList)
}

fn tab<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let label = label.to_owned();
    harness.get_by(move |n| n.role() == Role::Tab && n.label().as_deref() == Some(label.as_str()))
}

fn button<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let label = label.to_owned();
    harness.get_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(label.as_str()))
}

/// The strip's active descendant while it has the focus.
fn active(harness: &Harness<'_, Host>) -> Option<String> {
    strip(harness).accesskit_node().active_descendant().and_then(|n| n.label())
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Tab past "Before" into the strip.
fn focus(harness: &mut Harness<'_, Host>) {
    key(harness, Modifiers::NONE, Key::Tab);
    key(harness, Modifiers::NONE, Key::Tab);
    assert!(active(harness).is_some(), "Tab reaches the strip");
}

fn click_at(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// Lets the double-click delay pass, so the next click is a single click.
fn pause(harness: &mut Harness<'_, Host>) {
    harness.run_steps(30);
}

fn menu_items(harness: &Harness<'_, Host>) -> Vec<(String, Option<bool>)> {
    harness
        .query_all_by(|n| matches!(n.role(), Role::MenuItem | Role::MenuItemCheckBox))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.accesskit_node().toggled().map(|t| t == egui::accesskit::Toggled::True)))
        .collect()
}

fn focus_colour() -> Color32 {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut colour = Color32::TRANSPARENT;
    ctx.run_ui(egui::RawInput::default(), |ui| colour = Tokens::of(ui).color.line.focus.to_egui()).drop_without_applying_deltas();
    colour
}

#[test]
fn the_strip_is_one_tab_stop_and_the_arrows_move_the_focus_without_switching() {
    let mut harness = harness(Host::new(4, 2));
    let stops: Vec<String> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
    assert_eq!(stops, ["Before", "Doc 2", "After"], "the strip is one stop, landing on the active tab");

    key(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(active(&harness).as_deref(), Some("Doc 2"));
    a11y::assert_focus_ring(&harness, focus_colour());
    let mut seen = Vec::new();
    for k in [Key::ArrowRight, Key::ArrowRight, Key::Home, Key::End, Key::ArrowLeft, Key::ArrowLeft] {
        key(&mut harness, Modifiers::NONE, k);
        seen.push(active(&harness).unwrap_or_default());
    }
    assert_eq!(seen, ["Doc 3", "Doc 3", "Doc 0", "Doc 3", "Doc 2", "Doc 1"], "no wrap; Home and End go to the ends");
    assert!(harness.state_mut().take().is_empty(), "focus moves don't activate");

    key(&mut harness, Modifiers::NONE, Key::Enter);
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(1), TabEvent::Activate(0)]);
    assert_eq!(active(&harness).as_deref(), Some("Doc 0"), "the strip keeps the focus");
}

#[test]
fn the_arrows_swap_right_to_left() {
    let mut harness = harness_in(Host::new(4, 1), Direction::Rtl);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(active(&harness).as_deref(), Some("Doc 2"), "← moves on in reading order");
    assert!(tab(&harness, "Doc 2").rect().center().x < tab(&harness, "Doc 1").rect().center().x, "the strip runs from the right");
}

#[test]
fn clicks_activate_the_close_closes_and_double_clicks_report_the_tab_or_the_strip() {
    let mut harness = harness(Host::new(4, 0));
    let at = tab(&harness, "Doc 1").rect().center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(1)]);
    pause(&mut harness);

    let at = button(&harness, "Close Doc 3").rect().center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state_mut().take(), [TabEvent::Close(3)], "a click on the × doesn't activate");
    pause(&mut harness);

    let at = tab(&harness, "Doc 0").rect().center();
    click_at(&mut harness, at, PointerButton::Middle);
    let at = button(&harness, "Close Doc 2").rect().center();
    click_at(&mut harness, at, PointerButton::Middle);
    assert_eq!(harness.state_mut().take(), [TabEvent::Close(0), TabEvent::Close(2)], "a middle click closes a document, on the × too");
    pause(&mut harness);

    let at = tab(&harness, "Doc 1").rect().center();
    click_at(&mut harness, at, PointerButton::Primary);
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(1), TabEvent::DoubleClick(Some(1))]);
    pause(&mut harness);

    let strip = harness.state().strip;
    let empty = pos2(strip.right() - 10.0, strip.center().y);
    click_at(&mut harness, empty, PointerButton::Primary);
    click_at(&mut harness, empty, PointerButton::Primary);
    assert_eq!(harness.state_mut().take(), [TabEvent::DoubleClick(None)]);
    pause(&mut harness);

    harness.state_mut().kind = TabKind::Panel;
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "Close Doc 1").is_none(), "panel tabs have no ×");
    let at = tab(&harness, "Doc 1").rect().center();
    click_at(&mut harness, at, PointerButton::Middle);
    assert!(harness.state_mut().take().is_empty(), "a middle click doesn't close a panel");
}

#[test]
fn context_menus_are_asked_for_with_the_tab_and_its_anchor() {
    let mut harness = harness(Host::new(4, 2));
    let doc_1 = tab(&harness, "Doc 1").rect();
    let pointer = doc_1.center();
    click_at(&mut harness, pointer, PointerButton::Secondary);
    assert_eq!(harness.state_mut().take(), [TabEvent::ContextMenu(TabMenuRequest { tab: Some(1), anchor: doc_1, pointer: Some(pointer) })]);

    let strip = harness.state().strip;
    let empty = pos2(strip.right() - 10.0, strip.center().y);
    click_at(&mut harness, empty, PointerButton::Secondary);
    assert_eq!(harness.state_mut().take(), [TabEvent::ContextMenu(TabMenuRequest { tab: None, anchor: strip, pointer: Some(empty) })]);

    focus(&mut harness);
    let doc_2 = tab(&harness, "Doc 2").rect();
    key(&mut harness, Modifiers::SHIFT, Key::F10);
    key(&mut harness, Modifiers::NONE, Key::Menu);
    let request = TabMenuRequest { tab: Some(2), anchor: doc_2, pointer: None };
    assert_eq!(harness.state_mut().take(), [TabEvent::ContextMenu(request.clone()), TabEvent::ContextMenu(request.clone())]);

    let (target_node, target_tree) = tab(&harness, "Doc 2").accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest { action: Action::ShowContextMenu, target_node, target_tree, data: None }));
    harness.run();
    assert_eq!(harness.state_mut().take(), [TabEvent::ContextMenu(request)], "a screen reader's Show Context Menu");
}

#[test]
fn the_overflow_menu_lists_every_document_or_the_hidden_panels_and_activates_the_row() {
    let mut host = Host::new(12, 0);
    host.width = 300.0;
    let mut harness = harness(host);
    let more = button(&harness, "More tabs");
    assert_eq!((more.accesskit_node().has_popup(), more.accesskit_node().data().is_expanded()), (Some(HasPopup::Menu), Some(false)));
    more.click();
    harness.run();
    let items = menu_items(&harness);
    assert_eq!(items.len(), 12, "every document: {items:?}");
    assert_eq!(items[0], ("Doc 0".to_owned(), Some(true)), "the active one checked");
    assert_eq!(items[9], ("Doc 9".to_owned(), Some(false)));
    assert_eq!(button(&harness, "More tabs").accesskit_node().data().is_expanded(), Some(true));
    harness.get_by(|n| n.role() == Role::MenuItemCheckBox && n.label().as_deref() == Some("Doc 9")).click();
    harness.run();
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(9)]);
    harness.run();
    assert!(harness.state().shown.iter().any(|(key, _)| *key == 9), "the activated document is shown");

    harness.state_mut().kind = TabKind::Panel;
    harness.run();
    let shown: Vec<u32> = harness.state().shown.iter().map(|(key, _)| *key).collect();
    button(&harness, "More tabs").click();
    harness.run();
    let hidden: Vec<String> = (0..12).filter(|key| !shown.contains(key)).map(|key| format!("Doc {key}")).collect();
    let labels: Vec<String> = menu_items(&harness).into_iter().map(|(label, _)| label).collect();
    assert_eq!(labels, hidden, "the hidden panels only");
}

#[test]
fn the_keyboard_reaches_the_overflow_button_opens_its_menu_and_escape_returns() {
    let mut host = Host::new(12, 0);
    host.width = 300.0;
    let mut harness = harness(host);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::End);
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(active(&harness).as_deref(), Some("More tabs"), "the button is the strip's last focus position");
    a11y::assert_focus_ring(&harness, focus_colour());
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(menu_items(&harness).len(), 12, "Enter opens the menu");
    assert!(harness.state_mut().take().is_empty());
    key(&mut harness, Modifiers::NONE, Key::Escape);
    harness.run();
    assert!(menu_items(&harness).is_empty(), "Escape closes it");
    assert_eq!(active(&harness).as_deref(), Some("More tabs"), "and the focus comes back to the button");
    key(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert!(active(&harness).is_some_and(|label| label.starts_with("Doc")));
}

#[test]
fn accesskit_names_the_tab_list_and_its_tabs_and_takes_their_actions() {
    let mut host = Host::new(12, 0);
    host.width = 300.0;
    host.tabs[1] = Tab::new(1, "Doc 1").detail("RGB/8").modified(true).closable(true);
    let mut harness = harness(host);
    let list = strip(&harness);
    let node = list.accesskit_node();
    assert_eq!((node.label().as_deref(), node.orientation()), (Some("Documents"), Some(Orientation::Horizontal)));
    assert_eq!(node.active_descendant().map(|n| n.id()), None, "no active descendant without the focus");
    let modified = tab(&harness, "Doc 1, RGB/8, modified");
    let node = modified.accesskit_node();
    assert_eq!((node.is_selected(), node.position_in_set(), node.size_of_set()), (Some(false), Some(1), Some(12)), "the set counts hidden tabs");
    assert!(tab(&harness, "Doc 0").accesskit_node().is_selected() == Some(true));
    let close = button(&harness, "Close Doc 1");
    let tab_id = modified.accesskit_node().id();
    assert!(std::iter::successors(close.accesskit_node().parent(), |n| n.parent()).any(|n| n.id() == tab_id), "the × is inside its tab");
    let shown = harness.state().shown.len();
    assert_eq!(harness.query_all_by_role(Role::Tab).count(), shown, "hidden tabs have no node");

    modified.click_accesskit();
    harness.run();
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(1)], "Click activates");
    tab(&harness, "Doc 0").focus();
    harness.run();
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("Doc 0"), "Focus gives the strip the focus on that tab");
    assert!(harness.state_mut().take().is_empty(), "without activating it");
    a11y::assert_labelled(&harness);
}

fn wheel(harness: &mut Harness<'_, Host>, points: f32, modifiers: Modifiers) {
    let at = tab(harness, "Doc 0").rect().center();
    harness.event(Event::PointerMoved(at));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, points),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers,
    });
    // egui's smooth scrolling settles over a few frames.
    harness.run_steps(10);
}

#[test]
fn the_wheel_activates_the_next_or_previous_tab_and_stops_at_the_ends() {
    let mut host = Host::new(4, 1);
    host.wheel = true;
    let mut harness = harness(host);
    wheel(&mut harness, -40.0, Modifiers::NONE);
    wheel(&mut harness, -20.0, Modifiers::NONE);
    wheel(&mut harness, -20.0, Modifiers::NONE);
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(2), TabEvent::Activate(3)], "a notch or 40 points each");
    wheel(&mut harness, -40.0, Modifiers::NONE);
    wheel(&mut harness, 40.0, Modifiers::COMMAND);
    assert!(harness.state_mut().take().is_empty(), "stops at the end; nothing with Cmd or Ctrl");
    wheel(&mut harness, 40.0, Modifiers::NONE);
    assert_eq!(harness.state_mut().take(), [TabEvent::Activate(2)]);
}

fn drag(harness: &mut Harness<'_, Host>, from: Pos2, by: f32) {
    harness.event(Event::PointerMoved(from));
    harness.event(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    for step in 1..=4 {
        harness.event(Event::PointerMoved(from + vec2(by * step as f32 / 4.0, 0.0)));
        harness.run();
    }
    let to = from + vec2(by, 0.0);
    harness.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
}

#[test]
fn a_locked_strip_gives_a_drag_on_a_tab_to_the_background() {
    let mut harness = harness(Host::new(3, 0));
    let at = tab(&harness, "Doc 1").rect().center();
    drag(&mut harness, at, 30.0);
    assert!(!harness.state().background_dragged, "a reorderable tab keeps its drag");
    harness.state_mut().reorderable = false;
    harness.run();
    let at = tab(&harness, "Doc 1").rect().center();
    drag(&mut harness, at, 30.0);
    assert!(harness.state().background_dragged, "a locked tab's drag moves the group");
    assert!(harness.state_mut().take().is_empty(), "and activates nothing");
}

#[test]
fn the_insertion_marker_sits_inside_the_tab_at_the_slots_edge() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 26.0));
    let input = |rtl| StripPlanInput {
        kind: TabKind::Document,
        rect,
        rtl,
        leading: 0.0,
        trailing: 0.0,
        natural: &[80.0, 80.0, 80.0, 80.0, 80.0],
        selected: Some(0),
        min_width: 40.0,
        overflow_width: 20.0,
    };
    let plan = StripPlan::new(&input(false));
    assert_eq!(plan.shown.len(), 3);
    let at = |index: usize| plan.shown[index].rect;
    assert_eq!(marker_at(&plan, false, 1), Some((at(1), true)), "before tab 1: its left edge");
    assert_eq!(marker_at(&plan, false, 3), Some((at(2), false)), "after the last shown: its right edge");
    assert_eq!(marker_at(&plan, false, 5), Some((at(2), false)), "a hidden slot goes after the last shown before it");
    let rtl = StripPlan::new(&input(true));
    assert_eq!(marker_at(&rtl, true, 1), Some((rtl.shown[1].rect, false)), "before tab 1 right to left: its right edge");
    assert_eq!(marker_at(&rtl, true, 3), Some((rtl.shown[2].rect, true)));
}
