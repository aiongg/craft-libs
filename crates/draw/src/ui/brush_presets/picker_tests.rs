//! Tests of the Brush Preset picker (task BR-BP2).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::direction::{self, Direction};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::popover::{CloseReason, Popover, PopoverAnchor, any_open};
use craft_ui::presets::{PresetEvent, PresetRow};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::*;

type Key32 = u32;

struct Host {
    open: bool,
    state: BrushPickerState<Key32>,
    rows: Vec<PresetRow<Key32>>,
    current: Option<Key32>,
    diameter: f64,
    hardness: Option<f64>,
    import: bool,
    chip: egui::Rect,
    popover: egui::Id,
    picks: Vec<BrushPick<Key32>>,
    closed: Vec<CloseReason>,
    hardness_events: Vec<EditEvent<f64>>,
    settings: Vec<BrushPickerSettings>,
    time: f64,
}

impl Host {
    fn new() -> Self {
        let labels = BrushCardLabels::default();
        let numbers = LocaleNumbers::from_tag("en");
        let row = |key, name: &str, diameter| BrushPresetCards::row(key, name, 1, diameter, &labels, &numbers);
        let rows = vec![
            PresetRow::group(100, "Round", 0),
            row(1, "Soft Round", 30.0),
            row(2, "Hard Round", 20.0),
            PresetRow::group(101, "Chalk", 0),
            row(3, "Chalk Dust", 60.0),
            row(4, "Soft Chalk", 45.0),
        ];
        Self {
            open: false,
            state: BrushPickerState::new(),
            rows,
            current: None,
            diameter: 30.0,
            hardness: Some(0.8),
            import: false,
            chip: egui::Rect::NOTHING,
            popover: egui::Id::NULL,
            picks: Vec::new(),
            closed: Vec::new(),
            hardness_events: Vec::new(),
            settings: Vec::new(),
            time: 0.0,
        }
    }
}

fn app(ui: &mut Ui, host: &mut Host) {
    let chip = ui.button("Chip");
    host.chip = chip.rect;
    host.popover = Popover::id_in(ui, (&egui::IdSalt::new("picker"), "popover"));
    if chip.clicked() {
        host.open = !host.open;
    }
    let out = BrushPresetPicker::new("picker", PopoverAnchor::Control(chip.clone()), &mut host.state, &host.rows)
        .current(host.current.as_ref())
        .values(BrushPickerValues { diameter: host.diameter, hardness: host.hardness })
        .toggle(chip.id)
        .import(host.import)
        .show(ui, &mut host.open);
    if let Some(value) = out.hardness.as_ref().and_then(|e| e.value()) {
        host.hardness = Some(*value);
    }
    host.hardness_events.extend(out.hardness);
    host.picks.extend(out.picks);
    host.closed.extend(out.closed_by);
    host.settings.extend(out.settings);
}

fn harness(host: Host) -> Harness<'static, Host> {
    harness_in(host, Direction::Ltr)
}

fn harness_in(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut h = Harness::<Host>::builder()
        .with_size([480.0, 640.0])
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(app, host);
    h.run();
    h
}

fn opened(host: Host) -> Harness<'static, Host> {
    opened_in(host, Direction::Ltr)
}

fn opened_in(host: Host, dir: Direction) -> Harness<'static, Host> {
    let mut h = harness_in(host, dir);
    h.state_mut().open = true;
    h.run_steps(4);
    h
}

fn frame(h: &mut Harness<'_, Host>, gap: f64, events: Vec<Event>) {
    h.state_mut().time += gap;
    let time = h.state().time;
    h.input_mut().time = Some(time);
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn click(h: &mut Harness<'_, Host>, pos: Pos2) {
    frame(h, 1.0, vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    frame(h, 0.016, vec![]);
    frame(h, 0.016, vec![]);
}

fn double_click(h: &mut Harness<'_, Host>, pos: Pos2) {
    click(h, pos);
    frame(h, 0.1, vec![button(pos, true), button(pos, false)]);
    frame(h, 0.016, vec![]);
}

fn key(h: &mut Harness<'_, Host>, key: Key) {
    let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    frame(h, 0.5, vec![event(true)]);
    frame(h, 0.016, vec![event(false)]);
    frame(h, 0.016, vec![]);
}

fn type_text(h: &mut Harness<'_, Host>, text: &str) {
    frame(h, 0.5, vec![Event::Text(text.to_owned())]);
    frame(h, 0.016, vec![]);
}

fn node<'h>(h: &'h Harness<'_, Host>, role: Role, label: &'h str) -> Node<'h> {
    h.get_by_role_and_label(role, label)
}

fn centre(h: &Harness<'_, Host>, role: Role, label: &str) -> Pos2 {
    node(h, role, label).rect().center()
}

fn card(h: &Harness<'_, Host>, label: &str) -> Pos2 {
    let label = label.to_owned();
    h.get_by(move |n| matches!(n.role(), Role::TreeItem | Role::ListBoxOption) && n.label().as_deref() == Some(label.as_str())).rect().center()
}

fn has_card(h: &Harness<'_, Host>, label: &str) -> bool {
    h.query_all_by_label(label).any(|n| matches!(n.accesskit_node().role(), Role::TreeItem | Role::ListBoxOption))
}

fn press_button(h: &mut Harness<'_, Host>, label: &str) {
    let at = centre(h, Role::Button, label);
    click(h, at);
}

fn gear_row(h: &mut Harness<'_, Host>, role: Role, label: &str) {
    press_button(h, "Brush Preset Options");
    let at = centre(h, role, label);
    click(h, at);
    h.run_steps(2);
}

// A click on a card reports `List(Select)` and keeps the picker open; a double-click reports
// `List(Activate)` and closes it with `CloseReason::Content`.
#[test]
fn a_click_picks_and_a_double_click_picks_and_closes() {
    let mut h = opened(Host::new());
    let at = card(&h, "Soft Round, 30 px");
    click(&mut h, at);
    assert!(matches!(h.state().picks[..], [BrushPick::List(PresetEvent::Select { key: 1, .. })]), "{:?}", h.state().picks);
    assert!(h.state().open && h.state().closed.is_empty());

    h.state_mut().picks.clear();
    let at = card(&h, "Hard Round, 20 px");
    double_click(&mut h, at);
    assert!(h.state().picks.iter().any(|p| matches!(p, BrushPick::List(PresetEvent::Activate { key: 2, .. }))), "{:?}", h.state().picks);
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Content]);
}

// Esc closes; Enter closes unless a card rename is open (then it commits the rename only).
#[test]
fn escape_closes_and_enter_closes_unless_renaming() {
    let mut h = opened(Host { current: Some(1), ..Host::new() });
    gear_row(&mut h, Role::MenuItem, "Rename Brush…");
    assert!(h.query_by_role_and_label(Role::TextInput, "Name").is_some(), "the rename bar is open");
    type_text(&mut h, "Renamed");
    key(&mut h, Key::Enter);
    assert!(h.state().open && h.state().closed.is_empty(), "Enter ended the rename only: {:?}", h.state().closed);
    assert!(
        h.state().picks.iter().any(|p| matches!(p, BrushPick::List(PresetEvent::Rename { key: 1, text }) if text == "Renamed")),
        "{:?}",
        h.state().picks
    );

    gear_row(&mut h, Role::MenuItem, "Rename Brush…");
    key(&mut h, Key::Escape);
    assert!(h.state().open, "Esc cancelled the rename only");
    assert!(h.query_by_role_and_label(Role::TextInput, "Name").is_none());

    key(&mut h, Key::Escape);
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Escape]);

    h.state_mut().open = true;
    h.run_steps(4);
    key(&mut h, Key::Enter);
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Escape, CloseReason::Enter]);
}

// A press on the `toggle` control does not close the picker as outside (the control's own click
// toggles it); a press elsewhere closes it; a press in the open gear menu does not.
#[test]
fn a_press_on_the_toggle_control_does_not_count_as_outside() {
    let mut h = harness(Host::new());
    let chip = h.state().chip.center();
    click(&mut h, chip);
    assert!(h.state().open, "the chip opens the picker");
    click(&mut h, chip);
    assert!(!h.state().open);
    assert!(h.state().closed.is_empty(), "the chip toggled it: {:?}", h.state().closed);

    click(&mut h, chip);
    assert!(h.state().open);
    press_button(&mut h, "Brush Preset Options");
    let at = centre(&h, Role::MenuItemCheckBox, "Brush Stroke");
    click(&mut h, at);
    assert!(h.state().open && h.state().closed.is_empty(), "a press in the gear menu is inside");

    click(&mut h, pos2(470.0, 630.0));
    assert!(!h.state().open);
    assert_eq!(h.state().closed, [CloseReason::Outside]);
    assert!(!any_open(&h.ctx));
}

// `hardness: None` shows no Hardness slider; `Some` shows it in percent and reports 0..=1.
#[test]
fn hardness_hides_for_a_sampled_tip() {
    let mut sampled = opened(Host { hardness: None, ..Host::new() });
    assert!(sampled.query_by_role_and_label(Role::Slider, "Hardness").is_none());
    assert!(sampled.query_by_role_and_label(Role::Slider, "Size").is_some());
    sampled.run();

    let mut h = opened(Host::new());
    h.get_by_role_and_label(Role::Slider, "Hardness").focus();
    h.run();
    key(&mut h, Key::ArrowRight);
    assert_eq!(h.state().hardness_events.len(), 1, "one arrow press is one hardness event: {:?}", h.state().hardness_events);
    assert!(h.state().hardness_events.iter().all(|e| e.value().is_some_and(|v| (0.0..=1.0).contains(v))), "{:?}", h.state().hardness_events);
    let value = h.state().hardness.expect("the host keeps a hardness");
    assert!((value - 0.81).abs() < 1e-9, "80 % plus one step is 81 %: {value}");
}

// Typing "SOFT" shows the presets whose names contain "soft" and opens their groups.
#[test]
fn search_filters_in_any_case_and_opens_groups() {
    let mut host = Host::new();
    host.state.cards.browser.toggled.insert(101);
    let mut h = opened(host);
    assert!(!has_card(&h, "Soft Chalk, 45 px"), "the Chalk group starts closed");
    let field = centre(&h, Role::SearchInput, "Search Brushes");
    click(&mut h, field);
    type_text(&mut h, "SOFT");
    assert_eq!(h.state().state.cards.browser.filter, "SOFT");
    assert!(has_card(&h, "Soft Round, 30 px") && has_card(&h, "Soft Chalk, 45 px"), "the matching presets show, their groups open");
    assert!(!has_card(&h, "Hard Round, 20 px") && !has_card(&h, "Chalk Dust, 60 px"));
}

// A size dragged with the grip is the size the next open starts at.
#[test]
fn picker_size_is_remembered_between_opens() {
    let mut h = opened(Host::new());
    let dialog = |h: &Harness<'_, Host>| h.ctx.memory(|m| m.area_rect(h.state().popover)).unwrap_or(egui::Rect::NOTHING);
    let before = dialog(&h);
    let at = before.right_bottom() - vec2(8.0, 8.0);
    frame(&mut h, 1.0, vec![Event::PointerMoved(at), button(at, true)]);
    frame(&mut h, 0.016, vec![Event::PointerMoved(at + vec2(30.0, 20.0))]);
    frame(&mut h, 0.016, vec![Event::PointerMoved(at + vec2(30.0, 20.0)), button(at + vec2(30.0, 20.0), false)]);
    frame(&mut h, 0.016, vec![]);
    let grown = dialog(&h);
    assert!((grown.size() - before.size() - vec2(30.0, 20.0)).length() < 2.0, "{before:?} -> {grown:?}");

    h.state_mut().open = false;
    h.run_steps(2);
    h.state_mut().open = true;
    h.run_steps(4);
    assert!((dialog(&h).size() - grown.size()).length() < 1.0, "{:?} vs {grown:?}", dialog(&h));
}

// Gear: Rename opens the current card's rename field and Delete reports `List(Delete(current))`;
// both are disabled without a current preset; Import shows only with `import(true)`.
#[test]
fn gear_rename_and_delete_act_on_the_current_preset() {
    let mut none = opened(Host::new());
    press_button(&mut none, "Brush Preset Options");
    assert!(node(&none, Role::MenuItem, "Rename Brush…").accesskit_node().is_disabled());
    assert!(node(&none, Role::MenuItem, "Delete Brush").accesskit_node().is_disabled());
    assert!(none.query_by_role_and_label(Role::MenuItem, "Import Brushes…").is_none());

    let mut h = opened(Host { current: Some(2), import: true, ..Host::new() });
    press_button(&mut h, "Brush Preset Options");
    assert!(!node(&h, Role::MenuItem, "Delete Brush").accesskit_node().is_disabled());
    assert!(h.query_by_role_and_label(Role::MenuItem, "Import Brushes…").is_some());
    let delete = centre(&h, Role::MenuItem, "Delete Brush");
    click(&mut h, delete);
    assert!(matches!(h.state().picks[..], [BrushPick::List(PresetEvent::Delete(2))]), "{:?}", h.state().picks);

    gear_row(&mut h, Role::MenuItem, "Rename Brush…");
    assert!(h.query_by_role_and_label(Role::TextInput, "Name").is_some(), "the rename bar opens on the current card");
    assert!(
        h.get_by_role_and_label(Role::TextInput, "Name").accesskit_node().is_focused(),
        "the rename field has the focus the closing menu did not take back"
    );
}

// The head, the search row and the card parts mirror: the buttons follow the search field at the
// trailing end, the size field sits at the trailing end of its row.
#[test]
fn the_picker_head_and_search_row_mirror_in_right_to_left() {
    let ltr = opened_in(Host::new(), Direction::Ltr);
    let rtl = opened_in(Host::new(), Direction::Rtl);
    let dialog = |h: &Harness<'_, Host>| h.ctx.memory(|m| m.area_rect(h.state().popover)).unwrap_or(egui::Rect::NOTHING);
    let side = |h: &Harness<'_, Host>, role, label| centre(h, role, label).x - dialog(h).center().x;
    assert!(side(&ltr, Role::SpinButton, "Size") > 0.0, "left to right: the size field ends the row");
    assert!(side(&rtl, Role::SpinButton, "Size") < 0.0, "right to left: the size field ends the row on the left");
    let apart = |h: &Harness<'_, Host>| centre(h, Role::Button, "Brush Preset Options").x - centre(h, Role::Button, "Brush Settings…").x;
    assert!(apart(&ltr) > 0.0 && apart(&rtl) < 0.0, "the gear follows Brush Settings: {} / {}", apart(&ltr), apart(&rtl));
}

// Gear part boxes: clearing the last checked box leaves it on; a change reports `settings`.
#[test]
fn gear_part_boxes_keep_one_on_and_report_settings() {
    let mut host = Host::new();
    host.state.cards.parts = BrushCardParts { name: false, stroke: false, tip: true };
    let mut h = opened(host);
    press_button(&mut h, "Brush Preset Options");
    let tip = centre(&h, Role::MenuItemCheckBox, "Brush Tip");
    click(&mut h, tip);
    assert!(h.state().state.cards.parts.tip, "the last box stays on");
    assert!(h.state().settings.is_empty(), "nothing changed, nothing to save");

    let name = centre(&h, Role::MenuItemCheckBox, "Brush Name");
    click(&mut h, name);
    assert_eq!(h.state().state.cards.parts, BrushCardParts { name: true, stroke: false, tip: true });
    assert_eq!(h.state().settings, [BrushPickerSettings { parts: BrushCardParts { name: true, stroke: false, tip: true }, scale: 1.0 }]);
    assert!(h.state().open, "the menu and the picker stay open");
}

// Brush Settings, New Brush Preset and Import report their picks and close the picker.
#[test]
fn picks_that_open_something_else_close_the_picker() {
    for (label, expected) in [("Brush Settings…", BrushPick::OpenSettings), ("New Brush Preset", BrushPick::NewPreset)] {
        let mut h = opened(Host::new());
        press_button(&mut h, label);
        assert_eq!(h.state().picks, [expected], "{label}");
        assert!(!h.state().open, "{label} closes the picker");
        assert_eq!(h.state().closed, [CloseReason::Content]);
    }
    let mut h = opened(Host { import: true, ..Host::new() });
    gear_row(&mut h, Role::MenuItem, "Import Brushes…");
    assert_eq!(h.state().picks, [BrushPick::Import]);
    assert!(!h.state().open);
}
