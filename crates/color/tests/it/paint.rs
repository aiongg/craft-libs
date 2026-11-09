//! The paint chip and list, the swatch pair, the LUT picker and the profile prompts through the fixture.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_behavior::PartKey;
use craft_color::ui::{
    FillKind, GradientPreview, ImagePreview, LutChoice, LutPicker, Paint, PaintChip, PaintLink, PaintLinkKind, PaintList, PaintListEvent,
    PaintListOptions, PaintRow, PairWell, ProfileAction, ProfileMismatchDialog, ProfileMode, ProfileQuestion, ProfileStatus, ProfileWords,
    SwatchPair, Well,
};
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};

const FALLBACK: Srgba = Srgba::rgb(0.2, 0.4, 0.8);

fn chip_rig(paint: Paint, gradient: bool, images: bool) -> (Fixture, Rc<RefCell<Paint>>, Rc<Cell<Option<Id>>>) {
    let state = Rc::new(RefCell::new(paint));
    let (shared, root) = (state.clone(), Rc::new(Cell::new(None)));
    let id = root.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut paint = shared.borrow().clone();
        let mut chip = PaintChip::new("Fill", Bind::bound(&mut paint), FALLBACK).none_allowed();
        if gradient {
            chip = chip.gradient_editor(GradientPreview::new(vec![(0.0, FALLBACK), (1.0, Srgba::rgb(1.0, 1.0, 1.0))]), |_: &mut egui::Ui| None);
        }
        if images {
            let tile = ImagePreview::new("Bubbles", None);
            chip = chip
                .pattern_editor(tile.clone(), |_: &mut egui::Ui| None)
                .picture_editor(ImagePreview::new("photo.png", None), |_: &mut egui::Ui| None);
        }
        id.set(Some(chip.show(ui).edit.response.id));
        *shared.borrow_mut() = paint;
    });
    fixture.settle();
    (fixture, state, root)
}

fn open(fixture: &mut Fixture) {
    let chip = fixture.by_label("Fill").expect("the chip is shown");
    fixture.click(&chip);
    fixture.idle(2);
}

fn pick_tab(fixture: &mut Fixture, label: &str) {
    let tab = fixture.by_label(label).unwrap_or_else(|| panic!("the {label} tab is shown"));
    fixture.click(&tab);
    fixture.idle(2);
}

#[trial]
fn switching_tabs_sets_the_paint_and_back_to_solid_restores_the_last_colour() {
    let solid = Srgba::rgb(0.9, 0.1, 0.1);
    let (mut fixture, state, _) = chip_rig(Paint::Solid(solid), true, false);
    open(&mut fixture);
    pick_tab(&mut fixture, "No Color");
    assert_eq!(*state.borrow(), Paint::None);
    pick_tab(&mut fixture, "Gradient");
    assert!(matches!(&*state.borrow(), Paint::Gradient(preview) if preview.stops.len() == 2));
    pick_tab(&mut fixture, "Solid");
    assert_eq!(*state.borrow(), Paint::Solid(solid), "Solid starts from the last solid colour");
}

#[trial]
fn a_chip_that_never_held_a_solid_colour_switches_to_the_fallback() {
    let (mut fixture, state, _) = chip_rig(Paint::None, false, false);
    open(&mut fixture);
    pick_tab(&mut fixture, "Solid");
    assert_eq!(*state.borrow(), Paint::Solid(FALLBACK), "a chip that never held a solid colour takes the fallback");
}

#[trial]
fn the_gradient_pattern_and_picture_tabs_show_only_when_their_editor_is_given() {
    let (mut fixture, _, _) = chip_rig(Paint::Solid(FALLBACK), false, false);
    open(&mut fixture);
    for label in ["No Color", "Solid"] {
        assert!(fixture.by_label(label).is_some(), "{label} is a tab");
    }
    for label in ["Gradient", "Pattern", "Picture"] {
        assert!(fixture.by_label(label).is_none(), "{label} has no editor");
    }
    let (mut fixture, _, _) = chip_rig(Paint::Solid(FALLBACK), true, true);
    open(&mut fixture);
    for label in ["Gradient", "Pattern", "Picture"] {
        assert!(fixture.by_label(label).is_some(), "{label} is offered");
    }
    assert_eq!(FillKind::of(&Paint::Picture(ImagePreview::default())), FillKind::Picture);
}

#[trial]
fn the_chip_names_its_paint_to_a_screen_reader() {
    let value = |paint: Paint| {
        let (fixture, _, root) = chip_rig(paint, true, false);
        fixture.tree().value(root.get().expect("shown")).map(str::to_owned)
    };
    let values = [
        value(Paint::None),
        value(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0))),
        value(Paint::Gradient(GradientPreview::default())),
        value(Paint::Pattern(ImagePreview::new("Bubbles", None))),
    ];
    assert!(values.iter().all(Option::is_some), "every paint kind has a value");
    for (i, a) in values.iter().enumerate() {
        assert!(values[i + 1..].iter().all(|b| a != b), "the value differs per paint kind: {values:?}");
    }
}

fn list_rig(options: PaintListOptions) -> (Fixture, Rc<RefCell<Vec<PaintListEvent>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shared = events.clone();
    let paints = vec![("Brand".to_owned(), Paint::Solid(FALLBACK)), ("Sky".to_owned(), Paint::Solid(Srgba::rgb(0.5, 0.7, 1.0)))];
    let rows = vec![PaintRow::linked(PaintLink::new("Primary", PaintLinkKind::Style)), PaintRow::default()];
    let selected = Rc::new(Cell::new(None::<usize>));
    let mut fixture = Fixture::new(move |ui| {
        let mut held = selected.get();
        let out = PaintList::new("Paints", &paints, Bind::bound(&mut held)).options(options).rows(&rows).show(ui);
        selected.set(out.selected.or(held));
        shared.borrow_mut().extend(out.events);
    });
    fixture.settle();
    (fixture, events)
}

#[trial]
fn a_paint_list_row_reports_its_remove_and_select_users_actions() {
    let options = PaintListOptions::default().remove(true).select_users(true);
    let (mut fixture, events) = list_rig(options);
    let remove = fixture.by_label("Remove Brand").expect("the first row has a Remove button");
    fixture.click(&remove);
    let users = fixture.by_label("Select layers with Brand").expect("the first row has a select users button");
    fixture.click(&users);
    let seen = events.borrow().clone();
    assert!(seen.contains(&PaintListEvent::Remove(0)), "{seen:?}");
    assert!(seen.contains(&PaintListEvent::SelectUsers(0)), "{seen:?}");
}

#[trial]
fn a_bound_row_detaches_and_an_unbound_row_binds() {
    let (mut fixture, events) = list_rig(PaintListOptions::default().bind(true));
    let detach = fixture.by_label("Detach style").expect("the bound row detaches");
    fixture.click(&detach);
    let bind = fixture.by_label("Color styles").expect("the unbound row binds");
    fixture.click(&bind);
    let seen = events.borrow().clone();
    assert!(seen.contains(&PaintListEvent::Detach(0)) && seen.contains(&PaintListEvent::Bind(1)), "{seen:?}");
}

#[derive(Default, Clone, Copy)]
struct PairSeen {
    clicked: Option<Well>,
    double_clicked: Option<Well>,
    active: Well,
    activated: bool,
    swap: bool,
    defaults: bool,
    open: Option<Well>,
}

fn pair_rig(popover: bool, click_activates: bool) -> (Fixture, Rc<Cell<PairSeen>>, Rc<Cell<Option<Id>>>) {
    let seen = Rc::new(Cell::new(PairSeen::default()));
    let root = Rc::new(Cell::new(None));
    let (shared, id) = (seen.clone(), root.clone());
    let active = Rc::new(Cell::new(Well::Primary));
    let mut fixture = Fixture::new(move |ui| {
        let mut held = active.get();
        let wells = [PairWell::new("Foreground colour", Srgba::rgb(0.0, 0.0, 0.0)), PairWell::new("Background colour", Srgba::rgb(1.0, 1.0, 1.0))];
        let out = SwatchPair::new("Colours", wells, Bind::bound(&mut held))
            .swap_button("Swap colours")
            .default_button("Default colours")
            .popover(popover)
            .click_activates(click_activates)
            .show(ui);
        active.set(held);
        id.set(Some(out.response.id));
        let mut now = shared.get();
        now.active = out.active;
        now.activated |= out.activated;
        now.clicked = out.clicked.or(now.clicked);
        now.double_clicked = out.double_clicked.or(now.double_clicked);
        now.swap |= out.swap;
        now.defaults |= out.defaults;
        now.open = out.open;
        shared.set(now);
    });
    fixture.settle();
    (fixture, seen, root)
}

/// The well at `at` (0 the foreground, 1 the background), found by its accessible name.
fn well(fixture: &Fixture, at: usize) -> craft_ui::testing::Target {
    let name = ["Foreground colour", "Background colour"][at];
    fixture.by_label(name).expect("the well is a node")
}

#[trial]
fn a_click_makes_a_well_active_reports_it_and_a_double_click_opens_the_popup() {
    let (mut fixture, seen, root) = pair_rig(true, true);
    let secondary = well(&fixture, 1);
    fixture.click(&secondary);
    assert_eq!(seen.get().clicked, Some(Well::Secondary), "the default mode reports clicks as well as activating");
    assert_eq!(seen.get().active, Well::Secondary);
    assert!(seen.get().activated);
    fixture.click(&secondary);
    fixture.click(&secondary);
    fixture.idle(2);
    assert_eq!(seen.get().open, Some(Well::Secondary), "a double click opens the clicked well's popup");
}

#[trial]
fn the_buttons_report_swap_and_default() {
    let (mut fixture, seen, _) = pair_rig(true, true);
    let swap = fixture.by_label("Swap colours").expect("the swap button");
    fixture.click(&swap);
    let defaults = fixture.by_label("Default colours").expect("the default button");
    fixture.click(&defaults);
    assert!(seen.get().swap && seen.get().defaults);
}

#[trial]
fn in_host_picker_mode_a_click_is_reported_and_opens_no_popup() {
    let (mut fixture, seen, root) = pair_rig(false, false);
    let secondary = well(&fixture, 1);
    fixture.click(&secondary);
    assert_eq!(seen.get().clicked, Some(Well::Secondary));
    assert_eq!(seen.get().active, Well::Primary, "the host keeps the active well");
    assert_eq!(seen.get().open, None);
}

#[trial]
fn in_host_picker_mode_enter_clicks_the_focused_well_and_the_arrows_only_move_the_focus() {
    let (mut fixture, seen, _) = pair_rig(false, false);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(seen.get().active, Well::Primary, "an arrow moves the focus and leaves the host's active well");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert!(seen.get().clicked.is_some(), "Enter clicks the focused well");
    assert_eq!(seen.get().open, None);
}

fn lut_rig(value: LutChoice) -> (Fixture, Rc<RefCell<LutChoice>>) {
    let state = Rc::new(RefCell::new(value));
    let shared = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut choice = shared.borrow().clone();
        LutPicker::new(Bind::bound(&mut choice)).show(ui);
        *shared.borrow_mut() = choice;
    });
    fixture.settle();
    (fixture, state)
}

#[trial]
fn the_lut_rows_follow_photocraft() {
    let (mut fixture, _) = lut_rig(LutChoice::loaded("Film", 33));
    let select = fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the select is shown");
    fixture.click(&select);
    fixture.idle(2);
    let names: Vec<String> =
        fixture.by_role(Role::LIST_BOX_OPTION).iter().filter_map(|t| fixture.tree().name_of(t.node).map(str::to_owned)).collect();
    assert!(names.len() >= 3, "Load, None, the built-in looks and the loaded LUT: {names:?}");
    assert_ne!(names[0], names[1], "Load and None are distinct rows");
    assert!(names.last().is_some_and(|name| name.contains("Film")), "the loaded LUT comes last: {names:?}");
}

fn profile_rig(dialog: ProfileMismatchDialog) -> (Fixture, Rc<RefCell<Option<DialogResult<craft_color::ui::ProfileAnswer>>>>) {
    let result = Rc::new(RefCell::new(None));
    let shared = result.clone();
    let dialog = Rc::new(RefCell::new(dialog));
    let mut fixture = Fixture::new(move |ui| {
        let mut stack = DialogStack::default();
        let mut dialog = dialog.borrow_mut();
        stack.push(dialog.id(), None);
        if let Some(done) = dialog.show(ui, &stack) {
            *shared.borrow_mut() = Some(done);
        }
    });
    fixture.settle();
    (fixture, result)
}

fn mismatch() -> ProfileQuestion {
    ProfileQuestion::mismatch("Display P3", "sRGB IEC61966-2.1")
}

#[trial]
fn the_profile_choices_follow_the_question() {
    let names = |fixture: &Fixture| -> Vec<String> {
        fixture.by_role(Role::RADIO_BUTTON).iter().filter_map(|t| fixture.tree().name_of(t.node).map(str::to_owned)).collect()
    };
    let (fixture, _) = profile_rig(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()));
    let mismatch_choices = names(&fixture);
    assert!(mismatch_choices.len() >= 2, "{mismatch_choices:?}");
    let missing = ProfileQuestion::missing("sRGB IEC61966-2.1");
    let (fixture, _) = profile_rig(ProfileMismatchDialog::new("photo.psd", ProfileMode::Cmyk, missing));
    let missing_choices = names(&fixture);
    assert!(!missing_choices.is_empty() && missing_choices != mismatch_choices, "a missing profile offers other choices: {missing_choices:?}");
}

#[trial]
fn remember_shows_for_a_mismatch_only_and_comes_back_in_the_answer() {
    let boxes = |fixture: &Fixture| fixture.by_role(Role::CHECK_BOX).len();
    let (fixture, _) = profile_rig(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()));
    assert_eq!(boxes(&fixture), 0, "not rememberable");
    let missing = ProfileQuestion::missing("sRGB");
    let (fixture, _) = profile_rig(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, missing).rememberable(true));
    assert_eq!(boxes(&fixture), 0, "a missing profile has no policy");

    let (mut fixture, result) = profile_rig(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()).rememberable(true));
    assert_eq!(boxes(&fixture), 1);
    for label in ["Remember this choice for future images", "Convert to the working space (sRGB IEC61966-2.1)", "OK"] {
        let target = fixture.by_label(label).unwrap_or_else(|| panic!("{label} is shown"));
        fixture.click(&target);
    }
    fixture.idle(2);
    match result.borrow().as_ref() {
        Some(DialogResult::Ok(answer)) => {
            assert_eq!(answer.action, ProfileAction::ConvertToWorking);
            assert!(answer.remember);
        }
        other => panic!("{other:?}"),
    }
}

#[trial]
fn the_profile_status_states_read_differently() {
    let words = ProfileWords::default();
    assert_eq!(ProfileStatus::of(None), ProfileStatus::Untagged);
    assert_eq!(ProfileStatus::of(Some(b"not a profile")), ProfileStatus::Invalid);
    let srgb = craft_cms::builtin::srgb().to_bytes();
    assert!(matches!(ProfileStatus::of(Some(&srgb)), ProfileStatus::Named(name) if !name.is_empty()));
    let labels = [
        ProfileStatus::Untagged.label(ProfileMode::Rgb, &words),
        ProfileStatus::Invalid.label(ProfileMode::Rgb, &words),
        ProfileStatus::Unnamed.label(ProfileMode::Rgb, &words),
        ProfileStatus::Named("Display P3".into()).label(ProfileMode::Rgb, &words),
    ];
    for (i, a) in labels.iter().enumerate() {
        assert!(labels[i + 1..].iter().all(|b| a != b), "each state reads differently: {labels:?}");
    }
    assert!(labels[3].contains("Display P3"), "a named profile shows its name");
}
