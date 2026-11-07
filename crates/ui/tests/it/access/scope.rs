//! The label scope and the merge rule.

use crate::contract::kit::{Panel, Probe, Seen};
use craft_behavior::Sense;
use craft_ui::contract::{Common, Component, Container, ContainerProps, Cx, FieldLabel, Props as _, Role as UiRole, Show as _, ShowContainer as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::{AccessTree, NodeKey, begin_label_scope, name};
use craft_ui::testing::memory::global;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Rect, Response};
use std::cell::RefCell;

fn field_label() -> FieldLabel {
    FieldLabel::new(global("access.label", "name"), Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 10.0)), "Name")
}

/// One frame of a container with a side label around `controls` probes.
fn field(frames: &mut Frames, controls: usize, keeps_name: bool) -> (Id, Vec<Id>, AccessTree) {
    let container = global("access.field", "scope");
    let mut ids = Vec::new();
    frames.frame(vec![], |ui| {
        ids.clear();
        Panel::new().id(container).show(ui, |ui, _| {
            // The container's body opens the scope; its report step closes it and patches the nodes registered in it.
            begin_label_scope(ui.ctx(), container, field_label());
            for at in 0..controls {
                let seen = RefCell::new(Seen::default());
                Probe::new(&seen).label(format!("Control {at}")).keeps_name(keeps_name).show(ui);
                ids.extend(seen.borrow().id);
            }
        });
    });
    (container, ids, frames.tree())
}

#[trial]
fn one_control_is_labelled_by_the_field_label() {
    let mut frames = Frames::new();
    let (_, ids, tree) = field(&mut frames, 1, false);
    assert_eq!(tree.labelled_by(ids[0]), [NodeKey::of(field_label().id)]);
    assert_eq!((tree.role(field_label().id), tree.name(field_label().id)), (Some(UiRole::LABEL), Some("Name")));
}

#[trial]
fn the_name_rule_follows_labelled_by() {
    let mut frames = Frames::new();
    let (_, ids, tree) = field(&mut frames, 1, false);
    let control = NodeKey::of(ids[0]);
    assert_eq!(tree.name(ids[0]), None, "the label replaces the control's own name");
    assert_eq!(name(&tree, control), Some("Name"), "a control named only through labelled_by has a name for the checker");
}

#[trial]
fn a_control_that_keeps_its_name_is_described_by_the_label() {
    let mut frames = Frames::new();
    let (_, ids, tree) = field(&mut frames, 1, true);
    assert_eq!((tree.name(ids[0]), tree.labelled_by(ids[0]).len()), (Some("Control 0"), 0));
    assert_eq!(tree.described_by(ids[0]), [NodeKey::of(field_label().id)]);
}

#[trial]
fn several_controls_make_the_container_labelled() {
    let mut frames = Frames::new();
    let (container, ids, tree) = field(&mut frames, 2, false);
    assert_eq!(tree.labelled_by(container), [NodeKey::of(field_label().id)]);
    assert!(ids.iter().all(|id| tree.labelled_by(*id).is_empty()), "the controls keep their own names");
}

/// A decorator: it emits no node and merges into the one component its body shows.
#[derive(craft_ui::contract::Component)]
#[component(container)]
struct Decor {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

#[derive(Clone, Debug, Default, PartialEq, craft_ui::style::ComponentStyle)]
struct DecorLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

impl Component for Decor {
    const ROLE: UiRole = UiRole::Merged;
    type Style = DecorLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DecorStyleKey) -> DecorLook {
        DecorLook::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Decor {}

#[trial]
fn a_decorator_merges_its_name_and_description_into_the_child() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut decor = None;
    frames.frame(vec![], |ui| {
        let decorator = Decor { common: Common::new(), props: ContainerProps::new() }.label("3 unread").description("New mail");
        let out = decorator.show(ui, |ui, _| {
            Probe::new(&seen).label("Inbox").show(ui);
        });
        decor = Some(out.output.id);
    });
    let tree = frames.tree();
    assert!(!tree.has(decor.expect("shown")), "the decorator has no node");
    let child = seen.borrow().id.expect("the child updated");
    assert_eq!((tree.name(child), tree.description(child)), (Some("Inbox, 3 unread"), Some("New mail")));
}

// A container control inside a field is the one control: the label names it, not the field and not its children.
#[trial]
fn a_container_control_inside_a_field_is_the_one_control() {
    let mut frames = Frames::new();
    let container = global("access.field", "container-control");
    let group = RefCell::new(Seen::default());
    let (first, second) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    frames.frame(vec![], |ui| {
        Panel::new().id(container).show(ui, |ui, _| {
            begin_label_scope(ui.ctx(), container, field_label());
            Panel::new().seen(&group).show(ui, |ui, _| {
                Probe::new(&first).label("First").show(ui);
                Probe::new(&second).label("Second").show(ui);
            });
        });
    });
    let tree = frames.tree();
    let id = |seen: &RefCell<Seen>| seen.borrow().id.expect("updated");
    assert_eq!(tree.labelled_by(id(&group)), [NodeKey::of(field_label().id)]);
    assert!(tree.labelled_by(container).is_empty(), "the field is not made a group");
    assert!(tree.labelled_by(id(&first)).is_empty() && tree.labelled_by(id(&second)).is_empty());
}

// A decorator around a container merges into the container's node, not into one of its children.
#[trial]
fn a_decorator_around_a_container_merges_into_the_container() {
    let mut frames = Frames::new();
    let group = RefCell::new(Seen::default());
    let child = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        let decorator = Decor { common: Common::new(), props: ContainerProps::new() }.label("3 unread");
        decorator.show(ui, |ui, _| {
            Panel::new().seen(&group).show(ui, |ui, _| {
                Probe::new(&child).label("Inbox").show(ui);
            });
        });
    });
    let tree = frames.tree();
    let id = |seen: &RefCell<Seen>| seen.borrow().id.expect("updated");
    assert_eq!(tree.name(id(&child)), Some("Inbox"), "the grandchild keeps its own name");
    assert!(tree.name(id(&group)).is_some_and(|name| name.contains("3 unread")));
}
