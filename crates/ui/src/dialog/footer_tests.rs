//! Footer layout, keys, Reset, initial focus and AccessKit in a harness.

use egui::os::OperatingSystem;
use egui::{Key, Modifiers, Rect, Vec2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::controls::TextField;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Standard,
    Side,
}

struct App {
    state: DialogState,
    open: bool,
    shape: Shape,
    modeless: bool,
    ok_enabled: bool,
    apply: bool,
    alternates: usize,
    reset: bool,
    enter: EnterRule,
    field: bool,
    below: bool,
    focus: InitialFocus,
    outcomes: Vec<Outcome>,
    last: Option<DialogResponse>,
}

impl App {
    fn new(shape: Shape) -> Self {
        Self {
            state: DialogState::new(),
            open: true,
            shape,
            modeless: false,
            ok_enabled: true,
            apply: false,
            alternates: 0,
            reset: false,
            enter: EnterRule::Default,
            field: false,
            below: false,
            focus: InitialFocus::Auto,
            outcomes: Vec::new(),
            last: None,
        }
    }
}

fn harness(app: App, os: OperatingSystem) -> Harness<'static, App> {
    HarnessBuilder::default().with_size(WINDOW).with_os(os).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = &ui.ctx().clone();
            if !app.open {
                return;
            }
            let mut footer = match app.shape {
                Shape::Standard => Footer::standard().apply_enabled(true),
                Shape::Side => Footer::side().extra("Add to Swatches"),
            }
            .ok_enabled(app.ok_enabled)
            .enter(app.enter);
            if app.below {
                footer = footer.below(|ui| {
                    ui.label("Under the buttons");
                });
            }
            if app.apply {
                footer = footer.apply("Apply");
            }
            for label in ["Don't Save", "Discard"].into_iter().take(app.alternates) {
                footer = footer.alternate(label);
            }
            if app.reset {
                footer = footer.reset(Reset::AltCancel("Reset".into()));
            }
            let mut dialog = Dialog::new("footer", "Canvas Size").footer(footer).initial_focus(app.focus);
            if app.modeless {
                dialog = dialog.modeless();
            }
            let field = app.field;
            let shown = dialog.show(ctx, &mut app.state, &mut (), |ui, ()| {
                ui.label("Body");
                if field {
                    let _ = TextField::new("name", "Name", "Untitled").show(ui);
                }
            });
            app.outcomes.extend(shown.outcome);
            app.open &= !shown.outcome.is_some_and(Outcome::closes);
            app.last = Some(shown);
        },
        app,
    )
}

fn rect_of(harness: &Harness<'_, App>, label: &str) -> Rect {
    harness.get_by_label(label).rect()
}

#[test]
fn the_row_follows_the_platform_order_at_the_trailing_edge() {
    for (os, left, right) in [(OperatingSystem::Windows, "OK", "Cancel"), (OperatingSystem::Mac, "Cancel", "OK")] {
        let mut app = App::new(Shape::Standard);
        app.apply = true;
        let mut harness = harness(app, os);
        harness.run();
        let (left_rect, right_rect) = (rect_of(&harness, left), rect_of(&harness, right));
        assert!(left_rect.right() < right_rect.left(), "{os:?}: {left} before {right}: {left_rect:?} {right_rect:?}");
        let apply = rect_of(&harness, "Apply");
        let frame = harness.state().last.as_ref().expect("shown").rect;
        let last = if os == OperatingSystem::Mac { rect_of(&harness, "OK") } else { apply };
        assert!((frame.right() - 7.0 - last.right()).abs() < 1.5, "{os:?}: the row ends at the frame's padding: {frame:?} {last:?}");
        assert!(left_rect.width() >= 84.0 - 0.5);
    }
}

#[test]
fn enter_answers_ok_and_esc_cancels() {
    let mut harness = harness(App::new(Shape::Standard), OperatingSystem::Nix);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Ok]);

    let mut harness = self::harness(App::new(Shape::Standard), OperatingSystem::Nix);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Cancel]);
}

#[test]
fn enter_does_nothing_while_ok_is_disabled_or_the_body_owns_it() {
    for (ok_enabled, enter) in [(false, EnterRule::Default), (true, EnterRule::Body)] {
        let mut app = App::new(Shape::Standard);
        app.ok_enabled = ok_enabled;
        app.enter = enter;
        app.focus = InitialFocus::None;
        let mut harness = harness(app, OperatingSystem::Nix);
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert!(harness.state().outcomes.is_empty(), "ok_enabled {ok_enabled}");
    }
}

#[test]
fn enter_on_a_focused_cancel_button_presses_it() {
    let mut harness = harness(App::new(Shape::Standard), OperatingSystem::Nix);
    harness.run();
    harness.get_by_label("Cancel").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Cancel]);
}

#[test]
fn a_modeless_dialog_answers_enter_only_with_the_focus_inside() {
    let mut app = App::new(Shape::Standard);
    app.modeless = true;
    app.field = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.ctx.memory_mut(|m| m.focused().inspect(|id| m.surrender_focus(*id)));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().outcomes.is_empty(), "nothing in the dialog has the focus");

    harness.get_by_label("Name").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Ok]);
}

#[test]
fn esc_closes_a_modeless_dialog_with_a_footer_when_the_focus_is_inside() {
    let mut app = App::new(Shape::Standard);
    app.modeless = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Close], "the default button took the focus when it opened");
}

#[test]
fn apply_leaves_the_dialog_open() {
    let mut app = App::new(Shape::Standard);
    app.apply = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Apply]);
    assert!(harness.state().state.is_open());
}

#[test]
fn an_alternate_reports_which_one_was_pressed() {
    let mut app = App::new(Shape::Standard);
    app.alternates = 2;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Alternate]);
    assert_eq!(harness.state().last.as_ref().and_then(|shown| shown.alternate), Some(1));
}

#[test]
fn alt_turns_cancel_into_reset_and_esc_stays_cancel() {
    let mut app = App::new(Shape::Standard);
    app.reset = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    assert!(harness.query_by_label("Reset").is_none());
    harness.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Space);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Reset]);
    assert!(harness.state().state.is_open(), "Reset keeps the dialog");

    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Reset, Outcome::Cancel]);
}

#[test]
fn the_first_field_takes_the_focus_when_the_dialog_opens_else_the_default_button() {
    let mut app = App::new(Shape::Standard);
    app.field = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    assert!(harness.get_by_label("Name").accesskit_node().is_focused());

    let mut harness = self::harness(App::new(Shape::Standard), OperatingSystem::Nix);
    harness.run();
    assert!(harness.get_by_label("OK").accesskit_node().is_focused());
}

#[test]
fn the_default_button_is_described_and_a_disabled_ok_is_disabled() {
    let mut harness = harness(App::new(Shape::Standard), OperatingSystem::Nix);
    harness.run();
    let ok = harness.get_by_label("OK");
    assert_eq!(ok.accesskit_node().role(), egui::accesskit::Role::Button);
    assert_eq!(ok.accesskit_node().description().as_deref(), Some("Default"));
    assert_eq!(harness.get_by_label("Cancel").accesskit_node().description(), None);

    let mut app = App::new(Shape::Standard);
    app.ok_enabled = false;
    let mut harness = self::harness(app, OperatingSystem::Nix);
    harness.run();
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
}

#[test]
fn the_side_column_stacks_ok_above_cancel_at_the_trailing_side() {
    let mut harness = harness(App::new(Shape::Side), OperatingSystem::Mac);
    harness.run();
    let (ok, cancel, extra) = (rect_of(&harness, "OK"), rect_of(&harness, "Cancel"), rect_of(&harness, "Add to Swatches"));
    assert!(ok.bottom() < cancel.top() && cancel.bottom() < extra.top(), "{ok:?} {cancel:?} {extra:?}");
    assert!((ok.width() - extra.width()).abs() < 0.5 && ok.width() >= 96.0 - 0.5, "{ok:?} {extra:?}");
    let frame = harness.state().last.as_ref().expect("shown").rect;
    assert!((frame.right() - 7.0 - ok.right()).abs() < 1.5, "{frame:?} {ok:?}");
    assert!(rect_of(&harness, "Body").right() < ok.left());

    harness.get_by_label("Add to Swatches").click();
    harness.run();
    assert!(harness.state().outcomes.is_empty() && harness.state().state.is_open());
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Ok]);
}

#[test]
fn the_side_column_has_a_slot_under_its_last_button() {
    let mut app = App::new(Shape::Side);
    app.below = true;
    let mut harness = harness(app, OperatingSystem::Nix);
    harness.run();
    let (extra, slot) = (rect_of(&harness, "Add to Swatches"), rect_of(&harness, "Under the buttons"));
    assert!(slot.top() >= extra.bottom() && slot.left() >= extra.left() - 0.5 && slot.left() < extra.left() + 1.0, "{extra:?} {slot:?}");
}

/// The row's allocation is as tall as its buttons, so whatever follows it starts below them.
#[test]
fn the_button_row_allocates_the_buttons_height() {
    let gap = std::rc::Rc::new(std::cell::Cell::new(f32::NAN));
    let seen = gap.clone();
    let _h = Harness::builder()
        .with_size([400.0, 200.0])
        .with_context_setup(|ctx: &egui::Context| crate::theme::install(ctx, &crate::theme::Base::suite_default()))
        .build_ui(move |ui| {
            let shown = super::FooterButtons::new("row", vec![super::FooterButton::new(super::ButtonRole::Default, "OK")]).show(ui);
            let button_bottom = shown.responses.iter().map(|r| r.rect.bottom()).fold(0.0, f32::max);
            seen.set(ui.min_rect().bottom() - button_bottom);
        });
    assert!(gap.get() >= -0.01, "the buttons reach {} past the row", -gap.get());

    let bottom = std::rc::Rc::new(std::cell::Cell::new(f32::NAN));
    let seen = bottom.clone();
    let mut h = Harness::builder()
        .with_size([400.0, 200.0])
        .with_context_setup(|ctx: &egui::Context| crate::theme::install(ctx, &crate::theme::Base::suite_default()))
        .build_ui(move |ui| {
            let cx = super::footer::FooterCx { id: ui.make_persistent_id("footer"), platform: super::buttons::Platform::of(ui.ctx()) };
            super::footer::Footer::standard().show_row(ui, cx, &mut super::preview::Preview::default());
            seen.set(ui.min_rect().bottom());
        });
    h.run();
    let button_bottom = h.get_by_label("OK").rect().bottom();
    assert!(bottom.get() - button_bottom >= -0.01, "the standard footer's buttons reach {} past the row", button_bottom - bottom.get());
}

#[test]
fn ok_enabled_with_follows_an_edit_of_the_same_pass() {
    for side in [false, true] {
        let mut harness = HarnessBuilder::default().with_size(WINDOW).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            move |ui, state: &mut (DialogState<bool>, bool)| {
                let ctx = &ui.ctx().clone();
                let valid = std::cell::Cell::new(state.1);
                let footer = if side { Footer::side() } else { Footer::standard() }.ok_enabled_with(|| valid.get());
                let (dialog, fields) = (&mut state.0, &mut state.1);
                Dialog::new("footer-with", "Name").footer(footer).show(ctx, dialog, fields, |ui, filled| {
                    if ui.button("Fill").clicked() {
                        *filled = true;
                    }
                    valid.set(*filled);
                });
            },
            (DialogState::new(), false),
        );
        harness.run();
        assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
        harness.get_by_label("Fill").click();
        harness.step();
        assert!(!harness.get_by_label("OK").accesskit_node().is_disabled(), "side: {side}");
    }
}
