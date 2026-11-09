//! The Blend If bar, the contour field and the Contour Editor.

use craft_imaging::layer_style::Contour;
use craft_imaging::ui::layer_style::{ContourAction, ContourEditorDialog, ContourField, ContourKey, ContourRequest, ContourWords, contour_rows};
use craft_ui::contract::{Bind, Role};
use craft_ui::dialog::DialogHost;
use craft_ui::patterns::presets::PresetState;
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

/// Picking a built-in contour from the picker edits the contour to it.
#[trial]
fn picking_a_preset_edits_the_contour() {
    let held = Rc::new(RefCell::new(Contour::linear()));
    let shared = Rc::clone(&held);
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<ContourWords>();
        let rows = contour_rows(&Contour::presets(), &[], &words, None);
        let mut state = PresetState::<ContourKey>::new();
        let mut contour = shared.borrow_mut();
        let _ = ContourField::new("Contour", Bind::bound(&mut contour), &rows, Bind::bound(&mut state)).show(ui);
    });
    fixture.settle();
    let field = fixture.by_label("Contour").expect("the contour field");
    fixture.click(&field);
    fixture.idle(3);
    let cone = fixture.by_label("Cone").expect("the Cone contour");
    fixture.click(&cone);
    fixture.idle(3);
    assert_eq!(held.borrow().name, "Cone");
}

/// New asks for a name that starts as the contour word, and a named New is reported as an action.
#[trial]
fn new_in_the_contour_editor_suggests_the_contour_word() {
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().open(ContourEditorDialog::new(ContourRequest::new("Contour", Contour::linear())));
    let shared = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| shared.borrow_mut().run(ui));
    fixture.settle();
    let new = fixture.by_label("New…").expect("the New button");
    fixture.click(&new);
    fixture.idle(3);
    let field = fixture.by_role(Role::TEXT_INPUT).into_iter().next().expect("the name field");
    assert!(
        fixture.texts().iter().any(|text| text.text == "Contour" && field.rect.contains_rect(text.rect)),
        "the name field shows the contour word"
    );
    let named = host.borrow_mut().get_mut::<ContourEditorDialog>().and_then(ContourEditorDialog::take_action);
    assert_eq!(named, None::<ContourAction>);
}
