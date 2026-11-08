//! The access tree reports a node's selected state, and a component can declare the document role.

use crate::contract::kit::PlainStyle;
use craft_behavior::IconId;
use craft_ui::buttons::IconButton;
use craft_ui::contract::{Common, Component, Cx, Props as _, Role, Show as _};
use craft_ui::style::StyleCx;
use craft_ui::testing::{Fixture, trial};
use egui::{Response, vec2};

#[trial]
fn a_selected_icon_button_reads_as_selected_and_an_unselected_one_does_not() {
    let mut fixture = Fixture::new(|ui| {
        IconButton::new(IconId::new("check")).label("On").selected(true).id_salt("on").show(ui);
        IconButton::new(IconId::new("check")).label("Off").id_salt("off").show(ui);
    });
    fixture.settle();
    let selected = |label: &str| {
        let button = fixture.by_label(label).expect("the button is shown");
        fixture.tree().selected_of(button.node)
    };
    assert_eq!(selected("On"), Some(true));
    assert_ne!(selected("Off"), Some(true));
}

/// A component with the document role.
#[derive(craft_ui::contract::Component)]
struct Page {
    #[common]
    common: Common,
}

impl Component for Page {
    const ROLE: Role = Role::DOCUMENT;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PageStyleKey) -> PlainStyle {
        PlainStyle::min(vec2(200.0, 20.0))
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn a_component_with_the_document_role_reports_a_document_node() {
    let mut fixture = Fixture::new(|ui| {
        Page { common: Common::new() }.label("Page").show(ui);
    });
    fixture.settle();
    let page = fixture.by_role(Role::DOCUMENT);
    assert_eq!(page.len(), 1, "the page is one document node");
    assert_eq!(fixture.tree().role_of(page[0].node), Some(Role::Node(accesskit::Role::Document)));
}
