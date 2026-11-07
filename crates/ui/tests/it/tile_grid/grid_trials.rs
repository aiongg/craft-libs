//! The tile grid in use: cards keep their pictures' aspect, the grid decides where captions go, navigation skips
//! hidden tiles, and ratings, checks and activation each have one route.

use craft_behavior::{DisplayOrder, ItemSource, Selection, SizeHint};
use craft_ui::contract::{ActivateOn, Bind, CollectionEvent, ShowCollection as _};
use craft_ui::list::{ThumbAspect, ToggleState};
use craft_ui::testing::{Fixture, trial};
use craft_ui::tile_grid::{BadgeVisibility, CaptionPlace, GridEvent, RatingKeys, Tile, TileGrid, TileGroup, TilePartEvent};
use egui::{Key, Modifiers, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const NAMES: [&str; 12] = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet", "kilo", "lima"];
/// Width over height of every picture: a 3:2 photo.
const ASPECT: f32 = 1.5;
/// How far a measured aspect may differ from the picture's.
const CLOSE: f32 = 0.02;
const ROOM: Vec2 = Vec2::new(520.0, 420.0);

/// Tiles named after `NAMES`, every picture 3:2.
struct Photos(usize);

impl DisplayOrder<usize> for Photos {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(NAMES[at].into())
    }
}

impl ItemSource<usize> for Photos {
    fn size_hint(&self, _at: usize) -> SizeHint {
        SizeHint::Aspect(ASPECT)
    }
}

/// What the last frames of a grid showed and reported.
#[derive(Default)]
struct Seen {
    pictures: Vec<(usize, Rect)>,
    cards: Vec<(usize, Rect)>,
    events: Vec<CollectionEvent<usize, TilePartEvent>>,
    grid: Vec<GridEvent<usize>>,
    selection: Selection<usize>,
}

impl Seen {
    fn picture(&self, key: usize) -> Rect {
        self.pictures.iter().rev().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the picture was painted")
    }

    fn card(&self, key: usize) -> Rect {
        self.cards.iter().rev().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the card was shown")
    }

    /// The ratings the grid reported: the tiles and the rating.
    fn rates(&self) -> Vec<(Vec<usize>, u8)> {
        self.grid
            .iter()
            .filter_map(|event| if let GridEvent::Rate { keys, rating, .. } = event { Some((keys.clone(), *rating)) } else { None })
            .collect()
    }

    fn items(&self) -> impl Iterator<Item = &TilePartEvent> {
        self.events.iter().filter_map(|event| if let CollectionEvent::Item(_, item) = event { Some(item) } else { None })
    }
}

/// A grid of `count` tiles in a fixture. `build` sets the grid up; `dress` the tile of a key.
fn rig(
    count: usize,
    build: impl Fn(TileGrid<'_, usize>) -> TileGrid<'_, usize> + 'static,
    dress: impl Fn(usize, Tile<'static>) -> Tile<'static> + 'static,
) -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut selection = shown.borrow().selection.clone();
        let grid = build(TileGrid::new("Library")).selection(Bind::bound(&mut selection));
        let photos = Photos(count);
        let out = grid.show_items(ui, &photos, &mut |key, cx| {
            shown.borrow_mut().cards.push((*key, cx.rect));
            let record = shown.clone();
            let at = *key;
            let tile = Tile::new(NAMES[at]).thumb(ThumbAspect::new(3, 2), move |_, rect| record.borrow_mut().pictures.push((at, rect)));
            dress(at, tile)
        });
        let mut seen = shown.borrow_mut();
        seen.selection = selection;
        seen.events.extend(out.response.events);
        seen.grid.extend(out.events);
    });
    fixture.room(ROOM);
    fixture.settle();
    (fixture, seen)
}

fn plain(_: usize, tile: Tile<'static>) -> Tile<'static> {
    tile
}

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("{label} is a node"));
    fixture.click(&target);
    fixture.idle(2);
}

/// A 3:2 picture is 3:2 wherever the tiling puts it: the card is the picture plus what the tile adds around it in
/// equal cards, justified rows and groups alike.
#[trial]
fn a_three_by_two_picture_keeps_its_aspect_in_every_tiling() {
    type Tiling = fn(TileGrid<'_, usize>) -> TileGrid<'_, usize>;
    let tilings: [(&str, Tiling); 3] = [
        ("equal", |grid| grid.aspect(ASPECT)),
        ("justified", |grid| grid.justified()),
        ("grouped", |grid| {
            let groups: &'static [TileGroup] = Box::leak(Box::new([TileGroup::new(0, 6, "A"), TileGroup::new(6, 6, "B")]));
            grid.groups(groups).aspect(ASPECT)
        }),
    ];
    for (name, build) in tilings {
        let (_fixture, seen) = rig(12, build, plain);
        let seen = seen.borrow();
        assert!(!seen.pictures.is_empty(), "{name}: pictures were painted");
        for (key, picture) in &seen.pictures {
            let aspect = picture.width() / picture.height();
            assert!((aspect - ASPECT).abs() < CLOSE, "{name}: tile {key} is {aspect} wide over high");
        }
    }
}

/// The grid says where captions go: a default tile follows it. A start caption sits beside the picture, so the card
/// has no band under the picture; a caption below leaves one.
#[trial]
fn the_grid_puts_every_caption_where_it_says() {
    let gaps = |place| {
        let (_fixture, seen) = rig(4, move |grid| grid.caption(place), plain);
        let seen = seen.borrow();
        let (card, picture) = (seen.card(0), seen.picture(0));
        (picture.left() - card.left(), picture.top() - card.top(), card.bottom() - picture.bottom())
    };
    let (_, top, bottom) = gaps(CaptionPlace::Below);
    assert!(bottom > top + 1.0, "a caption below leaves a band under the picture");
    let (start, top, bottom) = gaps(CaptionPlace::Start);
    assert!((bottom - top).abs() < 1.0, "a start caption leaves no band under the picture");
    assert!(start > top + 1.0, "the caption has its column before the picture");
}

/// End goes to the last tile that shows, not to a tile of a closed group.
#[trial]
fn end_lands_on_a_shown_tile_when_the_last_group_is_closed() {
    let groups: &'static [TileGroup] = Box::leak(Box::new([TileGroup::new(0, 6, "Open"), TileGroup::new(6, 6, "Closed").open(false)]));
    let (mut fixture, seen) = rig(12, move |grid| grid.groups(groups), plain);
    click(&mut fixture, "alpha");
    fixture.key(Key::End, Modifiers::NONE);
    fixture.idle(2);
    let lead = seen.borrow().selection.lead.expect("End moves the lead");
    assert!(lead < 6, "the lead is on tile {lead}, a tile of the closed group");
    assert!(seen.borrow().cards.iter().all(|(_, card)| card.min.is_finite() && card.max.is_finite()), "no card is infinite");
}

/// Digits rate the selection the grid keeps with no binding of the app's; a letter still moves by type-ahead.
#[trial]
fn digits_rate_the_collection_selection_and_letters_still_type_ahead() {
    let (mut fixture, seen) = rig_unbound(|grid| grid.rating_keys(RatingKeys::Digits));
    click(&mut fixture, "alpha");
    fixture.type_text("c");
    fixture.idle(2);
    fixture.type_text("3");
    fixture.idle(2);
    let rated = seen.borrow().rates();
    assert_eq!(rated, [(vec![2], 3)], "the digit rates charlie, where type-ahead put the selection");
}

/// A grid whose selection is the item step's: the app gives no binding.
fn rig_unbound(build: impl Fn(TileGrid<'_, usize>) -> TileGrid<'_, usize> + 'static) -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = build(TileGrid::new("Library")).show_items(ui, &Photos(6), &mut |key, _| Tile::new(NAMES[*key]));
        let mut seen = shown.borrow_mut();
        seen.events.extend(out.response.events);
        seen.grid.extend(out.events);
    });
    fixture.room(ROOM);
    fixture.settle();
    (fixture, seen)
}

/// A click on a star and a digit key give the same event, and the tile's own rating event is not a second route.
#[trial]
fn a_star_click_and_a_digit_key_give_the_same_rate_event() {
    let rated = |tile: Tile<'static>| tile.rating(2).bar(BadgeVisibility::Always);
    let (mut fixture, seen) = rig(1, |grid| grid.rating_keys(RatingKeys::Digits), move |_, tile| rated(tile));
    // The rated tile's name carries its rating, so the tile is clicked by its place.
    let card = seen.borrow().card(0);
    fixture.click_at(card.max - Vec2::splat(4.0));
    fixture.idle(2);
    // The stars are one node; the fourth of five lies at seven tenths of its width.
    let stars = fixture.by_label("Rating of alpha").expect("the rating is a node");
    fixture.click_at(egui::Pos2::new(stars.rect.left() + stars.rect.width() * 0.7, stars.center().y));
    fixture.idle(2);
    fixture.type_text("4");
    fixture.idle(2);
    let seen = seen.borrow();
    assert_eq!(seen.rates(), [(vec![0], 4), (vec![0], 4)], "both gestures give the one event: {:?}", seen.grid);
    assert!(!seen.items().any(|item| matches!(item, TilePartEvent::Rate(_))), "the tile's rating does not reach the collection's events");
}

/// Space toggles the lead tile's check, as a click on its check box does.
#[trial]
fn space_toggles_the_lead_tiles_check() {
    let (mut fixture, seen) = rig(4, |grid| grid.checks(true), |_, tile| tile.check(ToggleState::Off));
    click(&mut fixture, "bravo");
    fixture.key(Key::Space, Modifiers::NONE);
    fixture.idle(2);
    let checked: Vec<_> = seen
        .borrow()
        .events
        .iter()
        .filter_map(|event| if let CollectionEvent::Item(key, TilePartEvent::Check { .. }) = event { Some(*key) } else { None })
        .collect();
    assert_eq!(checked, [1], "Space reports the check of the lead tile");
}

/// A single click activates a tile when the grid says so, and a click that only selects does not.
#[trial]
fn activate_on_click_reports_the_clicked_tile() {
    let activated = |seen: &Rc<RefCell<Seen>>| -> Vec<usize> {
        seen.borrow().events.iter().filter_map(|event| if let CollectionEvent::Activate(key) = event { Some(*key) } else { None }).collect()
    };
    let (mut fixture, seen) = rig(4, |grid| grid.activate(ActivateOn::Click), plain);
    click(&mut fixture, "charlie");
    assert_eq!(activated(&seen), [2], "a click activates the clicked tile");
    assert_eq!(seen.borrow().selection.lead, Some(2), "and it is selected as well");
    let (mut fixture, seen) = rig(4, |grid| grid, plain);
    click(&mut fixture, "charlie");
    assert!(activated(&seen).is_empty(), "a single click only selects by default");
    assert_eq!(seen.borrow().selection.lead, Some(2));
}
