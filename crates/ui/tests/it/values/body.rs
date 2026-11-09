//! A binding read and written from a container body, where no pipeline step runs for it.

use super::kit::PlainStyle;
use craft_behavior::{EditEvent, GestureId, Mixed};
use craft_ui::contract::{Bind, Common, Component, Container, ContainerProps, Cx, Role, ShowContainer as _, Validity};
use craft_ui::runtime::env::Message;
use craft_ui::style::StyleCx;
use craft_ui::testing::{Frames, trial};
use egui::{Response, Ui};
use std::cell::{Cell, RefCell};

#[derive(Component)]
#[component(container)]
struct Host {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl Host {
    fn new() -> Self {
        Host { common: Common::new(), props: ContainerProps::new() }
    }
}

impl Component for Host {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HostStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Host {}

fn set(value: f32) -> EditEvent<f32> {
    EditEvent::set(GestureId::default(), Mixed::Same(0.0), value, "Set".into(), None)
}

/// Runs `body` in a host container for several frames; `body` gets the frame number.
fn frames_of(count: usize, mut body: impl FnMut(&mut craft_ui::contract::BodyCx<'_>, usize)) {
    let mut frames = Frames::new();
    let frame = Cell::new(0);
    let mut show = |ui: &mut Ui| {
        Host::new().show(ui, |_, cx| body(cx, frame.get()));
    };
    for at in 0..count {
        frame.set(at);
        frames.idle(1, &mut show);
    }
}

/// A kept binding written through the body context reads back the written value in the next frames.
#[trial]
fn a_kept_binding_written_in_a_body_reads_back_next_frame() {
    let seen = RefCell::new(vec![None; 4]);
    frames_of(4, |cx, frame| {
        let mut bind = Bind::<f32>::kept();
        let mut bind = bind.reborrow();
        // The body runs in a measuring run and then the real run; the last read of a frame is the real run's.
        seen.borrow_mut()[frame] = bind.read(cx);
        if frame == 1 {
            assert!(bind.write(cx, &set(5.0)), "a new value is a change");
            assert!(!bind.write(cx, &set(5.0)), "the same value is not");
        }
    });
    let seen = seen.take();
    assert_eq!(seen[0], None, "nothing kept yet");
    assert_eq!(seen[3], Some(Mixed::Same(5.0)));
}

/// A given binding never writes: the caller reports the event.
#[trial]
fn a_given_binding_is_not_written_in_a_body() {
    let seen = RefCell::new(None);
    frames_of(2, |cx, _| {
        let mut bind = Bind::given(2.0f32);
        let mut bind = bind.reborrow();
        assert!(!bind.write(cx, &set(5.0)));
        *seen.borrow_mut() = bind.read(cx);
    });
    assert_eq!(seen.take(), Some(Mixed::Same(2.0)));
}

/// A refused value writes nothing; a fixed one writes the fix.
#[trial]
fn a_check_decides_what_a_body_write_stores() {
    let seen = RefCell::new(Vec::new());
    frames_of(3, |cx, frame| {
        let check = |v: &f32| match *v {
            v if v > 10.0 => Validity::Refuse(Message::new("too big")),
            v if v < 0.0 => Validity::Fix(0.0),
            _ => Validity::Ok,
        };
        let mut bind = Bind::<f32>::kept().check(check);
        let mut bind = bind.reborrow();
        match frame {
            0 => assert!(!bind.write(cx, &set(50.0)), "refused"),
            1 => assert!(bind.write(cx, &set(-3.0)), "fixed"),
            _ => {}
        }
        seen.borrow_mut().push(bind.read(cx));
    });
    let seen = seen.take();
    assert_eq!(seen[0], None, "the refused value was not stored");
    assert_eq!(seen[2], Some(Mixed::Same(0.0)), "the fixed value was");
}
