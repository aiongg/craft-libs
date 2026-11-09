//! `ReferencePoint`: the nine points are one Tab stop; the arrows move the focus over the grid in both directions and
//! stop at its edges; Space and Enter pick; AccessKit reports nine radios in one group with the pick checked.

use craft_behavior::{Anchor9, Mixed, PartKey};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
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

/// Tab enters the grid at the picked point, a second Tab leaves it, and Shift+Tab comes back to the picked point.
/// The arrows move the focus in both axes and stop at the edges without wrapping to the next row; Home and End go to
/// the first and last point; the pick stays until Space or Enter picks the focused point.
#[trial]
fn the_grid_is_one_tab_stop_and_the_arrows_move_the_focus_and_space_picks() {
    let mut rig = rig(Anchor9::Right);
    rig.press(Key::Tab);
    assert_eq!(rig.focused(), Some(at(Anchor9::Right)), "Tab arrives at the picked point");
    rig.press(Key::Tab);
    assert_eq!(rig.focused(), None, "the second Tab leaves the grid: nine points, one stop");
    rig.fixture.key(Key::Tab, Modifiers::SHIFT);
    rig.fixture.idle(2);
    assert_eq!(rig.focused(), Some(at(Anchor9::Right)), "Shift+Tab comes back to the picked point");

    let mut rig = self::rig(Anchor9::Centre);
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

/// AccessKit reports one radio group holding nine radios, named apart, with the picked point checked; a pick that is
/// not a single point (a mixed selection) checks no radio.
#[trial]
fn the_access_tree_has_nine_radios_in_one_group_with_the_pick_checked() {
    let rig = rig(Anchor9::BottomLeft);
    let tree = rig.fixture.tree();
    let groups = rig.fixture.by_role(Role::RADIO_GROUP);
    assert_eq!(groups.len(), 1);
    let radios = rig.fixture.by_role(Role::RADIO_BUTTON);
    assert_eq!(radios.len(), Anchor9::ALL.len());
    assert!(radios.iter().all(|radio| tree.parent_of_key(radio.node) == Some(groups[0].node)), "every radio sits in the group");
    let checked = radios.iter().filter(|radio| tree.toggled_of(radio.node) == Some(Mixed::Same(true))).count();
    assert_eq!(checked, 1, "one radio is checked");
    let mut names: Vec<_> = radios.iter().filter_map(|radio| tree.name_of(radio.node)).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), Anchor9::ALL.len(), "each point has its own name");

    let mut fixture = Fixture::new(|ui| {
        let mut mixed = Mixed::<Anchor9>::Mixed;
        ReferencePoint::new("Reference point", Bind::bound_mixed(&mut mixed)).show(ui);
    });
    fixture.settle();
    let radios = fixture.by_role(Role::RADIO_BUTTON);
    assert_eq!(radios.len(), Anchor9::ALL.len());
    assert!(radios.iter().all(|radio| fixture.tree().toggled_of(radio.node) != Some(Mixed::Same(true))));
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

/// The compact picker is an icon-sized grid that fits inside an options bar.
#[trial]
fn a_compact_picker_fits_inside_an_options_bar() {
    let mut fixture = Fixture::new(|ui| {
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            let mut point = Anchor9::Centre;
            ReferencePoint::new("Reference point", Bind::bound(&mut point)).compact().show(ui);
        });
    });
    fixture.settle();
    let picker = fixture.by_role(Role::RADIO_GROUP);
    let bar = fixture.by_role(Role::GROUP);
    let (picker, bar) = (picker.first().expect("the picker is shown"), bar.first().expect("the bar is shown"));
    assert!(bar.rect.contains_rect(picker.rect), "the compact picker sits inside the bar");
}
