//! `Fixture::frame_through`: a quick drag whose press, moves and release reach one OS frame begins at the press and
//! commits, as the app's follow-up frames deliver it.

use super::kit::{AT, Ev, FIRST, INITIAL, Scrub, ev_of, near};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[trial]
fn a_press_moves_and_release_in_one_frame_begin_the_drag_and_commit_it() {
    let value = Rc::new(Cell::new(INITIAL));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (shown, log) = (value.clone(), seen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        let out = Scrub::new(Bind::bound(&mut current)).id_salt("scrub").show_at(ui, AT);
        shown.set(current);
        log.borrow_mut().extend(ev_of(&out.event));
    });
    let start = Pos2::new(AT.left() + 10.0, AT.center().y);
    let to = start + Vec2::new(FIRST - 10.0, 0.0);
    fixture.idle(2);
    fixture.frame(vec![moved(start)]);
    fixture.frame_through(vec![moved(start), press(start), moved(start + Vec2::new(15.0, 0.0)), moved(to), release(to)]);
    let events = seen.borrow().clone();
    assert_eq!(events.first(), Some(&Ev::Begin), "the drag starts in a pass of its own: {events:?}");
    assert!(matches!(events.last(), Some(Ev::Commit(after)) if near(*after, FIRST)), "{events:?}");
    assert!(near(value.get(), FIRST), "{}", value.get());
}
