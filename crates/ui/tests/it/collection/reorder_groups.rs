//! Reordering into the groups of a tree, and the extra payload a row drag carries: where a drop lands, what the
//! Alt+arrow keys do at a group's edge, and a drop on a target of the mapped payload type.

use super::kit::{Dismissed, List, ROW_H, Row, RowKey, Words};
use crate::contract::kit::PlainStyle;
use craft_behavior::DropOperation;
use craft_ui::contract::{
    CollectionEvent, CollectionResponse, Common, Component, Cx, DropMark, DropTarget, MeasureCx, Props as _, Role, SelectMode, Show as _,
    ShowCollection as _,
};
use craft_ui::runtime::drag;
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, focus as testing_focus, input, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::RefCell;

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const SLOT: Rect = Rect { min: Pos2::new(330.0, 10.0), max: Pos2::new(430.0, 60.0) };
/// Far enough from the leading edge to read as the child indent, and near enough to read as the group's own.
const DEEP: f32 = 60.0;
const SHALLOW: f32 = 2.0;

/// `a`, an open group `G` holding `c` and `d`, `e`, and an open group `H` with no rows.
fn grove() -> Words {
    let mut words = Words::flat(&["a", "G", "c", "d", "e", "H"]);
    words.tree = vec![(0, false), (0, true), (1, false), (1, false), (0, false), (0, true)];
    words.open = RefCell::new(vec![false, true, false, false, false, true]);
    words
}

#[derive(Default)]
struct Run {
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    rows: RefCell<Vec<(usize, Rect)>>,
    dropped: RefCell<Vec<RowKey>>,
}

impl Run {
    fn row(&self, key: usize) -> Rect {
        self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was shown")
    }

    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.events.borrow_mut().extend(out.events);
    }

    /// Every reorder reported: the moved keys, the position and the group.
    fn moves(&self) -> Vec<(Vec<usize>, usize, Option<usize>)> {
        self.events
            .borrow()
            .iter()
            .filter_map(
                |event| if let CollectionEvent::Reorder(edit) = event { edit.value().map(|m| (m.keys.clone(), m.to, m.parent)) } else { None },
            )
            .collect()
    }
}

struct TakesRowKey;

impl DropTarget<RowKey> for TakesRowKey {
    type Drop = RowKey;

    fn accept(&self, payload: &RowKey, _: Pos2, _: DropOperation) -> Option<RowKey> {
        Some(*payload)
    }

    fn mark(&self, _: &RowKey) -> DropMark {
        DropMark::Into(SLOT)
    }
}

/// Takes the drops of the mapped payload.
#[derive(craft_ui::contract::Component)]
struct Slot<'a> {
    #[common]
    common: Common,
    run: &'a Run,
}

impl Component for Slot<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SlotStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        SLOT.size()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use craft_ui::contract::Pointer as _;
        if let Some(key) = cx.drop_target(&TakesRowKey) {
            self.run.dropped.borrow_mut().push(key);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn show_grove(ui: &mut Ui, list: List<'_>, words: &Words, run: &Run) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let out = list.id_salt("list").show_items(ui, words, &mut |key, cx| {
            let mut rows = run.rows.borrow_mut();
            rows.retain(|(held, _)| held != key);
            rows.push((*key, cx.rect));
            Row::new()
        });
        run.take(out);
    });
    Slot { common: Common::new(), run }.id_salt("slot").show_at(ui, SLOT);
}

/// Presses on row `from`, drags to `to` and holds there; the caller reads the mark and releases.
fn drag_to(frames: &mut Frames, run: &Run, from: usize, to: Pos2, show: &mut impl FnMut(&mut Ui)) -> Pos2 {
    let start = run.row(from).center();
    frames.frame(vec![moved(start)], &mut *show);
    frames.frame(vec![press(start)], &mut *show);
    for step in 1..=4 {
        frames.frame(vec![moved(start + (to - start) * (step as f32 / 4.0))], &mut *show);
    }
    frames.idle(1, &mut *show);
    to
}

fn release_at(frames: &mut Frames, at: Pos2, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![release(at)], &mut *show);
    frames.idle(2, &mut *show);
}

/// The drop point `dy` below the top of row `key`, `dx` right of its leading edge.
fn point(run: &Run, key: usize, dx: f32, dy: f32) -> Pos2 {
    let row = run.row(key);
    Pos2::new(row.min.x + dx, row.min.y + dy)
}

/// Dropping on the middle of a group row puts the row in the group, as its first child, and marks the group row.
#[trial]
fn a_drop_on_the_middle_of_a_group_row_lands_inside_the_group() {
    {
        let (words, run) = (grove(), Run::default());
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).tree().reorder_into_groups(), &words, &run);
        frames.idle(3, &mut show);
        let at = point(&run, 1, SHALLOW, ROW_H / 2.0);
        drag_to(&mut frames, &run, 4, at, &mut show);
        assert!(matches!(drag::mark(&frames.ctx), Some(DropMark::Into(rect)) if rect.contains(at)), "the group row is marked");
        release_at(&mut frames, at, &mut show);
        assert_eq!(run.moves(), [(vec![4], 2, Some(1))], "row 4 lands right after the group row, inside the group");
    }
    {
        let (words, run) = (grove(), Run::default());
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).tree().reorder_into_groups(), &words, &run);
        frames.idle(3, &mut show);
        let at = point(&run, 5, SHALLOW, ROW_H / 2.0);
        drag_to(&mut frames, &run, 0, at, &mut show);
        release_at(&mut frames, at, &mut show);
        assert_eq!(run.moves(), [(vec![0], 5, Some(5))]);
    }
}

/// Below the last row of an open group, the pointer's indent picks the group or the level above it.
#[trial]
fn a_drop_below_the_last_child_lands_at_the_depth_the_pointer_is_at() {
    let (words, run) = (grove(), Run::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).tree().reorder_into_groups(), &words, &run);
    frames.idle(3, &mut show);
    for (dx, group) in [(DEEP, Some(1)), (SHALLOW, None)] {
        let at = point(&run, 3, dx, ROW_H - 2.0);
        drag_to(&mut frames, &run, 0, at, &mut show);
        release_at(&mut frames, at, &mut show);
        assert_eq!(run.moves().last(), Some(&(vec![0], 3, group)), "dropped at indent {dx}");
    }
}

/// A group cannot be dropped inside itself.
#[trial]
fn a_group_does_not_drop_inside_itself() {
    let (words, run) = (grove(), Run::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).tree().reorder_into_groups(), &words, &run);
    frames.idle(3, &mut show);
    let at = point(&run, 2, DEEP, ROW_H / 2.0);
    drag_to(&mut frames, &run, 1, at, &mut show);
    assert!(drag::mark(&frames.ctx).is_none(), "no drop spot inside the dragged group's rows");
    release_at(&mut frames, at, &mut show);
    assert!(run.moves().is_empty());
}

/// Alt+Up on the row right below an open group makes it the group's last child; Alt+Down takes it out again.
#[trial]
fn alt_arrows_enter_and_leave_a_group_at_its_end() {
    let (words, run) = (grove(), Run::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).tree().reorder_into_groups(), &words, &run);
    frames.idle(3, &mut show);
    let click = point(&run, 4, 100.0, ROW_H / 2.0);
    frames.frame(vec![moved(click)], &mut show);
    frames.frame(vec![press(click)], &mut show);
    frames.frame(vec![release(click)], &mut show);
    frames.idle(2, &mut show);
    let press_key = |frames: &mut Frames, key: Key, show: &mut dyn FnMut(&mut Ui)| {
        frames.frame(vec![input::key_press(key, Modifiers::ALT)], &mut *show);
        frames.frame(vec![testing_focus::key_release(key, Modifiers::ALT)], &mut *show);
    };
    press_key(&mut frames, Key::ArrowUp, &mut show);
    assert_eq!(run.moves(), [(vec![4], 4, Some(1))], "the row stays where it shows and joins the group");
}

/// A list that maps its row drag to a payload reports a drop on a target of that type there, and a drag inside the list
/// still reorders.
#[trial]
fn a_row_drag_also_drops_on_a_target_of_the_mapped_payload() {
    {
        let (words, run) = (Words::flat(&["a", "b", "c"]), Run::default());
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).reorder().row_key_payload(), &words, &run);
        frames.idle(3, &mut show);
        drag_to(&mut frames, &run, 1, SLOT.center(), &mut show);
        release_at(&mut frames, SLOT.center(), &mut show);
        assert_eq!(*run.dropped.borrow(), [RowKey(1)]);
        assert!(run.moves().is_empty(), "the drop on the target is not a reorder");

        let bottom = Pos2::new(run.row(2).center().x, run.row(2).max.y - 1.0);
        drag_to(&mut frames, &run, 0, bottom, &mut show);
        release_at(&mut frames, bottom, &mut show);
        assert_eq!(run.moves(), [(vec![0], 2, None)]);
        assert_eq!(run.dropped.borrow().len(), 1);
    }
    {
        let (words, run) = (Words::flat(&["a", "b", "c"]), Run::default());
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| show_grove(ui, List::new(SelectMode::Single).row_key_payload_except(0), &words, &run);
        frames.idle(3, &mut show);
        drag_to(&mut frames, &run, 1, SLOT.center(), &mut show);
        release_at(&mut frames, SLOT.center(), &mut show);
        assert_eq!(*run.dropped.borrow(), [RowKey(1)], "a row with a payload drags to the target");
        assert!(run.moves().is_empty(), "without a reorder rule nothing reorders");

        drag_to(&mut frames, &run, 0, SLOT.center(), &mut show);
        release_at(&mut frames, SLOT.center(), &mut show);
        assert_eq!(run.dropped.borrow().len(), 1, "a row mapped to None does not drag");
    }
}
