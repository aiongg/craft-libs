//! Drag and drop (8.4): the session, drop targets, the operation, cancel and the ghost layer's preview.

use crate::windows_kit::Windows;
use craft_behavior::{DisplayOrder, DropOperation, ItemSource, PartKey, PressPhase};
use craft_ui::contract::{
    CollectionEvent, Common, Component, Cx, DragPreview, DropMark, DropTarget, MeasureCx, OsFiles, PartsCx, Pointer as _, Props as _, Role,
    Show as _, ShowCollection as _, WindowDropTarget,
};
use craft_ui::runtime::drag;
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::Block;
use craft_ui::testing::pointer::{moved, press, release, secondary};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2, ViewportBuilder, ViewportId, ViewportInfo};
use std::cell::{Cell, RefCell};
mod frame_label;

const GRIP: PartKey = PartKey::new("grip");
const CHIP: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(60.0, 40.0) };
const SLOT: Rect = Rect { min: Pos2::new(200.0, 10.0), max: Pos2::new(300.0, 60.0) };
const BADGE: &str = "3 files";

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PlainStyle {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// What the payload is: the number the source drags.
#[derive(Clone, Debug, PartialEq)]
struct Payload(u32);

/// What the components saw.
#[derive(Default)]
struct Seen {
    dropped: Vec<(u32, DropOperation)>,
    ended: Vec<DropOperation>,
}

/// Starts a drag from its grip and reads how the drop went.
#[derive(Component)]
struct Chip<'a> {
    #[common]
    common: Common,
    seen: &'a RefCell<Seen>,
    preview: DragPreview,
}

impl Component for Chip<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ChipStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        CHIP.size()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(GRIP, rect).sense(craft_behavior::Sense::DRAG).pointer_only("the grip is dragged with the pointer");
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if matches!(cx.part(GRIP).press.phase, PressPhase::DragStart { .. }) {
            cx.start_drag(Payload(7), self.preview.clone());
        }
        if let Some(op) = cx.drag_ended::<Payload>() {
            self.seen.borrow_mut().ended.push(op);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

struct Accepts;

impl DropTarget<Payload> for Accepts {
    type Drop = (u32, DropOperation);

    fn accept(&self, payload: &Payload, _: Pos2, op: DropOperation) -> Option<Self::Drop> {
        Some((payload.0, op))
    }

    fn mark(&self, _: &Self::Drop) -> DropMark {
        DropMark::Into(SLOT)
    }
}

/// Takes drops of the payload.
#[derive(Component)]
struct Slot<'a> {
    #[common]
    common: Common,
    seen: &'a RefCell<Seen>,
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
        if let Some(drop) = cx.drop_target(&Accepts) {
            self.seen.borrow_mut().dropped.push(drop);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn scene(seen: &RefCell<Seen>) -> impl FnMut(&mut Ui) + '_ {
    scene_previewing(seen, DragPreview::Badge(Label::Static(BADGE)))
}

/// The scene whose chip drags with `preview`.
fn scene_previewing(seen: &RefCell<Seen>, preview: DragPreview) -> impl FnMut(&mut Ui) + '_ {
    move |ui: &mut Ui| {
        Chip { common: Common::new(), seen, preview: preview.clone() }.id_salt("chip").show_at(ui, CHIP);
        Slot { common: Common::new(), seen }.id_salt("slot").show_at(ui, SLOT);
    }
}

/// The pointer on the chip with the button down, then moved over the slot: a drag in progress.
fn drag_over_slot(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) -> Pos2 {
    let from = CHIP.center();
    let over = SLOT.center();
    frames.idle(2, &mut *show);
    frames.frame(vec![moved(from)], &mut *show);
    frames.frame(vec![press(from)], &mut *show);
    for at in [from + Vec2::new(30.0, 0.0), from + Vec2::new(100.0, 0.0), over] {
        frames.frame(vec![moved(at)], &mut *show);
    }
    frames.idle(1, &mut *show);
    over
}

/// A drag begun with the secondary button drops when that button comes up, in the source window.
#[trial]
fn a_secondary_button_drag_drops_on_the_secondary_release() {
    let seen = RefCell::new(Seen::default());
    let mut show = scene(&seen);
    let mut frames = Frames::new();
    let from = CHIP.center();
    let over = SLOT.center();
    frames.idle(2, &mut show);
    frames.frame(vec![moved(from)], &mut show);
    frames.frame(vec![secondary(from, true)], &mut show);
    for at in [from + Vec2::new(30.0, 0.0), from + Vec2::new(100.0, 0.0), over] {
        frames.frame(vec![moved(at)], &mut show);
    }
    frames.idle(1, &mut show);
    assert!(drag::active::<Payload>(&frames.ctx).is_some(), "the secondary-button drag runs");
    frames.frame(vec![secondary(over, false)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(seen.borrow().dropped, vec![(7, DropOperation::Move)], "the target takes the drop on the secondary release");
}

/// Escape ends the session and the press: the release that follows drops nothing and the source is told nothing.
#[trial]
fn escape_cancels_a_drag_and_nobody_gets_a_drop() {
    let seen = RefCell::new(Seen::default());
    let mut show = scene(&seen);
    let mut frames = Frames::new();
    let over = drag_over_slot(&mut frames, &mut show);
    assert!(drag::active::<Payload>(&frames.ctx).is_some(), "the drag runs");
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    assert!(drag::active::<Payload>(&frames.ctx).is_none(), "Escape ends the session");
    frames.frame(vec![release(over)], &mut show);
    frames.idle(2, &mut show);
    let seen = seen.borrow();
    assert!(seen.dropped.is_empty() && seen.ended.is_empty(), "no drop and no end: {:?} {:?}", seen.dropped, seen.ended);
}

/// A release over no target ends the drag with nothing to report, and the not-allowed cursor shows while nothing accepts.
#[trial]
fn a_release_over_no_target_reports_nothing_and_the_cursor_says_so() {
    let seen = RefCell::new(Seen::default());
    let mut show = scene(&seen);
    let mut frames = Frames::new();
    let over = drag_over_slot(&mut frames, &mut show);
    assert_eq!(frames.cursor_icon(), egui::CursorIcon::Grabbing, "an accepting target shows the move cursor");
    let away = Pos2::new(500.0, 400.0);
    frames.frame(vec![moved(away)], &mut show);
    frames.idle(1, &mut show);
    assert_eq!(frames.cursor_icon(), egui::CursorIcon::NotAllowed, "nothing accepts here");
    frames.frame(vec![release(away)], &mut show);
    frames.idle(2, &mut show);
    let seen = seen.borrow();
    assert!(seen.dropped.is_empty() && seen.ended.is_empty(), "{:?} {:?}", seen.dropped, seen.ended);
    let _ = over;
}

/// Another window's pass in the middle of a drag leaves the session running.
#[trial]
fn another_viewports_pass_does_not_end_a_drag() {
    let seen = RefCell::new(Seen::default());
    let mut show = scene(&seen);
    let mut frames = Frames::new();
    drag_over_slot(&mut frames, &mut show);
    assert!(drag::active::<Payload>(&frames.ctx).is_some(), "the drag runs");
    frames.frame_in_viewport(egui::ViewportId::from_hash_of("child"), |_| {});
    assert!(drag::active::<Payload>(&frames.ctx).is_some(), "a pass of another viewport keeps the session");
}

/// A layer over a target hides it: a release there drops nothing and the not-allowed cursor shows.
#[trial]
fn a_target_under_another_layer_takes_no_drop() {
    let seen = RefCell::new(Seen::default());
    let covered = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        scene(&seen)(ui);
        if covered.get() {
            craft_ui::testing::frames::show_area(ui.ctx(), "cover", |cover| {
                Block::new(Vec2::new(400.0, 100.0)).show(cover);
            });
        }
    };
    let mut frames = Frames::new();
    let over = drag_over_slot(&mut frames, &mut show);
    frames.idle(1, &mut show);
    assert_eq!(frames.cursor_icon(), egui::CursorIcon::Grabbing, "the uncovered slot accepts");
    covered.set(true);
    frames.idle(2, &mut show);
    frames.frame(vec![moved(over + Vec2::new(1.0, 0.0))], &mut show);
    frames.idle(1, &mut show);
    assert_eq!(frames.cursor_icon(), egui::CursorIcon::NotAllowed, "the layer over the slot hides it");
    frames.frame(vec![release(over)], &mut show);
    frames.idle(2, &mut show);
    assert!(seen.borrow().dropped.is_empty(), "no drop lands under the layer: {:?}", seen.borrow().dropped);
}

/// A drag that starts in one window and ends over a target in another: the session maps the pointer to desktop
/// space, the other window's target takes the drop one frame after the release, and the source learns the
/// operation in the frame after that.
#[trial]
fn a_drag_crosses_windows_and_the_target_in_the_other_window_takes_the_drop() {
    let seen = RefCell::new(Seen::default());
    let windows = Windows::new();
    let other = ViewportId::from_hash_of("other");
    let at = |min: Pos2, size: Vec2| ViewportInfo { inner_rect: Some(Rect::from_min_size(min, size)), ..ViewportInfo::default() };
    windows.root_info(at(Pos2::ZERO, Vec2::new(800.0, 600.0)));
    windows.child_info(at(Pos2::new(100.0, 100.0), Vec2::new(400.0, 300.0)));
    let frame = |events: Vec<egui::Event>| {
        windows.frame(events, vec![], |ui| {
            Chip { common: Common::new(), seen: &seen, preview: DragPreview::Badge(Label::Static(BADGE)) }.id_salt("chip").show_at(ui, CHIP);
            ui.ctx().show_viewport_immediate(other, ViewportBuilder::default(), |other_ui, _| {
                Slot { common: Common::new(), seen: &seen }.id_salt("slot").show_at(other_ui, SLOT);
            });
        });
    };
    // The slot's centre in the other window is (250, 35); that window sits at (100, 100) on the desktop, so the
    // source window's pointer is at (350, 135).
    let over = Pos2::new(350.0, 135.0);
    let from = CHIP.center();
    for _ in 0..2 {
        frame(vec![]);
    }
    frame(vec![moved(from)]);
    frame(vec![press(from)]);
    for to in [from + Vec2::new(30.0, 0.0), from + Vec2::new(100.0, 0.0), over] {
        frame(vec![moved(to)]);
    }
    frame(vec![]);
    assert!(seen.borrow().dropped.is_empty(), "nothing drops while the button is down");
    frame(vec![release(over)]);
    for _ in 0..3 {
        frame(vec![]);
    }
    let seen = seen.borrow();
    assert_eq!(seen.dropped, vec![(7, DropOperation::Move)], "the other window's target took the drop once");
    assert_eq!(seen.ended, vec![DropOperation::Move], "the source saw the operation once");
}

const LIST: Rect = Rect { min: Pos2::new(20.0, 100.0), max: Pos2::new(220.0, 220.0) };
const ROW: f32 = 30.0;
const WINDOW: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(800.0, 600.0) };

/// Where files landed: the row of the list, or the window.
#[derive(Default)]
struct Landed {
    rows: Vec<(usize, Vec<String>)>,
    window: Vec<Vec<String>>,
}

fn names(files: &OsFiles) -> Vec<String> {
    files.files.iter().map(|file| file.name.clone()).collect()
}

/// A drop of files on the row under the pointer.
struct OnRow;

impl DropTarget<OsFiles> for OnRow {
    type Drop = (usize, Vec<String>);

    fn accept(&self, files: &OsFiles, at: Pos2, _: DropOperation) -> Option<Self::Drop> {
        LIST.contains(at).then(|| (((at.y - LIST.min.y) / ROW) as usize, names(files)))
    }

    fn mark(&self, (row, _): &Self::Drop) -> DropMark {
        DropMark::Into(Rect::from_min_size(LIST.min + Vec2::new(0.0, *row as f32 * ROW), Vec2::new(LIST.width(), ROW)))
    }
}

/// A drop of files anywhere in the window.
struct OnWindow;

impl WindowDropTarget<OsFiles> for OnWindow {
    type Drop = Vec<String>;

    fn accept(&self, files: &OsFiles, _: DropOperation) -> Option<Self::Drop> {
        Some(names(files))
    }

    fn mark(&self, _: &Self::Drop) -> DropMark {
        DropMark::Into(WINDOW)
    }
}

/// A list of rows that takes files on a row, with the window as the target of every other drop.
#[derive(Component)]
struct FileList<'a> {
    #[common]
    common: Common,
    landed: &'a RefCell<Landed>,
}

impl Component for FileList<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FileListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        LIST.size()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(row) = cx.drop_target(&OnRow) {
            self.landed.borrow_mut().rows.push(row);
        }
        if let Some(files) = cx.window_drop_target(&OnWindow) {
            self.landed.borrow_mut().window.push(files);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Files held over `at` and dropped there.
fn drop_files_at(at: Pos2, files: &[&str]) -> Landed {
    let landed = RefCell::new(Landed::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        FileList { common: Common::new(), landed: &landed }.id_salt("files").show_at(ui, LIST);
    };
    frames.idle(2, &mut show);
    frames.hover_files(files);
    frames.frame(vec![moved(at)], &mut show);
    frames.frame(vec![moved(at)], &mut show);
    frames.drop_files(files);
    frames.frame(vec![moved(at)], &mut show);
    frames.idle(1, &mut show);
    landed.into_inner()
}

#[trial]
fn a_file_drop_on_a_list_lands_at_the_pointer() {
    let files = ["a.png", "b.png"];
    let third_row = Pos2::new(60.0, LIST.min.y + 2.0 * ROW + 5.0);
    let landed = drop_files_at(third_row, &files);
    assert_eq!(landed.rows, vec![(2, files.map(String::from).to_vec())], "the files land on the row under the pointer");
    assert!(landed.window.is_empty(), "the window target does not take what a target over a component took");
}

#[trial]
fn a_file_drop_outside_every_component_lands_on_the_window_target() {
    let landed = drop_files_at(Pos2::new(500.0, 400.0), &["a.png"]);
    assert!(landed.rows.is_empty(), "the list takes no drop outside its rows");
    assert_eq!(landed.window, vec![vec![String::from("a.png")]], "the window target takes it");
}

#[trial]
fn files_that_leave_the_window_end_the_drag_without_a_drop() {
    let landed = RefCell::new(Landed::default());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        FileList { common: Common::new(), landed: &landed }.id_salt("files").show_at(ui, LIST);
    };
    frames.idle(2, &mut show);
    frames.hover_files(&["a.png"]);
    frames.frame(vec![moved(LIST.center())], &mut show);
    assert!(drag::active::<OsFiles>(&frames.ctx).is_some(), "hovering files are the active payload");
    frames.leave_files();
    frames.frame(vec![moved(LIST.center())], &mut show);
    assert!(drag::active::<OsFiles>(&frames.ctx).is_none(), "the drag ends when the files leave");
    assert!(landed.borrow().rows.is_empty() && landed.borrow().window.is_empty(), "nothing landed");
}

#[trial]
fn a_dropped_file_keeps_the_media_type_it_had_while_hovering() {
    let seen = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let landed = RefCell::new(Landed::default());
        FileList { common: Common::new(), landed: &landed }.id_salt("files").show_at(ui, LIST);
        if let Some(files) = drag::active::<OsFiles>(ui.ctx()) {
            *seen.borrow_mut() = files.files.iter().map(|file| (file.name.clone(), file.mime.clone())).collect();
        }
    };
    frames.idle(2, &mut show);
    frames.hover_typed_files(&[("a.png", "image/png"), ("b.txt", "text/plain")]);
    frames.frame(vec![moved(LIST.center())], &mut show);
    frames.drop_files(&["b.txt", "a.png", "c.bin"]);
    frames.frame(vec![moved(LIST.center())], &mut show);
    let expected = [("b.txt", "text/plain"), ("a.png", "image/png"), ("c.bin", "")].map(|(name, mime)| (name.to_owned(), mime.to_owned()));
    assert_eq!(*seen.borrow(), expected, "a drop carries the type the file had while hovering, and none for a file never seen");
}

/// `n` rows keyed by position.
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

/// Files dropped on the lower half of a row of a `List` that takes files land before the next row; a list that does
/// not take them reports nothing.
#[trial]
fn a_file_drop_on_a_list_reports_the_files_at_the_insert_position() {
    let files = ["a.png", "b.png"];
    let rows = Rows(5);
    let rects = RefCell::new(Vec::<Rect>::new());
    let dropped = |takes: bool| {
        let events = RefCell::new(Vec::new());
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| {
            craft_ui::testing::layout::in_region(ui, LIST, |ui| {
                let out = craft_ui::list::List::new("Files").takes_files(takes).id_salt("files").show_items(ui, &rows, &mut |_, cx| {
                    let mut seen = rects.borrow_mut();
                    seen.resize(rows.0, Rect::NOTHING);
                    seen[cx.at] = cx.rect;
                    craft_ui::list::ListRow::new("row")
                });
                events.borrow_mut().extend(out.events);
            });
        };
        frames.idle(3, &mut show);
        let row = rects.borrow()[2];
        let at = Pos2::new(row.center().x, row.max.y - 2.0);
        frames.hover_files(&files);
        frames.frame(vec![moved(at)], &mut show);
        frames.frame(vec![moved(at)], &mut show);
        frames.drop_files(&files);
        frames.frame(vec![moved(at)], &mut show);
        frames.idle(1, &mut show);
        events.into_inner()
    };
    let landed: Vec<_> = dropped(true)
        .into_iter()
        .filter_map(|event| match event {
            CollectionEvent::FilesDropped { insert, files, .. } => Some((insert, files.into_iter().map(|file| file.name).collect::<Vec<_>>())),
            _ => None,
        })
        .collect();
    assert_eq!(landed, vec![(3, files.map(String::from).to_vec())], "the files land before the row after the one under the pointer");
    assert!(
        !dropped(false).iter().any(|event| matches!(event, CollectionEvent::FilesDropped { .. })),
        "a list that does not take files reports no drop"
    );
}
