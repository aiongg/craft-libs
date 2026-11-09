//! `ReferencePoint`: the nine points are one Tab stop; the arrows move the focus over the grid in both directions and
//! stop at its edges; Space and Enter pick; AccessKit reports nine radios in one group with the pick checked.

use craft_behavior::{Anchor9, PartKey};
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::pickers::ReferencePoint;
use craft_ui::testing::focus::active_part;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    point: Rc<Cell<Anchor9>>,
    grid: Rc<Cell<Option<Id>>>,
}

fn rig(start: Anchor9) -> Rig {
    let point = Rc::new(Cell::new(start));
    let grid = Rc::new(Cell::new(None));
    let (shown, id) = (point.clone(), grid.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (mut picked, mut after) = (shown.get(), false);
        id.set(Some(ReferencePoint::new("Reference point", Bind::bound(&mut picked)).id_salt("grid").show(ui).response.id));
        Checkbox::new(Bind::bound(&mut after)).label("After").show(ui);
        shown.set(picked);
    });
    fixture.settle();
    Rig { fixture, point, grid }
}

impl Rig {
    fn press(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
        self.fixture.idle(2);
    }

    /// The index in `Anchor9::ALL` of the point that holds the focus, `None` when focus is outside the grid.
    fn focused(&self) -> Option<usize> {
        let part = active_part(self.fixture.ctx(), self.grid.get()?)?;
        (part.key == PartKey::new("cell")).then_some(part.index? as usize)
    }
}

fn at(point: Anchor9) -> usize {
    Anchor9::ALL.iter().position(|candidate| *candidate == point).unwrap_or_default()
}

/// The arrows move the focus in both axes and stop at the edges without wrapping to the next row; Home and End go to
/// the first and last point; the pick stays until Space or Enter picks the focused point.
#[trial]
fn arrows_move_the_focus_and_stop_at_the_edges_and_space_picks() {
    let mut rig = rig(Anchor9::Centre);
    rig.press(Key::Tab);
    let walk = [
        (Key::ArrowRight, Anchor9::Right),
        (Key::ArrowRight, Anchor9::Right),
        (Key::ArrowDown, Anchor9::BottomRight),
        (Key::ArrowDown, Anchor9::BottomRight),
        (Key::ArrowLeft, Anchor9::Bottom),
        (Key::ArrowLeft, Anchor9::BottomLeft),
        (Key::ArrowLeft, Anchor9::BottomLeft),
        (Key::ArrowUp, Anchor9::Left),
        (Key::ArrowUp, Anchor9::TopLeft),
        (Key::ArrowUp, Anchor9::TopLeft),
        (Key::End, Anchor9::BottomRight),
        (Key::Home, Anchor9::TopLeft),
    ];
    for (key, expected) in walk {
        rig.press(key);
        assert_eq!(rig.focused(), Some(at(expected)), "{key:?} moves the focus to {expected:?}");
    }
    assert_eq!(rig.point.get(), Anchor9::Centre, "moving the focus does not pick");
    rig.press(Key::ArrowRight);
    rig.press(Key::Space);
    assert_eq!(rig.point.get(), Anchor9::Top, "Space picks the focused point");
    rig.press(Key::ArrowDown);
    rig.press(Key::Enter);
    assert_eq!(rig.point.get(), Anchor9::Centre, "Enter picks the focused point");
}

/// An arrow shows in a cell next to the pick, away from the pick where the box grows and toward it where it shrinks;
/// cells farther away show none.
#[trial]
fn growth_arrows_follow_the_sign_of_the_change() {
    use Anchor9::*;
    assert_eq!(ReferencePoint::arrow(TopLeft, Right, (10.0, 10.0)), None);
    assert_eq!(ReferencePoint::arrow(TopLeft, Top, (10.0, 10.0)), Some((1, 0)));
    assert_eq!(ReferencePoint::arrow(TopLeft, Centre, (10.0, 10.0)), Some((1, 1)));
    assert_eq!(ReferencePoint::arrow(TopLeft, Centre, (-10.0, 10.0)), Some((-1, 1)));
    assert_eq!(ReferencePoint::arrow(Centre, TopLeft, (-10.0, -10.0)), Some((1, 1)));
    assert_eq!(ReferencePoint::arrow(Centre, Centre, (10.0, 10.0)), None);
}
