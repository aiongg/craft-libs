//! What the derives write: registrations and probes, style keys per instantiation, setters and style accessors.

use super::kit::PlainStyle;
use craft_behavior::CancelReason;
use craft_theme::{Contrast, ControlSize, Mode, Rgba, Tokens, Variant};
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolPath, ToolPointer};
use craft_ui::contract::PatternKind;
use craft_ui::contract::{
    Common, Component, Cx, EditResponse, Internal, MeasureCx, Meta, Pattern, Role, ShapeKind, Show as _, register_contract, shape,
};
use craft_ui::primitives::Draw;
use craft_ui::style::{StyleCx, StyleKey, TokenGroup};
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
    craft_ui::contract::REGISTRATIONS.iter().filter_map(|registration| registration.probe().copied()).collect()
}

fn named(name: &str) -> Vec<Probe> {
    probes().into_iter().filter(|probe| probe.type_name().contains(name)).collect()
}

#[trial]
fn a_registration_records_the_facts_the_contract_types_state() {
    {
        let tagged = named("::Tagged<");
        assert_eq!(tagged.len(), 2, "one probe per listed instantiation");
        assert!(
            tagged.iter().any(|probe| probe.type_name().ends_with("Plain>")) && tagged.iter().any(|probe| probe.type_name().ends_with("Number>"))
        );
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
    {
        let manual = named("::Manual");
        assert_eq!(manual.len(), 1);
        assert!(manual[0].overlay);
        assert_eq!(manual[0].exempt[0].case, "sizes");
        let mut frames = Frames::new();
        frames.frame(vec![], |ui| {
            Manual { common: Common::new() }.show(ui);
        });
    }
}

#[trial]
fn the_package_list_holds_the_library_types_and_not_the_integration_binarys_fixtures() {
    let listed: Vec<_> = registered().iter().filter_map(|registration| registration.probe()).map(|probe| probe.type_name()).collect();
    assert!(listed.iter().any(|name| name.ends_with("::Block")), "{listed:?}");
    assert!(!listed.iter().any(|name| name.contains("Tagged")), "a fixture of the `it` binary is not craft-ui's: {listed:?}");
    assert_eq!(named("::Tagged<")[0].crate_name, "craft-ui");
    assert_eq!(named("::Tagged<")[0].crate_target, "it");
    assert!(craft_ui::testing::registered_in("no-such-crate").is_empty());
}

static ROWS: std::sync::Mutex<Vec<(usize, egui::Rect)>> = std::sync::Mutex::new(Vec::new());

#[trial]
fn each_shape_has_an_entry_that_shows_an_instance() {
    use super::kit::{Panel, RowProbe, Rows, Table};
    use craft_behavior::ItemSource;
    use craft_ui::contract::{ItemFn, ShowEntry};
    let mut frames = Frames::new();
    let room = egui::Rect::from_min_size(egui::Pos2::new(10.0, 10.0), Vec2::new(100.0, 60.0));
    let mut shown = Vec::new();
    frames.frame(vec![], |ui| {
        for probe in probes() {
            match (probe.type_name(), probe.entry) {
                (name, ShowEntry::Leaf(entry)) if name.ends_with("::Manual") => {
                    shown.push((name, entry(ui, room, Box::new(Manual { common: Common::new() })).is_some()));
                    shown.push(("wrong box", entry(ui, room, Box::new(0u8)).is_none()));
                }
                (name, ShowEntry::Container(entry)) if name.contains("kit::Panel<") => {
                    let mut body_ran = false;
                    let response = entry(ui, room, Box::new(Panel::new()), &mut |_, _| body_ran = true);
                    shown.push((name, response.is_some() && body_ran));
                }
                (name, ShowEntry::Collection(entry)) if name.ends_with("kit::Table") => {
                    let source: Box<dyn ItemSource<usize>> = Box::new(Rows(3));
                    let mut item: Box<ItemFn<'static, Table>> = Box::new(|key, _| RowProbe::new(*key, &ROWS));
                    shown.push((name, entry(ui, Box::new(Table::new(20.0)), &source, &mut item).is_some()));
                }
                _ => {}
            }
        }
    });
    assert!(shown.iter().all(|(_, ok)| *ok), "{shown:?}");
    // A frame runs its closure once per pass.
    shown.sort_unstable();
    shown.dedup();
    assert_eq!(shown.len(), 4, "{shown:?}");
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

#[trial]
fn a_pattern_registers_with_its_kind_and_no_component_probe() {
    {
        let _ = (ToolRow, Handwritten);
        let patterns: Vec<_> = craft_ui::contract::REGISTRATIONS.iter().filter_map(|registration| registration.pattern_probe().copied()).collect();
        let row = patterns.iter().find(|pattern| pattern.type_name().ends_with("ToolRow")).expect("the derive registers the pattern");
        assert_eq!(row.purpose, "A row of tools: a pattern assembles components and has no contract shape of its own");
        assert!(patterns.iter().any(|pattern| pattern.type_name().ends_with("Handwritten")), "`register_contract!(Type; pattern)` registers too");
        assert!(named("ToolRow").is_empty() && named("Handwritten").is_empty(), "a pattern has no component probe");
    }
    {
        let _ = IdleTool;
        let kind_of = |name: &str| {
            craft_ui::contract::REGISTRATIONS
                .iter()
                .filter_map(|registration| registration.pattern_probe())
                .find(|pattern| pattern.type_name().ends_with(name))
                .map(|pattern| pattern.kind)
        };
        assert_eq!(kind_of("IdleTool"), Some(PatternKind::Tool));
        assert_eq!(kind_of("RecoveryDialog"), Some(PatternKind::Dialog));
        assert_eq!(kind_of("ToolRow"), Some(PatternKind::Composition));
    }
}
