//! Roles, names and sub-nodes of the nodes the pipeline emits, and announcements.

use crate::contract::kit::{Panel, Probe, RowProbe, Rows, Seen, Table};
use craft_behavior::{PartKey, Sense};
use craft_ui::contract::announce;
use craft_ui::contract::{
    Access, Common, Component, Cx, Name, Numeric, Politeness, Props as _, Role as UiRole, Show as _, ShowCollection as _, ShowContainer as _,
};
use craft_ui::style::StyleCx;
use craft_ui::testing::access::{Declared, NodeKey};
use craft_ui::testing::contract::findings;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Response, Vec2};
use std::cell::RefCell;
use std::sync::Mutex;

#[trial]
fn a_leaf_has_its_role_name_and_actions() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        Probe::new(&seen).label("Save").show(ui);
    });
    let id = seen.borrow().id.expect("the leaf updated");
    let tree = frames.tree();
    assert_eq!(tree.role(id), Some(UiRole::BUTTON));
    assert_eq!(tree.name(id), Some("Save"));
    assert!(tree.declares(id, Declared::Focus) && tree.declares(id, Declared::Click));
}

#[trial]
fn a_body_is_the_parent_of_the_nodes_it_shows() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut panel = None;
    frames.frame(vec![], |ui| {
        let out = Panel::new().label("Options").show(ui, |ui, _| {
            Probe::new(&seen).label("Ok").show(ui);
        });
        panel = Some(out.output.id);
    });
    let (panel, child) = (panel.expect("shown"), seen.borrow().id.expect("the child updated"));
    let tree = frames.tree();
    assert_eq!((tree.role(panel), tree.name(panel)), (Some(UiRole::GROUP), Some("Options")));
    assert_eq!(tree.parent_of(child), Some(NodeKey::of(panel)), "the child's nearest visible ancestor is the container's node");
}

static ROWS: Mutex<Vec<(usize, Rect)>> = Mutex::new(Vec::new());

#[trial]
fn items_are_named_nodes_below_their_collection() {
    let mut frames = Frames::new();
    let mut list = None;
    frames.frame(vec![], |ui| {
        let out = Table::new(20.0).label("Rows").show_items(ui, &Rows(3), &mut |key, _| RowProbe::new(*key, &ROWS).label(format!("Row {key}")));
        list = Some(out.id);
    });
    let list = list.expect("shown");
    let tree = frames.tree();
    let items = tree.with_role(UiRole::LIST_ITEM);
    let mut names: Vec<_> = items.iter().filter_map(|item| tree.name_of(*item).map(str::to_owned)).collect();
    names.sort();
    assert_eq!(names, ["Row 0", "Row 1", "Row 2"]);
    assert_eq!(tree.role(list), Some(UiRole::LIST));
    for item in items {
        assert_eq!(tree.parent_of_key(item), Some(NodeKey::of(list)), "an item's node sits below the list's node");
    }
}

/// A leaf with a faced sub-node and a numeric value.
#[derive(craft_ui::contract::Component)]
struct Meter {
    #[common]
    common: Common,
}

#[derive(Clone, Debug, Default, PartialEq, craft_ui::style::ComponentStyle)]
struct MeterLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

impl Component for Meter {
    const ROLE: UiRole = UiRole::SLIDER;
    type Style = MeterLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: MeterStyleKey) -> MeterLook {
        MeterLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(60.0, 20.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.numeric(level());
        access.node(PartKey::new("peak"), UiRole::PROGRESS_INDICATOR).name = Name::Text("Peak".into());
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn level() -> Numeric {
    Numeric::new(3.0, 0.0..=10.0).step(1.0).text("3 of 10")
}

impl Meter {
    fn new() -> Self {
        Meter { common: Common::new() }
    }
}

#[trial]
fn access_facts_reach_the_node_and_its_sub_nodes() {
    let mut frames = Frames::new();
    let mut id = None;
    frames.frame(vec![], |ui| {
        id = Some(Meter::new().label("Level").show(ui).id);
    });
    let id = id.expect("shown");
    let tree = frames.tree();
    assert_eq!(tree.numeric(id), Some(level()));
    assert!(tree.declares(id, Declared::Increment) && tree.declares(id, Declared::Decrement) && tree.declares(id, Declared::SetValue));
    let sub = tree.with_role(UiRole::PROGRESS_INDICATOR);
    assert_eq!(sub.len(), 1, "the sub-node of the `peak` part");
    assert_eq!(tree.name_of(sub[0]), Some("Peak"));
}

#[trial]
fn announcements_reach_the_frame_output_and_one_status_node() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        announce(ui.ctx(), "Saved", Politeness::Polite);
    });
    let tree = frames.tree();
    let status = tree.with_role(UiRole::STATUS);
    assert_eq!(status.len(), 1);
    assert_eq!(tree.name_of(status[0]), Some("Saved"));
    assert_eq!(craft_ui::runtime::frame::take_output(&frames.ctx).announcements, ["Saved"]);
    frames.frame(vec![], |_| {});
    let tree = frames.tree();
    let status = tree.with_role(UiRole::STATUS);
    assert_eq!(status.len(), 1, "the status node stays, so the next announcement is a change of a live node");
    assert_eq!(tree.name_of(status[0]), None, "and it says nothing");
}

// The same text announced in two frames in a row reads as a change both times, and a discarded pass that
// announces again adds nothing.
#[trial]
fn a_repeated_announcement_changes_the_status_text_and_a_discarded_pass_adds_nothing() {
    let mut frames = Frames::new();
    let mut said = Vec::new();
    for _ in 0..3 {
        frames.frame(vec![], |ui| {
            announce(ui.ctx(), "Saved", Politeness::Polite);
            announce(ui.ctx(), "Saved", Politeness::Polite);
        });
        let tree = frames.tree();
        let status = tree.with_role(UiRole::STATUS);
        said.push(tree.name_of(status[0]).map(str::to_owned));
        assert_eq!(craft_ui::runtime::frame::take_output(&frames.ctx).announcements, ["Saved"]);
    }
    assert!(said.windows(2).all(|pair| pair[0] != pair[1]), "{said:?}");
    assert!(said.iter().all(|text| text.as_deref().is_some_and(|text| text.starts_with("Saved") && !text.contains(". "))));
}

#[trial]
fn an_unnamed_button_is_a_role_rule_finding() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        Probe::new(&seen).label("").show(ui);
    });
    assert!(findings(&frames.ctx).iter().any(|finding| finding.rule == "RoleRule" && finding.subject.contains("a name")));
    frames.frame(vec![], |ui| {
        Probe::new(&seen).label("").tooltip("Save the file").show(ui);
    });
    let id = seen.borrow().id.expect("updated");
    assert_eq!(frames.tree().name(id), Some("Save the file"), "an icon-only control takes its tooltip as its name");
}

// A word tooltip that names an icon-only control is read in the scope's language.
#[trial]
fn a_word_tooltip_is_resolved_in_the_scopes_language() {
    use craft_ui::runtime::env::{Overrides, Word, WordRows, WordsOverride, with_env};
    const SAVE: Word = Word::keyed("trial.save", "Save");
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        with_env(
            ui,
            |o: &mut Overrides| o.words = Some(WordsOverride::Overlay(WordRows::new().row(SAVE, "Sichern"))),
            |ui| {
                Probe::new(&seen).label("").tooltip(SAVE).show(ui);
            },
        );
    });
    let id = seen.borrow().id.expect("updated");
    assert_eq!(frames.tree().name(id), Some("Sichern"));
}

/// A combo box that reports an open state only when it has one.
#[derive(craft_ui::contract::Component)]
struct Dropdown {
    #[common]
    common: Common,
    open: Option<bool>,
}

impl Component for Dropdown {
    const ROLE: UiRole = UiRole::COMBO_BOX;
    type Style = MeterLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DropdownStyleKey) -> MeterLook {
        MeterLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(60.0, 20.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        if let Some(open) = self.open {
            access.expanded(open);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

// Expand and Collapse are declared only where the node has an open state; a combo box without one is a finding.
#[trial]
fn expand_is_declared_only_where_the_node_has_an_open_state() {
    let shown = |open: Option<bool>| {
        let mut frames = Frames::new();
        let mut id = None;
        frames.frame(vec![], |ui| id = Some(Dropdown { common: Common::new(), open }.label("Size").show(ui).id));
        let id = id.expect("shown");
        let tree = frames.tree();
        let finding = findings(&frames.ctx).iter().any(|finding| finding.rule == "RoleRule" && finding.subject.contains("expanded"));
        (tree.declares(id, Declared::Expand), tree.declares(id, Declared::Collapse), finding)
    };
    assert_eq!(shown(None), (false, false, true));
    assert_eq!(shown(Some(false)), (true, false, false));
    assert_eq!(shown(Some(true)), (false, true, false));
}
