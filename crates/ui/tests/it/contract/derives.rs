//! What the derives write: registrations and probes, style keys per instantiation, setters and style accessors.

use super::kit::PlainStyle;
use craft_behavior::Metrics;
use craft_theme::{Contrast, ControlSize, Mode, Rgba, Tokens, Variant};
use craft_ui::contract::{
    Common, Component, Cx, EditResponse, Internal, MeasureCx, Meta, Props, Role, ShapeKind, Show as _, register_contract, shape,
};
use craft_ui::style::{ComponentStyle, FocusLook, StyleCx, StyleKey, SurfaceLook, TokenGroup};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, Probe, registered, trial};
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

fn probes() -> Vec<Probe> {
    registered().iter().filter_map(|registration| registration.probe().copied()).collect()
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

    let handle = named("::Handle");
    assert_eq!(handle.len(), 1);
    assert!(handle[0].canvas_control && !handle[0].carries_value());
    assert_eq!((handle[0].exempt[0].case, handle[0].exempt[0].reason), ("sizes", "fills the canvas"));

    assert_eq!(named("::Panel<")[0].shape, ShapeKind::Body);
    assert_eq!(named("::Table")[0].shape, ShapeKind::Items);
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
fn a_type_declared_in_the_library_is_registered_beside_the_binarys_own() {
    // `Block` is declared in craft-ui's library, `Tagged` in this binary; both carry the package name `craft-ui`.
    assert!(named("::Block").len() == 1, "{:?}", probes().iter().map(|p| p.type_name()).collect::<Vec<_>>());
    assert!(craft_ui::testing::registered_in("no-such-crate").is_empty());
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

/// Setters of each field form.
#[derive(Props)]
struct Fancy {
    /// An optional count.
    #[prop]
    count: Option<u8>,
    /// A title.
    #[prop]
    title: String,
    /// A subtitle converted from a `&str`.
    #[prop(into)]
    sub: Option<String>,
    /// Busy.
    #[prop(flag)]
    busy: bool,
    /// A plain boolean under another name.
    #[prop(name = "wide")]
    stretch: bool,
}

#[trial]
fn derived_setters_follow_the_field_type() {
    let fancy = Fancy { count: None, title: String::new(), sub: None, busy: false, stretch: false }.count(4).title("t").sub("s").busy().wide(true);
    assert_eq!((fancy.count, fancy.title.as_str(), fancy.sub.as_deref(), fancy.busy, fancy.stretch), (Some(4), "t", Some("s"), true, true));
}

/// A style with every part the derive reads.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[style(variants = [Variant::GHOST, Variant::PRIMARY])]
struct Parts {
    #[style(surface)]
    surface: Option<SurfaceLook>,
    #[style(focus)]
    focus: FocusLook,
    #[style(metrics)]
    metrics: Metrics,
}

#[trial]
fn a_derived_style_exposes_its_marked_parts() {
    let style = Parts { surface: None, focus: FocusLook::new(egui::Stroke::new(2.0, egui::Color32::RED), 1.0), metrics: Metrics::new().gap(3.0) };
    assert!(style.surface().is_none(), "an absent optional surface is no surface");
    assert_eq!(style.focus(), Some(&style.focus));
    assert_eq!(style.metrics().gap, 3.0);
    assert_eq!(<Parts as ComponentStyle>::VARIANTS, [Variant::GHOST, Variant::PRIMARY]);
}

#[derive(Clone, TokenGroup)]
#[group(path = "craft_ui_tests.same")]
struct First {
    colour: Rgba,
}

impl First {
    fn derive(_: &Tokens) -> Self {
        First { colour: Rgba::rgb(1, 2, 3) }
    }
}

#[derive(Clone, TokenGroup)]
#[group(path = "craft_ui_tests.same")]
struct Second;

impl Second {
    fn derive(_: &Tokens) -> Self {
        Second
    }
}

#[trial]
fn two_token_groups_with_one_path_are_a_debug_assertion() {
    let frames = Frames::new();
    let set = tokens(Mode::Light, Contrast::Standard);
    let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_cx(&frames.ctx, &set, Variant::SECONDARY, ControlSize::Md, |cx| {
            cx.token_group::<First>();
            cx.token_group::<Second>();
        });
    }));
    assert_eq!(read.is_err(), cfg!(debug_assertions));
}
