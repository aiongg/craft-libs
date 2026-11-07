//! The preview view maths and what a remembering dialog saves.

use craft_behavior::CanvasTransform;
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::patterns::form_dialogs::{
    DialogFields, FieldValue, HostRequests, Param, ParamDialog, ParamKind, ParamRows, PreviewPane, ResetFooter, ResetPick,
};
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::SliderScale;
use egui::{Key, Modifiers, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PICTURE: Vec2 = Vec2::new(400.0, 300.0);
const WELL: Vec2 = Vec2::new(200.0, 200.0);
const START: Vec2 = Vec2::new(10.0, -20.0);
const STEPPED: Vec2 = Vec2::new(15.0, -30.0);
const FAR: Vec2 = Vec2::new(5000.0, -5000.0);

/// Zoom moves one level at a time, keeps the picture's centre where it is on the screen and stops at the ends.
#[trial]
fn zoom_steps_through_the_levels_and_stops_at_the_ends() {
    let view = CanvasTransform::new(1.0, START);
    let bigger = PreviewPane::zoom_in(view);
    assert_eq!(bigger.scale, 1.5);
    assert_eq!(bigger.offset, STEPPED);
    assert_eq!(PreviewPane::zoom_out(view).scale, 0.75);

    let mut view = view;
    for _ in 0..20 {
        view = PreviewPane::zoom_in(view);
    }
    assert!(!PreviewPane::can_zoom_in(view) && PreviewPane::can_zoom_out(view));
    for _ in 0..20 {
        view = PreviewPane::zoom_out(view);
    }
    assert!(PreviewPane::can_zoom_in(view) && !PreviewPane::can_zoom_out(view));
}

/// A picture smaller than the well sits in its middle; a larger one never shows a gap at an edge.
#[trial]
fn the_view_keeps_the_picture_in_the_well() {
    let small = PreviewPane::settled(CanvasTransform::new(0.25, FAR), PICTURE, WELL);
    assert_eq!(small.offset, Vec2::ZERO);

    let panned = PreviewPane::settled(CanvasTransform::new(1.0, FAR), PICTURE, WELL);
    assert_eq!(panned.offset, Vec2::new((PICTURE.x - WELL.x) / 2.0, -(PICTURE.y - WELL.y) / 2.0));
}

/// Only the portable rows are remembered: numbers, choices, check boxes and unit choices, not colours or text.
#[trial]
fn a_remembering_dialog_saves_the_portable_values() {
    let params = vec![
        Param::new("radius", "Radius", ParamKind::range(0.1, 1000.0, 1.0, SliderScale::Log)),
        Param::new("smooth", "Smooth", ParamKind::bool(false)),
        Param::new("note", "Note", ParamKind::text("")),
        Param::new("scratch", "Scratch", ParamKind::bool(false)).not_remembered(),
    ];
    let mut fields = DialogFields::new();
    fields.set("radius", 4.0);
    fields.set("smooth", true);
    fields.set("note", "hello");
    fields.set("scratch", true);
    fields.set("stale", 1.0);

    let saved = ParamDialog::remembered(&params, &fields);
    assert_eq!(saved.number("radius"), Some(4.0));
    assert_eq!(saved.bool("smooth"), Some(true));
    assert!(!saved.contains("note") && !saved.contains("scratch") && !saved.contains("stale"));
    assert_eq!(fields.get("note"), Some(&FieldValue::Text("hello".into())));
}

/// A colour row's button files a request that the host takes once; taking it clears it.
#[trial]
fn a_requested_colour_is_taken_once_and_cleared() {
    let params = Param::parse_list(r#"{"tint":color=#336699}"#);
    let requests = Rc::new(RefCell::new(HostRequests::new()));
    let (held, fields) = (Rc::clone(&requests), Rc::new(RefCell::new(DialogFields::new())));
    let mut fixture = Fixture::new(move |ui| {
        let mut fields = fields.borrow_mut();
        let mut rows = ParamRows::new("Options", &params, Bind::bound(&mut *fields)).show(ui);
        held.borrow_mut().absorb(&mut rows);
    });
    fixture.settle();
    assert!(requests.borrow_mut().take_color().is_none(), "no request before the button is pressed");
    let buttons = fixture.by_role(Role::BUTTON);
    assert_eq!(buttons.len(), 1, "the colour row has one button");
    fixture.click(&buttons[0]);
    fixture.idle(2);
    let taken = requests.borrow_mut().take_color().expect("the press filed a request");
    assert_eq!(taken.key, "tint");
    assert!(requests.borrow_mut().take_color().is_none(), "a request is taken once");
}

/// A fixture with a parameter dialog over `fields`, which reports whether it is still open, with Alt held or not.
fn reset_rig(alt: bool, alt_resets: bool) -> (Fixture, Rc<RefCell<DialogFields>>, Rc<Cell<bool>>) {
    let params = vec![Param::number("width", "Width", 640.0, 1.0, 0)];
    let fields = Rc::new(RefCell::new(DialogFields::new()));
    let open = Rc::new(Cell::new(true));
    let (held, state) = (Rc::clone(&fields), Rc::clone(&open));
    let mut fixture = Fixture::new(move |ui| {
        let mut fields = held.borrow_mut();
        let mut now = state.get();
        ParamDialog::new("size", "Size", params.clone())
            .values(Bind::bound(&mut *fields))
            .opened(Bind::bound(&mut now))
            .alt_resets(alt_resets)
            .show(ui);
        state.set(now);
    });
    fixture.hold(if alt { Modifiers::ALT } else { Modifiers::NONE });
    fixture.settle();
    (fixture, fields, open)
}

/// Holding Alt turns Cancel into Reset, which restores the opening values and keeps the dialog open; with
/// `alt_resets(false)` the button stays Cancel.
#[trial]
fn alt_turns_cancel_into_reset_and_reset_keeps_the_dialog_open() {
    let (plain, ..) = reset_rig(false, true);
    assert!(plain.by_label("Cancel").is_some() && plain.by_label("Reset").is_none(), "Cancel without Alt");

    let (kept, ..) = reset_rig(true, false);
    assert!(kept.by_label("Cancel").is_some() && kept.by_label("Reset").is_none(), "Cancel with Alt when alt_resets is off");

    let (mut held, fields, open) = reset_rig(true, true);
    assert!(held.by_label("Reset").is_some() && held.by_label("Cancel").is_none(), "Reset with Alt");
    let opening = fields.borrow().number("width");
    fields.borrow_mut().set("width", 700.0);
    held.idle(3);
    assert_ne!(fields.borrow().number("width"), opening, "the value was edited");
    let reset = held.by_label("Reset").expect("the Reset button is shown");
    held.click(&reset);
    held.idle(3);
    assert_eq!(fields.borrow().number("width"), opening, "Reset restores the opening value");
    assert!(open.get(), "Reset keeps the dialog open");
}

/// A second dialog built on `ResetFooter` swaps its button the same way and reads the pick from `DialogOutput::picked`.
#[trial]
fn the_shared_reset_footer_serves_any_dialog() {
    let picked = Rc::new(Cell::new(None));
    for alt in [false, true] {
        let seen = Rc::clone(&picked);
        let mut fixture = Fixture::new(move |ui| {
            let mut open = true;
            let footer = ResetFooter::new(ui, None, true);
            let shown = Dialog::new("Other")
                .opened(Bind::bound(&mut open))
                .buttons(footer.buttons(FooterButton::new("OK", DialogRole::Accept), true))
                .show(ui, |_, _| {});
            if let Some(pick) = footer.pick(shown.output.picked) {
                seen.set(Some(pick));
            }
        });
        fixture.hold(if alt { Modifiers::ALT } else { Modifiers::NONE });
        fixture.settle();
        let (shown, hidden) = if alt { ("Reset", "Cancel") } else { ("Cancel", "Reset") };
        assert!(fixture.by_label(shown).is_some() && fixture.by_label(hidden).is_none(), "the second button is {shown} (alt: {alt})");
        let button = fixture.by_label(shown).expect("the second button is shown");
        fixture.click(&button);
        fixture.idle(2);
        assert_eq!(picked.get(), Some(if alt { ResetPick::Reset } else { ResetPick::Cancel }), "the footer answers with the pick (alt: {alt})");
    }
}

/// A fixture with a `ParamDialog` whose `close_guard` answers `allow` and counts its calls.
fn guarded_rig(allow: bool) -> (Fixture, Rc<Cell<usize>>, Rc<Cell<bool>>) {
    let params = vec![Param::new("smooth", "Smooth", ParamKind::bool(false))];
    let fields = Rc::new(RefCell::new(DialogFields::new()));
    let open = Rc::new(Cell::new(true));
    let asked = Rc::new(Cell::new(0));
    let (state, counted) = (Rc::clone(&open), Rc::clone(&asked));
    let mut fixture = Fixture::new(move |ui| {
        let mut fields = fields.borrow_mut();
        let mut now = state.get();
        let counted = Rc::clone(&counted);
        ParamDialog::new("size", "Size", params.clone())
            .values(Bind::bound(&mut *fields))
            .opened(Bind::bound(&mut now))
            .shaped(move |dialog| {
                dialog.initial_focus(Initial::First).close_guard(move |_, _| {
                    counted.set(counted.get() + 1);
                    allow
                })
            })
            .show(ui);
        state.set(now);
    });
    fixture.settle();
    (fixture, asked, open)
}

/// OK and Enter take the same way through the dialog's buttons: the `close_guard` is asked for both, and a refusal
/// keeps the dialog open.
#[trial]
fn param_dialog_ok_and_enter_go_through_the_close_guard() {
    let (mut refused, asked, open) = guarded_rig(false);
    let ok = refused.by_label("OK").expect("the OK button is shown");
    refused.click(&ok);
    refused.idle(3);
    assert_eq!(asked.get(), 1, "OK asks the guard");
    assert!(open.get(), "a refused OK keeps the dialog open");
    refused.key(Key::Enter, Modifiers::NONE);
    refused.idle(3);
    assert_eq!(asked.get(), 2, "Enter asks the guard through the OK button");
    assert!(open.get(), "a refused Enter keeps the dialog open");

    let (mut allowed, asked, open) = guarded_rig(true);
    allowed.key(Key::Enter, Modifiers::NONE);
    allowed.idle(3);
    assert_eq!(asked.get(), 1, "Enter asks the guard once");
    assert!(!open.get(), "an allowed Enter closes the dialog");
}
