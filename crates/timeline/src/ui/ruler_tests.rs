//! The ruler and the scroll bar (egui_kittest).

use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::testing::{HarnessExt as _, events, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, Pos2, Rect, Ui, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::ruler::{TimelineRuler, TimelineScrollBar, ruler_steps, ruler_ticks};
use super::style::TimelineStyle;
use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView};
use crate::timecode::{TimeFormat, format_time};

fn style() -> TimelineStyle {
    let mut style = None;
    egui::__run_test_ui(|ui| style = Some(TimelineStyle::of(&Tokens::of(ui))));
    style.unwrap()
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn format() -> TimeFormat {
    TimeFormat::timecode(FrameRate::new(24, 1))
}

// labels at least 110 pt apart, ticks at least 12 pt, every label on a tick.
#[test]
fn ruler_steps_keep_labels_and_ticks_apart() {
    let style = style();
    for frame_points in [0.01, 0.2, 1.0, 3.7, 10.0, 24.0, 90.0, 400.0] {
        let (tick, label) = ruler_steps(frame_points, 24, style.ruler_label_gap, style.ruler_tick_gap);
        assert!(label as f64 * frame_points >= 110.0 || label == 24 * 3600, "labels at {frame_points}: {label}");
        assert!(tick as f64 * frame_points >= 12.0 || tick == label, "ticks at {frame_points}: {tick}");
        assert_eq!(label % tick, 0, "every label sits on a tick");
    }
    assert_eq!(ruler_steps(10.0, 24, 110.0, 12.0), (2, 12), "12-frame labels with a tick every 2 frames");
    assert!(ruler_steps(1.0, i64::MAX, 110.0, 12.0).1 >= 110, "a huge base saturates");
}

// ruler_ticks labels major ticks with format_time.
#[test]
fn ruler_ticks_label_in_the_format() {
    let style = style();
    let format = format();
    let view = TimeView::new(secs(0.5), 240.0);
    let ticks = ruler_ticks(&format, &view, 480.0, &style);
    assert!(ticks[0].at <= view.start, "the first tick is at or before the left edge");
    let labelled: Vec<_> = ticks.iter().filter(|tick| tick.label.is_some()).collect();
    assert!(labelled.iter().all(|tick| tick.major));
    for tick in labelled {
        assert_eq!(tick.label.as_deref(), Some(format_time(tick.at, &format).as_str()));
    }
    assert!(ticks.iter().any(|tick| tick.label.as_deref() == Some("00:00:01:00")));
    assert!(ticks.windows(2).all(|pair| pair[0].at < pair[1].at));
    assert!(ruler_ticks(&format, &TimeView::new(Tick::ZERO, 0.0), 480.0, &style).is_empty());
}

struct App {
    view: TimeView,
    playhead: Tick,
    events: Vec<EditEvent<Tick>>,
    rect: Rect,
}

fn show_ruler(ui: &mut Ui, app: &mut App) {
    let out = TimelineRuler::new("ruler", &mut app.view, format(), secs(60.0)).playhead(app.playhead).show(ui);
    app.rect = out.response.rect;
    app.events.extend(out.scrub);
}

fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
    let mut harness = Harness::<S>::builder()
        .with_size([480.0, 120.0])
        // Two clicks of a double-click must land within egui's double-click delay.
        .with_step_dt(0.05)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, state);
    harness.run();
    harness
}

fn ruler() -> Harness<'static, App> {
    harness(show_ruler, App { view: TimeView::new(Tick::ZERO, 100.0), playhead: Tick::ZERO, events: Vec::new(), rect: Rect::NOTHING })
}

fn at(rect: Rect, seconds: f64) -> Pos2 {
    pos2(rect.left() + seconds as f32 * 100.0, rect.center().y + rect.height() * 0.2)
}

// press and drag report a Begin, Updates and a Commit of the time under the pointer; Esc cancels.
#[test]
fn dragging_the_ruler_scrubs_the_playhead() {
    let mut harness = ruler();
    let rect = harness.state().rect;
    harness.drag(at(rect, 1.0), at(rect, 3.0), Modifiers::NONE);
    let events = &harness.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(events.iter().any(|event| matches!(event, EditEvent::Update { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == secs(3.0)), "{events:?}");
    let times: Vec<Tick> = events.iter().filter_map(|event| event.value().copied()).collect();
    assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{times:?}");

    let mut harness = ruler();
    harness.state_mut().events.clear();
    harness.hover_at(at(rect, 1.0));
    harness.event(egui::Event::PointerButton { pos: at(rect, 1.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run_steps(2);
    harness.hover_at(at(rect, 2.0));
    harness.run_steps(2);
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::Escape));
    let events = &harness.state().events;
    assert!(events.iter().any(|event| matches!(event, EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{events:?}");
}

// Left and Right step one frame as Set events, Shift five.
#[test]
fn arrow_keys_step_the_playhead_one_frame() {
    let mut harness = ruler();
    harness.get_by_role_and_label(Role::Slider, "Time Ruler").focus();
    harness.run();
    harness.press(KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight));
    harness.press(KeyboardShortcut::new(Modifiers::SHIFT, Key::ArrowRight));
    let rate = FrameRate::new(24, 1);
    let sets: Vec<Tick> =
        harness.state().events.iter().filter(|event| matches!(event, EditEvent::Set { .. })).map(|event| *event.value().unwrap()).collect();
    assert_eq!(sets, [rate.tick_of(1), rate.tick_of(5)]);
    assert_eq!(harness.get_by_role_and_label(Role::Slider, "Time Ruler").value().as_deref(), Some("00:00:00:00"));
}

struct Bar {
    view: TimeView,
    rect: Rect,
}

fn show_bar(ui: &mut Ui, app: &mut Bar) {
    app.rect = TimelineScrollBar::new("bar", &mut app.view, secs(60.0)).show(ui).rect;
}

// dragging an end of the thumb zooms, the middle scrolls; the thumb is at least 34 pt.
#[test]
fn scroll_bar_ends_zoom_and_middle_scrolls() {
    let start = TimeView::new(Tick::ZERO, 100.0);
    let thumb_min = style().scroll_thumb_min;
    let mut scrolled = harness(show_bar, Bar { view: start, rect: Rect::NOTHING });
    let rect = scrolled.state().rect;
    let y = rect.center().y;
    // The view shows 4.8 s of 660: its thumb would be 3.5 pt, so the minimum applies.
    let middle = pos2(rect.left() + thumb_min / 2.0, y);
    scrolled.drag(middle, middle + egui::vec2(100.0, 0.0), Modifiers::NONE);
    let view = scrolled.state().view;
    assert!(view.start > secs(100.0) && view.start < secs(200.0), "{view:?}");
    assert_eq!(view.points_per_second, 100.0);

    let mut zoomed = harness(show_bar, Bar { view: start, rect: Rect::NOTHING });
    let end = pos2(rect.left() + thumb_min - 2.0, y);
    zoomed.drag(end, end + egui::vec2(100.0, 0.0), Modifiers::NONE);
    let view = zoomed.state().view;
    assert!(view.points_per_second < 10.0, "dragging the right end out zooms out: {view:?}");
    assert_eq!(view.start, Tick::ZERO, "the left end stays");
}

struct Marked {
    view: TimeView,
    markers: Vec<crate::markers::Marker>,
    edits: Vec<super::markers::MarkerEdit>,
    rect: Rect,
}

fn show_marked(ui: &mut Ui, app: &mut Marked) {
    let out = TimelineRuler::new("ruler", &mut app.view, format(), secs(60.0)).markers(&app.markers).show(ui);
    app.rect = out.response.rect;
    app.edits.extend(out.markers);
}

// a marker in the ruler's marker band is a list option, and Cmd-click deletes it.
#[test]
fn ruler_shows_markers() {
    use crate::markers::{Marker, MarkerColor, MarkerId};
    let marker =
        Marker { id: MarkerId(3), start: secs(2.0), duration: Tick::ZERO, name: "Cue".into(), comment: String::new(), color: MarkerColor(1) };
    let mut harness =
        harness(show_marked, Marked { view: TimeView::new(Tick::ZERO, 100.0), markers: vec![marker], edits: Vec::new(), rect: Rect::NOTHING });
    harness.get_by_role_and_label(Role::ListBoxOption, "Cue, 00:00:02:00");
    let rect = harness.state().rect;
    let on_flag = pos2(rect.left() + 200.0, rect.top() + rect.height() * 0.1);
    harness.click_at_with(on_flag, egui::PointerButton::Primary, Modifiers::COMMAND);
    assert_eq!(harness.state().edits, [super::markers::MarkerEdit::Delete(crate::markers::MarkerId(3))]);
}

struct Worked {
    view: TimeView,
    area: TimeRange,
    events: Vec<EditEvent<TimeRange>>,
    rect: Rect,
}

fn show_worked(ui: &mut Ui, app: &mut Worked) {
    let out = TimelineRuler::new("ruler", &mut app.view, format(), secs(60.0)).work_area(Some(app.area)).show(ui);
    app.rect = out.response.rect;
    app.events.extend(out.work_area);
}

fn worked() -> Harness<'static, Worked> {
    harness(
        show_worked,
        Worked { view: TimeView::new(Tick::ZERO, 100.0), area: TimeRange::new(secs(2.0), secs(2.0)), events: Vec::new(), rect: Rect::NOTHING },
    )
}

fn play(harness: &mut Harness<'_, Worked>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
    harness.run();
}

/// A point on the work-area bar at `seconds`.
fn on_bar(rect: Rect, seconds: f64) -> Pos2 {
    pos2(rect.left() + seconds as f32 * 100.0, rect.top() + style().ruler_marker_band + style().ruler_work_height / 2.0)
}

// the bar's middle moves the work area, its end moves that end; each is a gesture.
#[test]
fn dragging_the_work_area_moves_it_or_an_end() {
    let mut harness = worked();
    let rect = harness.state().rect;
    play(&mut harness, events::drag(on_bar(rect, 3.0), on_bar(rect, 3.5), Modifiers::NONE));
    let events = &harness.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(
        matches!(events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == TimeRange::new(secs(2.5), secs(2.0))),
        "{events:?}"
    );

    let mut harness = worked();
    play(&mut harness, events::drag(on_bar(rect, 4.0), on_bar(rect, 5.0), Modifiers::NONE));
    let events = &harness.state().events;
    assert!(
        matches!(events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == TimeRange::from_bounds(secs(2.0), secs(5.0))),
        "{events:?}"
    );
}

// a double-click on the bar fits it to what the ruler shows.
#[test]
fn double_clicking_the_work_area_sets_it_to_the_visible_span() {
    let mut harness = worked();
    let rect = harness.state().rect;
    play(&mut harness, events::click(on_bar(rect, 3.0), egui::PointerButton::Primary, 2, Modifiers::NONE));
    let view = harness.state().view;
    let expected = TimeRange::from_bounds(view.start, view.time_at(rect.right(), rect.left()));
    let events = &harness.state().events;
    assert!(matches!(events.last(), Some(EditEvent::Commit { value, changed: true, .. }) if *value == expected), "{events:?}");
}

// the wheel's zoom factor is FilmCraft's, `1 + dy / 100` within 0.5..=2.
#[test]
fn the_wheel_zoom_factor_follows_the_source() {
    use super::ruler::wheel_zoom_factor;
    assert_eq!(wheel_zoom_factor(0.0), 1.0);
    assert!((wheel_zoom_factor(10.0) - 1.1).abs() < 1e-6);
    assert!((wheel_zoom_factor(-10.0) - 0.9).abs() < 1e-6);
    assert_eq!(wheel_zoom_factor(500.0), 2.0);
    assert_eq!(wheel_zoom_factor(-500.0), 0.5);
}

// the ruler slider and the scroll bar tell AccessKit their numeric value and range.
#[test]
fn the_slider_and_the_scroll_bar_have_numeric_ranges() {
    let mut slider = ruler();
    slider.state_mut().playhead = secs(12.0);
    slider.run();
    let data = slider.get_by_role_and_label(Role::Slider, "Time Ruler").accesskit_node().data().clone();
    assert_eq!((data.numeric_value(), data.min_numeric_value(), data.max_numeric_value()), (Some(12.0), Some(0.0), Some(60.0)));

    let bar = harness(show_bar, Bar { view: TimeView::new(secs(5.0), 100.0), rect: Rect::NOTHING });
    let data = bar.get_by_role_and_label(Role::ScrollBar, "Zoom and Scroll").accesskit_node().data().clone();
    assert_eq!(data.numeric_value(), Some(5.0));
    assert_eq!(data.min_numeric_value(), Some(0.0));
    assert!(data.max_numeric_value().is_some_and(|max| max > 5.0));
}
