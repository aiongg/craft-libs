//! `SwatchPair` wells take the theme's well shape (`shape.wells`): a round well is round to the pointer too, and round
//! wells line up along the host's axis.

use std::cell::Cell;
use std::rc::Rc;

use craft_behavior::{Axis, PartKey, PartRef};
use craft_color::ui::{PairWell, SwatchPair, Well};
use craft_theme::{Srgba, ThemeId};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::runtime::env::Direction;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Vec2, pos2, vec2};

/// Where a click lands on the primary well in theme `theme`: at the well's middle and just inside the corner of its
/// square. Each is the well the click reached, if any.
fn clicks(theme: &str) -> [Option<Well>; 2] {
    let clicked = Rc::new(Cell::new(None));
    let seen = clicked.clone();
    let mut fixture = Fixture::new(move |ui| {
        let wells = [PairWell::new("Foreground colour", Srgba::rgb(0.1, 0.3, 0.8)), PairWell::new("Background colour", Srgba::rgb(1.0, 1.0, 1.0))];
        let shown = SwatchPair::new("Colours", wells, Bind::given(Well::Primary)).popover(false).show(ui);
        if shown.clicked.is_some() {
            seen.set(shown.clicked);
        }
    });
    fixture.theme_id(ThemeId::parse(theme).expect("a theme id")).room(vec2(200.0, 200.0));
    fixture.settle();
    let well = fixture.by_role(Role::RADIO_BUTTON).first().expect("the primary well").rect;
    [well.center(), well.min + Vec2::splat(1.5)].map(|at| {
        clicked.set(None);
        fixture.click_at(at);
        fixture.settle();
        clicked.get()
    })
}

/// Adwaita's wells are round: a click in the corner of a well's square misses it. Pro's are square: it hits.
#[trial]
fn the_well_corner_follows_the_theme_well_shape() {
    assert_eq!(clicks("craft.adwaita"), [Some(Well::Primary), None], "round wells");
    assert_eq!(clicks("craft.pro"), [Some(Well::Primary), Some(Well::Primary)], "square wells");
}

/// A tool rail's pair, as the framework placed it: the pair's rect, the wells' (primary first) and the default and swap
/// buttons'.
struct Placed {
    pair: Rect,
    wells: [Rect; 2],
    buttons: [Rect; 2],
}

fn rail(theme: &str, axis: Axis, direction: Direction) -> Placed {
    let shown = Rc::new(Cell::new(None));
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        let wells = [PairWell::new("Foreground colour", Srgba::rgb(0.1, 0.3, 0.8)), PairWell::new("Background colour", Srgba::rgb(1.0, 1.0, 1.0))];
        let out = SwatchPair::new("Colours", wells, Bind::given(Well::Primary))
            .popover(false)
            .click_activates(false)
            .default_button("Default colours")
            .swap_button("Swap colours")
            .lineup(axis)
            .show(ui);
        seen.set(Some((out.response.id, out.response.rect)));
    });
    fixture.theme_id(ThemeId::parse(theme).expect("a theme id")).direction(direction).room(vec2(200.0, 200.0));
    fixture.settle();
    let (id, pair) = shown.get().expect("the pair showed");
    let part = |part: PartRef| fixture.part_rect(id, part).expect("a part of the pair");
    let well = PartKey::new("well");
    Placed { pair, wells: [part(well.at(0)), part(well.at(1))], buttons: [part(PartKey::new("default").into()), part(PartKey::new("swap").into())] }
}

/// Whether `button` keeps clear of the round well `well`: no point of it inside the circle (its square may cross).
fn clear_of(button: Rect, well: Rect) -> bool {
    let centre = well.center();
    let nearest = pos2(centre.x.clamp(button.left(), button.right()), centre.y.clamp(button.top(), button.bottom()));
    nearest.distance(centre) >= well.width() / 2.0 - 0.01
}

/// Round wells (Adwaita) line up along the host's axis: stacked down a one-column rail, side by side across a
/// two-column one, which is the shorter; square wells (Pro) keep the diagonal. The buttons stay inside the pair, clear
/// of the wells' circles and of each other; the default button sits on the top well's shoulder.
#[trial]
fn round_wells_line_up_along_the_hosts_axis() {
    let column = rail("craft.adwaita", Axis::Y, Direction::Ltr);
    let [top, under] = column.wells;
    assert!((top.center().x - under.center().x).abs() < 0.5 && under.top() > top.top(), "stacked: {top:?} {under:?}");
    let [default, swap] = column.buttons;
    assert!(default.right() > top.right() && default.top() < top.center().y, "default beside the top well's upper end: {default:?}");
    assert!(default.left() < top.right() && default.bottom() > top.top(), "default on the top well's shoulder, not above it: {default:?}");
    assert!(swap.top() >= under.bottom() - 0.5, "swap under the wells: {swap:?}");

    let row = rail("craft.adwaita", Axis::X, Direction::Ltr);
    let [start, end] = row.wells;
    assert!((start.top() - end.top()).abs() < 0.5 && end.left() > start.left(), "side by side: {start:?} {end:?}");
    assert!(row.buttons[0].left() < start.left() && row.buttons.iter().all(|b| b.top() >= start.bottom() - 0.5), "buttons under");
    assert!(row.pair.height() < column.pair.height(), "side by side is shorter than stacked");

    let mirrored = rail("craft.adwaita", Axis::X, Direction::Rtl);
    assert!(mirrored.wells[1].left() < mirrored.wells[0].left() && mirrored.buttons[0].right() > mirrored.wells[0].right(), "mirrored");

    for placed in [&column, &row, &mirrored] {
        let [default, swap] = placed.buttons;
        assert!(!default.intersect(swap).is_positive(), "the buttons overlap: {default:?} {swap:?}");
        for button in placed.buttons {
            assert!(placed.pair.expand(0.5).contains_rect(button), "a button outside the pair: {button:?} {:?}", placed.pair);
            assert!(placed.wells.iter().all(|well| clear_of(button, *well)), "a button over a well: {button:?}");
        }
    }

    for axis in [Axis::Y, Axis::X] {
        let [primary, secondary] = rail("craft.pro", axis, Direction::Ltr).wells;
        assert!(secondary.left() > primary.left() && secondary.top() > primary.top(), "square wells keep the diagonal ({axis:?})");
    }
}
