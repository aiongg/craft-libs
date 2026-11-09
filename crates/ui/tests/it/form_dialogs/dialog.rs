//! The preview view maths and what a remembering dialog saves.

use craft_behavior::CanvasTransform;
use craft_ui::contract::tooltip::DELAY;
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::dialog::{Dialog, FullWindowDialog};
use craft_ui::patterns::form_dialogs::{
    DialogFields, FieldValue, HostRequests, Param, ParamDialog, ParamKind, ParamRows, PreviewPane, ResetFooter, ResetPick,
};
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::SliderScale;
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PICTURE: Vec2 = Vec2::new(400.0, 300.0);
const WELL: Vec2 = Vec2::new(200.0, 200.0);
const START: Vec2 = Vec2::new(10.0, -20.0);
const STEPPED: Vec2 = Vec2::new(15.0, -30.0);
const FAR: Vec2 = Vec2::new(5000.0, -5000.0);

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

/// A full-window dialog takes the Reset footer through `ResetFooter::install`: Reset runs the restore and keeps the
/// dialog open.
#[trial]
fn a_full_window_dialog_resets_through_the_installed_footer() {
    let restored = Rc::new(Cell::new(0));
    let open = Rc::new(Cell::new(true));
    let (count, state) = (Rc::clone(&restored), Rc::clone(&open));
    let mut fixture = Fixture::new(move |ui| {
        let mut now = state.get();
        let count = Rc::clone(&count);
        let footer = ResetFooter::new(ui, None, true);
        let dialog =
            footer.install(FullWindowDialog::new("Liquify").opened(Bind::bound(&mut now)), FooterButton::new("OK", DialogRole::Accept), move |_| {
                count.set(count.get() + 1)
            });
        dialog.show(ui, |_, cx| ResetFooter::report(cx, true, true));
        state.set(now);
    });
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset).settle();
    assert_eq!(restored.get(), 1, "Reset ran the restore");
    assert!(open.get(), "Reset keeps the dialog open");
}

/// A fixture with a `ParamDialog` whose `close_guard` answers `allow` and counts its calls.
fn guarded_rig(allow: bool) -> (Fixture, Rc<Cell<usize>>, Rc<Cell<bool>>) {
    let params = vec![Param::new("note", "Note", ParamKind::text(""))];
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

/// OK and Enter take the same way through the dialog's buttons: the `close_guard` is asked for both, also for Enter
/// right after a click on OK, and a refusal keeps the dialog open.
#[trial]
fn param_dialog_ok_and_enter_go_through_the_close_guard() {
    let (mut clicked, asked, open) = guarded_rig(false);
    let ok = clicked.by_label("OK").expect("the OK button is shown");
    clicked.click(&ok);
    let after_click = (asked.get(), open.get());
    clicked.key(Key::Enter, Modifiers::NONE);
    let after_enter = (asked.get(), open.get());

    let (mut fresh, asked, open) = guarded_rig(false);
    fresh.key(Key::Enter, Modifiers::NONE);
    let fresh_enter = (asked.get(), open.get());

    let (mut allowed, asked, open) = guarded_rig(true);
    allowed.key(Key::Enter, Modifiers::NONE);
    allowed.idle(1);
    let allowed_enter = (asked.get(), open.get());

    assert_eq!(
        (after_click, after_enter, fresh_enter, allowed_enter),
        ((1, true), (2, true), (1, true), (1, false)),
        "(asked, open) after a refused click, then Enter; after a refused Enter; after an allowed Enter"
    );
}

/// The tooltip a hover over the button named `label` shows, and the pointer moved away again.
fn tip_over(fixture: &mut Fixture, label: &str) -> Option<String> {
    let button = fixture.by_label(label).expect("the button is shown");
    fixture.frame(vec![moved(button.center())]);
    fixture.advance(DELAY);
    fixture.idle(3);
    let tip = fixture.by_role(Role::TOOLTIP).first().and_then(|tip| fixture.tree().name_of(tip.node)).map(str::to_owned);
    fixture.frame(vec![moved(Pos2::ZERO)]);
    fixture.idle(3);
    tip
}

