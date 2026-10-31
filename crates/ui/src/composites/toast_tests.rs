//! Toast tests: timers, pausing, the limit, results, placement and AccessKit.

use std::time::Duration;

use egui::accesskit::{Live, Role};
use egui::{Context, Key, Pos2, Rect, pos2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder, Node};

use super::*;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, install};

const BODY: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(640.0, 400.0) };

struct App {
    toasts: Vec<(u64, Tone, bool, Vec<String>)>,
    auto_hide: Option<Duration>,
    ended: Vec<(u64, ToastEnd)>,
    /// Draws a "Before" button ahead of the stack.
    before: bool,
    /// Where the "Before" button goes; the top left when `None`.
    before_at: Option<Rect>,
    /// Whether "Before" was clicked.
    clicked: bool,
}

impl App {
    fn new(ids: &[u64], auto_hide: Option<Duration>) -> Self {
        Self {
            toasts: ids.iter().map(|id| (*id, Tone::Neutral, false, Vec::new())).collect(),
            auto_hide,
            ended: Vec::new(),
            before: false,
            before_at: None,
            clicked: false,
        }
    }

    fn with_button_before(mut self) -> Self {
        self.before = true;
        self
    }
}

fn harness(dir: Direction, app: App) -> Harness<'static, App> {
    let builder: HarnessBuilder<App> = Harness::builder().with_size(BODY.size()).with_step_dt(0.1).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    });
    builder.build_ui_state(
        |ui, app| {
            if app.before {
                let button = crate::controls::Button::new("Before");
                let response = match app.before_at {
                    Some(rect) => ui.put(rect, button),
                    None => ui.add(button),
                };
                app.clicked |= response.clicked();
            }
            let toasts = app
                .toasts
                .iter()
                .map(|(id, tone, sticky, lines)| Toast::new(*id, format!("Toast {id}")).tone(*tone).sticky(*sticky).lines(lines.clone()));
            let ended = ToastStack::new("toasts").auto_hide(app.auto_hide).show(ui.ctx(), BODY, toasts);
            app.toasts.retain(|(id, ..)| !ended.iter().any(|(e, _)| e == id));
            app.ended.extend(ended);
        },
        app,
    )
}

/// The tokens of the suite's default theme.
fn tokens() -> std::sync::Arc<crate::theme::UiTokens> {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    crate::theme::current(&ctx)
}

#[allow(clippy::expect_used)]
fn dismiss<'a>(harness: &'a Harness<'_, App>, n: usize) -> Node<'a> {
    harness.get_all_by_label("Dismiss").nth(n).expect("dismiss button")
}

/// The card of toast `id`: the live region labelled by its title.
fn card<'a>(harness: &'a Harness<'_, App>, id: u64) -> Node<'a> {
    let title = format!("Toast {id}");
    harness.get_by(move |n| matches!(n.role(), Role::Status | Role::Alert) && n.label().is_some_and(|l| l.starts_with(&title)))
}

fn steps(harness: &mut Harness<'_, App>, n: usize) {
    for _ in 0..n {
        harness.step();
    }
}

fn pointer_away(harness: &mut Harness<'_, App>) {
    harness.hover_at(Pos2::new(5.0, 5.0));
}

#[test]
fn a_toast_expires_after_the_duration() {
    let mut harness = harness(Direction::Ltr, App::new(&[1], Some(Duration::from_secs(1))));
    steps(&mut harness, 5);
    assert!(harness.state().ended.is_empty(), "not yet");
    steps(&mut harness, 10);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Expired)]);
    assert!(harness.query_by_label("Dismiss").is_none());
}

#[test]
fn no_auto_hide_keeps_toasts() {
    let mut harness = harness(Direction::Ltr, App::new(&[1, 2], None));
    steps(&mut harness, 60);
    assert!(harness.state().ended.is_empty());
    assert_eq!(harness.get_all_by_label("Dismiss").count(), 2);
}

#[test]
fn the_pointer_over_the_stack_pauses_and_leaving_gives_a_fresh_delay() {
    let mut harness = harness(Direction::Ltr, App::new(&[1], Some(Duration::from_secs(1))));
    steps(&mut harness, 2);
    let at = dismiss(&harness, 0).rect().center();
    harness.hover_at(at);
    steps(&mut harness, 30);
    assert!(harness.state().ended.is_empty(), "hover time never counts");
    pointer_away(&mut harness);
    steps(&mut harness, 4);
    assert!(harness.state().ended.is_empty(), "a fresh delay starts when the pointer leaves");
    steps(&mut harness, 10);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Expired)]);
}

#[allow(clippy::expect_used)]
fn button<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    harness.query_by_label(label).expect("button")
}

#[test]
fn keyboard_focus_on_dismiss_pauses_and_leaving_gives_a_fresh_delay() {
    let mut harness = harness(Direction::Ltr, App::new(&[1], Some(Duration::from_secs(1))).with_button_before());
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    assert!(dismiss(&harness, 0).is_focused());
    steps(&mut harness, 30);
    assert!(harness.state().ended.is_empty(), "focused time never counts");
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    assert!(button(&harness, "Before").is_focused(), "focus left the stack");
    steps(&mut harness, 7);
    assert!(harness.state().ended.is_empty(), "a fresh delay starts when focus leaves");
    steps(&mut harness, 10);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Expired)]);
}

#[test]
fn dismissing_with_the_keyboard_moves_focus_to_the_next_card_then_back() {
    let mut harness = harness(Direction::Ltr, App::new(&[1, 2], None).with_button_before());
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    assert!(button(&harness, "Before").is_focused());
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    assert!(dismiss(&harness, 0).is_focused());
    harness.key_press(Key::Enter);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Closed)]);
    assert!(dismiss(&harness, 0).is_focused(), "focus moves to the remaining card");
    harness.key_press(Key::Enter);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Closed), (2, ToastEnd::Closed)]);
    assert!(button(&harness, "Before").is_focused(), "focus returns to where it was before the stack");
}

#[test]
fn dismissing_the_newest_card_moves_focus_to_the_previous_one() {
    let mut harness = harness(Direction::Ltr, App::new(&[1, 2], None));
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    assert!(dismiss(&harness, 1).is_focused());
    harness.key_press(Key::Enter);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(2, ToastEnd::Closed)]);
    assert!(dismiss(&harness, 0).is_focused());
}

#[test]
fn closing_an_older_toast_does_not_announce_the_newer_ones_again() {
    let mut harness = harness(Direction::Ltr, App::new(&[1, 2, 3], None));
    steps(&mut harness, 2);
    let before: Vec<_> = [2, 3].map(|id| (card(&harness, id).accesskit_node().id(), card(&harness, id).accesskit_node().label())).into();
    dismiss(&harness, 0).click();
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Closed)]);
    let after: Vec<_> = [2, 3].map(|id| (card(&harness, id).accesskit_node().id(), card(&harness, id).accesskit_node().label())).into();
    assert_eq!(before, after, "same nodes with the same names");
}

#[test]
fn dismiss_reports_closed_for_pointer_and_keyboard() {
    let mut harness = harness(Direction::Ltr, App::new(&[1, 2], None));
    steps(&mut harness, 2);
    dismiss(&harness, 0).click();
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Closed)]);
    harness.key_press(Key::Tab);
    steps(&mut harness, 2);
    harness.key_press(Key::Enter);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Closed), (2, ToastEnd::Closed)]);
}

#[test]
fn a_fourth_toast_drops_the_oldest_that_is_not_sticky() {
    let mut app = App::new(&[1, 2, 3, 4], None);
    app.toasts[0].2 = true;
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(2, ToastEnd::Dropped)]);
    assert_eq!(harness.get_all_by_label("Dismiss").count(), 3);
}

#[test]
fn all_sticky_drops_the_oldest() {
    let mut app = App::new(&[1, 2, 3, 4], None);
    app.toasts.iter_mut().for_each(|t| t.2 = true);
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 2);
    assert_eq!(harness.state().ended, [(1, ToastEnd::Dropped)]);
}

#[test]
fn a_repaint_is_requested_for_the_next_expiry() {
    let mut harness = harness(Direction::Ltr, App::new(&[1], Some(Duration::from_secs(10))));
    steps(&mut harness, 3);
    let delay = harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(delay > Duration::ZERO && delay <= Duration::from_secs(10), "{delay:?}");
}

#[test]
fn the_stack_sits_in_the_bottom_corner_at_the_reading_end() {
    let tokens = tokens();
    let [inset_x, inset_y] = tokens.component.toast.inset;
    let mut ltr = harness(Direction::Ltr, App::new(&[1, 2], None));
    steps(&mut ltr, 3);
    let pad = tokens.component.toast.pad;
    let (first, newest) = (dismiss(&ltr, 0).rect(), dismiss(&ltr, 1).rect());
    assert!((newest.right() + pad - (BODY.right() - inset_x)).abs() < 1.0, "{newest:?}");
    assert!((newest.bottom() + pad - (BODY.bottom() - inset_y)).abs() < 1.0, "{newest:?}");
    assert!(first.bottom() < newest.top(), "newest at the bottom");
    let mut rtl = harness(Direction::Rtl, App::new(&[1], None));
    steps(&mut rtl, 3);
    let button = dismiss(&rtl, 0).rect();
    assert!((button.left() - pad - (BODY.left() + inset_x)).abs() < 1.0, "{button:?}");
    assert!((button.bottom() + pad - (BODY.bottom() - inset_y)).abs() < 1.0, "{button:?}");
}

#[test]
fn a_new_toast_is_in_place_the_frame_it_appears() {
    let tokens = tokens();
    let [inset_x, inset_y] = tokens.component.toast.inset;
    let pad = tokens.component.toast.pad;
    let mut harness = harness(Direction::Ltr, App::new(&[1], None));
    steps(&mut harness, 3);
    let lines = (1..=4).map(|n| format!("line {n}")).collect();
    harness.state_mut().toasts.push((2, Tone::Warn, false, lines));
    harness.step();
    // Text and buttons carry the bounds: the first card is one row, "Dismiss" tops the newest.
    let (first, newest) = (dismiss(&harness, 0).rect(), dismiss(&harness, 1).rect());
    let last_line = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("line 4")).rect();
    assert!((last_line.bottom() + pad - (BODY.bottom() - inset_y)).abs() < 1.0, "{last_line:?}");
    assert!((newest.right() + pad - (BODY.right() - inset_x)).abs() < 1.0, "{newest:?}");
    let gap = tokens.component.toast.gap;
    assert!((newest.top() - pad - gap - (first.bottom() + pad)).abs() < 1.0, "{first:?} above {newest:?}");
}

#[test]
fn a_click_above_the_cards_reaches_what_is_under_the_column() {
    let mut app = App::new(&[1], None).with_button_before();
    app.before_at = Some(Rect::from_min_size(pos2(BODY.right() - 140.0, 40.0), egui::vec2(100.0, 24.0)));
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 3);
    button(&harness, "Before").click();
    steps(&mut harness, 2);
    assert!(harness.state().clicked);
}

#[test]
fn a_press_on_a_cards_padding_reaches_nothing_under_it() {
    let tokens = tokens();
    let [inset_x, inset_y] = tokens.component.toast.inset;
    let left = BODY.right() - inset_x - tokens.component.toast.width;
    let mut app = App::new(&[1], None).with_button_before();
    // Under the card's bottom left corner, which is padding.
    app.before_at = Some(Rect::from_min_size(pos2(left - 20.0, BODY.bottom() - inset_y - 30.0), egui::vec2(100.0, 40.0)));
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 3);
    let tap = |harness: &mut Harness<'_, App>, at: Pos2| {
        harness.hover_at(at);
        harness.step();
        harness.drag_at(at);
        harness.step();
        harness.drop_at(at);
        steps(harness, 2);
    };
    tap(&mut harness, pos2(left + 3.0, BODY.bottom() - inset_y - 3.0));
    assert!(!harness.state().clicked);
    tap(&mut harness, pos2(left - 10.0, BODY.bottom() - inset_y - 10.0));
    assert!(harness.state().clicked, "left of the card the button still takes clicks");
}

#[test]
fn a_card_is_a_live_region_per_tone_named_by_its_title_and_lines() {
    let mut app = App::new(&[1, 2], None);
    app.toasts[0] = (1, Tone::Warn, false, vec!["first".into(), "second".into()]);
    app.toasts[1] = (2, Tone::Bad, false, Vec::new());
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 3);
    let warn = card(&harness, 1);
    assert_eq!(warn.accesskit_node().role(), Role::Status);
    assert_eq!(warn.accesskit_node().live(), Live::Polite);
    assert_eq!(warn.accesskit_node().label().as_deref(), Some("Toast 1, first, second"));
    let bad = card(&harness, 2);
    assert_eq!(bad.accesskit_node().role(), Role::Alert);
    assert_eq!(bad.accesskit_node().live(), Live::Assertive);
    a11y::assert_labelled(&harness);
}

#[test]
fn everything_inside_a_card_is_silent() {
    let mut app = App::new(&[1], None);
    app.toasts[0].3 = vec!["line".into()];
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 3);
    let live_of = |n: &Node<'_>| n.accesskit_node().live();
    assert_eq!(live_of(&dismiss(&harness, 0)), Live::Off);
    assert_eq!(live_of(&harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Toast 1"))), Live::Off);
    assert_eq!(live_of(&harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("line"))), Live::Off);
}

#[test]
fn lines_beyond_eight_become_one_more_line() {
    let mut app = App::new(&[1], None);
    app.toasts[0].3 = (1..=10).map(|n| format!("w{n}")).collect();
    let mut harness = harness(Direction::Ltr, app);
    steps(&mut harness, 3);
    assert!(harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("w8")).is_some());
    assert!(harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("w9")).is_none());
    assert!(harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("…and 2 more")).is_some());
}

#[test]
fn the_story_is_labelled() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut args = (toast::stories()[0].args)();
        let show = toast::stories()[0].show;
        let mut harness = Harness::<()>::builder()
            .with_size([640.0, 480.0])
            .with_context_setup(move |ctx: &Context| {
                install(ctx, &Base::suite_default());
                direction::apply(ctx, dir);
            })
            .build_ui(move |ui| show(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
    }
}
