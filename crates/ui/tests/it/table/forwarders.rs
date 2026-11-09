//! `Table` takes the collection setters of `List` through the derive, and the collection behaves the same.

use craft_behavior::ItemSource;
use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder};
use craft_ui::contract::{CollectionEvent, ColumnSpec, LeadOnOpen, Props as _, ShowCollection as _};
use craft_ui::list::{List, ListRow};
use craft_ui::table::Table;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, pointer, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const NAME: ColumnKey = ColumnKey::new("name");

struct Rows(usize);

impl DisplayOrder<usize> for Rows {
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

impl ItemSource<usize> for Rows {}

/// The rows the lead visited while the pointer moved from the first row to the third, for a `List` or a `Table`
/// given the same props.
fn leads_under_a_moving_pointer(table: bool) -> Vec<usize> {
    let rows = Rows(4);
    let third = Cell::new(Rect::NOTHING);
    let first = Cell::new(Rect::NOTHING);
    let leads = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let note = |key: &usize, rect: Rect| match key {
                0 => first.set(rect),
                2 => third.set(rect),
                _ => {}
            };
            let events = if table {
                Table::new("rows", vec![ColumnSpec::new(NAME, "Name", ColumnWidth::Fixed(120.0))])
                    .id_salt("rows")
                    .lead_on_open(LeadOnOpen::First)
                    .lead_follows_hover(true)
                    .focus_on_press(false)
                    .show_items(ui, &rows, &mut |key, cx| {
                        note(key, cx.rect);
                        ListRow::new("row")
                    })
                    .events
            } else {
                List::new("rows")
                    .id_salt("rows")
                    .lead_on_open(LeadOnOpen::First)
                    .lead_follows_hover(true)
                    .focus_on_press(false)
                    .show_items(ui, &rows, &mut |key, cx| {
                        note(key, cx.rect);
                        ListRow::new("row")
                    })
                    .events
            };
            leads.borrow_mut().extend(events.into_iter().filter_map(|event| match event {
                CollectionEvent::LeadMoved { key, .. } => Some(key),
                _ => None,
            }));
        });
    };
    frames.idle(3, &mut show);
    frames.frame(vec![pointer::moved(first.get().center())], &mut show);
    frames.frame(vec![pointer::moved(third.get().center())], &mut show);
    leads.into_inner()
}

