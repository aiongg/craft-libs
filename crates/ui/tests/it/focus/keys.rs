//! The key stage: what egui never sees, what components receive, claims and menu-bar keys.

use super::{control, event_is_key, focus, focused, id, stop_with, tap};
use crate::contract::kit::PlainStyle;
use craft_behavior::{AccessRequest, EditCommand, Input, Intent, KeyRole, NavMove, PartKey};
use craft_keymap::{Chord, Mods};
use craft_ui::contract::{Common, Component, Cx, Focus as _, MeasureCx, MenuKey, Props as _, Role, Show as _};
use craft_ui::runtime::focus::{EditSet, KeyUse};
use craft_ui::style::StyleCx;
use craft_ui::testing::focus::Stop;
use craft_ui::testing::focus::{claim, intents_of, register_stop, take_claimed, widget};
use craft_ui::testing::input::{key_press, key_repeat};
use craft_ui::testing::{Frames, trial};
use egui::{Event, Key, Modifiers};
use std::cell::{Cell, RefCell};

/// Spec test row "the first arrow and Escape after a focus change act as intents, never as egui focus moves".
#[trial]
fn the_first_arrow_and_escape_after_a_focus_change_act_as_intents() {
    let mut frames = Frames::new();
    let moves = RefCell::new(Vec::new());
    let cancels = Cell::new(0);
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        control(ui, "b");
        let mut intents = intents_of(ui.ctx(), Some(id("a")), false);
        moves.borrow_mut().extend(intents.take_move().map(|(mv, _)| mv));
        cancels.set(cancels.get() + usize::from(intents.take(Intent::Cancel)));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "a", &mut show);
    // The arrow arrives in the pass right after the focus change, where egui's own focus would move to `b`.
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    assert_eq!(*moves.borrow(), [NavMove::Down]);
    assert_eq!(focused(&frames.ctx), Some(id("a")), "the arrow moved no egui focus");
    frames.frame(vec![craft_ui::testing::input::key_press(Key::Escape, Modifiers::NONE)], &mut show);
    assert_eq!(cancels.get(), 1, "Escape reaches the component as one Cancel");
    assert_eq!(focused(&frames.ctx), Some(id("a")), "Escape cleared no egui focus");
}

/// The key stage removes Tab, F6 and Escape from the raw input of the pass, and keeps arrows and Enter for
/// egui's `TextEdit`.
#[trial]
fn the_key_stage_strips_tab_f6_and_escape_and_keeps_arrows() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        let kept =
            [Key::Tab, Key::F6, Key::Escape, Key::ArrowLeft, Key::Enter].map(|key| ui.input(|i| i.events.iter().any(|e| event_is_key(e, key))));
        seen.borrow_mut().push(kept);
    };
    frames.idle(2, &mut show);
    let keys = [Key::Tab, Key::F6, Key::Escape, Key::ArrowLeft, Key::Enter].map(|key| craft_ui::testing::input::key_press(key, Modifiers::NONE));
    frames.frame(keys.to_vec(), &mut show);
    assert_eq!(*seen.borrow().last().unwrap(), [false, false, false, true, true]);
}

/// A component that keeps Tab (a code editor) receives it through egui; every other component loses it to
/// the focus service.
#[trial]
fn a_component_that_keeps_tab_keeps_it() {
    let mut frames = Frames::new();
    let kept = Cell::new(false);
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "code", KeyUse::CODE);
        kept.set(ui.input(|i| i.events.iter().any(|e| event_is_key(e, Key::Tab))));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "code", &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::Tab, Modifiers::NONE)], &mut show);
    assert!(kept.get());
    assert_eq!(focused(&frames.ctx), Some(id("code")), "Tab moved no focus");
}

/// Static claims hold in the frame after focus arrives; the chord is withheld from egui and read once through
/// `take_claimed`, and a claimed Escape is no `Cancel`.
#[trial]
fn claimed_chords_hold_after_focus_arrives_and_a_claimed_escape_is_no_cancel() {
    const CLAIMS: &[Chord] = &[Chord::plain(craft_keymap::Key::Up), Chord::plain(craft_keymap::Key::Escape)];
    let mut frames = Frames::new();
    let reads = RefCell::new((Vec::new(), 0, true));
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "canvas", KeyUse::CANVAS.with_claims(CLAIMS));
        let ctx = ui.ctx();
        let mut reads = reads.borrow_mut();
        reads.0.push((take_claimed(ctx, id("canvas"), CLAIMS[0]), take_claimed(ctx, id("canvas"), CLAIMS[1])));
        reads.1 += usize::from(intents_of(ctx, Some(id("canvas")), false).take(Intent::Cancel));
        reads.2 &= !ui.input(|i| i.events.iter().any(|e| event_is_key(e, Key::ArrowUp)));
    };
    frames.idle(2, &mut show);
    // The frame focus arrives in has no claims yet; the claim is read from the next frame on.
    request_and_press(&mut frames, &mut show);
    assert_eq!(reads.borrow().0.last(), Some(&(false, false)));
    reads.borrow_mut().2 = true;
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowUp, Modifiers::NONE)], &mut show);
    assert_eq!(reads.borrow().0.last(), Some(&(true, false)));
    assert!(reads.borrow().2, "the claimed Up never reached egui");
    frames.frame(vec![craft_ui::testing::input::key_press(Key::Escape, Modifiers::NONE)], &mut show);
    assert_eq!(reads.borrow().0.last(), Some(&(false, true)));
    assert_eq!(reads.borrow().1, 0, "a claimed Escape is not a Cancel");
    assert_eq!(focused(&frames.ctx), Some(id("canvas")), "the first Escape keeps focus");
    frames.idle(1, &mut show);
    assert_eq!(reads.borrow().0.last(), Some(&(false, false)), "a claim is read once");
}

fn request_and_press(frames: &mut Frames, show: &mut impl FnMut(&mut egui::Ui)) {
    craft_ui::runtime::focus::request(&frames.ctx, craft_ui::runtime::focus::FocusTarget::Id(id("canvas")));
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowUp, Modifiers::NONE)], show);
}

/// A claim added in `update` holds from the next frame.
#[trial]
fn a_state_dependent_claim_holds_from_the_next_frame() {
    let mut frames = Frames::new();
    let taken = Cell::new(false);
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "field", KeyUse::of(Input::Text));
        claim(ui.ctx(), id("field"), Chord::plain(craft_keymap::Key::Down));
        taken.set(take_claimed(ui.ctx(), id("field"), Chord::plain(craft_keymap::Key::Down)));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "field", &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    assert!(taken.get());
}

/// 11.3: a listed edit command reaches the component once as `Intent::Edit` and never reaches egui; an
/// unlisted one is read as its chord by the shortcut path alone, so egui never sees it either.
#[trial]
fn listed_edit_commands_arrive_as_intents_once() {
    let mut frames = Frames::new();
    let reads = RefCell::new((0, false));
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "field", KeyUse::of(Input::Text).with_edits(EditSet::NONE.with(EditCommand::Paste)));
        let mut reads = reads.borrow_mut();
        reads.0 += usize::from(intents_of(ui.ctx(), Some(id("field")), false).take(Intent::Edit(EditCommand::Paste)));
        reads.1 |= ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Paste(_) | Event::Copy)));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "field", &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![Event::Paste("x".into())], &mut show);
    assert_eq!(reads.borrow().0, 1);
    assert!(!reads.borrow().1, "egui never saw the paste");
    frames.frame(vec![Event::Copy], &mut show);
    assert_eq!(reads.borrow().0, 1, "Copy is not listed");
    assert!(!reads.borrow().1, "an unlisted clipboard event stands for its chord and is not left to egui");
}

/// 11.3: the raw chord of a listed edit command is removed with the clipboard events, so a field's own undo keys
/// never reach egui, and a rebound chord works the same way.
#[trial]
fn the_raw_chord_of_a_listed_edit_command_does_not_reach_egui() {
    let mut frames = Frames::new();
    let reads = RefCell::new((0, 0));
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "field", KeyUse::of(Input::Text).with_edits(EditSet::NONE.with(EditCommand::Undo)));
        let mut reads = reads.borrow_mut();
        reads.0 += usize::from(intents_of(ui.ctx(), Some(id("field")), false).take(Intent::Edit(EditCommand::Undo)));
        reads.1 += ui.input(|i| i.events.iter().filter(|e| matches!(e, Event::Key { key: egui::Key::Z, pressed: true, .. })).count());
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "field", &mut show);
    frames.idle(1, &mut show);
    *reads.borrow_mut() = (0, 0);
    frames.frame(vec![key_press(egui::Key::Z, egui::Modifiers::COMMAND)], &mut show);
    assert_eq!(*reads.borrow(), (1, 0), "the component takes the undo once and egui sees no Cmd+Z");
    *reads.borrow_mut() = (0, 0);
    frames.frame(vec![key_press(egui::Key::Z, egui::Modifiers::NONE)], &mut show);
    assert_eq!(*reads.borrow(), (0, 1), "a plain Z is no edit chord");
}

/// 9.2 rule 2: AccessKit requests go to the owner of their node whatever the focus; `Click` is `Activate`,
/// `Increment` is `Forward`, `Focus` focuses a focusable node and other actions arrive as `Intent::Access`.
#[trial]
fn accesskit_requests_reach_the_owner_of_their_node() {
    use craft_ui::testing::focus::{AccessActionKind as Action, access_request, is_focused, register_node};
    let mut frames = Frames::new();
    let other = PartKey::new("other");
    let log = RefCell::new((false, None, None));
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "focused");
        control(ui, "target");
        register_node(ui.ctx(), id("target"), id("target"), None, true);
        register_node(ui.ctx(), id("target-sub"), id("target"), Some(other.into()), false);
        let mut intents = intents_of(ui.ctx(), Some(id("target")), false);
        let mut log = log.borrow_mut();
        log.0 |= intents.take(Intent::Activate);
        log.1 = log.1.or(intents.take_move());
        log.2 = log.2.clone().or(intents.take_access());
    };
    let request = |action, node| access_request(action, id(node));
    frames.idle(2, &mut show);
    focus(&mut frames, "focused", &mut show);
    frames.frame(vec![request(Action::Click, "target")], &mut show);
    assert!(log.borrow().0, "Click arrives as Activate while another component has focus");
    assert!(is_focused(&frames.ctx, id("target")), "a focusable node takes focus with its request");
    frames.frame(vec![request(Action::Increment, "target")], &mut show);
    assert_eq!(log.borrow().1.map(|(mv, _)| mv), Some(NavMove::Forward));
    frames.frame(vec![request(Action::Expand, "target-sub")], &mut show);
    let access = log.borrow().2.clone();
    assert_eq!(access.as_ref().map(|r: &AccessRequest| (Action::Expand.is(r), r.target)), Some((true, Some(other.into()))));
}

/// A component that registers as a menu bar and reads the menu keys through its context.
#[derive(craft_ui::contract::Component)]
struct MenuBarProbe<'a> {
    #[common]
    common: Common,
    registers: bool,
    reads: bool,
    keys: &'a RefCell<Vec<MenuKey>>,
}

impl<'a> MenuBarProbe<'a> {
    fn new(keys: &'a RefCell<Vec<MenuKey>>, registers: bool, reads: bool) -> Self {
        MenuBarProbe { common: Common::new(), registers, reads, keys }.label("Menu bar")
    }
}

impl Component for MenuBarProbe<'_> {
    const ROLE: Role = Role::MENU_BAR;
    type Style = PlainStyle;
    type Output = egui::Response;

    fn style(_: &StyleCx<'_>, _: MenuBarProbeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> egui::Vec2 {
        egui::Vec2::new(40.0, 20.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.registers {
            cx.register_menu_bar();
        }
        if self.reads {
            self.keys.borrow_mut().extend(cx.take_menu_keys());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}

/// 9.7: F10, a lone Alt press and Alt with a letter are the menu bar's, only while a menu bar is registered.
#[trial]
fn menu_bar_keys_reach_a_registered_menu_bar_only() {
    let mut frames = Frames::new();
    let keys = RefCell::new(Vec::new());
    let bar = Cell::new(true);
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        MenuBarProbe::new(&keys, bar.get(), true).show(ui);
    };
    let alt = Modifiers { alt: true, ..Modifiers::NONE };
    frames.idle(3, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::F10, Modifiers::NONE)], &mut show);
    frames.frame(vec![Event::ModifiersChanged(alt)], &mut show);
    frames.frame(vec![Event::ModifiersChanged(Modifiers::NONE)], &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::F, alt)], &mut show);
    assert_eq!(*keys.borrow(), [MenuKey::Focus, MenuKey::Focus, MenuKey::Letter('F')]);
    // Alt with another key between press and release is no lone Alt.
    keys.borrow_mut().clear();
    frames.frame(
        vec![Event::ModifiersChanged(alt), craft_ui::testing::input::key_press(Key::A, alt), Event::ModifiersChanged(Modifiers::NONE)],
        &mut show,
    );
    assert_eq!(*keys.borrow(), [MenuKey::Letter('A')]);
    keys.borrow_mut().clear();
    bar.set(false);
    frames.idle(2, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::F10, Modifiers::NONE)], &mut show);
    assert!(keys.borrow().is_empty(), "no menu bar, no menu keys");
    tap(&mut frames, Key::A, alt, &mut show);
    assert!(keys.borrow().is_empty());
}

/// Focus-visible modality is one flag: keyboard use shows the ring, a pointer press hides it, and a
/// programmatic request leaves it as it was.
#[trial]
fn keyboard_use_shows_focus_and_a_pointer_press_hides_it() {
    use craft_ui::testing::focus::focus_visible;
    let mut frames = Frames::new();
    let visible = Cell::new(false);
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        control(ui, "b");
        visible.set(focus_visible(ui.ctx(), id("b")));
    };
    frames.idle(2, &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    tap(&mut frames, Key::Tab, Modifiers::NONE, &mut show);
    assert!(visible.get(), "Tab shows the ring");
    let at = egui::Pos2::new(700.0, 500.0);
    frames.frame(vec![Event::PointerMoved(at), craft_ui::testing::focus::primary_button(at, true)], &mut show);
    frames.idle(1, &mut show);
    focus(&mut frames, "b", &mut show);
    assert!(!visible.get(), "a pointer press hides the ring, and a request leaves modality unchanged");
}

/// 9.2: claims match through craft-keymap's layout rules, so a claimed `Shift+;` also holds the `:` egui reports.
#[trial]
fn a_claim_matches_through_the_layout_not_the_exact_chord() {
    const CLAIMS: &[Chord] = &[Chord::new(Mods::SHIFT, craft_keymap::Key::Semicolon)];
    let mut frames = Frames::new();
    let taken = Cell::new(false);
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "canvas", KeyUse::CANVAS.with_claims(CLAIMS));
        taken.set(take_claimed(ui.ctx(), id("canvas"), CLAIMS[0]));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "canvas", &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::Colon, Modifiers::SHIFT)], &mut show);
    assert!(taken.get());
}

/// 9.3: a state-dependent claim holds only while its owner is the focused component.
#[trial]
fn a_state_dependent_claim_holds_only_while_its_owner_has_focus() {
    let mut frames = Frames::new();
    let reads = RefCell::new((Vec::new(), false));
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        control(ui, "b");
        claim(ui.ctx(), id("b"), Chord::plain(craft_keymap::Key::Down));
        let mut reads = reads.borrow_mut();
        reads.0.extend(intents_of(ui.ctx(), Some(id("a")), false).take_move().map(|(mv, _)| mv));
        reads.1 = take_claimed(ui.ctx(), id("b"), Chord::plain(craft_keymap::Key::Down));
    };
    frames.idle(2, &mut show);
    focus(&mut frames, "a", &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    assert_eq!(reads.borrow().0, [NavMove::Down], "the unfocused component's claim left the key to the focused one");
    assert!(!reads.borrow().1);
    focus(&mut frames, "b", &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    assert!(reads.borrow().1, "the focused owner's claim holds");
}

/// 9.2: Ctrl+Tab is the focused component's only when it switches pages and does not keep Tab.
#[trial]
fn ctrl_tab_is_stripped_only_for_a_component_that_switches_pages() {
    let seen_by_egui = |keys: KeyUse, role: KeyRole| {
        let mut frames = Frames::new();
        let kept = Cell::new(false);
        let mut show = |ui: &mut egui::Ui| {
            widget(ui, id("x"));
            register_stop(ui.ctx(), Stop::new(id("x"), keys).role(role));
            kept.set(ui.input(|i| i.events.iter().any(|e| event_is_key(e, Key::Tab))));
        };
        frames.idle(2, &mut show);
        focus(&mut frames, "x", &mut show);
        frames.idle(1, &mut show);
        frames.frame(vec![craft_ui::testing::input::key_press(Key::Tab, Modifiers::CTRL)], &mut show);
        kept.get()
    };
    let pages = KeyRole::new().pages(true);
    assert!(seen_by_egui(KeyUse::of(Input::Pointer), KeyRole::new()), "no pages: Ctrl+Tab stays in the raw input");
    assert!(!seen_by_egui(KeyUse::of(Input::Pointer), pages), "a component with pages takes it");
    assert!(seen_by_egui(KeyUse::CODE, pages), "a component that keeps Tab keeps Ctrl+Tab too");
}

/// 9.7: the menu keys of a pass are that pass's: unread keys do not wait for a later pass.
#[trial]
fn unread_menu_keys_do_not_pile_up_across_passes() {
    let mut frames = Frames::new();
    let read = Cell::new(false);
    let keys = RefCell::new(Vec::new());
    let mut show = |ui: &mut egui::Ui| {
        control(ui, "a");
        MenuBarProbe::new(&keys, true, read.get()).show(ui);
    };
    frames.idle(3, &mut show);
    frames.frame(vec![craft_ui::testing::input::key_press(Key::F10, Modifiers::NONE)], &mut show);
    read.set(true);
    frames.frame(Vec::new(), &mut show);
    assert!(keys.borrow().is_empty());
}

/// A dropdown whose list has a search field: the field keeps the typed text, and Up, Down and Enter typed in it
/// reach the list in the same layer.
#[trial]
fn a_search_field_in_a_dropdown_hands_its_list_keys_to_the_list() {
    use craft_ui::contract::{Bind, Entry};
    use craft_ui::pickers::Select;
    use craft_ui::testing::Fixture;
    let value = std::rc::Rc::new(Cell::new(0u8));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let entries = [Entry::new(0u8, "Normal"), Entry::new(1, "Burn"), Entry::new(2, "Blur"), Entry::new(3, "Screen")];
        let mut mode = shown.get();
        Select::new("Blend mode", Bind::bound(&mut mode), &entries).searchable().id_salt("mode").show(ui);
        shown.set(mode);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    let field = fixture.by_role(Role::TEXT_INPUT).pop().expect("the search field shows");
    fixture.click(&field);
    fixture.idle(2);
    fixture.type_text("b");
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    let typed = fixture.focused().and_then(|id| fixture.tree().value(id).map(str::to_owned));
    assert_eq!(typed.as_deref(), Some("b"), "the arrows left the typed text in the field");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(10);
    assert_eq!(value.get(), 2, "the second match is picked");
}

/// A field's Enter that raises a modal alert, held down, leaves the alert open: its repeats began before the layer
/// opened. A fresh Enter answers it.
#[trial]
fn a_held_enter_that_opened_a_modal_does_not_answer_it() {
    use craft_behavior::Input;
    use craft_ui::contract::Bind;
    use craft_ui::dialog::Alert;
    use craft_ui::testing::focus::key_release;
    let mut frames = Frames::new();
    let open = Cell::new(false);
    let mut show = |ui: &mut egui::Ui| {
        stop_with(ui, "field", KeyUse::of(Input::Text));
        if intents_of(ui.ctx(), Some(id("field")), false).take(Intent::Submit) {
            open.set(true);
        }
        let mut now = open.get();
        Alert::new("Error", "The name is taken.").opened(Bind::bound(&mut now)).show(ui);
        open.set(now);
    };
    frames.idle(2, &mut show);
    open.set(false);
    frames.idle(2, &mut show);
    focus(&mut frames, "field", &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    let repeat = || key_repeat(Key::Enter, Modifiers::NONE);
    for _ in 0..4 {
        frames.idle(3, &mut show);
        frames.frame(vec![repeat()], &mut show);
    }
    frames.idle(3, &mut show);
    assert!(open.get(), "the repeating Enter leaves the alert open");
    frames.frame(vec![key_release(Key::Enter, Modifiers::NONE)], &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.idle(3, &mut show);
    assert!(!open.get(), "a fresh Enter answers the alert");
}

/// A search field over a list it drives, inline or in a dialog, and the keys of the rows the list activated.
fn drive_fixture(in_modal: bool) -> (craft_ui::testing::Fixture, std::rc::Rc<RefCell<Vec<usize>>>) {
    use craft_behavior::{DisplayOrder, ItemSource};
    use craft_ui::contract::{Bind, CollectionEvent, Props as _, SelectMode, ShowCollection as _, ShowContainer as _};
    use craft_ui::dialog::Dialog;
    use craft_ui::list::{List, ListRow};
    use craft_ui::runtime::env::Label;
    use craft_ui::sections::{DialogRole, FooterButton};
    use craft_ui::testing::Fixture;
    use craft_ui::text::TextField;

    const NAMES: [&str; 4] = ["Normal", "Burn", "Blur", "Screen"];
    struct Names;
    impl DisplayOrder<usize> for Names {
        fn len(&self) -> usize {
            NAMES.len()
        }
        fn key(&self, at: usize) -> usize {
            at
        }
    }
    impl ItemSource<usize> for Names {}

    let activated = std::rc::Rc::new(RefCell::new(Vec::new()));
    let sink = activated.clone();
    let query = std::rc::Rc::new(RefCell::new(String::new()));
    let fixture = Fixture::new(move |ui| {
        let list_id = ui.scope_id().with("names");
        let body = |ui: &mut egui::Ui| {
            let mut text = query.borrow().clone();
            TextField::search("Search", Bind::bound(&mut text)).drives(list_id).show(ui);
            let out = List::new("Names").id(list_id).select(SelectMode::Single).filter(&text, |key: &usize| Label::from(NAMES[*key])).show_items(
                ui,
                &Names,
                &mut |key, _| ListRow::new(NAMES[*key]),
            );
            sink.borrow_mut().extend(out.events.iter().filter_map(|event| match event {
                CollectionEvent::Activate(key) => Some(*key),
                _ => None,
            }));
            *query.borrow_mut() = text;
        };
        if in_modal {
            let mut open = true;
            Dialog::new("Pick").opened(Bind::bound(&mut open)).buttons([FooterButton::new("Cancel", DialogRole::Cancel)]).show(ui, |ui, _| body(ui));
        } else {
            body(ui);
        }
    });
    (fixture, activated)
}

/// A text field that drives a list hands it Down and Enter inline and in a modal layer, and keeps the typed text.
#[trial]
fn a_field_that_drives_a_list_hands_it_its_list_keys_inline_and_in_a_modal() {
    for in_modal in [false, true] {
        let (mut fixture, activated) = drive_fixture(in_modal);
        fixture.settle();
        let field = fixture.by_role(Role::TEXT_INPUT).pop().expect("the search field shows");
        fixture.click(&field);
        fixture.idle(2);
        fixture.type_text("b");
        fixture.idle(2);
        for _ in 0..2 {
            fixture.key(Key::ArrowDown, Modifiers::NONE);
            fixture.idle(2);
        }
        let typed = fixture.focused().and_then(|id| fixture.tree().value(id).map(str::to_owned));
        assert_eq!(typed.as_deref(), Some("b"), "the arrows left the typed text in the field (modal: {in_modal})");
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.idle(4);
        assert_eq!(*activated.borrow(), [2], "Enter picks the second match (modal: {in_modal})");
    }
}
