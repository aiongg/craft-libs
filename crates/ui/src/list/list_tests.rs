//! List tests: the virtualised viewport with 10,000 rows and with mixed heights, focus and keys,
//! clicks, AccessKit, rename coordination, sweeps whose row scrolls away or leaves, a column's
//! key, the context-menu hook and type-ahead.

use egui::accesskit::Role;
use egui::{Event, Key, KeyboardShortcut, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, TouchPhase, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::view::{RowsArgs, SelectionArg, show_rows};
use super::*;
use crate::behavior::{RowInfo, Selection};
use crate::edit::EditEvent;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

const LIST_HEIGHT: f32 = 200.0;

/// A host with its rows, labels, eye states and selection, and the events its list reported.
struct Host {
    rows: Vec<RowInfo<usize>>,
    names: Vec<String>,
    visible: Vec<bool>,
    selection: Selection<usize>,
    events: Vec<ListEvent<usize>>,
    heights: Option<fn(usize) -> f32>,
    allow_empty: bool,
    type_ahead: bool,
    start_rename: Option<usize>,
    draft: Option<fn(usize) -> Option<String>>,
    /// Keys of rows the list may focus but not select.
    unselectable: Vec<usize>,
    /// Keys of rows drawn as spacers (also sections of `rows`).
    spacers: Vec<usize>,
}

impl Host {
    fn new(count: usize) -> Self {
        Self::named((0..count).map(|i| format!("Row {i}")).collect())
    }

    fn named(names: Vec<String>) -> Self {
        let count = names.len();
        Host {
            rows: (0..count).map(RowInfo::leaf).collect(),
            names,
            visible: vec![true; count],
            selection: Selection::only(0),
            events: Vec::new(),
            heights: None,
            allow_empty: true,
            type_ahead: false,
            start_rename: None,
            draft: None,
            unselectable: Vec::new(),
            spacers: Vec::new(),
        }
    }

    fn take_events(&mut self) -> Vec<ListEvent<usize>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let hide = KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma);
    let Host { rows, names, visible, selection, events, heights, allow_empty, type_ahead, start_rename, draft, unselectable, spacers } = host;
    let mut list = List::new("list", "Layers", rows.as_slice())
        .selection(selection)
        .max_height(LIST_HEIGHT)
        .renamable(true)
        .allow_empty(*allow_empty)
        .type_ahead(*type_ahead);
    if let Some(heights) = *heights {
        list = list.heights(heights);
    }
    if let Some(key) = start_rename.take() {
        list = list.start_rename(key);
    }
    if let Some(draft) = *draft {
        list = list.rename_draft(move |&key| draft(key));
    }
    let out = list.show(ui, |_, &key| {
        if spacers.contains(&key) {
            return ListRow::spacer(key);
        }
        ListRow::new(key, names[key].clone())
            .toggle(ToggleColumn::eye("Visibility {row}", visible[key]).key(hide))
            .selectable(!unselectable.contains(&key))
    });
    for event in out.events {
        match &event {
            ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) => visible[value.row] = value.on,
            ListEvent::Row(RowEvent::Renamed { row, text }) if !text.is_empty() => names[*row].clone_from(text),
            _ => {}
        }
        events.push(event);
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, host);
    harness.run();
    harness
}

/// Tab into the list.
fn focus(harness: &mut Harness<'_, Host>) {
    harness.key_press(Key::Tab);
    harness.run();
    assert!(focused(harness), "Tab reaches the list");
}

/// Whether the list has the focus: AccessKit then reports its active descendant, which a screen
/// reader treats as focused.
fn focused(harness: &Harness<'_, Host>) -> bool {
    active(harness).is_some()
}

fn list<'h>(harness: &'h Harness<'_, Host>) -> Node<'h> {
    harness.get_by_role(Role::ListBox)
}

fn row<'h>(harness: &'h Harness<'_, Host>, at: usize) -> Node<'h> {
    let label = format!("Row {at}");
    harness.get_by(move |n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label.as_str()))
}

/// The positions of the rows drawn.
fn drawn(harness: &Harness<'_, Host>) -> Vec<usize> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().position_in_set()).collect()
}

fn active(harness: &Harness<'_, Host>) -> Option<String> {
    list(harness).accesskit_node().active_descendant().and_then(|n| n.label())
}

/// Whether the row's rect lies inside the list's.
fn shown(harness: &Harness<'_, Host>, at: usize) -> bool {
    list(harness).rect().expand(0.5).contains_rect(row(harness, at).rect())
}

fn wheel(harness: &mut Harness<'_, Host>, points: f32) {
    let center = list(harness).rect().center();
    harness.event(Event::PointerMoved(center));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, points),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    // The scroll area settles over a few frames.
    harness.run_steps(20);
}

/// Lets the double-click delay pass, so the next click is a single click.
fn pause(harness: &mut Harness<'_, Host>) {
    harness.run_steps(30);
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn click_at(harness: &mut Harness<'_, Host>, pos: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

#[test]
fn a_list_of_10000_rows_draws_the_rows_in_view_and_keeps_the_focus_while_they_scroll() {
    let mut harness = harness(Host::new(10_000));
    let stops = a11y::tab_order(&mut harness);
    let stops: Vec<_> = stops.iter().map(|s| (s.role, s.label.as_str())).collect();
    assert_eq!(stops, [(Role::ListBoxOption, "Row 0")], "one Tab stop: the list, whose focus is on its lead");
    assert_eq!(list(&harness).accesskit_node().size_of_set(), Some(10_000));
    let rows = drawn(&harness);
    let row_height = row(&harness, 0).rect().height();
    assert!(rows.len() as f32 <= (LIST_HEIGHT / row_height).ceil() + 2.0, "only the rows in view and one below: {rows:?}");
    assert_eq!(rows.first(), Some(&0));

    key(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(harness.state().selection, Selection::only(9999));
    assert!(shown(&harness, 9999), "the lead is scrolled into view");
    assert_eq!(active(&harness).as_deref(), Some("Row 9999"));

    wheel(&mut harness, 1e6);
    assert!(drawn(&harness).contains(&0), "scrolled back to the top");
    assert!(focused(&harness), "the list keeps the focus while its lead scrolls away");
    assert_eq!(active(&harness).as_deref(), Some("Row 9999"), "the lead stays drawn for the active descendant");
    assert!(!shown(&harness, 9999));

    key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(harness.state().selection, Selection::only(9998));
    assert!(shown(&harness, 9998), "a key move reveals the lead");
}

#[test]
fn mixed_heights_place_each_row_by_its_own_height() {
    let mut host = Host::new(1000);
    host.heights = Some(|at| if at % 2 == 0 { 20.0 } else { 40.0 });
    let mut harness = harness(host);
    let (first, second) = (row(&harness, 0).rect(), row(&harness, 1).rect());
    assert_eq!((first.height(), second.height()), (20.0, 40.0));
    assert_eq!(second.top(), first.bottom());
    assert!(drawn(&harness).len() <= (LIST_HEIGHT / 30.0) as usize + 3, "{:?}", drawn(&harness));

    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::End);
    assert!(shown(&harness, 999));
    assert_eq!(row(&harness, 999).rect().height(), 40.0);
}

/// The top of row `at` in the content of a list whose rows are 20 and 40 high in turn.
fn mixed_top(at: usize) -> f32 {
    (at / 2) as f32 * 60.0 + if at % 2 == 1 { 20.0 } else { 0.0 }
}

/// Whether the pinned row `at` is drawn where its height and the rows above put it, against the
/// first row in view.
fn placed(harness: &Harness<'_, Host>, at: usize) -> bool {
    let list_top = list(harness).rect().top();
    let Some(&first) = drawn(harness).iter().filter(|&&d| row(harness, d).rect().bottom() > list_top + 0.5).min() else { return false };
    let (pinned, reference) = (row(harness, at).rect(), row(harness, first).rect());
    let height = if at % 2 == 1 { 40.0 } else { 20.0 };
    ((pinned.top() - reference.top()) - (mixed_top(at) - mixed_top(first))).abs() < 0.5 && (pinned.height() - height).abs() < 0.5
}

#[test]
fn mixed_heights_keep_pinned_rows_in_place_and_page_by_the_rows_in_view() {
    let host = || {
        let mut host = Host::new(1000);
        host.heights = Some(|at| if at % 2 == 0 { 20.0 } else { 40.0 });
        host
    };
    let mut harness = harness(host());
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::PageDown);
    assert_eq!(harness.state().selection, Selection::only(7), "rows 0 to 6 fill the 200 points: 20 + 40 + ... + 20");
    assert!(shown(&harness, 7));

    wheel(&mut harness, -3000.0);
    assert!(!shown(&harness, 7), "the lead scrolled away");
    assert!(placed(&harness, 7), "the lead stays drawn at its place");

    let mut harness = self::harness(host());
    harness.state_mut().start_rename = Some(3);
    harness.run();
    wheel(&mut harness, -3000.0);
    assert!(!shown(&harness, 3) && placed(&harness, 3), "the row being renamed");

    let mut harness = self::harness(host());
    let start = eye(&harness, 5);
    button(&mut harness, start, true);
    wheel(&mut harness, -3000.0);
    assert!(!shown(&harness, 5) && placed(&harness, 5), "the row that took a sweep's press");
    button(&mut harness, start, false);
}

#[test]
fn keys_select_extend_move_the_lead_toggle_and_activate() {
    let mut harness = harness(Host::new(10));
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state().selection, Selection::only(1));
    key(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(harness.state().selection, Selection { items: vec![1, 2], lead: Some(2), anchor: Some(1) });
    harness.state_mut().take_events();

    key(&mut harness, Modifiers::CTRL, Key::ArrowDown);
    assert_eq!(harness.state().selection, Selection { items: vec![1, 2], lead: Some(3), anchor: Some(1) }, "Ctrl moves the lead only");
    assert_eq!(harness.state_mut().take_events(), [ListEvent::LeadMoved(3)]);
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state().selection.items, [1, 2, 3], "Space toggles the lead's row");
    harness.state_mut().take_events();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state_mut().take_events(), [ListEvent::Activate(3)]);
    assert!(focused(&harness));
}

#[test]
fn clicks_select_and_a_click_on_the_empty_space_clears() {
    let mut host = Host::new(3);
    host.selection = Selection::new();
    let mut harness = harness(host);
    row(&harness, 1).click();
    harness.run();
    assert_eq!(harness.state().selection, Selection::only(1));
    assert!(focused(&harness), "a press on a row gives the list the focus");
    pause(&mut harness);
    row(&harness, 2).click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(harness.state().selection.items, [1, 2]);
    pause(&mut harness);
    row(&harness, 0).click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(harness.state().selection, Selection { items: vec![0, 1, 2], lead: Some(0), anchor: Some(2) });

    pause(&mut harness);
    let below = list(&harness).rect().center_bottom() - vec2(0.0, 10.0);
    click_at(&mut harness, below, PointerButton::Primary);
    assert!(harness.state().selection.is_empty(), "a click below the rows clears");

    let mut host = Host::new(3);
    host.allow_empty = false;
    let mut harness = self::harness(host);
    click_at(&mut harness, below, PointerButton::Primary);
    assert_eq!(harness.state().selection, Selection::only(0), "a list that keeps a current row");
}

#[test]
fn accesskit_names_the_list_and_its_rows_and_takes_their_actions() {
    let mut harness = harness(Host::new(3));
    let node = list(&harness);
    let node = node.accesskit_node();
    assert_eq!((node.label().as_deref(), node.is_multiselectable(), node.size_of_set()), (Some("Layers"), true, Some(3)));
    let rows: Vec<_> =
        harness.query_all_by_role(Role::ListBoxOption).map(|n| (n.accesskit_node().position_in_set(), n.accesskit_node().is_selected())).collect();
    assert_eq!(rows, [(Some(0), Some(true)), (Some(1), Some(false)), (Some(2), Some(false))]);

    row(&harness, 2).click_accesskit();
    harness.run();
    assert_eq!(harness.state().selection, Selection::only(2), "Click selects the row alone");
    harness.state_mut().take_events();
    row(&harness, 1).focus();
    harness.run();
    assert_eq!(harness.state_mut().take_events(), [ListEvent::LeadMoved(1)], "Focus makes the row the lead");
    assert_eq!(harness.state().selection.items, [2]);
    assert!(focused(&harness));
    assert_eq!(active(&harness).as_deref(), Some("Row 1"));
}

fn renamed_events(events: &[ListEvent<usize>]) -> Vec<RowEvent<usize>> {
    events
        .iter()
        .filter_map(|e| match e {
            ListEvent::Row(event @ (RowEvent::Renamed { .. } | RowEvent::RenameEnded { .. })) => Some(event.clone()),
            _ => None,
        })
        .collect()
}

fn field(harness: &Harness<'_, Host>) -> Option<String> {
    harness.query_by_role(Role::TextInput).and_then(|n| n.value())
}

// A rename draft is the text the field opens with and the text that counts as unchanged; a row without one opens on its label.
#[test]
fn a_rename_draft_replaces_the_label_the_field_starts_from() {
    let mut host = Host::new(4);
    host.draft = Some(|key| match key {
        1 => Some(String::new()),
        2 => Some("Draft".to_owned()),
        _ => None,
    });
    let mut harness = harness(host);
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::F2);
    assert_eq!(field(&harness).as_deref(), Some("Row 0"), "no draft keeps the label");
    key(&mut harness, Modifiers::NONE, Key::Tab);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the field is open");
    assert_eq!(field(&harness).unwrap_or_default(), "", "an empty draft opens an empty field");
    key(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(field(&harness).as_deref(), Some("Draft"));
    assert_eq!(
        renamed_events(&harness.state_mut().take_events()),
        [RowEvent::RenameEnded { row: 0, end: RenameEnd::Tab }, RowEvent::RenameEnded { row: 1, end: RenameEnd::Tab }],
        "the draft left as it was reports no Renamed"
    );
    harness.event(Event::Text("!".into())); // replaces the selected draft
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(
        renamed_events(&harness.state_mut().take_events()),
        [RowEvent::Renamed { row: 2, text: "!".into() }, RowEvent::RenameEnded { row: 2, end: RenameEnd::Enter }]
    );
}

#[test]
fn rename_moves_on_with_tab_commits_when_another_starts_or_its_row_leaves() {
    let mut harness = harness(Host::new(5));
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::F2);
    assert_eq!(field(&harness).as_deref(), Some("Row 0"), "F2 renames the lead");
    harness.event(Event::Text("Sky".into()));
    key(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(
        renamed_events(&harness.state_mut().take_events()),
        [RowEvent::Renamed { row: 0, text: "Sky".into() }, RowEvent::RenameEnded { row: 0, end: RenameEnd::Tab }]
    );
    assert_eq!(field(&harness).as_deref(), Some("Row 1"), "Tab renames the next row");

    harness.state_mut().start_rename = Some(3);
    harness.run();
    assert_eq!(renamed_events(&harness.state_mut().take_events()), [RowEvent::RenameEnded { row: 1, end: RenameEnd::Blur }], "unchanged");
    assert_eq!(field(&harness).as_deref(), Some("Row 3"));
    harness.event(Event::Text("Sea".into()));
    harness.run();
    harness.state_mut().rows.retain(|r| r.key != 3);
    harness.run();
    assert_eq!(
        renamed_events(&harness.state_mut().take_events()),
        [RowEvent::Renamed { row: 3, text: "Sea".into() }, RowEvent::RenameEnded { row: 3, end: RenameEnd::Removed }]
    );
    assert_eq!(field(&harness), None);

    pause(&mut harness);
    let label = row(&harness, 2).rect().center();
    // The press and the release are separate passes, as from a real pointer: the click reports on the
    // release, and a press in the pass the field opens in would drop the field's focus.
    harness.event(Event::PointerMoved(label));
    harness.run();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton { pos: label, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
            harness.step();
        }
    }
    harness.run();
    assert_eq!(field(&harness).as_deref(), Some("Row 2"), "a double click on the label renames");
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(field(&harness), None);
    assert!(focused(&harness), "Enter gives the focus back to the list");
}

fn toggle_events(events: &[ListEvent<usize>]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            ListEvent::Row(RowEvent::Toggle(event)) => Some(match event {
                EditEvent::Begin { .. } => "begin".to_owned(),
                EditEvent::Update { value, .. } => format!("update {} {}", value.row, value.on),
                EditEvent::Commit { value, .. } => format!("commit {}", value.row),
                EditEvent::Cancel { reason, .. } => format!("cancel {reason:?}"),
                EditEvent::Set { value, .. } => format!("set {}", value.row),
            }),
            _ => None,
        })
        .collect()
}

fn eye(harness: &Harness<'_, Host>, at: usize) -> Pos2 {
    harness.get_by_label(&format!("Visibility Row {at}")).rect().center()
}

fn button(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    harness.run();
}

#[test]
fn a_sweep_ends_on_its_row_scrolled_away_and_is_cancelled_when_the_row_leaves() {
    let mut harness = harness(Host::new(100));
    let (start, end) = (eye(&harness, 0), eye(&harness, 2));
    button(&mut harness, start, true);
    for t in [0.5, 1.0] {
        harness.event(Event::PointerMoved(start + (end - start) * t));
        harness.run();
    }
    wheel(&mut harness, -1e6);
    assert!(!shown(&harness, 0), "the pressed row scrolled away");
    let release = list(&harness).rect().center();
    button(&mut harness, release, false);
    assert_eq!(toggle_events(&harness.state().events), ["begin", "update 0 false", "update 1 false", "update 2 false", "commit 0"]);

    let mut harness = self::harness(Host::new(10));
    let start = eye(&harness, 0);
    button(&mut harness, start, true);
    harness.state_mut().rows.retain(|r| r.key != 0);
    harness.run();
    button(&mut harness, start, false);
    assert_eq!(toggle_events(&harness.state().events), ["begin", "update 0 false", "cancel Removed"]);
}

#[test]
fn a_columns_key_toggles_the_selected_rows_as_one_gesture() {
    let mut host = Host::new(4);
    host.selection = Selection { items: vec![0, 2], lead: Some(2), anchor: Some(2) };
    let mut harness = harness(host);
    focus(&mut harness);
    key(&mut harness, Modifiers::COMMAND, Key::Comma);
    assert_eq!(toggle_events(&harness.state_mut().take_events()), ["begin", "update 0 false", "update 2 false", "commit 2"]);
    assert_eq!(harness.state().visible, [false, true, false, true]);
    harness.state_mut().visible[0] = true;
    key(&mut harness, Modifiers::COMMAND, Key::Comma);
    assert_eq!(toggle_events(&harness.state_mut().take_events()), ["begin", "update 2 true", "commit 2"], "the lead's state decides");
}

#[test]
fn a_columns_key_inside_an_edit_scope_reports_only_its_updates() {
    let mut host = Host::new(4);
    host.selection = Selection { items: vec![0, 2], lead: Some(2), anchor: Some(2) };
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 300.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host| crate::edit::scope(ui, |ui| show(ui, host)).inner, host);
    harness.run();
    focus(&mut harness);
    key(&mut harness, Modifiers::COMMAND, Key::Comma);
    assert_eq!(toggle_events(&harness.state_mut().take_events()), ["update 0 false", "update 2 false"], "a dialog's gesture holds them");
}

#[test]
fn context_menus_are_asked_for_with_the_row_and_its_anchor() {
    let menus = |events: Vec<ListEvent<usize>>| -> Vec<ContextMenuRequest<usize>> {
        events.into_iter().filter_map(|e| if let ListEvent::ContextMenu(m) = e { Some(m) } else { None }).collect()
    };
    let mut harness = harness(Host::new(3));
    row(&harness, 1).click_secondary();
    harness.run();
    let rect = row(&harness, 1).rect();
    let got = menus(harness.state_mut().take_events());
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].row, got[0].selected, got[0].pointer.is_some()), (Some(1), false, true));
    assert!(rect.expand(0.5).contains_rect(got[0].anchor) && got[0].anchor.expand(0.5).contains_rect(rect));
    assert_eq!(harness.state().selection, Selection::only(0), "a secondary click doesn't select");

    focus(&mut harness);
    let lead: Rect = row(&harness, 0).rect();
    for (modifiers, pressed) in [(Modifiers::SHIFT, Key::F10), (Modifiers::NONE, Key::Menu)] {
        key(&mut harness, modifiers, pressed);
        let got = menus(harness.state_mut().take_events());
        assert_eq!(got.len(), 1, "{pressed:?}");
        assert_eq!((got[0].row, got[0].selected, got[0].pointer), (Some(0), true, None));
        assert!(lead.expand(0.5).contains_rect(got[0].anchor), "anchored at the lead's row");
    }

    let mut harness = self::harness(Host::new(100));
    focus(&mut harness);
    wheel(&mut harness, -3000.0);
    assert!(!shown(&harness, 0), "the lead scrolled away");
    key(&mut harness, Modifiers::SHIFT, Key::F10);
    let got = menus(harness.state_mut().take_events());
    assert_eq!(got.len(), 1);
    assert!(shown(&harness, 0), "the menu key reveals the lead");
    let (lead, anchor) = (row(&harness, 0).rect(), got[0].anchor);
    assert!(lead.expand(0.5).contains_rect(anchor) && anchor.expand(0.5).contains_rect(lead), "anchored at the revealed row: {anchor:?}, {lead:?}");
}

#[test]
fn type_ahead_is_off_by_default_and_moves_by_label_when_on() {
    for on in [false, true] {
        let mut host = Host::named(["Apple", "Banana", "Cherry", "Cranberry"].map(String::from).to_vec());
        host.type_ahead = on;
        let mut harness = harness(host);
        focus(&mut harness);
        harness.event(Event::Text("c".into()));
        harness.run();
        let expected = if on { Selection::only(2) } else { Selection::only(0) };
        assert_eq!(harness.state().selection, expected, "type-ahead {on}");
    }
}

#[test]
fn the_list_story_names_every_control() {
    for mixed in [false, true] {
        let mut args = RowsArgs { selection: SelectionArg::Multiple, mixed_heights: mixed, striped: false, height: 320.0 };
        let harness = Harness::builder()
            .with_size([400.0, 420.0])
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui(move |ui| show_rows(ui, &mut args));
        assert_eq!(a11y::unlabelled(&harness), Vec::<String>::new());
        assert_eq!(harness.get_by_role(Role::ListBox).accesskit_node().size_of_set(), Some(10_000));
    }
}

/// Two sections of two items: Section 0, Row 1, Row 2, Section 3, Row 4, Row 5.
fn sectioned() -> Host {
    let mut host = Host::named(["Atoms", "Row 1", "Row 2", "Molecules", "Row 4", "Row 5"].map(String::from).to_vec());
    host.rows[0] = RowInfo::section(0);
    host.rows[3] = RowInfo::section(3);
    host.selection = Selection::only(1);
    host
}

#[test]
fn arrow_keys_cross_a_section_and_home_and_end_land_on_items() {
    let mut harness = harness(sectioned());
    focus(&mut harness);
    for (pressed, expected) in [(Key::ArrowDown, 2), (Key::ArrowDown, 4), (Key::ArrowDown, 5), (Key::ArrowDown, 5)] {
        key(&mut harness, Modifiers::NONE, pressed);
        assert_eq!(harness.state().selection, Selection::only(expected), "{pressed:?}");
        assert_eq!(active(&harness).as_deref(), Some(format!("Row {expected}").as_str()));
    }
    for (pressed, expected) in [(Key::ArrowUp, 4), (Key::ArrowUp, 2), (Key::ArrowUp, 1), (Key::ArrowUp, 1)] {
        key(&mut harness, Modifiers::NONE, pressed);
        assert_eq!(harness.state().selection, Selection::only(expected), "{pressed:?}");
    }
    key(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(harness.state().selection, Selection::only(5));
    key(&mut harness, Modifiers::NONE, Key::Home);
    assert_eq!(harness.state().selection, Selection::only(1), "Home skips the section above the first item");
}

#[test]
fn a_section_is_a_heading_that_is_not_selected_focused_or_counted() {
    let mut harness = harness(sectioned());
    let tab_stops = a11y::tab_order(&mut harness);
    assert_eq!(tab_stops.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(), ["Row 1"], "one Tab stop, on an item");
    assert_eq!(list(&harness).accesskit_node().size_of_set(), Some(4), "the sections are not counted");
    assert_eq!(drawn(&harness), [0, 1, 2, 3], "item positions skip the sections");
    for name in ["Atoms", "Molecules"] {
        let heading = harness.get_by(|n| n.role() == Role::Heading && n.label().as_deref() == Some(name));
        let at = heading.rect().center();
        click_at(&mut harness, at, PointerButton::Primary);
        assert_eq!(harness.state().selection, Selection::only(1), "a click on {name} changes nothing");
        assert!(focused(&harness) && active(&harness).as_deref() == Some("Row 1"), "the list is focused on its lead, not on {name}");
    }
    assert!(harness.state_mut().take_events().iter().all(|e| !matches!(e, ListEvent::Selection(_) | ListEvent::Activate(_) | ListEvent::Clicked(_))));
}

/// Row 0, a spacer, Row 2 (not selectable), Row 3, Row 4 (not selectable).
fn gapped() -> Host {
    let mut host = Host::new(5);
    host.rows[1] = RowInfo::section(1);
    host.spacers = vec![1];
    host.unselectable = vec![2, 4];
    host.heights = Some(|at| if at == 1 { 12.0 } else { 24.0 });
    host
}

#[test]
fn a_spacer_takes_room_without_a_node_and_a_press_on_it_keeps_the_selection() {
    let mut harness = harness(gapped());
    let (above, below) = (row(&harness, 0).rect(), row(&harness, 2).rect());
    assert_eq!(below.top() - above.bottom(), 12.0, "the spacer's height");
    assert_eq!(list(&harness).accesskit_node().size_of_set(), Some(4), "a spacer is not counted");
    let gap = Rect::from_min_max(above.left_bottom(), below.right_top()).expand2(vec2(1.0, 0.5));
    let inside =
        harness.query_all_by(|_| true).filter(|n| n.accesskit_node().bounding_box().is_some()).filter(|n| gap.contains_rect(n.rect())).count();
    assert_eq!(inside, 0, "no node sits in the spacer's rect");
    let at = above.left_bottom() + vec2(20.0, 6.0);
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state().selection, Selection::only(0), "a press on the spacer does not clear the selection");
    assert!(focused(&harness));
}

#[test]
fn keys_move_the_lead_onto_rows_that_are_not_selectable_without_selecting_them() {
    let mut harness = harness(gapped());
    focus(&mut harness);
    harness.state_mut().take_events();
    for (pressed, lead) in [(Key::ArrowDown, "Row 2"), (Key::ArrowDown, "Row 3")] {
        key(&mut harness, Modifiers::NONE, pressed);
        assert_eq!(active(&harness).as_deref(), Some(lead), "{pressed:?}");
    }
    assert_eq!(harness.state().selection, Selection::only(3), "Row 3 is selectable");
    key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(active(&harness).as_deref(), Some("Row 2"));
    assert_eq!(harness.state().selection.items, [3], "the lead moved onto Row 2 and the selection stayed");
    key(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(active(&harness).as_deref(), Some("Row 4"));
    assert_eq!(harness.state().selection.items, [3]);
    key(&mut harness, Modifiers::NONE, Key::Home);
    assert_eq!(harness.state().selection, Selection::only(0), "Home lands on the selectable first row");
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    key(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(harness.state().selection.items, [0], "Space on a row that is not selectable selects nothing");
    let events = harness.state_mut().take_events();
    assert!(events.contains(&ListEvent::LeadMoved(2)), "{events:?}");
}

#[test]
fn a_click_on_a_row_that_is_not_selectable_makes_it_the_lead_only() {
    let mut harness = harness(gapped());
    let at = row(&harness, 2).rect().center();
    harness.state_mut().take_events();
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state().selection.items, [0]);
    assert_eq!(active(&harness).as_deref(), Some("Row 2"));
    let events = harness.state_mut().take_events();
    assert!(events.contains(&ListEvent::LeadMoved(2)) && events.contains(&ListEvent::Clicked(2)), "{events:?}");
    assert!(events.iter().all(|e| !matches!(e, ListEvent::Selection(_))), "{events:?}");
}

#[test]
fn a_shift_range_over_rows_that_are_not_selectable_leaves_them_out() {
    let mut harness = harness(gapped());
    focus(&mut harness);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("Row 3"));
    key(&mut harness, Modifiers::SHIFT, Key::Home);
    let mut chosen = harness.state().selection.items.clone();
    chosen.sort_unstable();
    assert_eq!(chosen, [0, 3], "Row 2 is left out of the range");
}

#[test]
fn a_primary_click_on_the_selected_row_is_reported_though_the_selection_stays() {
    let mut harness = harness(sectioned());
    assert_eq!(harness.state().selection, Selection::only(1));
    let row = harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Row 1"));
    let at = row.rect().center();
    harness.state_mut().take_events();
    click_at(&mut harness, at, PointerButton::Primary);
    let events = harness.state_mut().take_events();
    assert!(events.contains(&ListEvent::Clicked(1)), "{events:?}");
    assert!(events.iter().all(|e| !matches!(e, ListEvent::Selection(_))), "the selection did not change: {events:?}");
}

#[test]
fn type_ahead_skips_sections() {
    let mut host = Host::named(["Citrus", "Apple", "Cherries", "Cranberry"].map(String::from).to_vec());
    host.rows[0] = RowInfo::section(0);
    host.rows[2] = RowInfo::section(2);
    host.selection = Selection::only(1);
    host.type_ahead = true;
    let mut harness = harness(host);
    focus(&mut harness);
    harness.event(Event::Text("c".into()));
    harness.run();
    assert_eq!(harness.state().selection, Selection::only(3));
}

/// A reordering list of three layer rows with an extra thumbnail each; the thumbnails' rects of
/// the last pass and the events reported.
type Dragging = (Vec<RowInfo<usize>>, std::cell::RefCell<Vec<Rect>>, Vec<ListEvent<usize>>);

// In a reordering list, a ThumbDrag row event starts no row drag (no ListDrag payload, no Begin);
// a drag from the row's label still does.
#[test]
fn a_thumbnail_drag_starts_no_row_drag() {
    use crate::behavior::{Axis, ListDrag, Reorder};
    use egui::DragAndDrop;
    let mut harness = Harness::<Dragging>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (rows, thumbs, events): &mut Dragging| {
                thumbs.get_mut().clear();
                let thumbs = &*thumbs;
                let out = List::new("list", "Layers", rows.as_slice()).reorder(Reorder::new(Axis::Vertical)).show(ui, |_, &key| {
                    ListRow::new(key, format!("Row {key}"))
                        .thumb(ThumbSize::Medium, |_, _| {})
                        .extra_thumb(|_, rect| thumbs.borrow_mut().push(rect))
                        .thumb_drags(true)
                });
                events.extend(out.events);
            },
            ((0..3).map(RowInfo::leaf).collect(), std::cell::RefCell::default(), Vec::new()),
        );
    harness.run();
    let drag = |harness: &mut Harness<'_, Dragging>, start: Pos2| {
        harness.event(egui::Event::PointerMoved(start));
        harness.run();
        harness.event(egui::Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
        harness.run();
        for dy in [8.0, 20.0, 40.0] {
            harness.event(egui::Event::PointerMoved(start + vec2(0.0, dy)));
            harness.run();
        }
    };
    let release = |harness: &mut Harness<'_, Dragging>| {
        harness.event(egui::Event::PointerButton { pos: Pos2::ZERO, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
        harness.run();
    };
    let thumb = harness.state().1.borrow()[0].center();
    drag(&mut harness, thumb);
    assert!(DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).is_none(), "a thumbnail drag leaves the payload to the host");
    assert!(
        harness.state().2.iter().all(|e| !matches!(e, ListEvent::Reorder(_)))
            && harness.state().2.iter().any(|e| matches!(e, ListEvent::Row(RowEvent::ThumbDrag { .. }))),
        "{:?}",
        harness.state().2
    );
    release(&mut harness);
    harness.state_mut().2.clear();

    let label = harness.get_by_label("Row 0").rect().center() + vec2(40.0, 0.0);
    drag(&mut harness, label);
    assert!(DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).is_some(), "a drag from the label starts the row drag");
    assert!(harness.state().2.iter().any(|e| matches!(e, ListEvent::Reorder(_))));
}

/// A list with a rename and drag policy, for the hooks a list gives its host.
struct Hooks {
    rows: Vec<RowInfo<usize>>,
    events: Vec<ListEvent<usize>>,
    drawn: Vec<(usize, Rect)>,
    icons: std::cell::RefCell<Vec<(usize, Rect)>>,
    paint_depth: Option<usize>,
    widths: Option<(f32, f32, f32)>,
    start_rename: Option<usize>,
}

fn hooks(rows: Vec<RowInfo<usize>>, tree: bool) -> Harness<'static, Hooks> {
    use crate::behavior::{Axis, Reorder};
    let mut harness = Harness::<Hooks>::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            move |ui, h: &mut Hooks| {
                h.icons.get_mut().clear();
                let (icons, paint_depth) = (&h.icons, h.paint_depth);
                let mut list = List::new("hooks", "Hooks", h.rows.as_slice())
                    .tree(tree)
                    .reorder(Reorder::new(Axis::Vertical))
                    .can_rename(|&key| key != 1)
                    .can_drag(|&key| key != 1);
                if let Some(key) = h.start_rename.take() {
                    list = list.start_rename(key);
                }
                let out = list.show(ui, |_, &key| {
                    let mut row =
                        ListRow::new(key, format!("Row {key}")).rename_field_name("Name").glyph(move |_, rect| icons.borrow_mut().push((key, rect)));
                    if let (2, Some(depth)) = (key, paint_depth) {
                        row = row.paint_depth(depth);
                    }
                    row
                });
                if h.widths.is_none() {
                    let row = ListRow::new(0usize, "x");
                    let tree = row.min_width_in_tree(ui, true);
                    h.widths = Some((row.min_width(ui), tree, ListRow::<usize>::style(&Tokens::of(ui)).disclosure));
                }
                h.drawn = out.rows;
                h.events.extend(out.events);
            },
            Hooks { rows, events: Vec::new(), drawn: Vec::new(), icons: Default::default(), paint_depth: None, widths: None, start_rename: None },
        );
    harness.run();
    harness
}

fn press_drag(harness: &mut Harness<'_, Hooks>, start: Pos2) {
    harness.event(Event::PointerMoved(start));
    harness.run();
    harness.event(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    for dy in [8.0, 20.0, 40.0] {
        harness.event(Event::PointerMoved(start + vec2(0.0, dy)));
        harness.run();
    }
}

#[test]
fn a_double_click_on_the_label_of_a_row_that_cannot_be_renamed_reports_it_before_the_activation() {
    let mut harness = hooks((0..3).map(RowInfo::leaf).collect(), false);
    let at = harness.get_by_label("Row 1").rect().center() + vec2(40.0, 0.0);
    harness.run_steps(30);
    for _ in 0..2 {
        harness.event(Event::PointerMoved(at));
        for pressed in [true, false] {
            harness.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
        harness.run();
    }
    let events = &harness.state().events;
    let double: Vec<_> =
        events.iter().filter(|e| matches!(e, ListEvent::Row(RowEvent::LabelDoubleClicked { .. }) | ListEvent::Activate(_))).collect();
    assert_eq!(double, [&ListEvent::Row(RowEvent::LabelDoubleClicked { row: 1 }), &ListEvent::Activate(1)], "{events:?}");
}

#[test]
fn a_refused_row_starts_no_drag_and_an_allowed_row_does() {
    use crate::behavior::ListDrag;
    use egui::DragAndDrop;
    let mut harness = hooks((0..3).map(RowInfo::leaf).collect(), false);
    let refused = harness.get_by_label("Row 1").rect().center();
    press_drag(&mut harness, refused);
    assert!(DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).is_none(), "no payload for a refused row");
    assert!(harness.state().events.iter().all(|e| !matches!(e, ListEvent::Reorder(_))), "{:?}", harness.state().events);
    harness.event(Event::PointerButton { pos: refused, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();

    let allowed = harness.get_by_label("Row 0").rect().center();
    press_drag(&mut harness, allowed);
    assert!(DragAndDrop::payload::<ListDrag<usize>>(&harness.ctx).is_some());
}

#[test]
fn the_output_lists_the_rects_of_the_rows_drawn() {
    let harness = hooks((0..3).map(RowInfo::leaf).collect(), false);
    for (key, rect) in &harness.state().drawn {
        assert_eq!(rect.round(), harness.get_by_label(&format!("Row {key}")).rect().round());
    }
    assert_eq!(harness.state().drawn.iter().map(|(k, _)| *k).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn the_rename_field_takes_the_name_the_row_gives() {
    let mut harness = hooks((0..3).map(RowInfo::leaf).collect(), false);
    harness.state_mut().start_rename = Some(0);
    harness.run();
    assert_eq!(harness.get_by_role(Role::TextInput).accesskit_node().label().as_deref(), Some("Name"));
}

#[test]
fn the_tree_width_adds_the_disclosure_column_of_a_group() {
    let harness = hooks((0..3).map(RowInfo::leaf).collect(), true);
    let (plain, tree, disclosure) = harness.state().widths.expect("measured");
    assert_eq!(tree - plain, disclosure);
}

#[test]
fn a_paint_depth_indents_the_row_as_the_true_depth_does() {
    let rows = |depth| vec![RowInfo::group(0, true), RowInfo::leaf(1).at_depth(1), RowInfo::leaf(2).at_depth(depth)];
    let x_of = |harness: &Harness<'_, Hooks>| harness.state().icons.borrow().iter().find(|(k, _)| *k == 2).map(|(_, r)| r.left()).unwrap();
    let deep = hooks(rows(1), true);
    let plain = x_of(&deep);
    let mut filtered = hooks(rows(0), true);
    filtered.state_mut().paint_depth = Some(1);
    filtered.run();
    assert_eq!(x_of(&filtered), plain);
    let unpainted = hooks(rows(0), true);
    assert!(x_of(&unpainted) < plain, "without a paint depth the row sits at its list depth");
}

// The popover a list is drawn in doesn't take its keys: ↓ moves the selection and Enter activates.
#[test]
fn a_list_in_a_popover_moves_with_down_and_picks_with_enter() {
    use crate::popover::{Popover, PopoverAnchor};

    struct App {
        selection: Selection<usize>,
        activated: Vec<usize>,
    }
    let app = App { selection: Selection::only(0), activated: Vec::new() };
    let mut harness = Harness::builder().with_size(vec2(320.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let anchor = ui.button("Open");
            let mut open = true;
            Popover::new("popover", PopoverAnchor::Control(anchor)).show(ui, &mut open, |ui, _| {
                let rows: Vec<RowInfo<usize>> = (0..3).map(RowInfo::leaf).collect();
                let out = List::new("list", "Items", rows.as_slice())
                    .selection(&mut app.selection)
                    .show(ui, |at, &key| ListRow::new(key, format!("Item {at}")));
                ui.memory_mut(|m| m.request_focus(out.response.id));
                for event in out.events {
                    if let ListEvent::Activate(key) = event {
                        app.activated.push(key);
                    }
                }
            });
        },
        app,
    );
    harness.run();
    for pressed in [Key::ArrowDown, Key::Enter] {
        harness.key_press(pressed);
        harness.run();
    }
    assert_eq!(harness.state().selection, Selection::only(1));
    assert_eq!(harness.state().activated, [1]);
}
