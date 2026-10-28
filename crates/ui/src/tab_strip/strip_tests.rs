//! Tab strip tests: one Tab stop and the focus keys, clicks on tabs, their × and the empty strip,
//! context-menu requests, the overflow menu by pointer and keyboard, AccessKit, the wheel, a
//! locked strip's drags, where the insertion marker goes, drag reorder with its targets,
//! tear-off and every way a drag ends, and the reorder keys.

use egui::accesskit::{Action, ActionRequest, HasPopup, Orientation, Role};
use egui::{
    Color32, Context, DragAndDrop, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, Sense, TouchPhase, Ui, pos2,
    vec2,
};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::strip::marker_at;
use super::*;
use crate::behavior::{DropPosition, ListDrag, Move};
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
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
    tear_off: bool,
    /// The drop targets a drag showed, each change once.
    targets: Vec<Option<Move<u32>>>,
    dragging_out: Option<DragOut<u32>>,
    /// A drop zone below the strip: the tabs a drag left there.
    zone: Rect,
    dropped: Option<Vec<u32>>,
    /// Skip drawing the strip in a pass.
    hidden: bool,
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
            tear_off: false,
            targets: Vec::new(),
            dragging_out: None,
            zone: Rect::NOTHING,
            dropped: None,
            hidden: false,
        }
    }

    fn take(&mut self) -> Vec<TabEvent<u32>> {
        std::mem::take(&mut self.events)
    }
}

/// Applies a move as a host does: the tab goes next to the target.
fn apply(tabs: &mut Vec<Tab<u32>>, mv: &Move<u32>) {
    let Some(from) = tabs.iter().position(|t| *t.key() == mv.items[0]) else { return };
    let moved = tabs.remove(from);
    let Some(target) = tabs.iter().position(|t| t.key() == &mv.target) else { return };
    tabs.insert(if mv.position == DropPosition::After { target + 1 } else { target }, moved);
}

fn show(ui: &mut Ui, host: &mut Host) {
    if host.hidden {
        return;
    }
    let _ = ui.button("Before");
    let out = ui
        .allocate_ui(vec2(host.width, 0.0), |ui| {
            TabStrip::new("strip", "Documents", host.kind)
                .selected(&host.selected)
                .reorderable(host.reorderable)
                .wheel_selects(host.wheel)
                .tear_off(host.tear_off)
                .show(ui, &host.tabs)
        })
        .inner;
    let _ = ui.button("After");
    let (_, zone) = ui.allocate_exact_size(vec2(200.0, 40.0), Sense::hover());
    host.zone = zone.rect;
    if let Some(drag) = zone.dnd_release_payload::<ListDrag<u32>>() {
        host.dropped = Some(drag.items.clone());
    }
    for event in &out.events {
        match event {
            TabEvent::Activate(key) => host.selected = *key,
            TabEvent::Reorder(EditEvent::Commit { value: mv, .. } | EditEvent::Set { value: mv, .. }) => apply(&mut host.tabs, mv),
            _ => {}
        }
    }
    let dragging = DragAndDrop::payload::<ListDrag<u32>>(ui.ctx()).is_some();
    if (dragging || out.drop_target.is_some()) && host.targets.last() != Some(&out.drop_target) {
        host.targets.push(out.drop_target.clone());
    }
    host.dragging_out = out.dragging_out.clone();
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

fn tear_off_band() -> f32 {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut band = 0.0;
    ctx.run_ui(egui::RawInput::default(), |ui| band = Tab::<u32>::style(&Tokens::of(ui)).tear_off).drop_without_applying_deltas();
    band
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
    assert_eq!(node.size_of_set(), Some(12), "the container carries the set size, hidden tabs counted");
    assert_eq!(node.active_descendant().map(|n| n.id()), None, "no active descendant without the focus");
    let modified = tab(&harness, "Doc 1, RGB/8, modified");
    let node = modified.accesskit_node();
    assert_eq!(
        (node.is_selected(), node.position_in_set(), node.size_of_set()),
        (Some(false), Some(1), None),
        "the tab has its position; the strip has the size"
    );
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
    let events = harness.state_mut().take();
    assert!(matches!(events.first(), Some(TabEvent::Reorder(EditEvent::Begin { .. }))), "it reorders instead");
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

type Reordered = EditEvent<Move<u32>>;

fn order(harness: &Harness<'_, Host>) -> Vec<u32> {
    harness.state().tabs.iter().map(|t| *t.key()).collect()
}

fn mv(item: u32, target: u32, position: DropPosition) -> Move<u32> {
    Move { items: vec![item], target, position, copy: false }
}

/// A point in the tab `label`, `share` of its width from its left edge.
fn at(harness: &Harness<'_, Host>, label: &str, share: f32) -> Pos2 {
    let rect = tab(harness, label).rect();
    pos2(rect.left() + rect.width() * share, rect.center().y)
}

fn pointer(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: Option<bool>) {
    harness.event(Event::PointerMoved(pos));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

/// Presses on the tab `label` and moves past egui's drag threshold.
fn grab(harness: &mut Harness<'_, Host>, label: &str) {
    let start = at(harness, label, 0.5);
    pointer(harness, start, Some(true));
    pointer(harness, start + vec2(0.0, 8.0), None);
}

fn payload(harness: &Harness<'_, Host>) -> Option<ListDrag<u32>> {
    DragAndDrop::payload::<ListDrag<u32>>(&harness.ctx).map(|p| (*p).clone())
}

fn focus_window(harness: &mut Harness<'_, Host>, focused: bool) {
    harness.input_mut().focused = focused;
    harness.event(Event::WindowFocused(focused));
    harness.run();
}

/// How a drag ended: the move dropped, or why it was cancelled.
type Ending = Result<Move<u32>, CancelReason>;

/// The drop targets the drag showed since the last call (without the empty start), and how it
/// ended.
fn summary(harness: &mut Harness<'_, Host>) -> (Vec<Move<u32>>, Option<Ending>) {
    let targets = std::mem::take(&mut harness.state_mut().targets).into_iter().flatten().collect();
    let end = harness.state_mut().take().iter().find_map(|event| match event {
        TabEvent::Reorder(EditEvent::Commit { value, .. }) => Some(Ok(value.clone())),
        TabEvent::Reorder(EditEvent::Cancel { reason, .. }) => Some(Err(*reason)),
        _ => None,
    });
    (targets, end)
}

#[test]
fn a_drag_moves_a_tab_to_a_half_of_another_and_after_the_last_shown_tab() {
    let mut harness = harness(Host::new(4, 0));
    grab(&mut harness, "Doc 0");
    assert!(matches!(harness.state_mut().take()[..], [TabEvent::Reorder(EditEvent::Begin { .. })]));
    let drag = payload(&harness).expect("the payload");
    assert_eq!((drag.items, drag.label.as_str()), (vec![0], "Doc 0"), "one tab, labelled");
    assert_eq!(harness.state().selected, 0, "a drag doesn't select");
    for label in ["Doc 1", "Doc 2"] {
        let pos = at(&harness, label, 0.75);
        pointer(&mut harness, pos, None);
    }
    let pos = at(&harness, "Doc 2", 0.75);
    pointer(&mut harness, pos, Some(false));
    let (targets, end) = summary(&mut harness);
    assert_eq!(targets, [mv(0, 2, DropPosition::Before), mv(0, 3, DropPosition::Before)], "a slot between tabs is always before the next tab");
    assert_eq!(end, Some(Ok(mv(0, 3, DropPosition::Before))));
    assert_eq!(order(&harness), [1, 2, 0, 3]);
    assert!(payload(&harness).is_none(), "a drop on the strip takes the payload");
    assert_eq!(harness.state().dropped, None);

    harness.run_steps(30);
    grab(&mut harness, "Doc 3");
    let pos = at(&harness, "Doc 2", 0.25);
    pointer(&mut harness, pos, None);
    pointer(&mut harness, pos, Some(false));
    assert_eq!(summary(&mut harness).1, Some(Ok(mv(3, 2, DropPosition::Before))), "leftwards: the half before the tab");
    assert_eq!(order(&harness), [1, 3, 2, 0]);

    let mut host = Host::new(10, 0);
    host.width = 300.0;
    let mut harness = self::harness(host);
    let shown = harness.state().shown.len();
    assert!(shown < 10, "the strip overflows");
    let last = format!("Doc {}", shown - 1);
    grab(&mut harness, "Doc 0");
    for pos in [at(&harness, &last, 0.75), pos2(harness.state().strip.right() - 2.0, harness.state().strip.center().y)] {
        pointer(&mut harness, pos, None);
        assert_eq!(
            harness.state().targets.last(),
            Some(&Some(mv(0, shown as u32 - 1, DropPosition::After))),
            "after the last shown tab, past it too"
        );
    }
    let pos = at(&harness, &last, 0.75);
    pointer(&mut harness, pos, Some(false));
    assert_eq!(summary(&mut harness).1, Some(Ok(mv(0, shown as u32 - 1, DropPosition::After))));
}

#[test]
fn a_drop_that_changes_nothing_has_no_target_and_drops_nothing() {
    let mut harness = harness(Host::new(4, 0));
    grab(&mut harness, "Doc 1");
    for pos in [at(&harness, "Doc 1", 0.25), at(&harness, "Doc 1", 0.75), at(&harness, "Doc 2", 0.25), at(&harness, "Doc 0", 0.75)] {
        pointer(&mut harness, pos, None);
        assert_eq!(harness.state().targets.last(), Some(&None), "no marker on the tab or its neighbours' edges");
    }
    let pos = at(&harness, "Doc 2", 0.25);
    pointer(&mut harness, pos, Some(false));
    let (targets, end) = summary(&mut harness);
    assert!(targets.is_empty());
    assert_eq!(end, Some(Err(CancelReason::Dismissed)));
    assert_eq!(order(&harness), [0, 1, 2, 3]);
    assert!(payload(&harness).is_none(), "the drop took the payload");
}

#[test]
fn a_drop_across_hidden_tabs_is_a_move_even_when_the_shown_tabs_are_adjacent() {
    let mut host = Host::new(10, 0);
    host.width = 300.0;
    let mut harness = harness(host);
    let shown = harness.state().shown.len();
    assert!((3..10).contains(&shown), "the strip overflows");
    // Shown tabs are 0..shown-1 plus the selected one; select the last tab so a hidden run lies between.
    harness.state_mut().selected = 9;
    harness.run_steps(5);
    let shown: Vec<u32> = harness.state().shown.iter().map(|(key, _)| *key).collect();
    let (&before, &last) = (&shown[shown.len() - 2], shown.last().unwrap());
    assert!(last - before > 1, "hidden tabs lie between the last two shown: {shown:?}");
    grab(&mut harness, &format!("Doc {before}"));
    let pos = at(&harness, &format!("Doc {last}"), 0.25);
    pointer(&mut harness, pos, None);
    assert_eq!(harness.state().targets.last(), Some(&Some(mv(before, last, DropPosition::Before))));
    pointer(&mut harness, pos, Some(false));
    assert_eq!(summary(&mut harness).1, Some(Ok(mv(before, last, DropPosition::Before))));
    assert_eq!(order(&harness).iter().position(|&k| k == before), Some(last as usize - 1));
}

/// The x of the drop line painted this frame: the rect filled with the `tab.drop` colour.
fn drop_line(harness: &Harness<'_, Host>) -> Vec<Rect> {
    fn collect(shape: &egui::Shape, colour: Color32, found: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, colour, found)),
            egui::Shape::Rect(rect) if rect.fill == colour => found.push(rect.rect),
            _ => {}
        }
    }
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut colour = Color32::TRANSPARENT;
    ctx.run_ui(egui::RawInput::default(), |ui| colour = Tab::<u32>::style(&Tokens::of(ui)).drop).drop_without_applying_deltas();
    let mut found = Vec::new();
    harness.output().shapes.iter().for_each(|clipped| collect(&clipped.shape, colour, &mut found));
    found
}

#[test]
fn a_slot_between_two_tabs_draws_one_drop_line_from_either_side() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut harness = harness_in(Host::new(4, 0), dir);
        grab(&mut harness, "Doc 0");
        let mut lines = Vec::new();
        // The two halves of the boundary between Doc 1 and Doc 2, in reading order.
        let (end_of_one, start_of_two) = if dir == Direction::Ltr { (0.75, 0.25) } else { (0.25, 0.75) };
        for (label, share) in [("Doc 1", end_of_one), ("Doc 2", start_of_two)] {
            let pos = at(&harness, label, share);
            pointer(&mut harness, pos, None);
            let found = drop_line(&harness);
            assert_eq!(found.len(), 1, "{dir:?} {label}: one drop line");
            lines.push(found[0]);
        }
        assert_eq!(lines[0], lines[1], "{dir:?}: the same placement on both sides of the boundary");
        let two = tab(&harness, "Doc 2").rect();
        let edge = if dir == Direction::Ltr { two.left() } else { two.right() };
        assert!(
            (lines[0].min.x - edge).abs() < 8.0 && lines[0].max.x > two.min.x && lines[0].min.x < two.max.x,
            "{dir:?}: inside Doc 2 at its start edge"
        );
    }
}

#[test]
fn a_drag_ends_on_escape_focus_loss_a_removed_tab_and_a_cleared_payload() {
    let mut harness = harness(Host::new(5, 0));
    focus_window(&mut harness, true);
    grab(&mut harness, "Doc 1");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Escape)));
    assert!(payload(&harness).is_none());
    let pos = at(&harness, "Doc 3", 0.5);
    pointer(&mut harness, pos, Some(false));
    assert!(harness.state_mut().take().is_empty(), "the rest of a cancelled drag does nothing");

    harness.run_steps(30);
    grab(&mut harness, "Doc 1");
    focus_window(&mut harness, false);
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::FocusLost)));
    focus_window(&mut harness, true);
    let pos = at(&harness, "Doc 3", 0.5);
    pointer(&mut harness, pos, Some(false));

    harness.run_steps(30);
    grab(&mut harness, "Doc 1");
    harness.state_mut().tabs.retain(|t| *t.key() != 1);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Removed)), "the dragged tab left the strip");
    assert!(payload(&harness).is_none());
    let pos = at(&harness, "Doc 3", 0.5);
    pointer(&mut harness, pos, Some(false));

    harness.run_steps(30);
    grab(&mut harness, "Doc 2");
    DragAndDrop::clear_payload(&harness.ctx);
    harness.run();
    assert_eq!(summary(&mut harness).1, Some(Err(CancelReason::Dismissed)), "the app cleared the payload: not an Escape");
}

#[test]
fn a_drag_whose_strip_skipped_passes_is_over_and_leaves_no_stale_state() {
    let mut harness = harness(Host::new(4, 0));
    grab(&mut harness, "Doc 1");
    let id = harness.state_mut().take().into_iter().find_map(|event| match event {
        TabEvent::Reorder(EditEvent::Begin { id, .. }) => Some(id),
        _ => None,
    });
    let id = id.expect("the drag began");
    assert!(payload(&harness).is_some());

    harness.state_mut().hidden = true;
    harness.run_steps(3);
    assert!(!crate::edit::is_live(&harness.ctx, id), "a control not drawn loses its gesture; the gesture host ends it as Removed");
    harness.state_mut().hidden = false;
    harness.run();
    let pos = at(&harness, "Doc 3", 0.5);
    pointer(&mut harness, pos, None);
    let events = harness.state_mut().take();
    assert!(!events.iter().any(|e| matches!(e, TabEvent::Reorder(_))), "the redrawn strip reports nothing for the lost drag: {events:?}");
    assert!(payload(&harness).is_none(), "the stale payload is cleared");
    assert!(harness.state().targets.last().is_none_or(Option::is_none), "no target");
    assert!(drop_line(&harness).is_empty(), "no marker");
    pointer(&mut harness, pos, Some(false));
    assert!(harness.state_mut().take().iter().all(|e| !matches!(e, TabEvent::Reorder(_))), "the release of the lost drag does nothing");
    assert_eq!(order(&harness), [0, 1, 2, 3]);

    harness.run_steps(30);
    grab(&mut harness, "Doc 2");
    let begun = harness.state_mut().take().into_iter().find_map(|event| match event {
        TabEvent::Reorder(EditEvent::Begin { id: new, .. }) => Some(new),
        _ => None,
    });
    assert!(begun.is_some_and(|new| new != id), "a fresh drag begins on its own");
}

#[test]
fn a_tab_dragged_past_the_band_leaves_and_a_release_there_leaves_the_payload() {
    let mut host = Host::new(4, 0);
    host.tear_off = true;
    let mut harness = harness(host);
    let band = tear_off_band();
    grab(&mut harness, "Doc 1");
    assert!(harness.state().dragging_out.is_none(), "inside the band");
    let strip = harness.state().strip;
    let outside = pos2(strip.center().x, strip.bottom() + band + 8.0);
    pointer(&mut harness, outside, None);
    let out = harness.state().dragging_out.clone().expect("outside the band");
    assert_eq!((out.tab, out.pointer), (1, outside));
    assert_eq!(harness.state().targets.last(), Some(&None), "no target outside");

    let inside = at(&harness, "Doc 3", 0.75);
    pointer(&mut harness, inside, None);
    assert!(harness.state().dragging_out.is_none(), "back inside the band");
    assert_eq!(harness.state().targets.last(), Some(&Some(mv(1, 3, DropPosition::After))), "targets resume");

    let zone = harness.state().zone.center();
    assert!(zone.y > strip.bottom() + band, "the zone is outside the band");
    pointer(&mut harness, zone, None);
    pointer(&mut harness, zone, Some(false));
    let events = harness.state_mut().take();
    assert!(events.contains(&TabEvent::DroppedOut { tab: 1, pointer: zone }), "{events:?}");
    assert!(events.iter().any(|e| matches!(e, TabEvent::Reorder(EditEvent::Cancel { reason: CancelReason::Dismissed, .. }))));
    assert_eq!(harness.state().dropped, Some(vec![1]), "the widget under the pointer takes the payload");
    assert_eq!(order(&harness), [0, 1, 2, 3]);

    let mut harness = self::harness(Host::new(4, 0));
    grab(&mut harness, "Doc 1");
    let zone = harness.state().zone.center();
    pointer(&mut harness, zone, None);
    assert!(harness.state().dragging_out.is_none(), "without tear-off only the position along the strip counts");
}

fn press(harness: &mut Harness<'_, Host>, modifiers: Modifiers, k: Key) -> Option<Reordered> {
    key(harness, modifiers, k);
    harness.state_mut().take().into_iter().find_map(|event| match event {
        TabEvent::Reorder(event) => Some(event),
        _ => None,
    })
}

/// The move of a `Set`.
fn set_value(event: Option<Reordered>) -> Option<Move<u32>> {
    match event {
        Some(EditEvent::Set { value, .. }) => Some(value),
        other => {
            assert!(other.is_none(), "{other:?}");
            None
        }
    }
}

#[test]
fn the_reorder_keys_move_the_focused_tab_later_or_earlier_and_keep_the_focus() {
    let mut harness = harness(Host::new(4, 1));
    focus(&mut harness);
    let ctrl = Modifiers::CTRL;
    let shift = Modifiers::CTRL | Modifiers::SHIFT;
    let cases = [
        (ctrl, Key::CloseBracket, mv(1, 2, DropPosition::After), [0, 2, 1, 3]),
        (ctrl, Key::OpenBracket, mv(1, 2, DropPosition::Before), [0, 1, 2, 3]),
        (shift, Key::CloseBracket, mv(1, 3, DropPosition::After), [0, 2, 3, 1]),
        (shift, Key::OpenBracket, mv(1, 0, DropPosition::Before), [1, 0, 2, 3]),
    ];
    for (modifiers, k, expected, now) in cases {
        let event = press(&mut harness, modifiers, k);
        assert_eq!(set_value(event), Some(expected), "{k:?}");
        assert_eq!(order(&harness), now);
        assert_eq!(active(&harness).as_deref(), Some("Doc 1"), "the focus stays on the moved tab");
    }
    let event = press(&mut harness, ctrl, Key::OpenBracket);
    assert_eq!(set_value(event), None, "no move past the start");

    harness.state_mut().reorderable = false;
    harness.run();
    let event = press(&mut harness, ctrl, Key::CloseBracket);
    assert_eq!(set_value(event), None, "a locked strip takes no reorder keys");
}

#[test]
fn the_reorder_keys_run_over_every_tab_including_hidden_ones() {
    let mut host = Host::new(10, 0);
    host.width = 300.0;
    let mut harness = harness(host);
    let shown = harness.state().shown.len();
    assert!(shown < 10, "the strip overflows");
    focus(&mut harness);
    let last_shown = format!("Doc {}", shown - 1);
    tab(&harness, &last_shown).focus();
    harness.run();
    harness.run();
    let event = press(&mut harness, Modifiers::CTRL, Key::CloseBracket);
    assert_eq!(set_value(event), Some(mv(shown as u32 - 1, shown as u32, DropPosition::After)), "] moves past the last shown tab");
    assert_eq!(order(&harness)[shown], shown as u32 - 1, "it lands among the hidden tabs");

    let mut host = Host::new(10, 0);
    host.width = 300.0;
    let mut harness = self::harness(host);
    focus(&mut harness);
    let event = press(&mut harness, Modifiers::CTRL | Modifiers::SHIFT, Key::CloseBracket);
    assert_eq!(set_value(event), Some(mv(0, 9, DropPosition::After)), "Shift+] reaches the true end");
    assert_eq!(order(&harness).last(), Some(&0));
}

#[test]
fn command_and_the_brackets_reorder_on_macos() {
    let mut harness = Harness::<Host>::builder()
        .with_size([480.0, 300.0])
        .with_os(egui::os::OperatingSystem::Mac)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, Host::new(4, 1));
    harness.run();
    focus(&mut harness);
    let event = press(&mut harness, Modifiers::MAC_CMD, Key::CloseBracket);
    assert_eq!(set_value(event), Some(mv(1, 2, DropPosition::After)));
    assert_eq!(order(&harness), [0, 2, 1, 3]);
}

#[test]
fn right_to_left_targets_and_keys_follow_the_reading_direction() {
    let mut harness = harness_in(Host::new(4, 0), Direction::Rtl);
    assert!(tab(&harness, "Doc 0").rect().center().x > tab(&harness, "Doc 3").rect().center().x, "tabs read from the right");
    grab(&mut harness, "Doc 0");
    // The left quarter of a tab is its reading-end half.
    let pos = at(&harness, "Doc 2", 0.25);
    pointer(&mut harness, pos, None);
    pointer(&mut harness, pos, Some(false));
    assert_eq!(summary(&mut harness).1, Some(Ok(mv(0, 3, DropPosition::Before))));
    assert_eq!(order(&harness), [1, 2, 0, 3]);

    harness.run_steps(30);
    grab(&mut harness, "Doc 3");
    let pos = at(&harness, "Doc 2", 0.75);
    pointer(&mut harness, pos, None);
    pointer(&mut harness, pos, Some(false));
    assert_eq!(summary(&mut harness).1, Some(Ok(mv(3, 2, DropPosition::Before))));

    let mut harness = harness_in(Host::new(4, 1), Direction::Rtl);
    focus(&mut harness);
    let event = press(&mut harness, Modifiers::CTRL, Key::CloseBracket);
    assert!(matches!(event, Some(EditEvent::Set { value, .. }) if value == mv(1, 2, DropPosition::After)), "] moves later in reading order");
}
