//! Inspected regions. [`record`] shows a body in a scope node of its own; the region's record is read from what
//! ui-rt keeps of that scope's nodes: every component with its window rect and access facts, the hit shapes its parts
//! declared and the focus stops in the window's Tab order. The record is what the debug inspector draws and what a
//! ribbon reads its keytip places from.

use crate::contract::Ui;
use crate::contract::rt::node::{ComponentNode, Kept, ScopeNode};
use crate::contract::rt::overlay::LayerScope;
use crate::primitives::Id;
use crate::runtime::shift::RecordShift;
use craft_behavior::{PartRef, Sense};
use craft_keymap::CommandRef;
use craft_ui_rt::{HitShape, NodeId, Rect, Runtime, Vec2};
use std::collections::HashMap;
use std::sync::Arc;

/// What a node says, as its access node gives it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Facts {
    /// The AccessKit role's name.
    pub role: String,
    /// The accessible name.
    pub name: Option<String>,
    /// The nodes that label it.
    pub labelled_by: Vec<Id>,
    /// The value, as the component words it.
    pub value: Option<String>,
    /// The actions the node declares, in words.
    pub actions: Vec<&'static str>,
    /// The node is disabled.
    pub disabled: bool,
    /// The node takes focus or a click.
    pub interactive: bool,
}

/// One element an inspected region showed: a component, or a part of one with a node, a hit area or a focus stop.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Element {
    /// The component.
    pub id: Id,
    /// The part, `None` for the component itself.
    pub part: Option<PartRef>,
    /// The component's type name, without its path.
    pub component: &'static str,
    /// The number of components it sits in within the region.
    pub depth: usize,
    /// Its box, in window points.
    pub rect: Rect,
    /// The box inside its padding.
    pub content: Rect,
    /// What its node says; `None` for a component without a node, or while the window builds no access tree.
    pub facts: Option<Facts>,
    /// Its place in Tab order within the region, from 1, when keyboard focus reaches it.
    pub stop: Option<usize>,
    /// The command bound to the control, when it has one.
    pub command: Option<CommandRef>,
}

impl Element {
    /// The id of the access node the element stands for: the component's own id, or the sub-node of its part. A
    /// member of a collection's run is recorded as such a part of the collection.
    pub fn node(&self) -> Id {
        self.part.map_or(self.id, |part| crate::runtime::access::sub_node_id(self.id, part))
    }
}

/// A hit area a component of the region declared.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HitArea {
    /// The component the area belongs to.
    pub owner: Id,
    /// The part, `None` for the component's own area.
    pub part: Option<PartRef>,
    /// The area's bounds, in window points.
    pub rect: Rect,
}

/// What an inspected region showed.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Recorded {
    /// The elements in tree order: a component before what it shows, its parts right after it.
    pub elements: Vec<Element>,
    /// The hit areas, in the tree order of their components.
    pub hits: Vec<HitArea>,
}

/// A memo scope kept at another place returns the record its run made, moved with it.
impl crate::runtime::Shift for Arc<Recorded> {
    fn shifted(&self, by: &RecordShift) -> Self {
        let elements =
            self.elements.iter().map(|element| Element { rect: by.rect(element.rect), content: by.rect(element.content), ..element.clone() });
        let hits = self.hits.iter().map(|hit| HitArea { rect: by.rect(hit.rect), ..hit.clone() });
        Arc::new(Recorded { elements: elements.collect(), hits: hits.collect() })
    }

    fn snapped(&self, to: &Self, within: f32) -> Self {
        if self.elements.len() != to.elements.len() || self.hits.len() != to.hits.len() {
            return Arc::clone(self);
        }
        let elements = self.elements.iter().zip(&to.elements).map(|(element, to)| Element {
            rect: element.rect.snapped(&to.rect, within),
            content: element.content.snapped(&to.content, within),
            ..element.clone()
        });
        let hits = self.hits.iter().zip(&to.hits).map(|(hit, to)| HitArea { rect: hit.rect.snapped(&to.rect, within), ..hit.clone() });
        Arc::new(Recorded { elements: elements.collect(), hits: hits.collect() })
    }
}

impl Recorded {
    /// The rect of the element at `index` where Tab stops on it: its hit area when it has one, else its box.
    pub fn stop_rect(&self, index: usize) -> Option<Rect> {
        let element = self.elements.get(index)?;
        let hit = self.hits.iter().find(|hit| hit.owner == element.id && hit.part == element.part);
        Some(hit.map_or(element.rect, |hit| hit.rect))
    }
}

/// The session slot of a region: its latest record and the element its list points at.
#[derive(Clone, Default)]
struct Region {
    recorded: Option<Arc<Recorded>>,
    pointed: Option<usize>,
}

fn region_key(key: Id) -> Id {
    key.with("inspect")
}

/// The ui-rt type key of a region's scope node.
const REGION_TYPE: u64 = 0x1a5e_c7ed_0000_0001;

/// Shows `body` as the inspected region `key` and returns its record, which is also kept for [`Ui::inspect_recorded`].
/// A measuring run records nothing and returns `None`.
///
/// Rects are the window rects of ui-rt's last layout: a node this build shows for the first time has none yet, so
/// the region builds again in the next frame and its record then holds it.
pub fn record<R: Clone + 'static>(ui: &mut Ui<'_, '_>, key: Id, body: impl FnOnce(&mut Ui<'_, '_>) -> R) -> (R, Option<Arc<Recorded>>) {
    if ui.is_measuring() {
        return (body(ui), None);
    }
    let mut body = Some(body);
    let scope = crate::contract::rt::scope::ScopeShow {
        id: crate::contract::IdSource::Id(key),
        type_key: REGION_TYPE,
        memo: None,
        estimate: 0.0,
        reveal: false,
        lazy: false,
    };
    let shown = crate::contract::rt::scope::show_scope(ui, scope, None, &mut |ui| (body.take().expect("a region's body runs once per build"))(ui));
    let out = shown.expect("a region that is not lazy always shows").value;
    let cx = ui.build_cx();
    let Some(node) = crate::contract::rt::ids::node_of(cx, key) else { return (out, None) };
    let (recorded, unplaced) = read_region(cx.runtime(), cx.window(), node);
    if unplaced {
        let now = cx.now();
        cx.request_build_at(Some(now));
    }
    let recorded = Arc::new(recorded);
    let changed = ui.with_session::<Region, _>(region_key(key), |region| {
        let changed = region.recorded.as_deref() != Some(&*recorded);
        if changed {
            region.recorded = Some(Arc::clone(&recorded));
        }
        changed
    });
    if !changed {
        // The kept record, so readers that compare by pointer see no change.
        let kept = ui.with_session::<Region, _>(region_key(key), |region| region.recorded.clone());
        return (out, kept);
    }
    (out, Some(recorded))
}

impl Ui<'_, '_> {
    /// The latest record of the region `key` ([`record`]), read as a value: a new record reruns the reader.
    pub fn inspect_recorded(&mut self, key: Id) -> Option<Arc<Recorded>> {
        self.with_session::<Region, _>(region_key(key), |region| region.recorded.clone())
    }

    /// The element of the region `key` its list points at (the row under the pointer), read as a value.
    pub fn inspect_pointed(&mut self, key: Id) -> Option<usize> {
        self.with_session::<Region, _>(region_key(key), |region| region.pointed)
    }

    /// Points the list of the region `key` at the element at `index`, or at none; the readers of the old value rerun.
    pub fn inspect_point_at(&mut self, key: Id, index: Option<usize>) {
        if self.is_measuring() {
            return;
        }
        let unchanged = self.with_session::<Region, _>(region_key(key), |region| region.pointed == index);
        if !unchanged {
            self.with_session::<Region, _>(region_key(key), |region| region.pointed = index);
        }
    }
}

/// The kept node of a component, scope or layer.
fn kept_of(rt: &Runtime, node: NodeId) -> Option<&Kept> {
    rt.local::<ComponentNode>(node).or_else(|| rt.local::<ScopeNode>(node)).or_else(|| rt.local::<LayerScope>(node).map(|layer| &layer.kept))
}

/// The bounds of a kept hit shape, in window points, for a node at `origin`.
fn shape_rect(shape: &HitShape, origin: Vec2) -> Rect {
    match *shape {
        HitShape::Rect(rect) | HitShape::RoundedRect(rect, _) => rect.translate(origin),
        HitShape::Circle(center, radius) => Rect::from_center_size(center, Vec2::splat(radius * 2.0)).translate(origin),
    }
}

/// The type name `full` without its path and type arguments.
fn short_name(full: &'static str) -> &'static str {
    let name = full.split('<').next().unwrap_or(full);
    name.rsplit("::").next().unwrap_or(name)
}

/// The facts of an access node.
fn facts_of(node: &accesskit::Node, interactive: bool) -> Facts {
    use accesskit::Action;
    const ACTIONS: [(Action, &str); 9] = [
        (Action::Click, "click"),
        (Action::Focus, "focus"),
        (Action::Increment, "increment"),
        (Action::Decrement, "decrement"),
        (Action::SetValue, "set value"),
        (Action::Expand, "expand"),
        (Action::Collapse, "collapse"),
        (Action::ShowContextMenu, "context menu"),
        (Action::ScrollIntoView, "scroll into view"),
    ];
    Facts {
        role: format!("{:?}", node.role()),
        name: node.label().map(str::to_owned),
        labelled_by: Vec::new(),
        value: node.value().map(str::to_owned),
        actions: ACTIONS.iter().filter(|(action, _)| node.supports_action(*action)).map(|(_, word)| *word).collect(),
        disabled: node.is_disabled(),
        interactive,
    }
}

/// The record of the region whose scope node is `region`, and whether some component of it has no window rect yet.
fn read_region(rt: &Runtime, window: craft_ui_rt::WindowId, region: NodeId) -> (Recorded, bool) {
    let mut elements: Vec<Element> = Vec::new();
    let mut hits: Vec<HitArea> = Vec::new();
    let mut unplaced = false;
    let mut nodes: Vec<NodeId> = Vec::new();
    // Depth-first in tree order; the depth counts the components above a node inside the region.
    let mut pending: Vec<(NodeId, usize)> = rt.children(region).iter().rev().map(|&node| (node, 0)).collect();
    while let Some((node, depth)) = pending.pop() {
        let component = rt.local::<ComponentNode>(node);
        let below = depth + usize::from(component.is_some());
        pending.extend(rt.children(node).iter().rev().map(|&child| (child, below)));
        let Some(kept) = component else { continue };
        nodes.push(node);
        let Some(rect) = rt.window_rect(node) else {
            unplaced = true;
            continue;
        };
        let origin = rect.min.to_vec2();
        let interactive = kept.focus.stop.is_some() || kept.sense.intersects(Sense::CLICK);
        elements.push(Element {
            id: kept.id,
            part: None,
            component: short_name(kept.type_name),
            depth,
            rect,
            content: rect,
            facts: kept.access.as_ref().map(|node| facts_of(node, interactive)),
            stop: None,
            command: kept.command.clone(),
        });
        if kept.sense.intersects(Sense::CLICK | Sense::DRAG | Sense::HOVER) {
            hits.push(HitArea { owner: kept.id, part: None, rect });
        }
        for part in &kept.parts {
            let bounds = shape_rect(&part.shape, origin);
            hits.push(HitArea { owner: kept.id, part: Some(part.part), rect: bounds });
            let sub = kept.access_subs.iter().find(|sub| sub.key == part.id);
            if let Some(sub) = sub {
                let interactive = part.sense.intersects(Sense::CLICK);
                elements.push(Element {
                    id: kept.id,
                    part: Some(part.part),
                    component: short_name(kept.type_name),
                    depth: depth + 1,
                    rect: bounds,
                    content: bounds,
                    facts: Some(facts_of(&sub.node, interactive)),
                    stop: None,
                    command: None,
                });
            }
        }
    }
    // The window's Tab order, as ui-rt walks it, restricted to the region.
    let mut index: HashMap<(Id, Option<PartRef>), usize> = HashMap::new();
    for (at, element) in elements.iter().enumerate() {
        index.entry((element.id, element.part)).or_insert(at);
    }
    let order = rt.focus_order(window);
    let mut stop = 0;
    for node in order.into_iter().filter(|node| nodes.contains(node)) {
        let Some(kept) = kept_of(rt, node) else { continue };
        let part = kept.focus.part;
        let at = match index.get(&(kept.id, part)).copied() {
            Some(at) => at,
            None => {
                // A part stop with no access node of its own: listed after its component, where its hit area is.
                let Some(owner) = index.get(&(kept.id, None)).copied() else { continue };
                let rect = hits.iter().find(|hit| hit.owner == kept.id && hit.part == part).map_or(elements[owner].rect, |hit| hit.rect);
                let element = Element {
                    id: kept.id,
                    part,
                    component: elements[owner].component,
                    depth: elements[owner].depth + 1,
                    rect,
                    content: rect,
                    facts: None,
                    stop: None,
                    command: None,
                };
                let at = owner + 1;
                elements.insert(at, element);
                for old in index.values_mut() {
                    if *old >= at {
                        *old += 1;
                    }
                }
                index.insert((kept.id, part), at);
                at
            }
        };
        stop += 1;
        elements[at].stop = Some(stop);
    }
    (Recorded { elements, hits }, unplaced)
}
