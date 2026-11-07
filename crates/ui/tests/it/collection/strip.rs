//! A one-row horizontal layout whose items are as wide as they measure themselves: the widths the item step
//! feeds the layout, and the drop gap on the inline axis.

use super::kit::{Dismissed, ROW_H, RowOut};
use crate::contract::kit::PlainStyle;
use craft_behavior::{DisplayOrder, ItemLayout, ItemSource, NavShape, Proposal, ScrollPos, Selection};
use craft_i18n::Direction;
use craft_ui::contract::{
    Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, DropMark, ListDrag, MeasureCx, Props as _, ReorderRule,
    Role, SelectMode, ShowCollection as _, Sizing,
};
use craft_ui::runtime::drag;
use craft_ui::runtime::env::RootEnv;
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(400.0, 60.0);
const GLYPH: f32 = 8.0;
const MIN: f32 = 8.0;
/// The label lengths of the items, in glyphs.
const LABELS: [usize; 3] = [3, 12, 6];

/// The `(ideal, min)` widths the layout received, by item.
type Received = Arc<Mutex<Vec<Option<(f32, f32)>>>>;

/// Items side by side in one row, each as wide as it measures its ideal width.
#[derive(Clone)]
struct Strip {
    widths: Vec<f32>,
    received: Received,
}

impl Strip {
    fn new(count: usize, received: &Received) -> Self {
        Strip { widths: vec![40.0; count], received: received.clone() }
    }

    fn left(&self, at: usize) -> f32 {
        self.widths[..at].iter().sum()
    }
}

impl ItemLayout for Strip {
    fn content_size(&self) -> ScrollPos {
        ScrollPos { x: f64::from(self.widths.iter().sum::<f32>()), y: f64::from(ROW_H) }
    }

    fn visible(&self, _: ScrollPos, _: Vec2, out: &mut Vec<usize>) {
        out.extend(0..self.widths.len());
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        Rect::from_min_size(Pos2::new(self.left(at) - offset.x as f32, 0.0), Vec2::new(self.widths[at], ROW_H))
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        (0..self.widths.len()).find(|&at| self.rect(at, offset).contains(point))
    }

    fn nav(&self) -> NavShape {
        NavShape::RaggedGrid
    }

    fn row_len(&self, row: usize) -> Option<usize> {
        (row == 0).then_some(self.widths.len())
    }

    fn is_auto_width(&self, _: usize) -> bool {
        true
    }

    fn set_width(&mut self, at: usize, ideal: f32, min: f32) {
        self.widths[at] = ideal;
        self.received.lock().unwrap()[at] = Some((ideal, min));
    }
}

struct Labels;

impl DisplayOrder<usize> for Labels {
    fn len(&self) -> usize {
        LABELS.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < LABELS.len()).then_some(*key)
    }
}

impl ItemSource<usize> for Labels {}

/// An item whose ideal width follows its label and whose minimum is fixed.
#[derive(craft_ui::contract::Component)]
struct Tag {
    #[common]
    common: Common,
    glyphs: usize,
}

impl Component for Tag {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = RowOut;

    fn style(_: &StyleCx<'_>, _: TagStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let width = if cx.proposal == Proposal::Min { MIN } else { self.glyphs as f32 * GLYPH };
        Vec2::new(width, ROW_H)
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), event: None }
    }
}

#[derive(craft_ui::contract::Component)]
#[component(collection)]
struct Bar {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    received: Received,
}

impl Component for Bar {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: BarStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Bar {
    type Key = usize;
    type ItemLayout = Strip;
    type Selection = Selection<usize>;
    type Item<'r> = Tag;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, _: f32, _: &StyleCx<'_>) -> Strip {
        Strip::new(source.len(), &self.received)
    }
}

/// Shows the bar and records the rect of each item.
fn show_bar(ui: &mut Ui, received: &Received, rects: &RefCell<Vec<Rect>>) {
    in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
        let props = CollectionProps::new().select(SelectMode::Single).reorder(ReorderRule::new());
        let bar = Bar { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Fill), items: props, received: received.clone() };
        let mut seen = vec![Rect::NOTHING; LABELS.len()];
        bar.id_salt("bar").show_items(ui, &Labels, &mut |key, cx| {
            seen[*key] = cx.rect;
            Tag { common: Common::new(), glyphs: LABELS[*key] }
        });
        *rects.borrow_mut() = seen;
    });
}

/// An item asks for its widths, and the layout receives for each an ideal width not below its minimum, longer
/// labels wider than shorter ones.
#[trial]
fn a_layout_that_asks_for_widths_receives_each_items_ideal_and_minimum() {
    let (received, rects): (Received, _) = (Arc::new(Mutex::new(vec![None; LABELS.len()])), RefCell::new(Vec::new()));
    let mut frames = Frames::new();
    frames.idle(4, &mut |ui: &mut Ui| show_bar(ui, &received, &rects));
    let widths: Vec<(f32, f32)> = received.lock().unwrap().iter().map(|pair| pair.expect("every item was measured")).collect();
    assert!(widths.iter().all(|(ideal, min)| ideal >= min), "{widths:?}");
    assert!(widths[1].0 > widths[2].0 && widths[2].0 > widths[0].0, "ideal widths follow the labels: {widths:?}");
    let rects = rects.borrow();
    assert!(rects[1].width() > rects[2].width() && rects[2].width() > rects[0].width(), "the items are placed at the widths they measured");
}

/// A drag over the second half of item 0 of a one-row layout shows the drop mark between items 0 and 1 on the
/// inline axis; under right-to-left the second half and the mark lie on the other side.
#[trial]
fn a_drag_over_an_item_of_a_one_row_layout_marks_the_gap_on_the_inline_axis() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let (received, rects): (Received, _) = (Arc::new(Mutex::new(vec![None; LABELS.len()])), RefCell::new(Vec::new()));
        let mut frames = Frames::new();
        frames.root(RootEnv::default().direction(direction));
        let mut show = |ui: &mut Ui| show_bar(ui, &received, &rects);
        frames.idle(4, &mut show);
        let (first, last) = (rects.borrow()[0], rects.borrow()[2]);
        let rtl = direction.is_rtl();
        let (from, edge, side) = (last.center(), if rtl { first.min.x } else { first.max.x }, if rtl { -1.0 } else { 1.0 });
        let target = Pos2::new(first.center().x + side * 4.0, first.center().y);
        frames.frame(vec![pointer::moved(from)], &mut show);
        frames.frame(vec![pointer::press(from)], &mut show);
        for step in 1..=4 {
            frames.frame(vec![pointer::moved(from + (target - from) * (step as f32 / 4.0))], &mut show);
        }
        frames.idle(1, &mut show);
        assert!(drag::active::<ListDrag<usize>>(&frames.ctx).is_some_and(|payload| payload.grabbed == 2), "{direction:?}");
        match drag::mark(&frames.ctx) {
            Some(DropMark::Line { from, to, .. }) => {
                assert!((from.x - edge).abs() < 1.0 && (to.x - edge).abs() < 1.0, "{direction:?}: the mark lies on the x of item 0's far edge")
            }
            other => panic!("{direction:?}: the drag shows an insertion line, found {other:?}"),
        }
        frames.frame(vec![pointer::release(target)], &mut show);
    }
}
