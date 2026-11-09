//! The Blend If bar, the contour field and the Contour Editor.

use craft_imaging::layer_style::{BlendIfRange, Contour, ContourPoint};
use craft_imaging::ui::layer_style::{
    BlendIfBar, ContourAction, ContourEditorDialog, ContourField, ContourKey, ContourRequest, ContourWords, contour_rows, from_curve, to_curve,
};
use craft_ui::contract::{Bind, Role};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::patterns::presets::PresetState;
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn user_contour() -> Contour {
    let point = |input, output| ContourPoint { input, output, corner: false };
    Contour { name: String::from("Soft Step"), points: vec![point(0.0, 0.0), point(0.4, 0.1), point(1.0, 1.0)] }
}

/// The readout above the bar writes a split side as "low / high" and a joined one as the level.
#[trial]
fn the_blend_if_readout_writes_split_and_joined_levels() {
    let range = Rc::new(RefCell::new(BlendIfRange([0, 20, 255, 255])));
    let shared = Rc::clone(&range);
    let mut fixture = Fixture::new(move |ui| {
        let _ = BlendIfBar::new("This Layer", Bind::bound(&mut shared.borrow_mut())).show(ui);
    });
    fixture.settle();
    assert!(
        fixture.texts().iter().any(|text| text.text == "0 / 20    255"),
        "texts: {:?}",
        fixture.texts().iter().map(|t| &t.text).collect::<Vec<_>>()
    );
}

/// The picker lists the user's contours after the built-in ones.
#[trial]
fn the_picker_lists_user_contours() {
    let user = vec![user_contour()];
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<ContourWords>();
        let rows = contour_rows(&Contour::presets(), &user, &words, None);
        let mut contour = Contour::linear();
        let mut state = PresetState::<ContourKey>::new();
        let _ = ContourField::new("Contour", Bind::bound(&mut contour), &rows, Bind::bound(&mut state)).user(&user).show(ui);
    });
    fixture.settle();
    let field = fixture.by_label("Contour").expect("the contour field");
    fixture.click(&field);
    fixture.idle(3);
    assert!(fixture.by_label("Soft Step").is_some(), "the user's contour is listed");
    assert!(fixture.by_label("Cone").is_some(), "the built-in contours are listed");
}

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

/// The contour and its curve convert both ways; a drawn curve is no contour.
#[trial]
fn a_contour_converts_to_the_editor_curve_and_back() {
    let contour = user_contour();
    let curve = to_curve(&contour);
    assert_eq!(curve.points().map(<[_]>::len), Some(3));
    let back = from_curve(&curve, &contour.name).expect("a contour from the curve");
    assert_eq!(back.points.len(), 3);
    assert!((back.points[1].input - 0.4).abs() < 1e-4 && (back.points[1].output - 0.1).abs() < 1e-4);
}

/// The Contour Editor shows live values; OK answers the contour.
#[trial]
fn the_contour_editor_answers_the_contour_on_ok() {
    let answer: Rc<RefCell<Option<DialogResult<Contour>>>> = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&answer);
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().open(ContourEditorDialog::new(ContourRequest::new("Contour", Contour::linear())));
    let shared = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shared.borrow_mut();
        host.run(ui);
        if let Some(result) = host.take::<ContourEditorDialog>() {
            *sink.borrow_mut() = Some(result);
        }
    });
    fixture.settle();
    let ok = fixture.by_label("OK").expect("the OK button");
    fixture.click(&ok);
    fixture.idle(3);
    assert!(matches!(answer.borrow().as_ref(), Some(DialogResult::Ok(contour)) if *contour == Contour::linear()));
}

/// Cancel answers nothing.
#[trial]
fn the_contour_editor_answers_nothing_on_cancel() {
    let answer: Rc<RefCell<Option<DialogResult<Contour>>>> = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&answer);
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().open(ContourEditorDialog::new(ContourRequest::new("Contour", Contour::linear())));
    let shared = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shared.borrow_mut();
        host.run(ui);
        if let Some(result) = host.take::<ContourEditorDialog>() {
            *sink.borrow_mut() = Some(result);
        }
    });
    fixture.settle();
    let cancel = fixture.by_label("Cancel").expect("the Cancel button");
    fixture.click(&cancel);
    fixture.idle(3);
    assert!(matches!(answer.borrow().as_ref(), Some(DialogResult::Cancel)));
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
