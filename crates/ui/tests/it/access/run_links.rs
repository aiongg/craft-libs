//! Links inside a text run: a `Link` child of the run node, named by its characters and bounded by their extent.

use craft_behavior::PartKey;
use craft_ui::contract::{Access, Common, Component, Cx, Props as _, Role, Show as _};
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Response, Vec2};

const RUN: PartKey = PartKey::new("line");
const TEXT: &str = "See the docs here";
const ADVANCE: f32 = 8.0;
const TARGET: &str = "https://example.org/docs";

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
struct LineLook {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// A leaf that describes one line of text with a link in it.
#[derive(craft_ui::contract::Component)]
struct Line {
    #[common]
    common: Common,
}

impl Component for Line {
    const ROLE: Role = Role::GROUP;
    type Style = LineLook;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: LineStyleKey) -> LineLook {
        LineLook { metrics: craft_behavior::Metrics::new().min_size(Vec2::new(200.0, 20.0)) }
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::empty()
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let count = TEXT.chars().count();
        let positions = (0..count).map(|at| at as f32 * ADVANCE).collect();
        let bounds = Rect::from_min_size(cx.rect.min, Vec2::new(count as f32 * ADVANCE, cx.rect.height()));
        access.node(RUN, Role::TEXT_RUN).text_run(TEXT, positions, vec![ADVANCE; count], false).bounds(bounds).run_link(8..12, TARGET);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_link_in_a_text_run_is_a_link_child_named_by_its_text_inside_the_run() {
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        Line { common: Common::new() }.label("Line").show(ui);
    });
    let tree = frames.tree();
    let runs = tree.with_role(Role::TEXT_RUN);
    assert_eq!(runs.len(), 1);
    let links = tree.with_role(Role::LINK);
    assert_eq!(links.len(), 1, "one link in the run");
    assert_eq!(tree.name_of(links[0]), Some("docs"));
    assert_eq!(tree.parent_of_key(links[0]), Some(runs[0]), "the link is a child of the run node");
    let (run, link): (Rect, Rect) = (tree.bounds_of(runs[0]).expect("run bounds"), tree.bounds_of(links[0]).expect("link bounds"));
    assert!(run.contains_rect(link) && link.width() > 0.0, "the link lies inside the run: {link:?} in {run:?}");
    assert!(link.min.x > run.min.x, "and after the text that precedes it");
}
