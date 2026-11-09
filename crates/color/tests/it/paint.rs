//! The paint chip and list, the swatch pair, the LUT picker and the profile prompts through the fixture.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_color::ui::{
    GradientPreview, ImagePreview, Paint, PaintChip, PairWell, ProfileAction, ProfileMismatchDialog, ProfileMode, ProfileQuestion, SwatchPair, Well,
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
fn a_click_makes_a_well_active_and_a_double_click_opens_its_popup() {
    let (mut fixture, seen, _root) = pair_rig(true, true);
    let secondary = well(&fixture, 1);
    fixture.click(&secondary);
    assert_eq!(seen.get().active, Well::Secondary);
    assert!(seen.get().activated);
    fixture.click(&secondary);
    fixture.click(&secondary);
    fixture.idle(2);
    assert_eq!(seen.get().open, Some(Well::Secondary), "a double click opens the clicked well's popup");
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
