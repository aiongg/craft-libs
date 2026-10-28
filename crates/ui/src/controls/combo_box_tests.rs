use egui::accesskit::{self, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent, GestureHost, UndoMarks};
use crate::testing::a11y;
use crate::theme::{Base, install};

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

const SIZES: [&str; 8] = ["8 pt", "10 pt", "12 pt", "14 pt", "18 pt", "24 pt", "36 pt", "72 pt"];

/// A document holding the combo box's text, with an undo history of whole snapshots.
#[derive(Clone, Debug, Default)]
struct Doc {
    text: String,
    undo: Vec<String>,
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(text) = self.undo.get(mark) {
            self.text.clone_from(text);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Default)]
struct Host {
    doc: Doc,
    gestures: GestureHost<Doc>,
    events: Vec<EditEvent<String>>,
    submitted: bool,
    /// The combo box isn't drawn.
    hidden: bool,
}

/// A font-size combo box over a document behind a [`GestureHost`], then a button; Ctrl+Z cancels
/// every gesture as an undo would.
fn sizes(text: &str) -> Harness<'static, Host> {
    let host = Host { doc: Doc { text: text.to_owned(), undo: Vec::new() }, ..Host::default() };
    harness().build_ui_state(
        |ui, host: &mut Host| {
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
            }
            if !host.hidden {
                let edit = ComboBox::new("size", "Font size", &host.doc.text).options(SIZES).show(ui);
                host.submitted |= edit.submitted;
                if let Some(event) = edit.event {
                    host.events.push(event.clone());
                    host.gestures.handle(ui.ctx(), &mut host.doc, event, |doc, text| {
                        let before = std::mem::replace(&mut doc.text, text);
                        doc.undo.push(before);
                    });
                }
            }
            let _ = ui.button("Next");
            let _ = host.gestures.end_frame(ui.ctx(), &mut host.doc);
        },
        host,
    )
}

fn field<'h, State>(harness: &'h Harness<'_, State>) -> egui_kittest::Node<'h> {
    harness.get_by_role(Role::EditableComboBox)
}

fn options<State>(harness: &Harness<'_, State>) -> Vec<String> {
    harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect()
}

fn option<'h, State>(harness: &'h Harness<'_, State>, label: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(label))
}

fn expanded<State>(harness: &Harness<'_, State>) -> Option<bool> {
    field(harness).accesskit_node().data().is_expanded()
}

fn active<State>(harness: &Harness<'_, State>) -> Option<String> {
    field(harness).accesskit_node().active_descendant().and_then(|n| n.label())
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn field_rect<State>(harness: &Harness<'_, State>) -> Rect {
    field(harness).rect()
}

/// A point on the chevron, at the field's reading end.
fn chevron<State>(harness: &Harness<'_, State>) -> Pos2 {
    field_rect(harness).right_center() - egui::vec2(10.0, 0.0)
}

/// Whether egui's keyboard focus is in the field. AccessKit reports the active option focused
/// while the list has one.
fn field_focused<State>(harness: &Harness<'_, State>) -> bool {
    let node = field(harness).accesskit_node().locate().0;
    harness.ctx.memory(|m| m.focused()).is_some_and(|id| id.accesskit_id() == node)
}

/// A click at `pos`, pressed and released in one frame.
fn click_at<State>(harness: &mut Harness<'_, State>, pos: Pos2) {
    harness.hover_at(pos);
    harness.event(press(pos, true));
    harness.event(press(pos, false));
    harness.run();
}

fn focus<State>(harness: &mut Harness<'_, State>) {
    field(harness).focus();
    harness.run();
}

fn type_text<State>(harness: &mut Harness<'_, State>, text: &str) {
    harness.event(Event::Text(text.into()));
    harness.run();
}

fn key<State>(harness: &mut Harness<'_, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

#[test]
fn typing_opens_the_list_on_the_options_starting_with_the_text_then_those_containing_it() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    assert_eq!(expanded(&harness), Some(false), "focus alone opens nothing");
    type_text(&mut harness, "2");
    assert_eq!(expanded(&harness), Some(true), "typing opens the list");
    assert_eq!(options(&harness), ["24 pt", "12 pt", "72 pt"]);
    assert_eq!(active(&harness), None, "no option is active until the arrows move");
    type_text(&mut harness, "x");
    assert_eq!((expanded(&harness), options(&harness).len()), (Some(false), 0), "no match closes the list");
    assert_eq!(harness.state().doc.text, "2x", "the typed text is the value");
}

#[test]
fn choosing_replaces_the_text_in_one_undo_step_with_the_typing_and_without_a_run_sets_it() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    type_text(&mut harness, "1");
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("12 pt"));
    key(&mut harness, Key::Enter);
    harness.run();
    let host = harness.state();
    assert_eq!((host.doc.text.as_str(), host.doc.undo.len()), ("12 pt", 1), "typing and the choice are one undo step");
    assert!(matches!(host.events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if value == "12 pt"), "{:?}", host.events);
    assert!(!host.submitted, "Enter on an option chooses it and submits nothing");
    assert_eq!(expanded(&harness), Some(false));
    assert!(field_focused(&harness));

    let pos = chevron(&harness);
    click_at(&mut harness, pos);
    assert_eq!(expanded(&harness), Some(true), "a click on the chevron opens the list");
    assert_eq!(options(&harness), SIZES, "opened without typing, the list shows every option");
    assert!(option(&harness, "12 pt").accesskit_node().is_selected().unwrap_or(false), "the option of the field's text is selected");
    option(&harness, "18 pt").click();
    harness.run();
    harness.run();
    let host = harness.state();
    assert!(matches!(host.events.last(), Some(EditEvent::Set { value, .. }) if value == "18 pt"), "{:?}", host.events);
    assert_eq!((host.doc.text.as_str(), host.doc.undo.len()), ("18 pt", 2));
    assert!(field_focused(&harness), "the field keeps the focus");
    type_text(&mut harness, "!");
    assert_eq!(harness.state().doc.text, "18 pt!", "the caret waits at the end of the chosen text");
}

#[test]
fn enter_with_no_active_option_commits_the_typed_text_and_closes_the_list() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    type_text(&mut harness, "1");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("10 pt"));
    type_text(&mut harness, "2");
    assert_eq!((expanded(&harness), active(&harness)), (Some(true), None), "typing takes the active option back to the text");
    key(&mut harness, Key::Enter);
    let host = harness.state();
    assert!(host.submitted, "Enter submits as in a text field");
    assert!(matches!(host.events.last(), Some(EditEvent::Commit { value, .. }) if value == "12"), "{:?}", host.events);
    assert_eq!(expanded(&harness), Some(false));
}

#[test]
fn esc_closes_the_list_then_cancels_the_run_then_leaves() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    type_text(&mut harness, "1");
    key(&mut harness, Key::Escape);
    assert_eq!(expanded(&harness), Some(false), "the first Esc closes the list");
    assert_eq!(harness.state().doc.text, "1", "the run goes on");
    assert!(!harness.state().events.iter().any(|e| matches!(e, EditEvent::Cancel { .. })));
    assert!(field_focused(&harness));
    key(&mut harness, Key::Escape);
    harness.run();
    assert!(matches!(harness.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "the second cancels the run");
    assert_eq!(harness.state().doc.text, "");
    assert!(field_focused(&harness));
    key(&mut harness, Key::Escape);
    assert!(!field_focused(&harness), "the third leaves the field");
}

#[test]
fn tab_commits_the_typed_text_and_moves_on_without_choosing_the_active_option() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    type_text(&mut harness, "1");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("10 pt"));
    key(&mut harness, Key::Tab);
    harness.run();
    assert_eq!(harness.state().doc.text, "1", "free text wins");
    assert!(harness.get_by_label("Next").is_focused(), "the focus moved on");
    assert_eq!(expanded(&harness), Some(false));
}

#[test]
fn a_press_on_the_chevron_dragged_onto_an_option_chooses_it_on_release_and_a_click_in_the_text_opens_nothing() {
    let mut harness = sizes("12 pt");
    harness.run();
    let text = field_rect(&harness).left_center() + egui::vec2(10.0, 0.0);
    click_at(&mut harness, text);
    assert!(field_focused(&harness), "a click in the text focuses the field");
    assert_eq!(expanded(&harness), Some(false), "and opens nothing");

    let start = chevron(&harness);
    harness.hover_at(start);
    harness.event(press(start, true));
    harness.run();
    assert_eq!(expanded(&harness), Some(true), "the press opens the list");
    let target = option(&harness, "24 pt").rect().center();
    harness.hover_at(target);
    harness.run();
    assert_eq!(active(&harness).as_deref(), Some("24 pt"));
    harness.event(press(target, false));
    harness.run();
    harness.run();
    let host = harness.state();
    assert!(matches!(host.events[..], [EditEvent::Set { ref value, .. }] if value == "24 pt"), "{:?}", host.events);
    assert_eq!(expanded(&harness), Some(false));
}

#[test]
fn the_list_closes_with_its_run_and_with_its_combo_box() {
    let mut harness = sizes("");
    harness.run();
    focus(&mut harness);
    type_text(&mut harness, "1");
    assert_eq!(expanded(&harness), Some(true));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(expanded(&harness), Some(false), "an undo during the run closes its list");
    assert!(harness.query_all_by_role(Role::ListBox).next().is_none());

    let pos = chevron(&harness);
    click_at(&mut harness, pos);
    assert_eq!(expanded(&harness), Some(true));
    harness.state_mut().hidden = true;
    harness.run();
    harness.state_mut().hidden = false;
    harness.run();
    assert_eq!((expanded(&harness), options(&harness).len()), (Some(false), 0), "a combo box drawn again after a gap starts closed");
}

#[test]
fn the_field_is_an_editable_combo_box_that_names_its_list_and_active_option() {
    let mut harness = sizes("12 pt");
    harness.run();
    a11y::assert_labelled(&harness);
    let node = field(&harness).accesskit_node();
    assert_eq!((node.label().as_deref(), node.value().as_deref(), node.data().is_expanded()), (Some("Font size"), Some("12 pt"), Some(false)));
    assert_eq!(node.data().has_popup(), Some(accesskit::HasPopup::Listbox));
    assert!(node.data().controls().is_empty(), "a closed combo box controls nothing");
    focus(&mut harness);
    key(&mut harness, Key::ArrowDown);
    a11y::assert_labelled(&harness);
    assert_eq!(expanded(&harness), Some(true));
    let list = harness.get_by_role(Role::ListBox).accesskit_node().locate().0;
    assert_eq!(field(&harness).accesskit_node().data().controls(), [list], "the open combo box controls the list");
    assert_eq!(active(&harness).as_deref(), Some("12 pt"), "Down opens on the option of the field's text");
    assert_eq!(options(&harness), SIZES);
    key(&mut harness, Key::ArrowDown);
    assert_eq!(active(&harness).as_deref(), Some("14 pt"));
    assert!(field_focused(&harness), "the focus stays in the field");
}

#[test]
fn right_to_left_the_chevron_is_at_the_left_end_and_the_list_lines_up_with_the_right() {
    let mut harness = harness()
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, Direction::Rtl);
        })
        .build_ui_state(
            |ui, text: &mut String| {
                ui.with_layout(direction::column(ui), |ui| {
                    let _ = ComboBox::new("size", "Font size", text).options(SIZES).show(ui);
                });
            },
            String::new(),
        );
    harness.run();
    let rect = field_rect(&harness);
    click_at(&mut harness, rect.right_center() - egui::vec2(10.0, 0.0));
    assert_eq!(expanded(&harness), Some(false), "the reading start is text");
    click_at(&mut harness, rect.left_center() + egui::vec2(10.0, 0.0));
    assert_eq!(expanded(&harness), Some(true), "the chevron is at the reading end");
    let list = harness.ctx.memory(|m| m.layer_ids().filter(|l| l.order == egui::Order::Foreground).find_map(|l| m.area_rect(l.id)));
    let list = list.unwrap_or(Rect::NOTHING);
    assert!((list.right() - rect.right()).abs() < 1.0, "list {list:?}, field {rect:?}");
}
