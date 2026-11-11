//! The curve editor: pointer and key editing of the points, the fields, the pencil, the corners and the AccessKit
//! nodes, driven through the editor as a host would show it.

use craft_behavior::EditEvent;
use craft_imaging::ui::curve::{Curve, CurveChannel, CurveEditor};
use craft_imaging::ui::{ChannelColor, HistogramData};
use craft_theme::{Contrast, Mode};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::display::Text;
use craft_ui::testing::access::name;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, style, trial};
use egui::{Key, Modifiers, Pos2, Vec2, pos2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How the rig shows the editor.
#[derive(Clone, Copy, Default)]
struct Config {
    pencil: bool,
    corners: bool,
    percent: bool,
    between: bool,
    histogram: bool,
    max_points: Option<usize>,
}

struct Rig {
    fixture: Fixture,
    curve: Rc<RefCell<Curve>>,
    selected: Rc<Cell<Option<usize>>>,
    /// The active channel.
    channel: Rc<Cell<usize>>,
    /// The kind of each edit event, in order.
    kinds: Rc<RefCell<Vec<&'static str>>>,
    origin: Pos2,
    scale: Vec2,
}

fn kind(event: &EditEvent<Curve>) -> &'static str {
    match event {
        EditEvent::Begin { .. } => "begin",
        EditEvent::Update { .. } => "update",
        EditEvent::Commit { .. } => "commit",
        EditEvent::Cancel { .. } => "cancel",
        EditEvent::Set { .. } => "set",
        _ => "other",
    }
}

fn rig_with(curve: Curve, config: Config) -> Rig {
    let shared = Rc::new(RefCell::new(curve.clone()));
    let selected = Rc::new(Cell::new(None));
    let kinds = Rc::new(RefCell::new(Vec::new()));
    let channel = Rc::new(Cell::new(0usize));
    let (c, s, k, ch) = (shared.clone(), selected.clone(), kinds.clone(), channel.clone());
    let channels = [CurveChannel::new("RGB", ChannelColor::Neutral), CurveChannel::new("Red", ChannelColor::Red)];
    let bins = vec![10u32; 256];
    let mut fixture = Fixture::new(move |ui| {
        let mut value = c.borrow().clone();
        let histogram = HistogramData::tone(&bins, ChannelColor::Neutral);
        let mut editor = CurveEditor::new("Curve", Bind::bound(&mut value)).channels(&channels).channel(ch.get()).selection(s.get());
        if let Some(max) = config.max_points {
            editor = editor.max_points(max);
        }
        if config.pencil {
            editor = editor.pencil();
        }
        if config.corners {
            editor = editor.corners();
        }
        if config.percent {
            editor = editor.percent();
        }
        if config.histogram {
            editor = editor.histogram(&histogram);
        }
        if config.between {
            editor = editor.between(|ui, _| {
                Text::new("Between").show(ui);
            });
        }
        let out = editor.show(ui);
        for event in &out.edit.events {
            k.borrow_mut().push(kind(event));
        }
        s.set(out.selected);
        *c.borrow_mut() = value;
    });
    fixture.room(room());
    fixture.settle();
    let mut rig = Rig { fixture, curve: shared, selected, channel, kinds, origin: Pos2::ZERO, scale: Vec2::ZERO };
    // Pencil mode shows no points to calibrate against: the graph's own rect maps the levels.
    if config.pencil {
        rig.calibrate_graph(LEVELS);
    } else {
        rig.calibrate(&curve);
    }
    rig
}

fn rig(points: &[[f32; 2]]) -> Rig {
    rig_with(Curve::smooth(points.to_vec()), Config::default())
}

impl Rig {
    /// Maps levels to screen from the first and last point's nodes.
    fn calibrate(&mut self, curve: &Curve) {
        let points = curve.points().map(<[_]>::to_vec).unwrap_or_default();
        let (Some(first), Some(last)) = (points.first(), points.last()) else { return };
        let n = points.len();
        let a = self.fixture.by_label(&format!("Point 1 of {n}")).expect("the first point").center();
        let b = self.fixture.by_label(&format!("Point {n} of {n}")).expect("the last point").center();
        self.scale = vec2((b.x - a.x) / (last[0] - first[0]), (b.y - a.y) / (last[1] - first[1]));
        self.origin = pos2(a.x - first[0] * self.scale.x, a.y - first[1] * self.scale.y);
    }

    /// Maps `0..=levels` over the graph node's rect: the editor and its graph share the label, and the graph is the
    /// smaller of the two.
    fn calibrate_graph(&mut self, levels: f32) {
        let graph = self
            .fixture
            .by_role(Role::GROUP)
            .into_iter()
            .filter(|group| name(self.fixture.tree(), group.node) == Some("Curve"))
            .min_by(|a, b| (a.rect.width() * a.rect.height()).total_cmp(&(b.rect.width() * b.rect.height())))
            .expect("the graph");
        self.scale = vec2(graph.rect.width() / levels, -graph.rect.height() / levels);
        self.origin = graph.rect.left_bottom();
    }

    fn at(&self, q: [f32; 2]) -> Pos2 {
        pos2(self.origin.x + q[0] * self.scale.x, self.origin.y + q[1] * self.scale.y)
    }

    fn points(&self) -> Vec<[f32; 2]> {
        self.curve.borrow().points().map(<[_]>::to_vec).unwrap_or_default()
    }

    fn kinds(&self) -> Vec<&'static str> {
        self.kinds.borrow().clone()
    }

    fn press_at(&mut self, q: [f32; 2]) {
        let pos = self.at(q);
        self.fixture.frame(vec![moved(pos)]);
        self.fixture.frame(vec![press(pos)]);
    }

    fn move_to(&mut self, q: [f32; 2]) {
        self.fixture.frame(vec![moved(self.at(q))]);
    }

    fn release_at(&mut self, q: [f32; 2]) {
        self.fixture.frame(vec![release(self.at(q))]);
        self.fixture.idle(1);
    }

    fn click(&mut self, q: [f32; 2]) {
        self.press_at(q);
        self.release_at(q);
    }

    fn focus(&mut self) {
        self.fixture.key(Key::Tab, Modifiers::NONE);
    }

    fn key(&mut self, key: Key) {
        self.fixture.key(key, Modifiers::NONE);
        self.fixture.idle(1);
    }
}

/// The room is this many plot widths wide and tall: the editor at its greatest size and the rows under it.
const WIDE_PLOTS: u8 = 5;
const TALL_PLOTS: u8 = 6;

fn room() -> Vec2 {
    let unit = style::tokens(Mode::Light, Contrast::Standard).size.plot.width;
    Vec2::new(unit * f32::from(WIDE_PLOTS), unit * f32::from(TALL_PLOTS))
}
/// A point in whole levels.
fn level(x: u16, y: u16) -> [f32; 2] {
    [f32::from(x), f32::from(y)]
}

/// The editor's levels.
const LEVELS: f32 = 255.0;
const IDENTITY: [[f32; 2]; 2] = [[0.0, 0.0], [255.0, 255.0]];
const THREE: [[f32; 2]; 3] = [[0.0, 0.0], [100.0, 100.0], [255.0, 255.0]];

/// A press on the empty plot adds a point there, and the drag that follows is one gesture.
#[trial]
fn a_press_on_the_empty_graph_adds_a_point_and_its_drag_is_one_gesture() {
    let mut rig = rig(&IDENTITY);
    rig.press_at([128.0, 128.0]);
    rig.move_to([128.0, 180.0]);
    rig.move_to([130.0, 200.0]);
    rig.release_at([130.0, 200.0]);
    assert_eq!(rig.points(), vec![[0.0, 0.0], [130.0, 200.0], [255.0, 255.0]]);
    let kinds = rig.kinds();
    assert_eq!(kinds.iter().filter(|k| **k == "begin").count(), 1, "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"commit"), "{kinds:?}");
    assert_eq!(rig.selected.get(), Some(1));
}

/// A press on a point selects it and, without moving, edits nothing.
#[trial]
fn a_press_on_a_point_without_moving_selects_it_and_changes_nothing() {
    let mut rig = rig(&THREE);
    rig.click([100.0, 100.0]);
    assert_eq!(rig.selected.get(), Some(1));
    assert_eq!(rig.points(), THREE.to_vec());
    assert!(!rig.kinds().contains(&"update"), "{:?}", rig.kinds());
}

/// An interior point dragged far off the plot goes, and comes back on re-entry within the same drag.
#[trial]
fn a_point_dragged_off_the_graph_goes_and_comes_back_within_one_gesture() {
    let mut rig = rig(&THREE);
    rig.press_at([100.0, 100.0]);
    rig.move_to([100.0, 130.0]);
    let off = Pos2::new(rig.at([100.0, 100.0]).x, rig.origin.y + 600.0);
    rig.fixture.frame(vec![moved(off)]);
    assert_eq!(rig.points().len(), 2, "removed while off the graph");
    rig.move_to([140.0, 140.0]);
    rig.release_at([140.0, 140.0]);
    assert_eq!(rig.points(), vec![[0.0, 0.0], [140.0, 140.0], [255.0, 255.0]]);
    assert_eq!(rig.kinds().iter().filter(|k| **k == "begin").count(), 1);
}

/// Insert and Cmd+Enter add a point halfway to the next and select it.
#[trial]
fn insert_and_cmd_enter_add_a_point_halfway_to_the_next_and_select_it() {
    let mut rig = rig(&IDENTITY);
    rig.focus();
    rig.key(Key::Plus);
    rig.key(Key::Insert);
    assert_eq!(rig.points().len(), 3);
    assert_eq!(rig.selected.get(), Some(1));
    rig.fixture.key(Key::Enter, Modifiers::COMMAND);
    rig.fixture.idle(1);
    assert_eq!(rig.points().len(), 4, "from the new point to the next");
    assert_eq!(rig.selected.get(), Some(2));
}

/// A channel change clears the selection.
#[trial]
fn the_selection_clears_on_a_channel_change() {
    let mut rig = rig(&THREE);
    rig.click([100.0, 100.0]);
    assert_eq!(rig.selected.get(), Some(1));
    rig.channel.set(1);
    rig.fixture.idle(2);
    assert_eq!(rig.selected.get(), None);
}

/// Typing an Input value moves the selected point, and a value outside its neighbours is held to them.
#[trial]
fn the_input_field_moves_the_selected_point_between_its_neighbours() {
    let mut rig = rig(&THREE);
    rig.click([100.0, 100.0]);
    let field = rig.fixture.by_label("Input").expect("the Input field");
    rig.fixture.click(&field);
    rig.fixture.key(Key::A, Modifiers::COMMAND);
    rig.fixture.type_text("300");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    rig.fixture.idle(2);
    assert_eq!(rig.points()[1], level(254, 100), "strictly before the next input");
}

/// A pencil stroke is one gesture that draws a map, hides the points and reads as drawn.
#[trial]
fn a_pencil_stroke_is_one_gesture_and_draws_a_map_without_points() {
    let curve = Curve::smooth(IDENTITY.to_vec());
    let mut rig = rig_with(curve.clone(), Config { pencil: true, ..Config::default() });
    assert!(rig.fixture.by_role(Role::GRAPHICS_OBJECT).is_empty(), "no points in pencil mode");
    let (from, to) = ([LEVELS / 4.0; 2], [LEVELS / 2.0; 2]);
    rig.press_at(from);
    rig.move_to(to);
    rig.release_at(to);
    assert!(matches!(*rig.curve.borrow(), Curve::Map(_)), "a stroke draws a map");
    let kinds = rig.kinds();
    assert_eq!(kinds.iter().filter(|k| **k == "begin").count(), 1, "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"commit"));
}

/// A corner point dragged off the plot and back keeps its flag; a new point is smooth.
#[trial]
fn a_corner_dragged_off_the_plot_and_back_keeps_its_flag() {
    let curve = Curve::from_parts(THREE.to_vec(), vec![false, true, false]);
    let mut rig = rig_with(curve, Config { corners: true, ..Config::default() });
    rig.press_at([100.0, 100.0]);
    rig.move_to([100.0, 130.0]);
    let off = Pos2::new(rig.at([100.0, 100.0]).x, rig.origin.y + 600.0);
    rig.fixture.frame(vec![moved(off)]);
    rig.move_to([150.0, 150.0]);
    rig.release_at([150.0, 150.0]);
    assert!(rig.curve.borrow().is_corner(1), "{:?}", rig.curve.borrow());
    rig.click([200.0, 190.0]);
    assert!(!rig.curve.borrow().is_corner(2), "a new point is smooth");
}
