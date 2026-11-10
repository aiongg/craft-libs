//! The label step: placement, mirroring, scrub, double-click reset and the reset dot.

mod align;
mod caption;
mod glyph;
pub mod group;
pub mod kit;
mod message;

use self::kit::{Dial, Field, LabelledDial, RESET, SIZE, Seen, Wide};
use craft_host::target::ShellInfo;
use craft_i18n::Direction;
use craft_ui::contract::{FieldLabel, FieldLayout, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::pointer::{drag, moved, press, release};
use craft_ui::testing::windows::FakeSystem;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Shape, Ui, Vec2};
use std::cell::RefCell;
use std::sync::Arc;

/// The label drag of the scrub trial, in points.
const DRAG: f32 = 30.0;
/// How far a scrub may differ from the drag: pixel rounding.
const ROUNDING: f64 = 1.5;
/// How far right of the row start a label beside its control begins at least.
const BESIDE: f32 = 20.0;
/// How far below the row start a stacked label begins at least.
const STACKED: f32 = 5.0;
/// The value the double-click trial starts from.
const START: f64 = 10.0;

const ROW_A: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 40.0) };
const ROW_B: Rect = Rect { min: Pos2::new(10.0, 60.0), max: Pos2::new(310.0, 90.0) };

/// Two rows of the same field: a container and a leaf composite.
struct Rows {
    container: f64,
    leaf: f64,
    seen_container: RefCell<Seen>,
    seen_leaf: RefCell<Seen>,
}

impl Rows {
    fn new(value: f64) -> Self {
        Rows { container: value, leaf: value, seen_container: RefCell::default(), seen_leaf: RefCell::default() }
    }

    fn show(&mut self, ui: &mut Ui, side: &SideLabel) {
        let (container, seen_container) = (&mut self.container, &self.seen_container);
        Field::new("Opacity", side.clone()).id_salt("container").show_at(ui, ROW_A, |ui, _| {
            Dial::new(&mut *container, seen_container).show(ui);
        });
        LabelledDial::new("Opacity", side.clone(), &mut self.leaf, &self.seen_leaf).id_salt("leaf").show_at(ui, ROW_B);
    }

    fn label(&self, container: bool) -> FieldLabel {
        let seen = if container { &self.seen_container } else { &self.seen_leaf };
        seen.borrow().label.clone().unwrap_or_else(|| panic!("the dial ran inside a label scope"))
    }

    fn control(&self, container: bool) -> Rect {
        (if container { &self.seen_container } else { &self.seen_leaf }).borrow().rect
    }
}

fn settled(direction: Direction, side: &SideLabel) -> (Frames, Rows) {
    let mut frames = Frames::new();
    let mut root = RootEnv::new();
    root.direction = direction;
    frames.root(root);
    let mut rows = Rows::new(START);
    frames.idle(3, |ui| rows.show(ui, side));
    (frames, rows)
}

/// Where the label and the control sit, from the row's start edge.
fn offsets(rows: &Rows, container: bool, row: Rect, rtl: bool) -> (f32, f32, f32, f32) {
    let (label, control) = (rows.label(container).rect, rows.control(container));
    let from = |rect: Rect| if rtl { row.max.x - rect.max.x } else { rect.min.x - row.min.x };
    (from(label), from(control), label.min.y - row.min.y, control.min.y - row.min.y)
}

#[trial]
fn a_labelled_leaf_and_a_labelled_container_line_up_their_labels_and_mirror_under_rtl() {
    let side = SideLabel::new();
    for rtl in [false, true] {
        let (_, rows) = settled(if rtl { Direction::Rtl } else { Direction::Ltr }, &side);
        let (container, leaf) = (offsets(&rows, true, ROW_A, rtl), offsets(&rows, false, ROW_B, rtl));
        assert_eq!(
            (container.0.round(), container.1.round(), container.2.round(), container.3.round()),
            (leaf.0.round(), leaf.1.round(), leaf.2.round(), leaf.3.round()),
            "the container and the leaf place label and control alike (rtl {rtl})"
        );
        assert!(container.1 > container.0, "the control follows the label in reading order (rtl {rtl})");
        let label = rows.label(true).rect;
        assert_eq!(label.min.x < rows.control(true).min.x, !rtl, "the label is on the start side, mirrored under RTL");
    }
}

#[trial]
fn dragging_a_label_that_scrubs_changes_the_value() {
    let side = SideLabel::new().scrub(true);
    let (mut frames, mut rows) = settled(Direction::Ltr, &side);
    for container in [true, false] {
        let before = if container { rows.container } else { rows.leaf };
        let from = rows.label(container).rect.center();
        drag(&mut frames, from, Vec2::new(DRAG, 0.0), &mut |ui| rows.show(ui, &side));
        let after = if container { rows.container } else { rows.leaf };
        assert!(
            (after - before - f64::from(DRAG)).abs() < ROUNDING,
            "a drag on the label scrubs by its length: {before} -> {after} (container: {container})"
        );
    }
}

/// The window's right edge as the scrub trial's pointer reaches it, and the point the warp puts it at.
const EDGE_X: f32 = 799.0;
const LANDED_X: f32 = 4.0;
/// How far two values may differ and still count as the same.
const SAME: f64 = 0.01;

#[trial]
fn a_wrapped_scrub_keeps_its_value_continuous() {
    let system = Arc::new(FakeSystem::new());
    let mut frames = Frames::uninstalled();
    let installed = craft_ui::install(&frames.ctx, InstallSpec::new(ShellInfo::default(), system.clone()));
    assert!(installed.is_ok(), "the context takes the runtime once");
    let side = SideLabel::new().scrub(true);
    let seen = RefCell::new(Seen::default());
    let mut value = START;
    let show = |ui: &mut Ui, value: &mut f64| {
        Field::new("Opacity", side.clone()).id_salt("warp").show_at(ui, ROW_A, |ui, _| {
            Dial::new(value, &seen).warping().show(ui);
        });
    };
    frames.idle(3, |ui| show(ui, &mut value));
    let label = seen.borrow().label.clone().map(|label| label.rect.center());
    let from = label.unwrap_or_else(|| panic!("the dial ran inside a label scope"));
    let step = |frames: &mut Frames, value: &mut f64, events: Vec<egui::Event>, at: Pos2| {
        system.set_pointer(Some([at.x, at.y]));
        frames.frame(events, |ui| show(ui, value));
    };
    step(&mut frames, &mut value, vec![moved(from)], from);
    step(&mut frames, &mut value, vec![press(from)], from);
    step(&mut frames, &mut value, vec![moved(from + Vec2::new(20.0, 0.0))], from + Vec2::new(20.0, 0.0));
    let edge = Pos2::new(EDGE_X, from.y);
    step(&mut frames, &mut value, vec![moved(edge)], edge);
    assert_eq!(system.warps().len(), 1, "reaching the edge warps the pointer once");
    assert_eq!(frames.cursor_icon(), egui::CursorIcon::None, "the scrub hides the pointer");
    let before = value;
    let landed = Pos2::new(LANDED_X, from.y);
    step(&mut frames, &mut value, vec![moved(landed)], landed);
    assert!((value - before).abs() < SAME, "the warp does not move the value: {before} -> {value}");
    let on = landed + Vec2::new(DRAG, 0.0);
    step(&mut frames, &mut value, vec![moved(on)], on);
    assert!((value - before - f64::from(DRAG)).abs() < SAME, "the drag goes on from the value before the warp: {before} -> {value}");
    assert_eq!(system.warps().len(), 1, "the pointer far from an edge does not warp again");
}

fn double_click(frames: &mut Frames, at: Pos2, show: &mut impl FnMut(&mut Ui)) {
    frames.frame(vec![moved(at)], &mut *show);
    for _ in 0..2 {
        frames.frame(vec![press(at)], &mut *show);
        frames.frame(vec![release(at)], &mut *show);
    }
    frames.frame(vec![], &mut *show);
}

#[trial]
fn double_clicking_the_label_resets_the_value() {
    let side = SideLabel::new();
    let (mut frames, mut rows) = settled(Direction::Ltr, &side);
    for container in [true, false] {
        let at = rows.label(container).rect.center();
        double_click(&mut frames, at, &mut |ui| rows.show(ui, &side));
        let value = if container { rows.container } else { rows.leaf };
        assert_eq!(value, RESET, "a double click on the label restores the reset value (container: {container})");
    }
}

fn circles(shapes: &[Shape]) -> usize {
    shapes
        .iter()
        .map(|shape| match shape {
            Shape::Circle(_) => 1,
            Shape::Vec(inner) => circles(inner),
            _ => 0,
        })
        .sum()
}

#[trial]
fn the_reset_dot_shows_only_while_the_value_differs_and_resets_on_a_click() {
    let side = SideLabel::new();
    let (mut frames, mut rows) = settled(Direction::Ltr, &side);
    let at_reset = {
        rows.container = RESET;
        frames.idle(2, |ui| rows.show(ui, &side));
        circles(&frames.frame_shapes(|ui| rows.show(ui, &side)))
    };
    rows.container = RESET + 5.0;
    frames.idle(2, |ui| rows.show(ui, &side));
    let differing = circles(&frames.frame_shapes(|ui| rows.show(ui, &side)));
    assert_eq!(differing, at_reset + 1, "one dot while the value differs, none at the reset value");
    let label = rows.label(true).rect;
    let dot = Pos2::new(label.right() - 1.0, label.center().y);
    craft_ui::testing::pointer::click(&mut frames, dot, &mut |ui| rows.show(ui, &side));
    assert_eq!(rows.container, RESET, "a click on the dot resets the value");
    frames.idle(2, |ui| rows.show(ui, &side));
    assert_eq!(circles(&frames.frame_shapes(|ui| rows.show(ui, &side))), at_reset, "the dot is gone after the reset");
}

#[trial]
fn a_label_stays_beside_a_fill_width_control_and_follows_the_row_width() {
    let mut frames = Frames::new();
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let at = |frames: &mut Frames, width: f32, count: usize| {
        let row = Rect::from_min_size(ROW_A.min, Vec2::new(width, 60.0));
        frames.idle(count, |ui| {
            Field::new("Opacity", SideLabel::new()).id_salt("fill").show_at(ui, row, |ui, _| {
                rect.set(Wide::new().show(ui).rect);
            });
        });
        rect.get()
    };
    let wide = at(&mut frames, 300.0, 6);
    assert!(wide.min.x > ROW_A.min.x + BESIDE, "a wide row keeps the label beside a control that fills its width: {wide:?}");
    let narrow = at(&mut frames, 80.0, 4);
    assert!(narrow.min.y > ROW_A.min.y + STACKED, "a row narrower than label, gap and the control's minimum stacks the label: {narrow:?}");
    let again = at(&mut frames, 300.0, 4);
    assert!(again.min.x > ROW_A.min.x + BESIDE, "the label returns beside the control when the row widens: {again:?}");
}

#[trial]
fn an_after_label_follows_its_control_and_mirrors_under_rtl() {
    let side = SideLabel::new().layout(FieldLayout::After);
    for rtl in [false, true] {
        let (_, rows) = settled(if rtl { Direction::Rtl } else { Direction::Ltr }, &side);
        for (container, row) in [(true, ROW_A), (false, ROW_B)] {
            let (label, control, _, _) = offsets(&rows, container, row, rtl);
            assert!(control < label, "the control is first in reading order (rtl {rtl}, container {container})");
            assert!(
                label >= control + rows.control(container).width(),
                "the label starts after the control's end, on the end side under RTL (rtl {rtl}, container {container})"
            );
            assert!(control < BESIDE, "the control stays at the start edge: the row is not split into a label column");
        }
    }
}

#[trial]
fn a_body_labelled_row_takes_the_height_of_its_placement() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut value = START;
    let used = std::cell::Cell::new(0.0);
    frames.idle(3, |ui| {
        Field::new("Group", SideLabel::new().kind(craft_ui::contract::LabelKind::Hidden)).id_salt("body").show_at(ui, ROW_A, |ui, cx| {
            let before = ui.available_rect_before_wrap().min.y;
            cx.labelled(ui, "dial", "Opacity".into(), SideLabel::new(), Dial::new(&mut value, &seen));
            // The body's gap follows every row; the row itself ends before it.
            used.set(ui.available_rect_before_wrap().min.y - before - ui.spacing().item_spacing.y);
        });
    });
    assert!(
        (used.get() - SIZE.y).abs() < f32::from(1u8) / 100.0,
        "a label beside its child adds no height: the row is the child's own, not a multiple of it"
    );
}

/// An after-label whose detail does not fit beside the control wraps in the room left, and the row grows to hold it.
#[trial]
fn an_after_labels_detail_wraps_in_the_room_the_control_leaves() {
    let used_by = |detail: &str, width: f32| {
        let mut frames = Frames::new();
        let seen = RefCell::new(Seen::default());
        let mut value = START;
        let used = std::cell::Cell::new(0.0);
        let area = Rect::from_min_size(ROW_A.min, Vec2::new(width, 200.0));
        frames.idle(3, |ui| {
            Field::new("Group", SideLabel::new().kind(craft_ui::contract::LabelKind::Hidden)).id_salt("after").show_at(ui, area, |ui, cx| {
                let before = ui.available_rect_before_wrap().min.y;
                let side = SideLabel::new().layout(FieldLayout::After).detail(detail.to_owned());
                cx.labelled(ui, "dial", "Opacity".into(), side, Dial::new(&mut value, &seen));
                used.set(ui.available_rect_before_wrap().min.y - before - ui.spacing().item_spacing.y);
            });
        });
        used.get()
    };
    let long = "A description far longer than the room that is left beside the control, which must wrap";
    assert!(used_by(long, 160.0) > used_by("Short", 400.0), "the wrapped detail makes the row taller than a one-line label does");
}
