//! The full-window dialog in a harness: modality, keys and the footer buttons.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Key, Rect, Vec2, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

struct App {
    state: DialogState,
    open: bool,
    enter: EnterRule,
    behind_clicks: usize,
    outcomes: Vec<Outcome>,
    footer_rect: Rect,
    last: Option<DialogResponse>,
}

fn harness(enter: EnterRule) -> Harness<'static, App> {
    let app = App { state: DialogState::new(), open: true, enter, behind_clicks: 0, outcomes: Vec::new(), footer_rect: Rect::NOTHING, last: None };
    HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = &ui.ctx().clone();
                if ui.button("Behind").clicked() {
                    app.behind_clicks += 1;
                }
                if !app.open {
                    return;
                }
                let strip = Rect::from_min_size(pos2(300.0, 400.0), vec2(300.0, 28.0));
                let mut footer = Rect::NOTHING;
                let shown = FullWindowDialog::new("liquify").title("Liquify").enter(app.enter).show(ctx, &mut app.state, &mut (), |ui, (), fx| {
                    ui.label("Workspace");
                    let row = fx.buttons(
                        ui,
                        strip,
                        vec![
                            FooterButton::new(ButtonRole::Default, "OK").min_width(120.0),
                            FooterButton::new(ButtonRole::Cancel, "Cancel"),
                            FooterButton::new(ButtonRole::Apply, "Apply"),
                        ],
                    );
                    footer = row.responses.iter().map(|r| r.rect).reduce(|a, b| a.union(b)).unwrap_or(Rect::NOTHING);
                });
                app.footer_rect = footer;
                app.outcomes.extend(shown.outcome);
                app.open &= !shown.outcome.is_some_and(Outcome::closes);
                app.last = Some(shown);
            },
            app,
        )
}

#[test]
fn the_dialog_is_modal_and_covers_the_window() {
    let mut harness = harness(EnterRule::Default);
    harness.run();
    assert!(harness.ctx.memory(|m| m.top_modal_layer().is_some()));
    assert_eq!(harness.state().last.as_ref().unwrap().rect.size(), WINDOW);
    harness.get_by_label("Behind").click();
    harness.run();
    assert_eq!(harness.state().behind_clicks, 0, "a press under the dialog is not delivered");
}

#[test]
fn the_buttons_sit_at_the_trailing_edge_of_the_given_rect() {
    let mut harness = harness(EnterRule::Default);
    harness.run();
    let footer = harness.state().footer_rect;
    assert!((footer.right() - 600.0).abs() < 1.5, "{footer:?}");
    assert!(footer.top() >= 400.0 && footer.bottom() <= 428.5, "{footer:?}");
}

#[test]
fn enter_answers_ok_and_esc_cancels() {
    let mut harness = harness(EnterRule::Default);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Ok]);

    let mut harness = self::harness(EnterRule::Default);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Cancel]);
}

#[test]
fn a_workspace_that_owns_enter_does_not_answer_it() {
    let mut harness = harness(EnterRule::Body);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().outcomes.is_empty());
}

#[test]
fn apply_keeps_the_dialog_open_and_a_button_answers() {
    let mut harness = harness(EnterRule::Default);
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Apply]);
    assert!(harness.state().open);
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().outcomes, [Outcome::Apply, Outcome::Cancel]);
}

#[test]
fn the_dialog_is_named_by_its_title() {
    let mut harness = harness(EnterRule::Default);
    harness.run();
    assert_eq!(harness.get_by_role_and_label(Role::Dialog, "Liquify").accesskit_node().role(), Role::Dialog);
}
