//! The item step under the layers around it: a list in a content-sized layer, a cover over the list, a zoomed
//! graph, a multiple selection dragged as one, a rename that ends elsewhere, the rows `parts` sees, and a press that
//! shares its frame with a wheel scroll.

use super::kit::{Count, Dismissed, List, ROW_H, Row, Words};
use craft_behavior::{CancelReason, CanvasTransform, EditEvent, Selection, keyed};
use craft_ui::contract::{Bind, CollectionEvent, CollectionResponse, Props as _, Role, SelectMode, Show as _, ShowCollection as _};
use craft_ui::contract::Entry;
use craft_ui::pickers::Select;
use craft_ui::graph::{Node, NodeEvent, NodeGraph, NodeId, NodeKind, PortRef};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::testing::layout::in_region;
use craft_ui::testing::{Fixture, Frames, focus as testing_focus, input, pointer, trial};
use egui::{Event, Id, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};

const VIEW: Vec2 = Vec2::new(300.0, 200.0);
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const SAME: f32 = 0.5;

/// What a list reported and where its rows were.
#[derive(Default)]
struct Run {
    id: Cell<Option<Id>>,
    events: RefCell<Vec<CollectionEvent<usize, Dismissed>>>,
    rows: RefCell<Vec<(usize, Rect)>>,
}

impl Run {
    fn take(&self, out: CollectionResponse<usize, Dismissed>) {
        self.id.set(Some(out.response.id));
        self.events.borrow_mut().extend(out.events);
    }

    fn note(&self, key: usize, rect: Rect) {
        let mut rows = self.rows.borrow_mut();
        rows.retain(|(held, _)| *held != key);
        rows.push((key, rect));
    }

    fn row(&self, key: usize) -> Rect {
        self.rows.borrow().iter().find(|(held, _)| *held == key).map(|(_, rect)| *rect).expect("the row was shown")
    }

    fn id(&self) -> Id {
        self.id.get().expect("the list ran")
    }

    fn leads(&self) -> Vec<usize> {
        self.events.borrow().iter().filter_map(|event| if let CollectionEvent::LeadMoved(key) = event { Some(*key) } else { None }).collect()
    }
}

fn press(frames: &mut Frames, key: Key, mods: Modifiers, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![input::key_press(key, mods)], &mut *show);
    frames.frame(vec![testing_focus::key_release(key, mods)], &mut *show);
}

/// A popup list takes the height of its rows, not the room the layer offers.
#[trial]
fn a_three_option_select_list_is_three_rows_tall() {
    let mut fixture = Fixture::new(|ui| {
        let options = [Entry::new(0u8, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")];
        let mut mode = 0u8;
        Select::new("Blend mode", Bind::bound(&mut mode), &options).id_salt("mode").show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    let list = fixture.by_role(Role::LIST_BOX).first().copied().expect("the listbox showed");
    let options = fixture.by_role(Role::LIST_BOX_OPTION);
    assert_eq!(options.len(), 3);
    let rows: f32 = options.iter().map(|option| option.rect.height()).sum();
    assert_eq!(list.rect.height().round(), rows.round(), "the list is as tall as its rows: {} against {rows}", list.rect.height());
}

/// A secondary release inside something that covers the list asks the list for no menu.
#[trial]
fn a_secondary_release_under_a_cover_asks_for_no_menu() {
    let words = Words::flat(&["a", "b", "c"]);
    let (run, covered) = (Run::default(), Cell::new(false));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            run.take(List::new(SelectMode::Single).id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()));
        });
        if covered.get() {
            craft_ui::testing::sensing::cover(ui.ctx(), VIEW);
        }
    };
    frames.idle(3, &mut show);
    covered.set(true);
    frames.idle(2, &mut show);
    let at = ORIGIN + Vec2::new(30.0, ROW_H * 1.5);
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::secondary(at, true)], &mut show);
    frames.frame(vec![pointer::secondary(at, false)], &mut show);
    frames.idle(1, &mut show);
    assert!(!run.events.borrow().iter().any(|event| matches!(event, CollectionEvent::ContextMenu(_))), "the cover took the release");
}

struct OnePort;

impl NodeKind for OnePort {
    fn port_count(&self) -> usize {
        1
    }
}

/// A zoomed graph hits ports where it draws them: a wire dragged from one node's port to another's connects them.
#[trial]
fn a_wire_dragged_between_ports_of_a_zoomed_graph_connects_them() {
    const SCALE: f32 = 2.0;
    let kinds: [&dyn NodeKind; 2] = [&OnePort, &OnePort];
    let positions =
        [Rect::from_min_max(Pos2::new(20.0, 20.0), Pos2::new(120.0, 60.0)), Rect::from_min_max(Pos2::new(20.0, 100.0), Pos2::new(120.0, 140.0))];
    let ids = [NodeId::new(0), NodeId::new(1)];
    let source = keyed(&ids, |id| *id);
    let wired = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, Vec2::new(400.0, 400.0)), |ui| {
            let out = NodeGraph::new(&positions, &kinds, &[]).zoom(CanvasTransform::new(SCALE, Vec2::ZERO)).id_salt("graph").show_items(
                ui,
                &source,
                &mut |id, _| Node::new(*id, kinds[id.0 as usize]),
            );
            for event in out.events {
                if let CollectionEvent::Item(_, NodeEvent::Wired { from, to, .. }) = event {
                    wired.borrow_mut().push((from, to));
                }
            }
        });
    };
    frames.idle(3, &mut show);
    let port = |node: usize| ORIGIN + (positions[node].min.to_vec2() + Vec2::new(0.0, positions[node].height() / 2.0)) * SCALE;
    pointer::drag(&mut frames, port(0), port(1) - port(0), &mut show);
    assert_eq!(*wired.borrow(), [(PortRef::new(NodeId::new(0), 0), PortRef::new(NodeId::new(1), 0))]);
}

/// Dragging one row of a selected range moves the whole range, and the event names every moved row.
#[trial]
fn dragging_a_selected_range_reorders_every_row_of_it() {
    let words = Count(8);
    let (run, selection) = (Run::default(), RefCell::new(Selection::new(vec![1, 2], Some(2), Some(1))));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let mut held = selection.borrow().clone();
            let out =
                List::new(SelectMode::Multi).reorder().selection(Bind::bound(&mut held)).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                    run.note(*key, cx.rect);
                    Row::new()
                });
            *selection.borrow_mut() = held;
            run.take(out);
        });
    };
    frames.idle(3, &mut show);
    let (from, below) = (run.row(1).center(), run.row(4));
    let target = Pos2::new(from.x, below.max.y - 1.0);
    frames.frame(vec![pointer::moved(from)], &mut show);
    frames.frame(vec![pointer::press(from)], &mut show);
    for step in 1..=4 {
        frames.frame(vec![pointer::moved(from + (target - from) * (step as f32 / 4.0))], &mut show);
    }
    frames.frame(vec![pointer::release(target)], &mut show);
    frames.idle(2, &mut show);
    let moves: Vec<(Vec<usize>, usize)> = run
        .events
        .borrow()
        .iter()
        .filter_map(|event| if let CollectionEvent::Reorder(edit) = event { edit.value().map(|moved| (moved.keys.clone(), moved.to)) } else { None })
        .collect();
    assert_eq!(moves, [(vec![1, 2], 3)], "rows 1 and 2 land together before the row that followed row 4");
    assert_eq!(selection.borrow().items, [1, 2], "the press on a selected row kept the range for the drag");
}

/// A click on a row of a selected range narrows the selection to that row on the release.
#[trial]
fn a_click_on_a_row_of_a_selected_range_narrows_the_selection_on_release() {
    let words = Count(8);
    let (run, selection) = (Run::default(), RefCell::new(Selection::new(vec![1, 2], Some(2), Some(1))));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            let mut held = selection.borrow().clone();
            let out = List::new(SelectMode::Multi).selection(Bind::bound(&mut held)).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                run.note(*key, cx.rect);
                Row::new()
            });
            *selection.borrow_mut() = held;
            run.take(out);
        });
    };
    frames.idle(3, &mut show);
    let at = run.row(1).center();
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::press(at)], &mut show);
    assert_eq!(selection.borrow().items, [1, 2], "the press alone keeps the range");
    frames.frame(vec![pointer::release(at)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(selection.borrow().items, [1]);
}

/// A rename that ends because focus went to another field leaves focus there.
#[trial]
fn a_rename_that_ends_by_focusing_another_list_keeps_that_focus() {
    let words = Words::flat(&["one", "two"]);
    let (first, second) = (Run::default(), Run::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            first.take(List::new(SelectMode::Single).rename().text_editor().id_salt("first").show_items(ui, &words, &mut |_, _| Row::new()));
        });
        in_region(ui, Rect::from_min_size(Pos2::new(350.0, 10.0), VIEW), |ui| {
            second.take(List::new(SelectMode::Single).id_salt("second").show_items(ui, &words, &mut |_, _| Row::new()));
        });
    };
    frames.idle(2, &mut show);
    request(&frames.ctx, FocusTarget::Id(first.id()));
    frames.idle(2, &mut show);
    press(&mut frames, Key::ArrowDown, Modifiers::NONE, &mut show);
    press(&mut frames, Key::F2, Modifiers::NONE, &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![Event::Text("x".into())], &mut show);
    frames.idle(1, &mut show);
    request(&frames.ctx, FocusTarget::Id(second.id()));
    frames.idle(3, &mut show);
    let ended = first
        .events
        .borrow()
        .iter()
        .any(|event| matches!(event, CollectionEvent::Rename(EditEvent::Commit { .. } | EditEvent::Cancel { reason: CancelReason::Blur, .. })));
    assert!(ended, "leaving the editor ended the rename");
    assert_eq!(testing_focus::egui_focused(&frames.ctx), Some(second.id()), "focus stays on the field it went to");
}

/// A collection's `parts` reads the rects the item step gives the rows this pass, scrolled or not.
#[trial]
fn parts_read_the_rect_of_a_row_after_the_scroll() {
    const KEY: usize = 8;
    let words = Count(40);
    let (run, seen) = (Run::default(), Cell::new(None));
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            run.take(List::new(SelectMode::None).probe_parts(KEY, &seen).id_salt("list").show_items(ui, &words, &mut |key, cx| {
                run.note(*key, cx.rect);
                Row::new()
            }));
        });
    };
    frames.idle(3, &mut show);
    frames.frame(vec![pointer::moved(ORIGIN + VIEW / 2.0)], &mut show);
    frames.frame(vec![pointer::wheel(-60.0)], &mut show);
    let given = seen.get().expect("the row is in view after the scroll");
    assert!((given.min - run.row(KEY).min).abs().max_elem() < SAME, "parts saw {given:?}, the row sat at {:?}", run.row(KEY));
}

/// A press in the frame of a wheel scroll lands on the row the user saw under the pointer.
#[trial]
fn a_press_in_the_frame_of_a_wheel_scroll_hits_the_row_the_user_saw() {
    let words = Count(40);
    let run = Run::default();
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(ORIGIN, VIEW), |ui| {
            run.take(List::new(SelectMode::Single).id_salt("list").show_items(ui, &words, &mut |_, _| Row::new()));
        });
    };
    frames.idle(3, &mut show);
    let at = ORIGIN + Vec2::new(30.0, ROW_H * 2.5);
    frames.frame(vec![pointer::moved(at)], &mut show);
    frames.frame(vec![pointer::wheel(-60.0), pointer::press(at)], &mut show);
    assert_eq!(run.leads().last(), Some(&2), "the press hit row 2 as it stood before the scroll");
}
