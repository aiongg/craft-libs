//! The label step: placement, mirroring, scrub, double-click reset and the reset dot.

pub mod kit;

use self::kit::{Dial, Field, LabelledDial, RESET, Seen, Wide};
use craft_host::target::ShellInfo;
use craft_i18n::Direction;
use craft_ui::contract::{FieldLabel, FieldLabelRole, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::access::NodeKey;
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
fn a_narrow_row_puts_the_label_above_the_control() {
    let mut frames = Frames::new();
    let narrow = Rect::from_min_size(ROW_A.min, Vec2::new(60.0, 60.0));
    let seen = RefCell::new(Seen::default());
    let mut value = 10.0;
    frames.idle(3, |ui| {
        Field::new("Opacity", SideLabel::new()).id_salt("narrow").show_at(ui, narrow, |ui, _| {
            Dial::new(&mut value, &seen).show(ui);
        });
    });
    let label = seen.borrow().label.clone().map(|label| label.rect);
    assert!(label.is_some_and(|label| seen.borrow().rect.min.y >= label.max.y), "the control sits below the label");
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
fn a_label_that_describes_leaves_the_control_its_name() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut value = 1.0;
    let describes = SideLabel::new().role(FieldLabelRole::Describes);
    frames.idle(2, |ui| {
        Field::new("Opacity", describes.clone()).id_salt("describes").show_at(ui, ROW_A, |ui, _| {
            Dial::new(&mut value, &seen).label("Alpha").show(ui);
        });
    });
    let tree = frames.tree();
    let (id, label) = (seen.borrow().id, seen.borrow().label.clone());
    let (Some(id), Some(label)) = (id, label) else { panic!("the dial ran in a label scope") };
    assert_eq!(tree.name(id), Some("Alpha"), "the control keeps its own name");
    assert_eq!(tree.described_by(id), [NodeKey::of(label.id)], "the label describes it");
}

#[trial]
fn double_clicking_a_value_component_resets_it_and_a_single_click_does_not() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut value = START;
    let show = |ui: &mut Ui, value: &mut f64| {
        Dial::new(value, &seen).id_salt("alone").show_at(ui, ROW_A);
    };
    frames.idle(2, |ui| show(ui, &mut value));
    let at = ROW_A.center();
    craft_ui::testing::pointer::click(&mut frames, at, &mut |ui| show(ui, &mut value));
    assert_eq!(value, START, "one click leaves the value");
    double_click(&mut frames, at, &mut |ui| show(ui, &mut value));
    assert_eq!(value, RESET, "a double click on the control's root restores the reset value");
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

/// A label with an icon and a detail names its control by the text alone; the detail joins the control's
/// description, and the icon and detail sit inside the row.
#[trial]
fn a_label_with_an_icon_and_detail_names_by_text_and_describes_by_detail() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut value = 1.0;
    let side = SideLabel::new().icon(craft_behavior::IconId::new("eye")).detail("12 layers");
    let row = |ui: &mut Ui, value: &mut f64| {
        Field::new("Opacity", side.clone()).id_salt("detailed").show_at(ui, ROW_A, |ui, _| {
            Dial::new(value, &seen).show(ui);
        });
    };
    frames.idle(3, |ui| row(ui, &mut value));
    let tree = frames.tree();
    let (id, label) = (seen.borrow().id, seen.borrow().label.clone());
    let (Some(id), Some(label)) = (id, label) else { panic!("the dial ran in a label scope") };
    assert_eq!(tree.labelled_by(id), [NodeKey::of(label.id)], "the label names the control");
    assert_eq!(tree.name(label.id), Some("Opacity"), "the name is the text alone");
    assert_eq!(tree.description(id), Some("12 layers"), "the detail is the control's description");
    assert!(ROW_A.contains_rect(label.rect), "the icon, text and detail sit inside the row: {:?}", label.rect);
    let plain = {
        let mut plain_frames = Frames::new();
        let plain_seen = RefCell::new(Seen::default());
        let mut plain_value = 1.0;
        plain_frames.idle(3, |ui| {
            Field::new("Opacity", SideLabel::new()).id_salt("detailed").show_at(ui, ROW_A, |ui, _| {
                Dial::new(&mut plain_value, &plain_seen).show(ui);
            });
        });
        let rect = plain_seen.borrow().label.clone().map(|label| label.rect);
        rect
    };
    let plain = plain.expect("the plain label ran");
    assert!(label.rect.width() > plain.width(), "the icon and detail widen the label: {:?} against {plain:?}", label.rect);
}
