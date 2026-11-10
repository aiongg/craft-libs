//! The timecode field and panel, the transport bar, the media controls, the frame fit and the render queue.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use craft_behavior::EditEvent;
use craft_timeline::rates::FrameRate;
use craft_timeline::time::Tick;
use craft_timeline::timecode::{TimeFormat, TimecodeError};
use craft_timeline::ui::{TimecodeField, format_eta};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _, SideLabel};
use craft_ui::form::Field;
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, vec2};

fn rate() -> FrameRate {
    FrameRate::new(24, 1)
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
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
            shown_events.borrow_mut().extend(out.edit.events);
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

/// Text that is not a time commits nothing and is reported.
#[trial]
fn a_time_that_does_not_parse_commits_nothing() {
    let mut rig = field(secs(10.0));
    rig.type_in("xyz");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.time.get(), secs(10.0));
    assert!(rig.errors.borrow().contains(&TimecodeError::NotATime("xyz".into())), "{:?}", rig.errors.borrow());
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
