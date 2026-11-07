//! `List::filter`: the item step sees only the rows that match, and the lead stays on a shown row.

use craft_behavior::{DisplayOrder, ItemSource, Selection};
use craft_ui::contract::{Bind, SelectMode, ShowCollection as _};
use craft_ui::list::{List, ListRow};
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Frames, input, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const LABELS: [&str; 5] = ["Alpha", "Café", "Beta", "Cat", "Gamma"];

/// The labels keyed by position.
struct Labels;

impl DisplayOrder<usize> for Labels {
    fn len(&self) -> usize {
        LABELS.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl ItemSource<usize> for Labels {}

fn label(key: &usize) -> Label {
    Label::from(LABELS[*key])
}

/// The keys the item step asked for in the last frame, and the list's id.
#[derive(Default)]
struct Seen {
    rows: RefCell<Vec<usize>>,
    id: Cell<Option<egui::Id>>,
}

#[trial]
fn a_filtered_list_shows_only_matching_rows_and_keeps_its_lead_on_one() {
    let (query, selection, seen) = (RefCell::new(String::new()), RefCell::new(Selection::<usize>::default()), Seen::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut held = selection.borrow().clone();
        let query = query.borrow().clone();
        in_region(ui, Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(300.0, 200.0)), |ui| {
            seen.rows.borrow_mut().clear();
            let out = List::new("Items").select(SelectMode::Single).selection(Bind::bound(&mut held)).filter(&query, label).show_items(
                ui,
                &Labels,
                &mut |key, _| {
                    seen.rows.borrow_mut().push(*key);
                    ListRow::new(LABELS[*key])
                },
            );
            seen.id.set(Some(out.response.id));
        });
        *selection.borrow_mut() = held;
    };
    frames.idle(2, &mut show);
    assert_eq!(*seen.rows.borrow(), [0, 1, 2, 3, 4], "an empty query shows every row");

    request(&frames.ctx, FocusTarget::Id(seen.id.get().expect("the list ran")));
    frames.idle(2, &mut show);
    frames.frame(vec![input::key_press(Key::ArrowDown, Modifiers::NONE)], &mut show);
    frames.idle(1, &mut show);

    *query.borrow_mut() = "ca".to_owned();
    frames.idle(2, &mut show);
    assert_eq!(*seen.rows.borrow(), [1, 3], "accents fold and only matching rows reach the item step");

    for key in [Key::ArrowDown, Key::ArrowDown, Key::ArrowUp] {
        frames.frame(vec![input::key_press(key, Modifiers::NONE)], &mut show);
        frames.idle(1, &mut show);
        let lead = selection.borrow().lead;
        assert!(lead.is_none_or(|key| [1, 3].contains(&key)), "the lead moves over shown rows only: {lead:?}");
    }
}
