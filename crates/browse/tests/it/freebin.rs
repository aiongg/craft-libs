//! The free bin: moves with stacks, the marquee, foreign drops and the flow of unplaced cards.

use craft_behavior::{DisplayOrder, ItemSource, SelectionChange};
use craft_browse::ui::freebin::{FreeBin, FreeBinMove, FreeformCard};
use craft_i18n::Direction;
use craft_ui::contract::{CollectionEvent, ListDrag, Props as _, ReorderRule, ShowCollection as _};
use craft_ui::list::ThumbAspect;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, pointer, trial};
use craft_ui::tile_grid::{Tile, TileGrid, TilePartEvent};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = offset(700.0, 400.0);
const CLOSE: f32 = 1.5;

/// A position or displacement in the bin's coordinates, named so a trial's geometry reads as inputs.
const fn offset(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}
/// A picture side that keeps cards small enough to leave room between them in `VIEW`.
const ZOOM: f32 = 0.5;

/// Cards keyed by position.
struct Count(usize);

impl DisplayOrder<usize> for Count {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0).then_some(*key)
    }
}

impl ItemSource<usize> for Count {}

/// What the host keeps and what the bin reported.
#[derive(Default)]
struct Scene {
    cards: RefCell<Vec<FreeformCard<usize>>>,
    moving: RefCell<Option<FreeBinMove<usize>>>,
    moved: RefCell<Vec<(usize, Vec2)>>,
    events: RefCell<Vec<CollectionEvent<usize, TilePartEvent>>>,
    rects: RefCell<Vec<(usize, Rect)>>,
    grid_rects: RefCell<Vec<(usize, Rect)>>,
    background: RefCell<Option<ListDrag<usize>>>,
    cancel: Cell<bool>,
}

impl Scene {
    fn with_cards(cards: Vec<FreeformCard<usize>>) -> Self {
        Scene { cards: RefCell::new(cards), ..Scene::default() }
    }

    fn rect(&self, key: usize) -> Rect {
        self.rects.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the card was shown")
    }

    fn grid_rect(&self, key: usize) -> Rect {
        self.grid_rects.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the tile was shown")
    }

    fn moved(&self) -> Vec<(usize, Vec2)> {
        let mut moved = self.moved.borrow().clone();
        moved.sort_by_key(|(key, _)| *key);
        moved
    }

    fn dropped_on(&self) -> Vec<(usize, Vec<usize>)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| if let CollectionEvent::Dropped { key: Some(key), keys, .. } = event { Some((*key, keys.clone())) } else { None })
            .collect()
    }

    /// The last selection the bin reported with `Set`.
    fn selected(&self) -> Vec<usize> {
        self.events
            .borrow()
            .iter()
            .rev()
            .find_map(|event| if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event { Some(keys.clone()) } else { None })
            .unwrap_or_default()
    }
}

fn card(key: usize) -> Tile<'static> {
    Tile::new(format!("Clip {}", key + 1)).thumb(ThumbAspect::new(1, 1), |_, _| {})
}

fn show_bin(ui: &mut Ui, count: usize, scene: &Scene, region: Rect) {
    if scene.cancel.take() {
        pointer::cancel_captured_presses(ui.ctx());
    }
    in_region(ui, region, |ui| {
        let cards = scene.cards.borrow().clone();
        let mut bin = FreeBin::new("Clips", &cards).id_salt("bin").zoom(ZOOM).drop_into(|key| *key == 1);
        if let Some(live) = scene.moving.borrow().clone() {
            bin = bin.moving(live);
        }
        let out = bin.show_items(ui, &Count(count), &mut |key, cx| {
            let mut rects = scene.rects.borrow_mut();
            rects.retain(|(held, _)| held != key);
            rects.push((*key, cx.rect));
            card(*key)
        });
        *scene.moving.borrow_mut() = out.moving;
        for (key, pos) in &out.moved {
            let mut cards = scene.cards.borrow_mut();
            match cards.iter_mut().find(|held| held.key == *key) {
                Some(held) => held.pos = *pos,
                None => cards.push(FreeformCard::new(*key, *pos)),
            }
        }
        scene.moved.borrow_mut().extend(out.moved);
        if out.dropped_on_background.is_some() {
            *scene.background.borrow_mut() = out.dropped_on_background;
        }
        scene.events.borrow_mut().extend(out.response.events);
    });
}

fn bin_region() -> Rect {
    Rect::from_min_size(ORIGIN, VIEW)
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < CLOSE
}

/// Dragging a card moves the selection with the stacks of its cards and reports every new position once.
#[trial]
fn a_dragged_card_moves_with_its_stack() {
    let cards = vec![
        FreeformCard::new(0, offset(20.0, 20.0)).in_stack(1),
        FreeformCard::new(1, offset(300.0, 20.0)).in_stack(1),
        FreeformCard::new(2, offset(20.0, 200.0)),
    ];
    let scene = Scene::with_cards(cards);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_bin(ui, 3, &scene, bin_region());
    frames.idle(3, &mut show);
    let from = scene.rect(0).center();
    let by = offset(40.0, 30.0);
    frames.frame(vec![pointer::moved(from)], &mut show);
    frames.frame(vec![pointer::press(from)], &mut show);
    frames.frame(vec![pointer::moved(from + by)], &mut show);
    frames.idle(1, &mut show);
    let live = scene.moving.borrow().clone().expect("the drag is in progress");
    assert_eq!(live.keys, vec![0, 1], "the card's stack moves with it");
    assert!(scene.moved().is_empty(), "nothing is reported while the card is held");
    frames.frame(vec![pointer::release(from + by)], &mut show);
    frames.idle(2, &mut show);
    let moved = scene.moved();
    assert_eq!(moved.len(), 2, "the stack's two cards moved: {moved:?}");
    let (first, second) = (offset(60.0, 50.0), offset(340.0, 50.0));
    assert!(near(moved[0].1, first) && near(moved[1].1, second), "{moved:?}");
    assert!(scene.moving.borrow().is_none(), "the drag ends with the release");
}

/// Escape during the drag leaves the cards where the host has them.
#[trial]
fn escape_puts_dragged_cards_back() {
    let scene = Scene::with_cards(vec![FreeformCard::new(0, offset(20.0, 20.0)), FreeformCard::new(1, offset(300.0, 20.0))]);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_bin(ui, 2, &scene, bin_region());
    frames.idle(3, &mut show);
    let (from, by) = (scene.rect(0).center(), offset(60.0, 40.0));
    frames.frame(vec![pointer::moved(from)], &mut show);
    frames.frame(vec![pointer::press(from)], &mut show);
    frames.frame(vec![pointer::moved(from + by)], &mut show);
    scene.cancel.set(true);
    frames.idle(2, &mut show);
    frames.frame(vec![pointer::release(from + by)], &mut show);
    frames.idle(2, &mut show);
    assert!(scene.moved().is_empty(), "a cancelled drag reports no move");
    assert!(scene.moving.borrow().is_none());
    let home = offset(20.0, 20.0);
    assert!(near(scene.rect(0).min - ORIGIN, home), "the card is back at the host's place");
}

/// A foreign drag released on a card that takes it reports that card; released elsewhere it reports the background.
#[trial]
fn a_foreign_drop_reports_the_card_or_the_background() {
    let scene = Scene::with_cards(vec![FreeformCard::new(0, offset(20.0, 20.0)), FreeformCard::new(1, offset(160.0, 20.0))]);
    let grid_region = Rect::from_min_size(ORIGIN, offset(250.0, 380.0));
    let bin_region = Rect::from_min_size(ORIGIN + offset(300.0, 0.0), offset(400.0, 380.0));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, grid_region, |ui| {
            let _ = TileGrid::new("Source").id_salt("source").zoom(ZOOM).reorder(ReorderRule::new()).show_items(ui, &Count(2), &mut |key, cx| {
                let mut rects = scene.grid_rects.borrow_mut();
                rects.retain(|(held, _)| held != key);
                rects.push((*key, cx.rect));
                Tile::new(format!("Source {}", key + 1)).thumb(ThumbAspect::new(1, 1), |_, _| {})
            });
        });
        show_bin(ui, 2, &scene, bin_region);
    };
    frames.idle(3, &mut show);
    let from = scene.grid_rect(0).center();
    pointer::drag(&mut frames, from, scene.rect(1).center() - from, &mut show);
    assert_eq!(scene.dropped_on(), vec![(1, vec![0])], "the card that takes drops reports it");
    assert!(scene.background.borrow().is_none());
    pointer::drag(&mut frames, from, ORIGIN + offset(680.0, 360.0) - from, &mut show);
    assert!(scene.background.borrow().as_ref().is_some_and(|drag| drag.keys == vec![0]), "off the cards the bin reports the background");
    assert_eq!(scene.dropped_on().len(), 1, "the background drop is no drop on a card");
}

/// Cards without a position take the next free cell, from the reading start edge.
#[trial]
fn unplaced_cards_flow_from_the_reading_start() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let scene = Scene::default();
        let mut frames = Frames::new();
        frames.root(RootEnv::default().direction(direction));
        let mut show = |ui: &mut Ui| show_bin(ui, 3, &scene, bin_region());
        frames.idle(4, &mut show);
        let (first, second) = (scene.rect(0), scene.rect(1));
        assert!((first.min.y - second.min.y).abs() < CLOSE, "{direction:?}: the first row holds both cards");
        if direction.is_rtl() {
            assert!(second.center().x < first.center().x, "{direction:?}: the flow runs from the right");
        } else {
            assert!(second.center().x > first.center().x, "{direction:?}: the flow runs from the left");
        }
        let flow_first = first;
        let placed = Scene::with_cards(vec![FreeformCard::new(0, offset(300.0, 200.0))]);
        let mut frames = Frames::new();
        frames.root(RootEnv::default().direction(direction));
        let mut show = |ui: &mut Ui| show_bin(ui, 3, &placed, bin_region());
        frames.idle(4, &mut show);
        assert_eq!(placed.rect(1), flow_first, "{direction:?}: a card with no position takes the first free cell");
    }
}

/// A card without a position never lands on a placed card: it takes the next cell of the flow that is free.
#[trial]
fn an_unplaced_card_skips_the_cells_of_placed_cards() {
    let flowing = Scene::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_bin(ui, 3, &flowing, bin_region());
    frames.idle(4, &mut show);
    let first_cell = flowing.rect(0);
    let placed = Scene::with_cards(vec![FreeformCard::new(0, first_cell.min - ORIGIN)]);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_bin(ui, 3, &placed, bin_region());
    frames.idle(4, &mut show);
    assert_eq!(placed.rect(0), first_cell, "the placed card keeps its place");
    assert!(!placed.rect(1).intersects(placed.rect(0)), "the next card does not cover the placed one");
    assert_eq!(placed.rect(1), flowing.rect(1), "it takes the second cell of the flow");
    assert_eq!(placed.rect(2), flowing.rect(2), "the third card follows it");
}
