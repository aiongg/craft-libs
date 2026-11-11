//! The warp grid, its options-bar group and the mesh model.

use craft_behavior::kurbo::{Point, Rect as DocRect, Vec2 as DocVec};
use craft_behavior::{CanvasView, EditEvent};
use craft_imaging::ui::warp::{WarpEdit, WarpFieldValues, WarpFields, WarpFieldsEdit, WarpGrid, WarpMesh, WarpSplit, WarpStyleEntry};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Room, pointer, trial};
use egui::{Id, Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A move of the pointer, in points.
fn shift(dx: f32, dy: f32) -> Vec2 {
    Vec2::new(dx, dy)
}

/// A move in document units.
fn doc_shift(dx: f64, dy: f64) -> DocVec {
    DocVec::new(dx, dy)
}

const RECT: DocRect = DocRect::new(100.0, 80.0, 280.0, 200.0);

/// What one pass reported about the edit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Begin,
    Update,
    Commit,
    Cancel,
    Set,
}

struct Harness {
    fixture: Fixture,
    mesh: Rc<RefCell<WarpMesh>>,
    split: Rc<Cell<Option<WarpSplit>>>,
    preset: Rc<Cell<bool>>,
    bounds: Rc<Cell<DocRect>>,
    view: Rc<Cell<Option<CanvasView>>>,
    seen: Rc<RefCell<Vec<Seen>>>,
    edit: Rc<Cell<Option<WarpEdit>>>,
    picked: Rc<Cell<Option<usize>>>,
}

fn harness(mesh: WarpMesh) -> Harness {
    harness_in(mesh, 0.0, false)
}

fn harness_in(mesh: WarpMesh, rotation: f64, flip: bool) -> Harness {
    let id = Rc::new(Cell::new(Id::NULL));
    let mesh = Rc::new(RefCell::new(mesh));
    let split = Rc::new(Cell::new(None));
    let preset = Rc::new(Cell::new(false));
    let lines_only = Rc::new(Cell::new(false));
    let bounds = Rc::new(Cell::new(RECT));
    let view = Rc::new(Cell::new(None));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let edit = Rc::new(Cell::new(None));
    let picked = Rc::new(Cell::new(None));
    let shown = (
        Rc::clone(&id),
        Rc::clone(&mesh),
        Rc::clone(&split),
        Rc::clone(&preset),
        Rc::clone(&lines_only),
        Rc::clone(&bounds),
        Rc::clone(&view),
        Rc::clone(&seen),
        Rc::clone(&edit),
        Rc::clone(&picked),
    );
    let mut fixture = Fixture::new(move |ui| {
        let (id, mesh, split, preset, lines_only, bounds, view, seen, edit, picked) = &shown;
        let room = ui.available_rect_before_wrap();
        let centre = Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0);
        let shown_view = CanvasView::new(room, centre, 1.0).rotation(rotation).flip_x(flip);
        view.set(Some(shown_view));
        let canvas = Canvas::new("Document").view(shown_view).show(ui, |ui, body| {
            let mut held = mesh.borrow_mut();
            let mut grid = WarpGrid::new("Warp", Bind::bound(&mut *held), bounds.get());
            if let Some(tool) = split.get() {
                grid = grid.split(tool);
            }
            if preset.get() {
                grid = grid.preset();
            }
            if lines_only.get() {
                grid = grid.lines_only();
            }
            grid.show_at(ui, body.rect)
        });
        if let Some(output) = canvas.inner {
            id.set(output.mesh.response.id);
            if output.edit.is_some() {
                edit.set(output.edit);
            }
            picked.set(output.picked);
            let phases = output.mesh.events.iter().map(|event| match event {
                EditEvent::Begin { .. } => Seen::Begin,
                EditEvent::Update { .. } => Seen::Update,
                EditEvent::Commit { .. } => Seen::Commit,
                EditEvent::Cancel { .. } => Seen::Cancel,
                _ => Seen::Set,
            });
            seen.borrow_mut().extend(phases);
        }
    });
    fixture.settle();
    Harness { fixture, mesh, split, preset, bounds, view, seen, edit, picked }
}

impl Harness {
    fn screen(&self, p: Point) -> Pos2 {
        self.view.get().expect("drawn").to_screen(p)
    }

    /// The pointer at fractions `(u, v)` across `RECT`.
    fn at_fraction(&self, u: f64, v: f64) -> Pos2 {
        self.screen(Point::new(RECT.x0 + u * RECT.width(), RECT.y0 + v * RECT.height()))
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.fixture.frame(vec![pointer::moved(from)]);
        self.fixture.frame(vec![pointer::press(from)]);
        for step in 1..=4 {
            self.fixture.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 4.0))]);
        }
        self.fixture.frame(vec![pointer::release(to)]);
        self.fixture.frame(Vec::new());
    }

    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.fixture.hold(modifiers);
        self.fixture.click_at(at);
        self.fixture.hold(Modifiers::NONE);
    }

    /// Tab to the grid: the canvas holds the first stop, the grid's points the next.
    fn focus_grid(&mut self) {
        self.fixture.key(Key::Tab, Modifiers::NONE);
        self.fixture.key(Key::Tab, Modifiers::NONE);
    }

    fn set(&mut self, mutate: impl FnOnce(&Harness)) {
        mutate(self);
        self.fixture.settle();
    }

    fn mesh(&self) -> WarpMesh {
        self.mesh.borrow().clone()
    }

    fn size(&self) -> (usize, usize) {
        let mesh = self.mesh.borrow();
        (mesh.columns(), mesh.rows())
    }

    fn count(&self, phase: Seen) -> usize {
        self.seen.borrow().iter().filter(|seen| **seen == phase).count()
    }
}

fn tooled(mesh: WarpMesh, tool: WarpSplit, preset: bool) -> Harness {
    let mut h = harness(mesh);
    h.set(|h| {
        h.split.set(Some(tool));
        h.preset.set(preset);
    });
    h
}

#[trial]
fn the_nearest_point_within_reach_takes_the_press_in_a_turned_flipped_view() {
    let reach = craft_ui::testing::style::tokens(craft_theme::Mode::Light, craft_theme::Contrast::Standard).size.hit.target;
    for (offset, taken) in [(reach - 0.5, true), (reach + 0.5, false)] {
        let mut h = harness_in(WarpMesh::identity(RECT, 1, 1), 30.0, true);
        // Out of the mesh along the bottom right anchor's diagonal, as the view draws it.
        let anchor = h.screen(Point::new(RECT.x1, RECT.y1));
        let out = (anchor - h.screen(RECT.center())).normalized();
        let at = anchor + out * offset;
        let pull = 20.0_f32;
        h.drag(at, at + out * pull);
        match h.edit.get() {
            Some(WarpEdit::Point { index, by, .. }) => {
                assert!(taken, "{offset} pt");
                assert_eq!(index, 15);
                assert!((by.hypot() - f64::from(pull)).abs() < 1e-3, "{by:?}");
            }
            other => assert!(!taken && other.is_none(), "{offset} pt: {other:?}"),
        }
    }

    // On a small mesh a handle 3 pt away beats its anchor 5 pt away.
    let mut h = harness(WarpMesh::identity(DocRect::new(190.0, 140.0, 214.0, 158.0), 1, 1));
    let at = h.screen(Point::new(195.0, 140.0));
    h.drag(at, at + shift(0.0, -10.0));
    assert!(matches!(h.edit.get(), Some(WarpEdit::Point { index: 1, .. })), "{:?}", h.edit.get());
    assert_eq!(h.picked.get(), Some(1));
}

#[trial]
fn a_press_on_the_surface_pulls_it_at_its_parameters() {
    let mut h = harness(WarpMesh::identity(RECT, 1, 1));
    let from = h.screen(RECT.center());
    h.drag(from, from + shift(20.0, 10.0));
    assert_eq!(h.seen.borrow().first(), Some(&Seen::Begin));
    assert_eq!(h.count(Seen::Commit), 1, "one drag commits once");
    let Some(WarpEdit::Surface { s, t, by, .. }) = h.edit.get() else { panic!("{:?}", h.edit.get()) };
    assert!((s - 0.5).abs() < 1e-3 && (t - 0.5).abs() < 1e-3, "({s}, {t})");
    assert!((by - doc_shift(20.0, 10.0)).hypot() < 1e-3, "{by:?}");
    assert!(h.mesh().eval(0.5, 0.5).distance(RECT.center() + by) < 1e-6, "the pull follows the pointer");

    // Off the mesh and off the points, with no modifier and no armed split, the press does nothing.
    h.edit.set(None);
    let off = h.screen(Point::new(40.0, 40.0));
    h.drag(off, off + shift(10.0, 0.0));
    assert_eq!(h.edit.get(), None);
}

#[trial]
fn a_mesh_names_its_anchors_and_handles() {
    let m = WarpMesh::identity(RECT, 3, 2);
    assert!(m.is_valid());
    assert_eq!((m.width(), m.height(), m.points.len()), (10, 7, 70));
    // Column 4, row 3 is an anchor; column 5 of that row its right handle; column 4, row 4 the
    // down handle; column 5, row 4 a point inside a patch.
    assert_eq!(m.anchor_of(3 * 10 + 3), Some(33));
    assert_eq!(m.anchor_of(3 * 10 + 4), Some(33));
    assert_eq!(m.anchor_of(3 * 10 + 5), Some(36));
    assert_eq!(m.anchor_of(4 * 10 + 3), Some(33));
    assert_eq!(m.anchor_of(4 * 10 + 4), None);
}

#[trial]
fn a_split_tool_inserts_the_lines_it_names_at_the_click() {
    for (split, size) in [(WarpSplit::Horizontal, (1, 2)), (WarpSplit::Vertical, (2, 1)), (WarpSplit::Cross, (2, 2))] {
        let mut h = tooled(WarpMesh::identity(RECT, 1, 1), split, true);
        let at = h.at_fraction(0.25, 0.4);
        h.click(at, Modifiers::NONE);
        assert_eq!(h.count(Seen::Set), 1, "{split:?}: one Set, no drag");
        let Some(WarpEdit::Split { split: edit, column: 0, row: 0, u, v, .. }) = h.edit.get() else { panic!("{:?}", h.edit.get()) };
        assert_eq!(edit, split);
        assert!((u - 0.25).abs() < 0.01 && (v - 0.4).abs() < 0.01, "({u}, {v})");
        assert_eq!(h.size(), size, "{split:?}");
        // The new row and column of anchors lie on the click's lines.
        if split != WarpSplit::Vertical {
            assert!((h.mesh().point(0, 3).y - (RECT.y0 + 0.4 * RECT.height())).abs() < 0.5);
        }
        if split != WarpSplit::Horizontal {
            assert!((h.mesh().point(3, 0).x - (RECT.x0 + 0.25 * RECT.width())).abs() < 0.5);
        }
    }

    // Alt does not change the tool's lines.
    let mut h = tooled(WarpMesh::identity(RECT, 1, 1), WarpSplit::Horizontal, false);
    let at = h.at_fraction(0.25, 0.4);
    h.click(at, Modifiers::ALT);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split: WarpSplit::Horizontal, .. })), "{:?}", h.edit.get());

    // Off the surface nothing is inserted, and a press on an anchor does not drag it.
    let mut h = tooled(WarpMesh::identity(RECT, 1, 1), WarpSplit::Cross, false);
    let off = h.screen(Point::new(RECT.x1 + 60.0, RECT.y1 + 40.0));
    h.click(off, Modifiers::NONE);
    let corner = h.screen(Point::new(RECT.x0, RECT.y0));
    h.click(corner, Modifiers::NONE);
    assert!(h.seen.borrow().is_empty(), "{:?}", h.seen.borrow());
}

#[trial]
fn a_split_on_an_existing_line_inserts_nothing_and_a_cross_inserts_its_other_line() {
    let mut h = tooled(WarpMesh::identity(RECT, 2, 1), WarpSplit::Vertical, true);
    // The middle line of a 2 x 1 mesh is at half the width.
    let on = h.at_fraction(0.5, 0.4);
    h.click(on, Modifiers::NONE);
    assert!(h.seen.borrow().is_empty(), "{:?}", h.seen.borrow());

    let mut h = tooled(WarpMesh::identity(RECT, 2, 1), WarpSplit::Cross, true);
    let on = h.at_fraction(0.5, 0.4);
    h.click(on, Modifiers::NONE);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split: WarpSplit::Horizontal, .. })), "{:?}", h.edit.get());
    assert_eq!(h.size(), (2, 2));
}

#[trial]
fn alt_or_ctrl_with_no_tool_is_a_quick_split_chosen_by_the_pointer() {
    // In the open a cross.
    let mut h = harness(WarpMesh::identity(RECT, 2, 2));
    let open = h.at_fraction(0.25, 0.25);
    h.click(open, Modifiers::ALT);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split: WarpSplit::Cross, .. })), "{:?}", h.edit.get());
    assert_eq!(h.size(), (3, 3));

    // Close to an existing line of a direction, the perpendicular line.
    for (modifiers, u, v, want) in [(Modifiers::COMMAND, 0.51, 0.25, WarpSplit::Horizontal), (Modifiers::ALT, 0.25, 0.49, WarpSplit::Vertical)] {
        let mut h = harness(WarpMesh::identity(RECT, 2, 2));
        let at = h.at_fraction(u, v);
        h.click(at, modifiers);
        assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split, .. }) if split == want), "{:?}", h.edit.get());
    }

    // Off the mesh, Alt or Cmd splits at the nearest point of the mesh.
    for modifiers in [Modifiers::ALT, Modifiers::COMMAND] {
        let mut h = harness(WarpMesh::identity(RECT, 2, 2));
        let above = h.screen(Point::new(RECT.x0 + 0.25 * RECT.width(), RECT.y0 - 30.0));
        h.click(above, modifiers);
        assert!(matches!(h.edit.get(), Some(WarpEdit::Split { .. })), "{modifiers:?}: {:?}", h.edit.get());
        assert_eq!(h.size().0, 3, "{modifiers:?}: the line is cut at the nearest point's column");
    }

    // A one-patch mesh's thirds lines count as existing lines.
    let mut h = harness(WarpMesh::identity(RECT, 1, 1));
    let at = h.at_fraction(1.0 / 3.0 + 0.01, 0.5);
    h.click(at, Modifiers::ALT);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split: WarpSplit::Horizontal, .. })), "{:?}", h.edit.get());

    // Without the key the press pulls the surface as ever.
    let mut h = harness(WarpMesh::identity(RECT, 1, 1));
    let from = h.at_fraction(0.5, 0.5);
    h.drag(from, from + shift(10.0, 0.0));
    assert!(matches!(h.edit.get(), Some(WarpEdit::Surface { .. })), "{:?}", h.edit.get());
}

#[trial]
fn enter_splits_the_focused_anchors_patch_in_the_middle() {
    let mut h = tooled(WarpMesh::identity(RECT, 2, 1), WarpSplit::Cross, false);
    h.focus_grid();
    h.fixture.key(Key::Enter, Modifiers::NONE);
    let Some(WarpEdit::Split { split: WarpSplit::Cross, column: 0, row: 0, u, v, .. }) = h.edit.get() else { panic!("{:?}", h.edit.get()) };
    assert_eq!((u, v), (0.5, 0.5));
    assert_eq!(h.size(), (3, 2));

    // The last anchor of a row splits the last patch.
    let mut h = tooled(WarpMesh::identity(RECT, 2, 1), WarpSplit::Vertical, false);
    h.focus_grid();
    h.fixture.key(Key::End, Modifiers::NONE);
    h.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split: WarpSplit::Vertical, column: 1, row: 0, .. })), "{:?}", h.edit.get());

    // With no tool set Enter does nothing.
    let mut h = harness(WarpMesh::identity(RECT, 1, 1));
    h.focus_grid();
    h.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(h.seen.borrow().is_empty());
}

#[trial]
fn the_first_split_of_a_custom_one_patch_mesh_keeps_its_thirds() {
    // A cross at the middle of the default grid: 4 x 4 patches with knots 0, 1/3, 1/2, 2/3, 1.
    let mut h = tooled(WarpMesh::identity(RECT, 1, 1), WarpSplit::Cross, false);
    let at = h.at_fraction(0.5, 0.5);
    h.click(at, Modifiers::NONE);
    assert!(matches!(h.edit.get(), Some(WarpEdit::Split { thirds: true, .. })), "{:?}", h.edit.get());
    assert_eq!(h.size(), (4, 4));
    assert_knots(&knots(&h.mesh()), &[0.0, 1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0]);

    // A release inside the first third cuts that third.
    let mut h = tooled(WarpMesh::identity(RECT, 1, 1), WarpSplit::Vertical, false);
    let at = h.at_fraction(0.25, 0.5);
    h.click(at, Modifiers::NONE);
    assert_eq!(h.size(), (4, 3), "the horizontal thirds stay too");
    assert_knots(&knots(&h.mesh()), &[0.0, 0.25, 1.0 / 3.0, 2.0 / 3.0, 1.0]);

    // On a third's line nothing is cut.
    let mut h = tooled(WarpMesh::identity(RECT, 1, 1), WarpSplit::Vertical, false);
    let at = h.at_fraction(1.0 / 3.0, 0.5);
    h.click(at, Modifiers::NONE);
    assert!(h.seen.borrow().is_empty());
}

#[trial]
fn a_quick_split_near_an_uneven_line_cuts_across_it() {
    // The mesh's knots are 0, 0.1, 0.2, 1 across. With the source rectangle 400 wide the reach is
    // 0.02 of the mesh. At 0.224 the pointer is clear of the line at 0.2, and at 0.21 it is beside it.
    let cases = [(0.224, WarpSplit::Cross, (4, 2)), (0.21, WarpSplit::Horizontal, (3, 2))];
    for (s, want, size) in cases {
        let start = WarpMesh::identity_with_knots(RECT, vec![0.0, 0.1, 0.2, 1.0], vec![0.0, 1.0]).unwrap();
        let mut h = harness(start);
        h.set(|h| h.bounds.set(DocRect::new(RECT.x0, RECT.y0, RECT.x0 + 400.0, RECT.y1)));
        let at = h.at_fraction(s, 0.5);
        h.click(at, Modifiers::ALT);
        assert!(matches!(h.edit.get(), Some(WarpEdit::Split { split, .. }) if split == want), "{s}: {:?}", h.edit.get());
        assert_eq!(h.size(), size, "{s}");
    }
}

fn warped(mesh: &mut WarpMesh) {
    for (k, p) in mesh.points.iter_mut().enumerate() {
        *p += doc_shift((k % 5) as f64 * 3.0, (k % 7) as f64 * 2.0);
    }
}

/// The x of the anchors along the top row of the mesh, as fractions of `RECT`'s width.
fn knots(mesh: &WarpMesh) -> Vec<f64> {
    (0..=mesh.columns()).map(|k| (mesh.point(3 * k, 0).x - RECT.x0) / RECT.width()).collect()
}

fn assert_knots(got: &[f64], want: &[f64]) {
    assert!(got.len() == want.len() && got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-3), "{got:?} vs {want:?}");
}

fn uneven() -> WarpMesh {
    WarpMesh::identity_with_knots(RECT, vec![0.0, 0.1, 0.2, 1.0], vec![0.0, 0.7, 1.0]).unwrap()
}

/// The sample parameters the surface-keeping tests compare at.
const SAMPLES: [(f64, f64); 7] = [(0.05, 0.2), (0.15, 0.8), (0.4, 0.75), (0.9, 0.95), (0.5, 0.9), (0.1, 0.1), (0.7, 0.3)];

#[trial]
fn committing_the_thirds_keeps_the_surface_and_only_a_one_patch_mesh_has_them() {
    let mut mesh = WarpMesh::identity(RECT, 1, 1);
    warped(&mut mesh);
    let cut = mesh.commit_thirds();
    assert_eq!((cut.columns(), cut.rows()), (3, 3));
    for (s, t) in SAMPLES {
        assert!(mesh.eval(s, t).distance(cut.eval(s, t)) < 1e-9, "({s}, {t})");
    }
    let two = WarpMesh::identity(RECT, 2, 1);
    assert_eq!(two.commit_thirds(), two);
}

#[trial]
fn params_and_locate_follow_the_knots_and_a_mesh_hands_over_with_its_knots() {
    let mesh = uneven();
    let (s, t) = mesh.params(2, 1, 0.5, 0.5);
    assert!((s - 0.6).abs() < 1e-12 && (t - 0.85).abs() < 1e-12);
    let (column, row, u, v) = mesh.locate(0.6, 0.85);
    assert_eq!((column, row), (2, 1));
    assert!((u - 0.5).abs() < 1e-12 && (v - 0.5).abs() < 1e-12);
    // A knot belongs to the patch before it, as PhotoCraft's eval has it.
    assert_eq!(mesh.locate(0.1, 0.0).0, 0);

    let again = WarpMesh::new(mesh.us.clone(), mesh.vs.clone(), mesh.points.clone());
    assert_eq!(again, Some(mesh));
    assert!(WarpMesh::new(vec![0.0, 0.5], vec![0.0, 1.0], uneven().points).is_none());
    assert!(WarpMesh::identity_with_knots(RECT, vec![0.0, 0.5, 0.5, 1.0], vec![0.0, 1.0]).is_none());
}

#[trial]
fn an_uneven_split_keeps_the_surface_where_it_was() {
    let mut mesh = uneven();
    warped(&mut mesh);
    let cut = mesh.split(WarpSplit::Cross, 2, 1, 0.3, 0.6);
    for (s, t) in [(0.05, 0.2), (0.15, 0.8), (0.4, 0.75), (0.9, 0.95), (0.5, 0.9)] {
        assert!(mesh.eval(s, t).distance(cut.eval(s, t)) < 1e-9, "({s}, {t})");
    }
}

fn entries() -> Vec<WarpStyleEntry> {
    vec![WarpStyleEntry::new("none", "None"), WarpStyleEntry::new("arc", "Arc").preset(true), WarpStyleEntry::new("custom", "Custom").custom(true)]
}

/// The group on style `style`, with the edits of every pass collected.
struct Fields {
    fixture: Fixture,
    edits: Rc<RefCell<Vec<WarpFieldsEdit>>>,
}

fn fields(style: &'static str, on_type: bool) -> Fields {
    let edits = Rc::new(RefCell::new(Vec::new()));
    let shown = Rc::clone(&edits);
    let mut fixture = Fixture::new(move |ui| {
        let styles = entries();
        let mut values = WarpFieldValues::new(style);
        values.bend = 50.0;
        let mut group = WarpFields::new(&styles, values);
        if on_type {
            group = group.on_type();
        }
        shown.borrow_mut().extend(group.show(ui).edits);
    });
    // The options bar is one row of controls that fits a window, not a panel.
    fixture.in_room(Room::Window);
    fixture.settle();
    Fields { fixture, edits }
}

impl Fields {
    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("no node named {label}"));
        self.fixture.click(&target);
    }
}

#[trial]
fn a_type_layer_greys_split_and_grid_and_hides_custom() {
    let mut f = fields("arc", true);
    f.click("Split Warp Crosswise");
    assert!(f.edits.borrow().is_empty(), "Split is disabled on a type layer");
    f.click("Warp");
    f.fixture.settle();
    assert!(f.fixture.by_label("Arc").is_some());
    assert!(f.fixture.by_label("Custom").is_none(), "no Custom over a type layer");

    let mut f = fields("arc", false);
    f.click("Split Warp Crosswise");
    assert_eq!(*f.edits.borrow(), vec![WarpFieldsEdit::Split(Some(WarpSplit::Cross))]);
    f.click("Warp");
    f.fixture.settle();
    assert!(f.fixture.by_label("Custom").is_some());
}
