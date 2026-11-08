//! The timecode field and panel, the transport bar, the media controls, the frame fit and the render queue.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use craft_behavior::{EditEvent, IconId, Selection};
use craft_timeline::rates::{FrameRate, PixelAspect};
use craft_timeline::time::Tick;
use craft_timeline::timecode::{TimeDisplay, TimeFormat, TimecodeError};
use craft_timeline::ui::{
    FrameFit, MediaControls, RenderItemId, RenderQueueEvent, RenderQueueItem, RenderQueueList, RenderStatus, TimecodeField, TimecodePanel,
    TimecodePanelSettings, TimecodeRow, TimecodeText, TransportBar, TransportBarSettings, TransportButton, format_eta, frame_rect,
};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _, SideLabel};
use craft_ui::form::Field;
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Rect, pos2, vec2};

fn rate() -> FrameRate {
    FrameRate::new(24, 1)
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

/// The nodes named `label`.
fn named(fixture: &Fixture, label: &str) -> Vec<Target> {
    let tree = fixture.tree();
    tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some(label)).filter_map(|key| fixture.target(key)).collect()
}

fn secondary_click(fixture: &mut Fixture, at: egui::Pos2) {
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.idle(2);
}

/// A field at 10 s whose time, edit events and errors the test reads.
struct FieldRig {
    fixture: Fixture,
    time: Rc<Cell<Tick>>,
    events: Rc<RefCell<Vec<EditEvent<Tick>>>>,
    errors: Rc<RefCell<Vec<TimecodeError>>>,
}

fn field(start: Tick) -> FieldRig {
    let time = Rc::new(Cell::new(start));
    let events = Rc::new(RefCell::new(Vec::new()));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let (shown_time, shown_events, shown_errors) = (time.clone(), events.clone(), errors.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown_time.get();
        Field::new("Time").side(SideLabel::new().scrub(true)).show(ui, |ui, _| {
            let out = TimecodeField::new("Time", Bind::bound(&mut value), TimeFormat::timecode(rate())).show(ui);
            shown_events.borrow_mut().extend(out.edit.event);
            shown_errors.borrow_mut().extend(out.error);
        });
        shown_time.set(value);
    });
    fixture.settle();
    FieldRig { fixture, time, events, errors }
}

impl FieldRig {
    fn input(&self) -> Target {
        self.fixture.by_role(Role::SPIN_BUTTON).first().cloned().expect("the field is a spin button")
    }

    fn type_in(&mut self, text: &str) {
        let input = self.input();
        self.fixture.click(&input);
        self.fixture.type_text(text);
    }
}

/// A drag on the side label moves the time a frame per point, ten times that with Shift, and never before zero.
#[trial]
fn dragging_the_field_scrubs_a_frame_per_point() {
    for (modifiers, frames) in [(Modifiers::NONE, 10), (Modifiers::SHIFT, 100)] {
        let mut rig = field(secs(10.0));
        let label = rig.fixture.by_role(Role::LABEL).into_iter().find(|target| rig.fixture.tree().name_of(target.node) == Some("Time")).unwrap();
        rig.fixture.hold(modifiers);
        rig.fixture.drag(&label, vec2(10.0, 0.0));
        assert_eq!(rig.time.get(), rate().tick_of(240 + frames), "{:?}", rig.events.borrow());
        assert!(matches!(rig.events.borrow().first(), Some(EditEvent::Begin { .. })));
    }
    let mut rig = field(Tick::ZERO);
    let label = rig.fixture.by_role(Role::LABEL).into_iter().find(|target| rig.fixture.tree().name_of(target.node) == Some("Time")).unwrap();
    rig.fixture.drag(&label, vec2(-30.0, 0.0));
    assert!(rig.time.get() >= Tick::ZERO, "never before the sequence start");
}

/// The large readout has no side label: a drag on the field itself moves the time a frame per point.
#[trial]
fn dragging_the_large_readout_scrubs_a_frame_per_point() {
    let time = Rc::new(Cell::new(secs(10.0)));
    let shown = time.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut value = shown.get();
        TimecodeField::new("Time", Bind::bound(&mut value), TimeFormat::timecode(rate())).large().show(ui);
        shown.set(value);
    });
    fixture.settle();
    let input = fixture.by_role(Role::SPIN_BUTTON).first().cloned().expect("the readout is a spin button");
    fixture.drag(&input, vec2(10.0, 0.0));
    assert_eq!(time.get(), rate().tick_of(250));
}

/// Typing +15 commits fifteen frames later and 00:00:20:00 commits twenty seconds; Escape commits nothing.
#[trial]
fn typing_commits_relative_and_absolute_times() {
    let mut rig = field(secs(10.0));
    rig.type_in("+15");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.time.get(), rate().tick_of(255));

    let mut rig = field(secs(10.0));
    rig.type_in("00:00:20:00");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.time.get(), secs(20.0));

    let mut rig = field(secs(10.0));
    rig.type_in("+15");
    rig.fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.time.get(), secs(10.0));
}

/// Text that is not a time commits nothing and is reported.
#[trial]
fn a_time_that_does_not_parse_commits_nothing() {
    let mut rig = field(secs(10.0));
    rig.type_in("xyz");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.time.get(), secs(10.0));
    assert!(rig.errors.borrow().contains(&TimecodeError::NotATime("xyz".into())), "{:?}", rig.errors.borrow());
}

/// Left on the whole selected text puts the caret on its last digit: Backspace removes the last digit, and the digit
/// typed after it ends the time.
#[trial]
fn left_on_the_selected_value_goes_to_its_last_digit() {
    let mut rig = field(secs(10.0));
    let input = rig.input();
    rig.fixture.click(&input);
    rig.fixture.key(Key::ArrowLeft, Modifiers::NONE);
    rig.fixture.key(Key::Backspace, Modifiers::NONE);
    rig.fixture.type_text("5");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.time.get(), rate().tick_of(245));
}

struct PanelRig {
    fixture: Fixture,
    settings: Rc<RefCell<TimecodePanelSettings>>,
    changed: Rc<Cell<usize>>,
}

fn panel(rows: Vec<TimecodeRow>) -> PanelRig {
    let settings = Rc::new(RefCell::new(TimecodePanelSettings::new(rows)));
    let changed = Rc::new(Cell::new(0));
    let (shown, count) = (settings.clone(), changed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut local = shown.borrow().clone();
        let text = TimecodeText::new(|row| format!("{:?}", row.kind));
        let out = TimecodePanel::new("Timecode", Bind::bound(&mut local), text).show(ui);
        count.set(count.get() + usize::from(out.settings_changed));
        *shown.borrow_mut() = local;
    });
    fixture.settle();
    PanelRig { fixture, settings, changed }
}

/// A row that shows its own display can go back to the sequence's display.
#[trial]
fn a_row_can_return_to_the_sequence_display() {
    let mut rig = panel(vec![TimecodeRow::default().display(Some(TimeDisplay::Frames))]);
    let row = rig.fixture.by_label("Current Time \u{b7} Active Monitor: CurrentTime").expect("the row has a node");
    secondary_click(&mut rig.fixture, row.center());
    let item = rig.fixture.by_label("Sequence Display").expect("the row menu is open");
    rig.fixture.click(&item);
    assert_eq!(rig.settings.borrow().rows[0].display, None);
    assert_eq!(rig.changed.get(), 1);
}

/// "+" adds a row; the row menu's Remove Row removes it.
#[trial]
fn panel_rows_can_be_added_and_removed() {
    let mut rig = panel(vec![TimecodeRow::default()]);
    let add = rig.fixture.by_label("Add a timecode row").expect("the add button");
    rig.fixture.click(&add);
    assert_eq!(rig.settings.borrow().rows.len(), 2);
    assert_eq!(rig.changed.get(), 1);
    let second = rig.fixture.by_label("Duration \u{b7} Active Monitor: Duration").expect("the second row");
    secondary_click(&mut rig.fixture, second.center());
    let remove = rig.fixture.by_label("Remove Row").expect("the row menu is open");
    rig.fixture.click(&remove);
    assert_eq!(rig.settings.borrow().rows, vec![TimecodeRow::default()]);
    assert_eq!(rig.changed.get(), 2);
}

struct BarRig {
    fixture: Fixture,
    settings: Rc<RefCell<TransportBarSettings>>,
    pressed: Rc<RefCell<Vec<TransportButton>>>,
    changed: Rc<Cell<usize>>,
}

fn bar() -> BarRig {
    let settings = Rc::new(RefCell::new(TransportBarSettings::default()));
    let pressed = Rc::new(RefCell::new(Vec::new()));
    let changed = Rc::new(Cell::new(0));
    let (shown, presses, count) = (settings.clone(), pressed.clone(), changed.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut local = shown.borrow().clone();
        let out = TransportBar::new("Program transport", &TransportButton::PROGRAM).settings(Bind::bound(&mut local)).show(ui);
        presses.borrow_mut().extend(out.pressed);
        count.set(count.get() + usize::from(out.settings_changed));
        *shown.borrow_mut() = local;
    });
    fixture.settle();
    BarRig { fixture, settings, pressed, changed }
}

/// Play shows as Pause while playing; a click reports the button.
#[trial]
fn buttons_report_presses_and_show_pause_while_playing() {
    assert_eq!(TransportButton::PlayStop.icon(false), IconId::new("play"));
    assert_eq!(TransportButton::PlayStop.icon(true), IconId::new("pause"));
    let mut rig = bar();
    for label in ["Play-Stop Toggle (Space)", "Mark In (I)"] {
        let button = rig.fixture.by_label(label).expect("the button");
        rig.fixture.click(&button);
    }
    assert_eq!(*rig.pressed.borrow(), [TransportButton::PlayStop, TransportButton::MarkIn]);
}

/// Hidden buttons are left out; the editor's checkboxes and Reset change the hidden set.
#[trial]
fn hidden_buttons_are_left_out_and_the_editor_changes_them() {
    let mut rig = bar();
    assert!(rig.fixture.by_label("Loop").is_some(), "Loop shows until it is hidden");
    rig.settings.borrow_mut().hidden.push(TransportButton::Loop);
    rig.fixture.settle();
    assert!(rig.fixture.by_label("Loop").is_none());
    assert!(rig.fixture.by_label("Mark In (I)").is_some());

    let editor = rig.fixture.by_label("Button Editor").expect("the editor button");
    rig.fixture.click(&editor);
    rig.fixture.idle(2);
    let check = rig.fixture.by_role(Role::CHECK_BOX).into_iter().find(|t| rig.fixture.tree().name_of(t.node) == Some("Mark In (I)")).unwrap();
    rig.fixture.click(&check);
    assert_eq!(rig.settings.borrow().hidden, [TransportButton::Loop, TransportButton::MarkIn]);
    assert_eq!(rig.changed.get(), 1);

    let reset = rig.fixture.by_label("Reset").expect("Reset");
    rig.fixture.click(&reset);
    assert!(rig.settings.borrow().hidden.is_empty());
    assert_eq!(rig.changed.get(), 2);
}

/// The media position, the media length, and the arrow nudge, in seconds.
const MEDIA_POSITION: f64 = 6.5;
const MEDIA_LENGTH: f64 = 10.0;
const MEDIA_NUDGE: f64 = 0.25;
/// How far from the middle a click on the seek track may land and still count as the middle.
const MIDDLE_SLACK: f64 = 2.0;

/// Play toggles, the arrows nudge a quarter second, the track seeks.
#[trial]
fn media_controls_toggle_nudge_and_seek() {
    let toggles = Rc::new(Cell::new(0));
    let nudges = Rc::new(RefCell::new(Vec::new()));
    let seeks = Rc::new(RefCell::new(Vec::new()));
    let (shown_toggles, shown_nudges, shown_seeks) = (toggles.clone(), nudges.clone(), seeks.clone());
    let mut fixture = Fixture::new(move |ui| {
        let out = MediaControls::new("Media controls", MEDIA_POSITION, MEDIA_LENGTH).trim((2.0, 8.0)).show(ui);
        shown_toggles.set(shown_toggles.get() + usize::from(out.toggle));
        shown_nudges.borrow_mut().extend(out.nudge);
        shown_seeks.borrow_mut().extend(out.seek);
    });
    fixture.settle();
    for label in ["Play or pause media", "Back 0.25 seconds", "Forward 0.25 seconds"] {
        let button = fixture.by_label(label).expect("the button");
        fixture.click(&button);
    }
    assert_eq!(toggles.get(), 1);
    assert_eq!(*nudges.borrow(), [-MEDIA_NUDGE, MEDIA_NUDGE]);

    let seek = fixture.by_role(Role::SLIDER).first().cloned().expect("the seek slider");
    fixture.click(&seek);
    let value = seeks.borrow().iter().find_map(|event| event.value().copied()).expect("a click on the track seeks");
    let near_middle = (value - MEDIA_LENGTH / 2.0).abs() <= MIDDLE_SLACK;
    assert!(near_middle, "a click mid-track seeks about the middle: {value}");

    seeks.borrow_mut().clear();
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(seeks.borrow().iter().find_map(|event| event.value().copied()), Some(MEDIA_POSITION + MEDIA_NUDGE), "Right moves a quarter second");
}

/// A 4:3 frame in a 16:9 box is centred at full height.
#[trial]
fn letterbox_keeps_the_aspect() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0));
    let (target, uv) = frame_rect(area, vec2(640.0, 480.0), FrameFit::Letterbox);
    assert_eq!(target, Rect::from_min_size(pos2(200.0, 0.0), vec2(1200.0, 900.0)));
    assert_eq!(uv, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
}

/// A cropped frame fills the box with the crop as its texture coordinates, clamped.
#[trial]
fn crop_fills_the_box() {
    let area = Rect::from_min_size(pos2(10.0, 20.0), vec2(160.0, 90.0));
    let (target, uv) = frame_rect(area, vec2(640.0, 480.0), FrameFit::Crop([0.25, 0.1, f32::NAN, 5.0]));
    assert_eq!(target, area);
    assert_eq!(uv, Rect::from_min_max(pos2(0.25, 0.1), pos2(1.0, 0.11)));
}

struct QueueRig {
    fixture: Fixture,
    events: Rc<RefCell<Vec<RenderQueueEvent>>>,
}

fn item(id: u64, status: RenderStatus) -> RenderQueueItem {
    RenderQueueItem::new(RenderItemId::new(id), "Sequence 01", format!("out{id}.mp4"), "Match Source", status)
}

fn queue(items: Vec<RenderQueueItem>, running: bool) -> QueueRig {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shown = events.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut selection = Selection::<RenderItemId>::default();
        let out = RenderQueueList::new("Destinations", &items, Bind::bound(&mut selection)).running(running).show(ui);
        shown.borrow_mut().extend(out.events);
    });
    fixture.settle();
    QueueRig { fixture, events }
}

fn enabled(rig: &QueueRig, label: &str) -> bool {
    let target = &named(&rig.fixture, label)[0];
    !rig.fixture.tree().is_disabled_key(target.node)
}

/// Cancel for Ready and Encoding rows, Retry otherwise, and no Remove on the encoding row.
#[trial]
fn row_buttons_follow_the_status() {
    let encoding = RenderStatus::encoding(0.4, Some(Duration::from_secs(75)));
    let items =
        vec![item(1, RenderStatus::Ready), item(2, encoding), item(3, RenderStatus::Failed("disk full".to_owned())), item(4, RenderStatus::Done)];
    let mut rig = queue(items, true);
    assert_eq!(named(&rig.fixture, "Cancel").len(), 2);
    assert_eq!(named(&rig.fixture, "Retry").len(), 2);
    assert_eq!(named(&rig.fixture, "Remove").len(), 3, "the encoding row has no Remove");
    let retry = named(&rig.fixture, "Retry")[0].clone();
    rig.fixture.click(&retry);
    assert_eq!(*rig.events.borrow(), vec![RenderQueueEvent::Retry(RenderItemId::new(3))]);
}

/// Start Queue with a Ready item while stopped, Stop while running, Clear Finished with a finished item.
#[trial]
fn footer_buttons_follow_the_queue() {
    let stopped = queue(vec![item(1, RenderStatus::Ready)], false);
    assert!(enabled(&stopped, "Start Queue"));
    assert!(!enabled(&stopped, "Stop"));
    assert!(!enabled(&stopped, "Clear Finished"));

    let mut running = queue(vec![item(1, RenderStatus::Ready), item(2, RenderStatus::Done)], true);
    assert!(!enabled(&running, "Start Queue"));
    assert!(enabled(&running, "Stop"));
    assert!(enabled(&running, "Clear Finished"));
    let clear = named(&running.fixture, "Clear Finished")[0].clone();
    running.fixture.click(&clear);
    assert_eq!(*running.events.borrow(), vec![RenderQueueEvent::ClearFinished]);
}

/// Under a minute the time reads "45 s" (the count word), then m:ss and h:mm:ss, rounded up to the second.
#[trial]
fn time_left_reads_seconds_then_clock_digits() {
    let read = Rc::new(RefCell::new(Vec::new()));
    let shown = read.clone();
    let mut fixture = Fixture::new(move |ui| {
        let env = Env::of(ui);
        let eta = |millis: u64| format_eta(&env, Duration::from_millis(millis));
        *shown.borrow_mut() = [45_000, 44_200, 59_001, 125_000, 3_725_000].map(eta).to_vec();
    });
    fixture.settle();
    assert_eq!(*read.borrow(), ["45 s", "45 s", "1:00", "2:05", "1:02:05"]);
}

#[trial]
fn rate_and_aspect_labels_trim_trailing_zeros() {
    let labels = Rc::new(RefCell::new(Vec::<String>::new()));
    let shown = labels.clone();
    let mut fixture = Fixture::new(move |ui| {
        let units = Env::of(ui).units();
        let mut all: Vec<String> =
            [FrameRate::FPS_24, FrameRate::FPS_23_976, FrameRate::FPS_29_97, FrameRate::FPS_59_94].map(|rate| rate.label(&units)).into();
        all.push(PixelAspect::SQUARE.ratio_label(&units));
        all.push(PixelAspect { num: 10, den: 11 }.ratio_label(&units));
        *shown.borrow_mut() = all;
    });
    fixture.settle();
    assert_eq!(*labels.borrow(), ["24", "23.976", "29.97", "59.94", "1.0", "0.9091"]);
}
