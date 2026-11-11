//! Finding a component in the access tree: by name, role, ancestor and path, and its bounds in points.

use craft_behavior::{CanvasTransform, Sense};
use craft_ui::contract::{Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::runtime::egui::access::{NodeKey, Query};
use craft_ui::runtime::env::Scale;
use craft_ui::testing::{Fixture, trial};
use craft_ui_test_kit::contract::{Panel, Probe, Seen};
use egui::Vec2;
use std::cell::RefCell;
use std::rc::Rc;

/// What the story recorded: the "Opacity" probes of the "Layers" and the "Effects" panel, and the "Effects" panel.
struct Shown {
    layers_opacity: RefCell<Seen>,
    blend: RefCell<Seen>,
    effects_opacity: RefCell<Seen>,
    effects: RefCell<Seen>,
}

impl Shown {
    fn key(seen: &RefCell<Seen>) -> NodeKey {
        NodeKey::of(seen.borrow().id.expect("shown"))
    }
}

/// Two panels that each hold a button named "Opacity", the first also one named "Blend mode".
fn panels(scale: Scale) -> (Fixture, Rc<Shown>) {
    let shown = Rc::new(Shown {
        layers_opacity: RefCell::new(Seen::default()),
        blend: RefCell::new(Seen::default()),
        effects_opacity: RefCell::new(Seen::default()),
        effects: RefCell::new(Seen::default()),
    });
    let seen = shown.clone();
    let mut fixture = Fixture::new(move |ui| {
        Panel::new().label("Layers").id_salt("layers").show(ui, |ui, _| {
            Probe::new(&seen.layers_opacity).label("Opacity").show(ui);
            Probe::new(&seen.blend).label("Blend mode").show(ui);
        });
        Panel::new().label("Effects").id_salt("effects").seen(&seen.effects).show(ui, |ui, _| {
            Probe::new(&seen.effects_opacity).label("Opacity").show(ui);
        });
    });
    fixture.scale(scale);
    fixture.settle();
    (fixture, shown)
}

#[trial]
fn a_lookup_by_name_and_role_returns_the_matches_in_tree_order() {
    let (fixture, shown) = panels(Scale::default());
    let tree = fixture.tree();
    let opacity = [Shown::key(&shown.layers_opacity), Shown::key(&shown.effects_opacity)];
    assert_eq!(tree.find(&Query::named("Opacity")), opacity, "both buttons, the first shown first");
    assert_eq!(tree.find(&Query::name_contains("pacit")), opacity, "a part of the name finds the same nodes");
    assert_eq!(tree.find(&Query::named("Blend mode")), [Shown::key(&shown.blend)]);
    assert_eq!(tree.find(&Query::named("Effects").with_role(Role::GROUP)), [Shown::key(&shown.effects)]);
    assert!(tree.find(&Query::named("Effects").with_role(Role::BUTTON)).is_empty(), "the role narrows the name");
    assert!(tree.find(&Query::named("Opac")).is_empty(), "an exact name matches the whole name");
}

#[trial]
fn an_ancestor_or_a_path_picks_one_of_two_same_named_buttons() {
    let (fixture, shown) = panels(Scale::default());
    let tree = fixture.tree();
    let effects_opacity = [Shown::key(&shown.effects_opacity)];
    assert_eq!(tree.find(&Query::named("Opacity").within(Query::named("Effects"))), effects_opacity);
    assert_eq!(tree.find(&Query::path(["Effects", "Opacity"]).expect("a path")), effects_opacity, "a path is a chain of ancestors");
    assert_eq!(
        tree.find(&Query::path(["Layers", "Opacity"]).expect("a path")),
        [Shown::key(&shown.layers_opacity)],
        "the other panel picks the other button"
    );
    assert!(tree.find(&Query::path(["Effects", "Blend mode"]).expect("a path")).is_empty(), "a node outside the ancestor is not found");
    assert!(tree.find(&Query::named("Opacity").within(Query::named("Missing"))).is_empty());
    let ancestors: Vec<NodeKey> = tree.ancestors(Shown::key(&shown.effects_opacity)).collect();
    assert!(ancestors.contains(&Shown::key(&shown.effects)), "the panel is among the button's ancestors");
}

#[trial]
fn a_found_nodes_bounds_are_points_that_hit_it_at_every_scale() {
    let mut bounds = Vec::new();
    for scale in [Scale::new(1.0), Scale::new(2.0)] {
        let (mut fixture, shown) = panels(scale);
        let tree = fixture.tree();
        let [button] = tree.find(&Query::path(["Effects", "Opacity"]).expect("a path"))[..] else { panic!("one button") };
        let [panel] = tree.find(&Query::named("Effects"))[..] else { panic!("one panel") };
        let (at, inside) = (tree.bounds_of(button).expect("bounds"), tree.bounds_of(panel).expect("bounds"));
        assert!(inside.contains_rect(at), "the button lies inside its panel at {scale:?}");
        assert_eq!(at, shown.effects_opacity.borrow().rect, "the bounds are the rect the button was shown at, in points");
        fixture.click_at(at.center());
        assert_eq!(shown.effects_opacity.borrow().activations, 1, "a click at the bounds' centre presses the button at {scale:?}");
        assert_eq!(shown.layers_opacity.borrow().activations, 0);
        bounds.push(at);
    }
    assert_eq!(bounds[0], bounds[1], "bounds are in points, the same at every scale");
}

/// A button in an arena whose body is shown zoomed in on its own layer, as a node graph's is: its node's bounds are
/// where the screen shows it, not its rect in the body's layer, so a click at their centre presses it.
#[trial]
fn a_node_in_a_transformed_body_is_bounded_where_the_screen_shows_it() {
    let zoom = CanvasTransform::new(2.0, Vec2::new(10.0, 5.0));
    let inside = Rc::new(RefCell::new(Seen::default()));
    let seen = inside.clone();
    let filler = RefCell::new(Seen::default());
    let mut fixture = Fixture::new(move |ui| {
        Panel::new().label("Zoomed").arena(true).sense(Sense::CLICK).transform(zoom).show(ui, |ui, _| {
            Probe::new(&seen).label("Inside").show(ui);
            // Room below, so the zoomed button stays inside the body's clip.
            Probe::new(&filler).label("Filler").size(Vec2::splat(200.0)).show(ui);
        });
    });
    fixture.settle();
    let tree = fixture.tree();
    let [button] = tree.find(&Query::path(["Zoomed", "Inside"]).expect("a path"))[..] else { panic!("one button") };
    let at = tree.bounds_of(button).expect("bounds");
    let local = inside.borrow().rect;
    assert_eq!(at.size(), local.size() * zoom.scale, "the bounds carry the body's zoom");
    fixture.click_at(at.center());
    assert_eq!(inside.borrow().activations, 1, "a click at the bounds' centre presses the button");
}
