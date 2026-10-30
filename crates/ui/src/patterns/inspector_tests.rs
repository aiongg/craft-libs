//! Inspector tests: state keyed by the target, header truncation and kind hiding by width, the
//! empty state and the header's AccessKit node, with a raw egui context driven frame by frame.

use egui::accesskit::{self, NodeId, Role};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, vec2};

use super::*;
use crate::composites::SectionHeader;
use crate::icons::Icon;
use crate::theme::{Base, install};

const LONG: &str = "A layer with a name much too long for a narrow panel";

struct Host {
    ctx: egui::Context,
    width: f32,
    target: &'static str,
    empty: bool,
    header: Option<(&'static str, Option<&'static str>)>,
    events: Vec<Event>,
    time: f64,
    tree: Vec<(NodeId, accesskit::Node)>,
}

impl Host {
    fn new() -> Self {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut host = Self { ctx, width: 400.0, target: "a", empty: false, header: None, events: Vec::new(), time: 0.0, tree: Vec::new() };
        host.run(4);
        host
    }

    fn frame(&mut self) {
        self.time += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(self.width, 400.0))),
            time: Some(self.time),
            predicted_dt: 1.0 / 60.0,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let (target, empty, header) = (self.target, self.empty, self.header);
        let mut full = self.ctx.run_ui(raw, |ui| {
            let mut inspector = Inspector::new("properties").target(target).empty("No properties").show_empty(empty);
            if let Some((name, kind)) = header {
                let header = InspectorHeader::new(Icon::Layers, name);
                inspector = inspector.header(match kind {
                    Some(kind) => header.kind(kind),
                    None => header,
                });
            }
            inspector.show(ui, |ui| {
                SectionHeader::new("transform", "Transform").show(ui, |ui| {
                    ui.label("Body");
                });
            });
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    fn node(&self, role: Role, label: &str) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.role() == role && n.label() == Some(label))
    }

    fn by_role(&self, role: Role) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.role() == role)
    }

    fn body_shown(&self) -> bool {
        self.tree.iter().any(|(_, n)| n.value() == Some("Body"))
    }

    fn click_section(&mut self) {
        let b = self.node(Role::DisclosureTriangle, "Transform").and_then(|n| n.bounds()).expect("section header");
        let p = Pos2::new(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
        self.events.push(Event::PointerMoved(p));
        self.run(2);
        for pressed in [true, false] {
            self.events.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
        self.run(3);
    }

    /// The name label's rect and the header's rect.
    fn name_and_header(&self, name: &str) -> (accesskit::Rect, accesskit::Rect) {
        let label = self.tree.iter().map(|(_, n)| n).find(|n| n.role() == Role::Label && n.value() == Some(name)).expect("name label");
        (label.bounds().expect("name bounds"), self.by_role(Role::Header).and_then(|n| n.bounds()).expect("header bounds"))
    }
}

#[test]
fn a_target_change_resets_the_state_of_the_content() {
    let mut host = Host::new();
    assert!(host.body_shown());
    host.click_section();
    assert!(!host.body_shown(), "the section closed");
    host.target = "b";
    host.run(3);
    assert!(host.body_shown(), "a new target starts with the section open");
    host.target = "a";
    host.run(3);
    assert!(!host.body_shown(), "the first target keeps its own state");
}

#[test]
fn the_name_is_cut_to_the_row_and_the_kind_hides_when_the_row_is_narrow() {
    let mut host = Host::new();
    host.header = Some((LONG, Some("Pixel layer")));
    host.width = 400.0;
    host.run(3);
    assert!(host.tree.iter().any(|(_, n)| n.value() == Some("Pixel layer")), "the kind shows in a wide row");

    host.width = 110.0;
    host.run(3);
    assert!(!host.tree.iter().any(|(_, n)| n.value() == Some("Pixel layer")), "the kind is hidden in a narrow row");
    let (name, header) = host.name_and_header(LONG);
    assert!(name.x1 <= header.x1 + 0.5, "the name ends inside the row: {name:?} in {header:?}");
    assert!(name.y1 - name.y0 <= header.y1 - header.y0 + 0.5, "the name is one line");
}

#[test]
fn the_name_ends_before_the_kind_text() {
    let mut host = Host::new();
    host.header = Some((LONG, Some("Pixel layer")));
    host.width = 300.0;
    host.run(3);
    let kind = host.tree.iter().map(|(_, n)| n).find(|n| n.value() == Some("Pixel layer")).and_then(|n| n.bounds()).expect("kind");
    let (name, _) = host.name_and_header(LONG);
    assert!(name.x1 <= kind.x0 + 0.5, "{name:?} overlaps {kind:?}");
}

#[test]
fn the_header_node_is_named_by_the_name_and_the_kind() {
    let mut host = Host::new();
    host.header = Some(("Layer 1", Some("Pixel layer")));
    host.run(3);
    assert!(host.node(Role::Header, "Layer 1 - Pixel layer").is_some());
    host.header = Some(("Document", None));
    host.run(3);
    assert!(host.node(Role::Header, "Document").is_some(), "no kind, no separator");
}

#[test]
fn the_empty_state_is_a_polite_status_region_and_replaces_the_content() {
    let mut host = Host::new();
    host.empty = true;
    host.run(4);
    let status = host.by_role(Role::Status).expect("status region");
    assert_eq!(status.live(), Some(accesskit::Live::Polite));
    assert!(host.tree.iter().any(|(_, n)| n.value() == Some("No properties")));
    assert!(!host.body_shown());
}
