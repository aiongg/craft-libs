//! What the derives write: registrations and probes, style keys per instantiation, setters and style accessors.

use super::kit::PlainStyle;
use craft_behavior::CancelReason;
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolPath, ToolPointer};
use craft_ui::contract::{
    Common, Component, Cx, EditResponse, Internal, MeasureCx, Meta, Pattern, Role, ShapeKind, Show as _, register_contract, shape,
};
use craft_ui::primitives::Draw;
use craft_ui::style::{StyleCx, StyleKey};
use craft_ui::testing::{Frames, Probe, trial};
use egui::{Response, Vec2};
use std::any::TypeId;
use std::marker::PhantomData;

/// Two codecs, as type arguments of a generic component.
struct Plain;
struct Number;

/// A generic value leaf whose style depends on a kind field.
#[derive(Component)]
#[component(probe(Tagged<Plain>, Tagged<Number>))]
struct Tagged<C: 'static> {
    #[common]
    common: Common,
    #[style_key]
    kind: u8,
    codec: PhantomData<fn() -> C>,
}

impl<C: 'static> Component for Tagged<C> {
    const ROLE: Role = Role::SLIDER;
    type Style = PlainStyle;
    type Output = EditResponse<u8>;

    fn style(_: &StyleCx<'_>, _: TaggedStyleKey<C>) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<u8> {
        craft_ui::testing::contract::edit_response(cx.response.clone())
    }
}

/// A canvas control that is exempt from a shared case.
#[derive(Component)]
#[component(canvas_control, exempt(case = "sizes", reason = "fills the canvas"))]
struct Handle {
    #[common]
    common: Common,
}

impl Component for Handle {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HandleStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A leaf whose `Meta` is written by hand and registered with `register_contract!`.
struct Manual {
    common: Common,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ManualKey;

impl StyleKey for ManualKey {}

impl Meta for Manual {
    type Shape = shape::Leaf;
    type StyleKey = ManualKey;

    fn style_key(&self) -> ManualKey {
        ManualKey
    }

    fn common(&self, _: Internal) -> &Common {
        &self.common
    }

    fn common_mut(&mut self, _: Internal) -> &mut Common {
        &mut self.common
    }
}

impl Component for Manual {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ManualKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0, 10.0)
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

register_contract!(Manual; overlay, exempt(case = "sizes", reason = "hand-written"));

/// A row of tools: a pattern assembles components and has no contract shape of its own.
#[derive(Pattern)]
struct ToolRow;

/// A pattern whose impl is written by hand.
struct Handwritten;

impl Pattern for Handwritten {}

register_contract!(Handwritten; pattern);

/// A tool that does nothing, registered as a story host of the tool kind.
#[derive(craft_ui::contract::Pattern)]
#[pattern(tool)]
struct IdleTool;

impl CanvasTool for IdleTool {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Freehand("none; the tool does nothing")
    }
}

/// Every probe linked into this binary, the integration binary's own fixtures included.
fn probes() -> Vec<Probe> {
    craft_ui::contract::registrations().into_iter().filter_map(|registration| registration.probe().copied()).collect()
}

fn named(name: &str) -> Vec<Probe> {
    probes().into_iter().filter(|probe| probe.type_name().contains(name)).collect()
}

#[trial]
fn a_registration_records_the_facts_the_contract_types_state() {
    let tagged = named("::Tagged<");
    assert_eq!(tagged.len(), 2, "one probe per listed instantiation");
    assert!(tagged.iter().any(|probe| probe.type_name().ends_with("Plain>")) && tagged.iter().any(|probe| probe.type_name().ends_with("Number>")));
    for probe in &tagged {
        assert_eq!((probe.shape, probe.role, probe.crate_name), (ShapeKind::Leaf, Role::SLIDER, "craft-ui"));
        assert!(probe.carries_value() && !probe.overlay && !probe.canvas_control && probe.exempt.is_empty());
    }

    let handle = named("derives::Handle");
    assert_eq!(handle.len(), 1);
    assert!(handle[0].canvas_control && !handle[0].carries_value());
    assert_eq!((handle[0].exempt[0].case, handle[0].exempt[0].reason), ("sizes", "fills the canvas"));

    assert_eq!(named("::Panel<")[0].shape, ShapeKind::Body);
    assert_eq!(named("kit::Table")[0].shape, ShapeKind::Items);
    assert!(named("::Hooked<")[0].overlay, "`#[component(container, overlay)]` declares the open state");
}

#[trial]
fn register_contract_registers_a_hand_written_component() {
    let manual = named("::Manual");
    assert_eq!(manual.len(), 1);
    assert!(manual[0].overlay);
    assert_eq!(manual[0].exempt[0].case, "sizes");
    let mut frames = Frames::new();
    frames.frame(vec![], |ui| {
        Manual { common: Common::new() }.show(ui);
    });
}

#[trial]
fn each_instantiation_of_a_generic_component_has_its_own_style_key() {
    let plain = Tagged::<Plain> { common: Common::new(), kind: 3, codec: PhantomData };
    let number = Tagged::<Number> { common: Common::new(), kind: 3, codec: PhantomData };
    assert_ne!(TypeId::of::<<Tagged<Plain> as Meta>::StyleKey>(), TypeId::of::<<Tagged<Number> as Meta>::StyleKey>());
    assert_eq!(plain.style_key(), Tagged::<Plain> { common: Common::new(), kind: 3, codec: PhantomData }.style_key());
    assert_ne!(
        plain.style_key(),
        Tagged::<Plain> { common: Common::new(), kind: 4, codec: PhantomData }.style_key(),
        "a `#[style_key]` field is part of the key"
    );
    let _ = number;
}
