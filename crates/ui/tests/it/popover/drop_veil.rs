//! `DropVeil`: files dropped anywhere in the window reach the output once, a hover that ends without a drop reports
//! nothing, a modal blocks the veil behind it, and the veil shows only while it is the target that wins.

use crate::overlay::kit::{Opened, Opener, PlainStyle, Seen};
use craft_behavior::DropOperation;
use craft_ui::choice::Checkbox;
use craft_ui::contract::{
    Bind, Common, Component, Cx, DropMark, DropTarget, MeasureCx, OsFiles, Overlay, Pointer as _, Props as _, Role, Show as _, ShowContainer as _,
    Trigger,
};
use craft_ui::popover::DropVeil;
use craft_ui::style::StyleCx;
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const OPENER: Rect = Rect { min: Pos2::new(600.0, 100.0), max: Pos2::new(680.0, 120.0) };

fn show(ui: &mut Ui, dropped: &RefCell<Vec<OsFiles>>) {
    let out = DropVeil::new("Import 2 photos").show(ui, |ui, _| {
        Checkbox::new(Bind::given(false)).label("Keep originals").show(ui);
    });
    dropped.borrow_mut().extend(out.dropped.clone());
}

/// Files that hover or leave report nothing; one drop reports once.
#[trial]
fn a_drop_reports_once_and_a_hover_that_leaves_reports_nothing() {
    let mut frames = Frames::new();
    let dropped = RefCell::new(Vec::new());
    let mut run = |ui: &mut Ui| show(ui, &dropped);
    frames.frame(vec![moved(Pos2::new(300.0, 200.0))], &mut run);
    frames.hover_files(&["a.png", "b.png"]);
    frames.idle(3, &mut run);
    frames.leave_files();
    frames.idle(4, &mut run);
    assert!(dropped.borrow().is_empty(), "hovering files, even ones that leave, are not a drop");
    frames.hover_files(&["a.png", "b.png"]);
    frames.idle(3, &mut run);
    frames.drop_files(&["a.png", "b.png"]);
    frames.idle(4, &mut run);
    let dropped = dropped.borrow();
    assert_eq!(dropped.len(), 1, "one drop in one pass");
    assert_eq!(dropped[0].files.len(), 2, "with every file");
}

/// Files that hover before the window has any pointer position show the veil and drop on the whole window.
#[trial]
fn files_hovering_with_no_pointer_position_show_the_veil_and_drop_on_the_window() {
    let mut frames = Frames::new();
    let dropped = RefCell::new(Vec::new());
    let mut run = |ui: &mut Ui| show(ui, &dropped);
    let named = |frames: &mut Frames| {
        let tree = frames.tree();
        tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some("Import 2 photos")).count()
    };
    frames.idle(3, &mut run);
    let before = named(&mut frames);
    frames.hover_files(&["a.png", "b.png"]);
    frames.idle(3, &mut run);
    assert!(named(&mut frames) > before, "the card's text shows while the files hover");
    frames.drop_files(&["a.png", "b.png"]);
    frames.idle(4, &mut run);
    let names: Vec<_> = dropped.borrow().iter().flat_map(|files| files.files.iter().map(|file| file.name.clone())).collect();
    assert_eq!(names, ["a.png", "b.png"]);
}

/// The count of AccessKit nodes named by the veil's label: it grows by the card's text while the veil shows.
fn named(frames: &mut Frames) -> usize {
    let tree = frames.tree();
    tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some("Import 2 photos")).count()
}

/// A modal holds input, so a veil on the window behind it neither shows nor takes the drop.
#[trial]
fn a_modal_blocks_the_veil_behind_it() {
    let mut frames = Frames::new();
    let dropped = RefCell::new(Vec::new());
    let (opened, content) = (Opened::default(), RefCell::new(Seen::default()));
    let open = Cell::new(false);
    let mut run = |ui: &mut Ui| {
        opened.shown.set(false);
        show(ui, &dropped);
        Opener::new(&opened, &content, Trigger::Manual, Overlay::modal).open(open.get()).id_salt("opener").show_at(ui, OPENER);
    };
    frames.idle(3, &mut run);
    open.set(true);
    frames.frame(vec![], &mut run);
    open.set(false);
    frames.idle(30, &mut run);
    assert!(opened.shown.get(), "the modal shows");
    let before = named(&mut frames);
    frames.hover_files(&["a.png", "b.png"]);
    frames.idle(4, &mut run);
    assert_eq!(named(&mut frames), before, "the veil does not show while a modal holds input");
    frames.drop_files(&["a.png", "b.png"]);
    frames.idle(4, &mut run);
    assert!(dropped.borrow().is_empty(), "a drop reports nothing to the veil behind the modal");
}

/// The body list's target: files over the list.
struct OnList(Rect);

impl DropTarget<OsFiles> for OnList {
    type Drop = ();

    fn accept(&self, _: &OsFiles, at: Pos2, _: DropOperation) -> Option<()> {
        self.0.contains(at).then_some(())
    }

    fn mark(&self, _: &()) -> DropMark {
        DropMark::Into(self.0)
    }
}

/// A list in the veil's body that accepts files at the pointer.
#[derive(Component)]
struct FileList {
    #[common]
    common: Common,
}

impl Component for FileList {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FileListStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(200.0, 100.0)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let rect = cx.rect;
        cx.drop_target(&OnList(rect));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A target over a component beats the veil's whole-window target, and the veil then stays hidden.
#[trial]
fn the_veil_does_not_show_where_a_list_in_its_body_takes_the_drop() {
    let mut frames = Frames::new();
    let list = Cell::new(Rect::NOTHING);
    let mut run = |ui: &mut Ui| {
        DropVeil::new("Import 2 photos").show(ui, |ui, _| {
            list.set(FileList { common: Common::new() }.id_salt("list").show(ui).rect);
        });
    };
    frames.idle(3, &mut run);
    let before = named(&mut frames);
    let (inside, outside) = (list.get().center(), Pos2::new(700.0, 500.0));
    frames.hover_files(&["a.png", "b.png"]);
    frames.frame(vec![moved(inside)], &mut run);
    frames.idle(4, &mut run);
    assert_eq!(named(&mut frames), before, "the list takes the drop at the pointer, so the veil stays hidden");
    frames.frame(vec![moved(outside)], &mut run);
    frames.idle(4, &mut run);
    assert!(named(&mut frames) > before, "away from the list the veil is the target and shows");
}
