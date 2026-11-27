//! Count trials of the contract on craft-ui-rt (docs/contract-rt.md section 13): a frame runs only
//! the hooks it must. Relations only, no times.

extern crate craft_ui_contract as craft_ui;

mod access;
mod content;
mod env;
mod focus;
mod harness;
mod id;
mod input;
mod int5b;
mod items;
mod layout;
mod motion;
mod overlay;
mod pipeline;
mod services;
mod skeleton;
mod text;
mod text_field;
mod windows;

use crate::skeleton::{IconButton, Stack};
use craft_behavior::IconId;
use craft_ui_contract::contract::{MemoView, Ui};
use craft_ui_contract::runtime::windows::window_root;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Pos2, Rect, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;

/// A window with an "Other" button over a `MemoView` that shows a `Stack` of buttons A and B.
struct App {
    label_a: String,
    /// What a click on Other changes: nothing, or A's label (a prop of the view).
    rename_on_other: bool,
    other_clicks: u32,
    a_clicks: u32,
    /// Runs of the view's closure.
    view_runs: u32,
    other: Rect,
    a: Rect,
    b: Rect,
    /// Steps in which `show` returned a click.
    clicks_seen_in_step: Vec<u32>,
    step: u32,
}

impl App {
    fn new(rename_on_other: bool) -> App {
        App {
            label_a: "A".into(),
            rename_on_other,
            other_clicks: 0,
            a_clicks: 0,
            view_runs: 0,
            other: Rect::NOTHING,
            a: Rect::NOTHING,
            b: Rect::NOTHING,
            clicks_seen_in_step: Vec::new(),
            step: 0,
        }
    }
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    let other = IconButton::new(IconId::new("plus"), "Other").show(ui);
    app.other = other.rect;
    if other.clicked() {
        app.other_clicks += 1;
        app.clicks_seen_in_step.push(app.step);
        if app.rename_on_other {
            app.label_a = format!("A{}", app.other_clicks);
        }
    }
    let label = app.label_a.clone();
    let (a, b) = MemoView::new(label).show(ui, |ui, label| {
        app.view_runs += 1;
        Stack::new().show(ui, |ui| {
            let a = IconButton::new(IconId::new("brush"), label).show(ui);
            if a.clicked() {
                app.a_clicks += 1;
                app.clicks_seen_in_step.push(app.step);
            }
            let b = IconButton::new(IconId::new("eraser"), "B").show(ui);
            (a.rect, b.rect)
        })
    });
    app.a = a;
    app.b = b;
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    app.step += 1;
    rig.step_with(&mut |_, cx| window_root(cx, |ui| app_ui(ui, app)))
}

/// A rig with access on, run until it settles.
fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 300.0), 1.0, RecordingRenderer::default());
    let main = rig.main;
    rig.driver.runtime.set_access_enabled(main, true);
    let first = step(&mut rig, app);
    assert!(first.ran);
    assert_eq!((first.frame.scopes, first.frame.runs), (2, 4), "the first frame runs the root, the view, Other, the stack, A and B");
    assert!(app.a.min.y > app.other.max.y && app.b.min.y > app.a.max.y, "the buttons lie one under another");
    for _ in 0..4 {
        if !step(&mut rig, app).ran {
            return rig;
        }
    }
    panic!("the rig did not settle");
}

/// The pointer looks fade over the theme's hover time, read in paint: a later second ends them.
fn finish_fades(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    rig.driver.platform.advance(1.0);
    step(rig, app);
}

fn input(rig: &mut Rig<RecordingRenderer>, app: &mut App, input: Input) -> StepReport {
    let main = rig.main;
    rig.inject(main, input);
    step(rig, app)
}

#[test]
fn hover_over_a_kept_icon_button_repaints_one_node_and_runs_no_hook() {
    let mut app = App::new(false);
    let mut rig = rig(&mut app);
    let (a, b) = (app.a.center(), app.b.center());
    let entered = input(&mut rig, &mut app, Input::mouse_move(a));
    assert_eq!(entered.frame.painted, 1, "only A repaints: {entered:?}");
    assert_eq!((entered.frame.runs, entered.frame.scopes, entered.frame.built), (0, 0, 0), "no hook, scope or build runs");
    finish_fades(&mut rig, &mut app);
    let within = input(&mut rig, &mut app, Input::mouse_move(a + Vec2::new(1.0, 1.0)));
    assert_eq!(within.frame.painted, 0, "a move within A paints nothing");
    let to_b = input(&mut rig, &mut app, Input::mouse_move(b));
    assert_eq!((to_b.frame.painted, to_b.frame.runs, to_b.frame.scopes), (2, 0, 0), "A and B swap their looks, nothing runs");
    finish_fades(&mut rig, &mut app);
    let away = input(&mut rig, &mut app, Input::mouse_move(Pos2::new(390.0, 290.0)));
    assert_eq!((away.frame.painted, away.frame.runs), (1, 0));
}

#[test]
fn a_click_returns_clicked_from_show_in_the_frame_of_the_release() {
    let mut app = App::new(false);
    let mut rig = rig(&mut app);
    let at = app.a.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    let down = input(&mut rig, &mut app, Input::mouse_down(at));
    assert_eq!((down.frame.painted, down.frame.runs, down.frame.scopes), (1, 0, 0), "the press look is a kept variant");
    let up = input(&mut rig, &mut app, Input::mouse_up(at));
    assert_eq!(app.a_clicks, 1);
    assert_eq!(app.clicks_seen_in_step, [app.step], "show() returned the click in the step that delivered the release");
    assert_eq!(up.frame.runs, 1, "only A runs; Other, B and the stack's own hooks are kept: {up:?}");
    assert_eq!(up.frame.scopes, 2, "the root and the view on A's path run their closures");
    assert_eq!(app.view_runs, 2);
}

#[test]
fn a_settled_stack_in_an_unchanged_memo_view_runs_nothing_and_sends_nothing() {
    let mut app = App::new(false);
    let mut rig = rig(&mut app);
    let at = app.other.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    input(&mut rig, &mut app, Input::mouse_down(at));
    let views = app.view_runs;
    let up = input(&mut rig, &mut app, Input::mouse_up(at));
    assert_eq!(app.other_clicks, 1);
    assert_eq!(up.frame.scopes, 1, "the root runs; the view with unchanged props costs a key check");
    assert_eq!(app.view_runs, views, "the view's closure did not run");
    assert_eq!(up.frame.runs, 1, "only Other runs");
    assert_eq!(up.frame.painted, 1, "no display list under the view moves");
    assert_eq!(up.frame.access_nodes, 0, "no access node changed: Other's run gave the same node, and nothing under the view was visited");
}

#[test]
fn a_changed_prop_runs_that_child_and_keeps_its_sibling() {
    let mut app = App::new(true);
    let mut rig = rig(&mut app);
    let at = app.other.center();
    input(&mut rig, &mut app, Input::mouse_move(at));
    input(&mut rig, &mut app, Input::mouse_down(at));
    let views = app.view_runs;
    let up = input(&mut rig, &mut app, Input::mouse_up(at));
    assert_eq!(app.label_a, "A1");
    assert_eq!(app.view_runs, views + 1, "the view's props moved, so its closure runs");
    assert_eq!(up.frame.scopes, 2);
    assert_eq!(up.frame.runs, 2, "Other and A run; B and the stack's own hooks are kept: {up:?}");
}

#[test]
fn a_theme_token_change_runs_and_repaints_the_nodes_that_read_tokens() {
    let mut app = App::new(false);
    let mut rig = rig(&mut app);
    let mut tokens = craft_ui_test_kit::rt::rig::test_tokens();
    tokens.color.state.hover = craft_theme::Rgba([1, 2, 3, 255]);
    rig.driver.runtime.set_tokens(tokens);
    let changed = step(&mut rig, &mut app);
    assert_eq!(changed.frame.runs, 4, "Other, the stack, A and B read the tokens in their runs: {changed:?}");
    assert!(changed.frame.painted >= 3, "the three buttons repaint their recorded variants: {changed:?}");
    let settled = step(&mut rig, &mut app);
    assert_eq!((settled.frame.runs, settled.frame.painted), (0, 0), "the next frame runs and paints nothing: {settled:?}");
    let at = app.a.center();
    let hover = input(&mut rig, &mut app, Input::mouse_move(at));
    assert_eq!((hover.frame.painted, hover.frame.runs), (1, 0), "the new hover look is a kept variant of A");
}
