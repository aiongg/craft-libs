//! `LayersPanel`: the order of the rows, what a drop means and what the footer takes and reports.

use craft_behavior::EditEvent;
use craft_keymap::Mods;
use craft_layers::ui::filter::{FilterKind, LayerFilter};
use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::panel::{LayerDrop, LayerPosition, LayersEvent, LayersFooterAction, LayersFooterButton, LayersPanel};
use craft_layers::ui::tree::{
    LayerInfo, LayerKind, LayerPart, LayerProperties, LayerRowKey, LayerSelection, LayerTree, LayersFooterState, SubRowGroup, SubRows,
};
use craft_ui::contract::{Bind, Entry, Props as _, ReorderMove, ShowContainer as _};
use craft_ui::layout_components::Stack;
use craft_ui::testing::{Fixture, Target, layout, pointer, trial};
use egui::{Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// A size of `width` by `height` points.
const fn by(width: f32, height: f32) -> Vec2 {
    Vec2::new(width, height)
}

/// One panel's room: tall enough for the sample tree and the footer, and inside the 800 by 600 screen of a fixture.
const ONE_PANEL: Vec2 = by(400.0, 560.0);

/// Two panels side by side, each in a room of `ONE_PANEL`.
const TWO_PANELS: Vec2 = by(800.0, 560.0);

type Seen = Rc<RefCell<Vec<LayersEvent<u32>>>>;
type Shared<T> = Rc<RefCell<T>>;

/// What a rig holds: the fixture, the events its panel reported, the tree and the filter it shows.
struct Rig<T> {
    fixture: Fixture,
    seen: Seen,
    tree: Shared<T>,
    filter: Shared<LayerFilter>,
}

fn rig_of<T: LayerTree<Id = u32> + 'static>(tree: T) -> Rig<T> {
    let seen: Seen = Rc::default();
    let tree = Rc::new(RefCell::new(tree));
    let filter: Shared<LayerFilter> = Rc::default();
    let (sink, shown, by) = (Rc::clone(&seen), Rc::clone(&tree), Rc::clone(&filter));
    let mut fixture = Fixture::new(move |ui| {
        let tree = shown.borrow();
        let mut filter = by.borrow_mut();
        let out = LayersPanel::new(&*tree).filter(Bind::bound(&mut *filter)).show(ui);
        sink.borrow_mut().extend(out.events);
    });
    fixture.room(ONE_PANEL);
    fixture.settle();
    Rig { fixture, seen, tree, filter }
}

fn rig() -> Rig<FakeLayers> {
    rig_of(FakeLayers::sample())
}

impl<T> Rig<T> {
    fn row(&self, label: &str) -> Target {
        self.fixture.by_label(label).unwrap_or_else(|| panic!("no row named {label}"))
    }

    /// The labels of `labels` that are on screen, top to bottom.
    fn order(&self, labels: &[&'static str]) -> Vec<&'static str> {
        let mut shown: Vec<(f32, &str)> = labels.iter().filter_map(|label| self.fixture.by_label(label).map(|t| (t.rect.top(), *label))).collect();
        shown.sort_by(|a, b| a.0.total_cmp(&b.0));
        shown.into_iter().map(|(_, label)| label).collect()
    }

    /// Drags from `from` to `to` in small steps.
    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.seen.borrow_mut().clear();
        let f = &mut self.fixture;
        f.frame(vec![pointer::moved(from)]);
        f.frame(vec![pointer::press(from)]);
        for step in 1..=6 {
            f.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 6.0))]);
        }
        f.frame(vec![pointer::moved(to)]);
        f.frame(vec![pointer::release(to)]);
        f.idle(2);
    }

    /// Drags the row `from` onto the upper quarter, the middle or the lower quarter of the row `onto`.
    fn drop_on(&mut self, from: &str, onto: &str, at: Place) {
        let (source, target) = (self.row(from).center(), self.row(onto).rect);
        let y = match at {
            Place::Above => target.top() + target.height() * 0.2,
            Place::Middle => target.center().y,
            Place::Below => target.top() + target.height() * 0.8,
        };
        self.drag(source, Pos2::new(target.center().x, y));
    }

    fn drops(&self) -> Vec<Landing> {
        self.seen
            .borrow()
            .iter()
            .filter_map(|event| match event {
                LayersEvent::Drop(EditEvent::Set { after, .. }) => Some(match after {
                    LayerDrop::Layers(m) => Landing::Layers(m.layers.clone(), m.target, m.position, m.copy),
                    LayerDrop::Transfer(t) => Landing::Transfer(t.rows.clone(), t.to, t.copy),
                    LayerDrop::SubRows(m) => Landing::SubRows(m.layer, m.group, m.from, m.to, m.copy),
                    _ => Landing::Other,
                }),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
enum Place {
    Above,
    Middle,
    Below,
}

/// What a drop event says, in plain values.
#[derive(Debug, PartialEq)]
enum Landing {
    Layers(Vec<u32>, u32, LayerPosition, bool),
    Transfer(Vec<LayerRowKey<u32>>, u32, bool),
    SubRows(u32, SubRowGroup, usize, usize, bool),
    Other,
}

fn layer_move(layers: &[u32], target: u32, position: LayerPosition, copy: bool) -> Landing {
    Landing::Layers(layers.to_vec(), target, position, copy)
}

const SAMPLE_ORDER: [&str; 9] = ["Group 1", "Clipped", "Masked", "Title", "Levels 1", "Smart Object", "Styled", "Hidden", "Background"];

/// The rows run top first, a closed group hides its layers, and open effects and Smart Filters list under their layer.
#[trial]
fn rows_are_listed_top_first_and_closed_groups_hide_their_layers() {
    let mut rig = rig();
    assert_eq!(rig.order(&SAMPLE_ORDER), SAMPLE_ORDER);

    rig.tree.borrow_mut().layers[0].open = false;
    rig.fixture.idle(3);
    assert_eq!(rig.order(&SAMPLE_ORDER), ["Group 1", "Title", "Levels 1", "Smart Object", "Styled", "Hidden", "Background"]);

    {
        let mut tree = rig.tree.borrow_mut();
        tree.layers[5].effects_open = true;
        tree.layers[6].effects_open = true;
    }
    rig.fixture.idle(3);
    let all = ["Smart Object", "Gaussian Blur", "Sharpen", "Styled", "Effects", "Drop Shadow", "Stroke", "Hidden"];
    assert_eq!(rig.order(&all), all);
}

/// A filter lists the matching layers flat, without their groups or the layers it leaves out, and a layer from a group still shows
/// as nested.
#[trial]
fn a_filter_lists_matches_without_their_groups() {
    let mut rig = rig();
    rig.tree.borrow_mut().layers[0].open = false;
    rig.filter.borrow_mut().kinds = vec![FilterKind::Pixel];
    rig.fixture.idle(3);
    assert_eq!(rig.order(&SAMPLE_ORDER), ["Clipped", "Masked", "Styled", "Hidden", "Background"]);
    for left_out in ["Group 1", "Title", "Levels 1", "Smart Object"] {
        assert!(rig.fixture.by_label(left_out).is_none(), "{left_out} is filtered out");
    }
}

/// A filter reads fields the tree revision does not cover, so it applies even while the revision stands.
#[trial]
fn a_filter_applies_even_while_the_tree_revision_stands() {
    let mut rig = rig();
    rig.tree.borrow_mut().revision = Some(1);
    rig.filter.borrow_mut().kinds = vec![FilterKind::Pixel];
    rig.fixture.idle(3);
    assert!(rig.fixture.by_label("Clipped").is_some());
    rig.tree.borrow_mut().layers[1].info = LayerInfo::new("Clipped", LayerKind::Text);
    rig.fixture.idle(3);
    assert!(rig.fixture.by_label("Clipped").is_none(), "a kind change shows at the same revision");
}

/// Making a layer inside closed groups active opens its ancestors, outermost first, once; the first pass reports nothing.
#[trial]
fn a_new_active_layer_opens_its_closed_ancestors_once() {
    let deep = |active: u32| {
        let mut tree = FakeLayers::sample();
        tree.layers[0].open = false;
        tree.layers[1].open = false;
        tree.add(Some(2), LayerInfo::new("Deep", LayerKind::Pixel), false);
        tree.selection = LayerSelection::new(vec![active], Some(active), LayerPart::Pixels);
        tree
    };
    let opens = |rig: &Rig<FakeLayers>| -> Vec<(u32, bool)> {
        rig.seen
            .borrow()
            .iter()
            .filter_map(|event| if let LayersEvent::Open(change) = event { Some((change.key, change.open)) } else { None })
            .collect()
    };

    let first = rig_of(deep(10));
    assert_eq!(opens(&first), []);

    let mut rig = rig_of(deep(4));
    assert_eq!(opens(&rig), []);
    rig.tree.borrow_mut().selection = LayerSelection::new(vec![10], Some(10), LayerPart::Pixels);
    rig.fixture.idle(4);
    assert_eq!(opens(&rig), [(1, true), (2, true)]);
}

/// An Alt gesture that starts on an eye and sweeps over more is one Show Only, and reports no toggle.
#[trial]
fn an_alt_sweep_over_the_eyes_reports_show_only_once() {
    let mut rig = rig();
    let (from, to) = (rig.row("Visibility Title").center(), rig.row("Visibility Levels 1").center());
    rig.fixture.hold(Modifiers::ALT);
    rig.drag(from, to);
    let events = rig.seen.borrow();
    let shown_only: Vec<_> = events.iter().filter(|event| matches!(event, LayersEvent::ShowOnly(_))).collect();
    assert_eq!(shown_only, [&LayersEvent::ShowOnly(LayerRowKey::Layer(4))], "events: {events:?}");
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::Toggle(_))), "events: {events:?}");
}

/// Cmd on a thumbnail loads it as a selection, Shift or Alt on a mask does too, and Cmd on the Background's thumbnail is a plain
/// click.
#[trial]
fn modified_thumbnail_clicks_are_not_selections() {
    let mut rig = rig();
    let row = rig.row("Levels 1").rect;
    let link = rig.row("Unlink layer mask of Levels 1").rect;
    let pixels_x = link.left() - row.height() * 0.2;
    let mask_x = link.right() + row.height() * 0.2;
    let click = |rig: &mut Rig<FakeLayers>, x: f32, y: f32, modifiers: Modifiers| -> Vec<LayersEvent<u32>> {
        rig.seen.borrow_mut().clear();
        rig.fixture.hold(modifiers);
        rig.fixture.click_at(Pos2::new(x, y));
        rig.fixture.hold(Modifiers::NONE);
        rig.fixture.idle(2);
        rig.seen.borrow().clone()
    };
    let thumb_click = |events: &[LayersEvent<u32>], layer: u32, part: LayerPart| {
        events.iter().any(|event| matches!(event, LayersEvent::ThumbClick { layer: l, part: p, .. } if *l == layer && *p == part))
    };
    let selects = |events: &[LayersEvent<u32>]| events.iter().any(|event| matches!(event, LayersEvent::Select(_)));

    let events = click(&mut rig, pixels_x, row.center().y, Modifiers::COMMAND);
    assert!(thumb_click(&events, 5, LayerPart::Pixels), "events: {events:?}");
    assert!(!selects(&events), "events: {events:?}");
    let events = click(&mut rig, mask_x, row.center().y, Modifiers::SHIFT);
    assert!(thumb_click(&events, 5, LayerPart::LayerMask), "events: {events:?}");
    let events = click(&mut rig, pixels_x, row.center().y, Modifiers::NONE);
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::ThumbClick { .. })), "events: {events:?}");

    let background = rig.row("Background").rect;
    let events = click(&mut rig, pixels_x, background.center().y, Modifiers::COMMAND);
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::ThumbClick { .. })), "events: {events:?}");
    assert!(selects(&events), "events: {events:?}");
}

/// The sample tree, refusing every drop below its Background.
struct NothingBelowBackground(FakeLayers);

impl LayerTree for NothingBelowBackground {
    type Id = u32;

    fn children(&self, parent: Option<&u32>) -> Vec<u32> {
        self.0.children(parent)
    }
    fn parent(&self, id: &u32) -> Option<u32> {
        self.0.parent(id)
    }
    fn is_open(&self, id: &u32) -> bool {
        self.0.is_open(id)
    }
    fn effects_open(&self, id: &u32) -> bool {
        self.0.effects_open(id)
    }
    fn info(&self, id: &u32) -> LayerInfo {
        self.0.info(id)
    }
    fn selection(&self) -> LayerSelection<u32> {
        self.0.selection()
    }
    fn sub_rows(&self, id: &u32, group: SubRowGroup) -> Option<SubRows> {
        self.0.sub_rows(id, group)
    }
    fn properties(&self) -> Option<LayerProperties> {
        self.0.properties()
    }
    fn blend_modes(&self, group: bool, current: &str) -> Vec<Entry<String>> {
        self.0.blend_modes(group, current)
    }
    fn footer(&self) -> LayersFooterState {
        self.0.footer()
    }
    fn accepts_drop(&self, mv: &ReorderMove<LayerRowKey<u32>>, _mods: Mods) -> bool {
        // The Background is the last of eight other rows once the dragged one is out.
        mv.parent.is_some() || mv.to < 8
    }
}

/// A drop the app refuses takes nothing, while the drop just above it goes through.
#[trial]
fn a_drop_the_app_refuses_reports_nothing() {
    let mut rig = rig_of(NothingBelowBackground(FakeLayers::sample()));
    rig.drop_on("Title", "Background", Place::Below);
    assert_eq!(rig.drops(), []);
    rig.drop_on("Title", "Background", Place::Above);
    assert_eq!(rig.drops(), [layer_move(&[4], 9, LayerPosition::Before, false)]);
}

/// Effects dropped on another layer move to it (Alt copies); onto their own layer, between layers, or as Smart Filters onto a
/// layer that is not a smart object, they go nowhere.
#[trial]
fn effects_dropped_on_another_layer_transfer_to_it() {
    let mut rig = rig();
    {
        let mut tree = rig.tree.borrow_mut();
        tree.layers[5].effects_open = true;
        tree.layers[6].effects_open = true;
    }
    rig.fixture.idle(3);
    let effect = LayerRowKey::Effects(7, Some(0));
    rig.drop_on("Drop Shadow", "Title", Place::Middle);
    assert_eq!(rig.drops(), [Landing::Transfer(vec![effect.clone()], 4, false)]);

    rig.fixture.hold(Modifiers::ALT);
    rig.drop_on("Drop Shadow", "Title", Place::Middle);
    rig.fixture.hold(Modifiers::NONE);
    assert_eq!(rig.drops(), [Landing::Transfer(vec![effect], 4, true)]);

    rig.drop_on("Drop Shadow", "Styled", Place::Middle);
    assert_eq!(rig.drops(), [], "onto their own layer");
    rig.drop_on("Gaussian Blur", "Title", Place::Middle);
    assert_eq!(rig.drops(), [], "Smart Filters go onto smart objects only");
}

/// A Smart Filter moves within its own object, with the slot counted after taking it out; effects keep their order.
#[trial]
fn smart_filters_reorder_within_their_object_and_effects_keep_their_order() {
    let mut rig = rig();
    {
        let mut tree = rig.tree.borrow_mut();
        tree.layers[5].effects_open = true;
        tree.layers[6].effects_open = true;
    }
    rig.fixture.idle(3);
    let moved = |from, to| Landing::SubRows(6, SubRowGroup::SmartFilters, from, to, false);
    rig.drop_on("Gaussian Blur", "Sharpen", Place::Below);
    assert_eq!(rig.drops(), [moved(0, 1)]);
    rig.drop_on("Sharpen", "Gaussian Blur", Place::Above);
    assert_eq!(rig.drops(), [moved(1, 0)]);
    rig.drop_on("Gaussian Blur", "Gaussian Blur", Place::Above);
    assert_eq!(rig.drops(), [], "where it already is");
    rig.drop_on("Gaussian Blur", "Stroke", Place::Below);
    assert_eq!(rig.drops(), [], "another layer's filters");
    rig.drop_on("Drop Shadow", "Stroke", Place::Below);
    assert_eq!(rig.drops(), [], "effects keep their order");
}

/// Layers dropped above, on or below a layer's effects or Smart Filter rows land below that layer.
#[trial]
fn layers_dropped_beside_sub_rows_land_below_their_layer() {
    let mut rig = rig();
    {
        let mut tree = rig.tree.borrow_mut();
        tree.layers[5].effects_open = true;
        tree.layers[6].effects_open = true;
    }
    rig.fixture.idle(3);
    for (row, layer) in [("Effects", 7), ("Stroke", 7), ("Sharpen", 6)] {
        for place in [Place::Above, Place::Middle, Place::Below] {
            rig.drop_on("Title", row, place);
            assert_eq!(rig.drops(), [layer_move(&[4], layer, LayerPosition::After, false)], "{row}");
        }
    }
}

/// A footer press, in plain values.
#[derive(Debug, PartialEq)]
enum Pressed {
    NewLayer(bool, bool),
    NewGroup(bool, bool, bool),
    Delete(bool),
    AddMask(bool),
    Other,
}

fn footer_action(rig: &Rig<FakeLayers>) -> Option<Pressed> {
    rig.seen.borrow().iter().rev().find_map(|event| match event {
        LayersEvent::Footer(LayersFooterAction::NewLayer { ask, below, .. }) => Some(Pressed::NewLayer(*ask, *below)),
        LayersEvent::Footer(LayersFooterAction::NewGroup { from_selection, ask, below, .. }) => {
            Some(Pressed::NewGroup(*from_selection, *ask, *below))
        }
        LayersEvent::Footer(LayersFooterAction::Delete { confirm, .. }) => Some(Pressed::Delete(*confirm)),
        LayersEvent::Footer(LayersFooterAction::AddMask { invert, .. }) => Some(Pressed::AddMask(*invert)),
        LayersEvent::Footer(_) => Some(Pressed::Other),
        _ => None,
    })
}

fn press(rig: &mut Rig<FakeLayers>, label: &str, modifiers: Modifiers) -> Option<Pressed> {
    rig.seen.borrow_mut().clear();
    let button = rig.row(label);
    rig.fixture.hold(modifiers);
    rig.fixture.click(&button);
    rig.fixture.hold(Modifiers::NONE);
    rig.fixture.idle(2);
    footer_action(rig)
}

const NEW_LAYER: &str = "Create a new layer";
const NEW_GROUP: &str = "Create a new group";
const DELETE: &str = "Delete Layer";

/// New Layer: Alt asks, Cmd adds below. New Group: Shift or two selected layers group the selection, Alt asks, Cmd adds below.
/// Delete: Alt skips the confirmation. Mask: Alt inverts.
#[trial]
fn footer_modifiers_report_ask_below_and_from_selection() {
    let mut rig = rig();
    let layer = |ask, below| Some(Pressed::NewLayer(ask, below));
    assert_eq!(press(&mut rig, NEW_LAYER, Modifiers::NONE), layer(false, false));
    assert_eq!(press(&mut rig, NEW_LAYER, Modifiers::ALT), layer(true, false));
    assert_eq!(press(&mut rig, NEW_LAYER, Modifiers::COMMAND), layer(false, true));

    let group = |from_selection, ask, below| Some(Pressed::NewGroup(from_selection, ask, below));
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::NONE), group(false, false, false));
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::SHIFT), group(true, false, false));
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::ALT), group(false, true, false));
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::COMMAND), group(false, false, true));
    rig.tree.borrow_mut().selection = LayerSelection::new(vec![2, 4], Some(4), LayerPart::Pixels);
    rig.fixture.idle(2);
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::NONE), group(true, false, false), "two selected layers");

    let delete = |confirm| Some(Pressed::Delete(confirm));
    assert_eq!(press(&mut rig, DELETE, Modifiers::NONE), delete(true));
    assert_eq!(press(&mut rig, DELETE, Modifiers::ALT), delete(false));
    let mask = |invert| Some(Pressed::AddMask(invert));
    assert_eq!(press(&mut rig, "Add Layer Mask", Modifiers::NONE), mask(false));
    assert_eq!(press(&mut rig, "Add Layer Mask", Modifiers::ALT), mask(true));

    rig.tree.borrow_mut().selection = LayerSelection::new(Vec::new(), None, LayerPart::Pixels);
    rig.fixture.idle(2);
    assert_eq!(press(&mut rig, NEW_GROUP, Modifiers::SHIFT), group(false, false, false), "no active layer: an empty group");
}

fn footer_drops(rig: &Rig<FakeLayers>) -> Vec<(LayersFooterButton, Vec<LayerRowKey<u32>>)> {
    rig.seen
        .borrow()
        .iter()
        .filter_map(|event| if let LayersEvent::FooterDrop { button, rows, .. } = event { Some((*button, rows.clone())) } else { None })
        .collect()
}

/// A drag from another Layers panel (another document) is taken by none of this panel's footer buttons.
#[trial]
fn footer_buttons_refuse_a_drag_from_another_list() {
    let seen: [Seen; 2] = [Rc::default(), Rc::default()];
    let sinks = [Rc::clone(&seen[0]), Rc::clone(&seen[1])];
    let tree = FakeLayers::sample();
    let mut fixture = Fixture::new(move |ui| {
        for (at, sink) in sinks.iter().enumerate() {
            let left = Pos2::new(400.0 * at as f32, 0.0);
            layout::in_region(ui, Rect::from_min_size(left, ONE_PANEL), |ui| {
                Stack::new().id_salt(at).show(ui, |ui, _| {
                    let out = LayersPanel::new(&tree).show(ui);
                    sink.borrow_mut().extend(out.events);
                });
            });
        }
    });
    fixture.room(TWO_PANELS);
    fixture.settle();
    let all = |fixture: &Fixture, label: &str| -> Vec<Target> {
        let tree = fixture.tree();
        let mut found: Vec<Target> =
            tree.keys().into_iter().filter(|key| tree.name_of(*key) == Some(label)).filter_map(|key| fixture.target(key)).collect();
        found.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
        found
    };
    let (rows, new_layers) = (all(&fixture, "Title"), all(&fixture, NEW_LAYER));
    assert!(rows.len() >= 2 && new_layers.len() == 2, "two panels");
    let from = rows[0].center(); // the left panel's row
    let drops = |at: usize| seen[at].borrow().iter().filter(|event| matches!(event, LayersEvent::FooterDrop { .. })).count();

    for (button, count) in [(&new_layers[1], 0), (&new_layers[0], 1)] {
        seen[0].borrow_mut().clear();
        seen[1].borrow_mut().clear();
        let to = button.center();
        fixture.frame(vec![pointer::moved(from)]);
        fixture.frame(vec![pointer::press(from)]);
        for step in 1..=6 {
            fixture.frame(vec![pointer::moved(from + (to - from) * (step as f32 / 6.0))]);
        }
        fixture.frame(vec![pointer::moved(to)]);
        fixture.frame(vec![pointer::release(to)]);
        fixture.idle(2);
        assert_eq!((drops(0), drops(1)), (count, 0), "onto the footer of panel {count}");
    }
}
