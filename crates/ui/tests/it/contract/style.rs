//! The style step and what the pipeline keeps of a component's inputs: app replacements, style-affecting app
//! values, faces, the shown value and memory eviction.

use super::kit::{PlainStyle, Probe, Seen};
use craft_behavior::{Metrics, PartKey, Sense};
use craft_ui::buttons::ToggleButton;
use craft_ui::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Looks, MeasureCx, PaintUi, PartsCx, Props as _, Remember as _, Role, Show as _,
};
use craft_ui::primitives::PaintShape;
use craft_ui::runtime::env::{EnvValue, RootEnv};
use craft_ui::runtime::install;
use craft_ui::style::{ComponentStyle, Ink, StyleColor, StyleCx};
use craft_ui::testing::{Frames, trial};
use egui::{Context, CornerRadius, Rect, Response, Vec2};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// A leaf whose style reads a style-affecting app value.
#[derive(Component)]
struct Tinted {
    #[common]
    common: Common,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnvValue)]
#[env(style, root = Accent(1))]
struct Accent(u8);

impl Component for Tinted {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: TintedStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(f32::from(cx.env::<Accent>().0) * 10.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn width_of(frames: &mut Frames, root: RootEnv) -> f32 {
    frames.root(root);
    let mut width = 0.0;
    frames.frame(vec![], |ui| width = Tinted { common: Common::new() }.show(ui).rect.width());
    width
}

/// The style cache key includes the hash of the style-affecting app values, so a changed value rebuilds the style.
#[trial]
fn a_style_affecting_app_value_rebuilds_the_cached_style() {
    let mut frames = Frames::new();
    let mut root = RootEnv::new();
    root.values.set(Accent(3));
    let first = width_of(&mut frames, root.clone());
    root.values.set(Accent(5));
    let second = width_of(&mut frames, root);
    assert!(second > first, "the style followed the app value: {first} -> {second}");
}

/// An app replacement of a style function applies to every instance, and two for one key are an install error.
#[trial]
fn looks_replace_a_components_style_app_wide() {
    let size = |frames: &mut Frames| {
        let seen = RefCell::new(Seen::default());
        let mut width = 0.0;
        frames.frame(vec![], |ui| width = Probe::new(&seen).size(Vec2::new(10.0, 10.0)).show(ui).rect.width());
        width
    };
    let mut plain = Frames::uninstalled();
    assert!(install(&plain.ctx, craft_ui::testing::install_spec()).is_ok());
    let mut replaced = Frames::uninstalled();
    let mut looks = Looks::new();
    looks.style::<Probe<'static>>(|_, _| PlainStyle::min(Vec2::new(80.0, 10.0)));
    assert!(install(&replaced.ctx, craft_ui::testing::install_spec().looks(looks)).is_ok());
    assert!(size(&mut replaced) > size(&mut plain), "the replacement's minimum size applies");

    let mut twice = Looks::new();
    twice.style::<Probe<'static>>(|_, _| PlainStyle::default()).style::<Probe<'static>>(|_, _| PlainStyle::default());
    let ctx = Frames::uninstalled().ctx;
    assert!(install(&ctx, craft_ui::testing::install_spec().looks(twice)).is_err(), "two replacements for one style key are refused");
}

static FACE_UPDATES: AtomicU32 = AtomicU32::new(0);

/// A face: it paints a filled rect and counts the hooks a face must not run.
#[derive(Component)]
struct Dot {
    #[common]
    common: Common,
}

#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct DotStyle {
    color: StyleColor,
    #[style(metrics)]
    metrics: Metrics,
}

impl Component for Dot {
    const ROLE: Role = Role::None;
    type Style = DotStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: DotStyleKey) -> DotStyle {
        DotStyle { color: cx.ink(Ink::OnDark), metrics: Metrics::new() }
    }

    fn update(&mut self, _: &mut Cx<'_, Self>) {
        FACE_UPDATES.fetch_add(1, Ordering::SeqCst);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().fill(&PaintShape::Rect(cx.rect, CornerRadius::ZERO), cx.style.color);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

const FACE: PartKey = PartKey::new("face");

/// A composite that paints one part as a `Dot`.
#[derive(Component)]
struct Host {
    #[common]
    common: Common,
}

impl Component for Host {
    const ROLE: Role = Role::None;
    type Style = DotStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: HostStyleKey) -> DotStyle {
        DotStyle { color: cx.ink(Ink::OnPanel), metrics: Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(60.0, 30.0)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, Rect::from_min_size(rect.min + Vec2::new(5.0, 5.0), Vec2::splat(10.0))).sense(Sense::HOVER);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, _: &Cx<'_, Self>) {
        ui.face(FACE, Dot { common: Common::new() });
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A face paints at its part's rect with its own style and runs no `update`, id or memory of its own.
#[trial]
fn a_face_paints_its_part_and_runs_only_the_painting_steps() {
    let mut frames = Frames::new();
    let mut host = Rect::NOTHING;
    let mut shapes = Vec::new();
    for _ in 0..2 {
        shapes = frames.frame_shapes(|ui| host = Host { common: Common::new() }.show(ui).rect);
    }
    let part = Rect::from_min_size(host.min + Vec2::new(5.0, 5.0), Vec2::splat(10.0));
    let painted = shapes.iter().any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.rect == part && rect.fill != egui::Color32::TRANSPARENT));
    assert!(painted, "the face filled its part's rect: {shapes:?}");
    assert_eq!(FACE_UPDATES.load(Ordering::SeqCst), 0, "a face runs no `update`");
}

static CLONES: AtomicU32 = AtomicU32::new(0);

/// A value that counts its clones.
#[derive(Debug, PartialEq)]
struct Counted(u8);

impl Clone for Counted {
    fn clone(&self) -> Self {
        CLONES.fetch_add(1, Ordering::SeqCst);
        Counted(self.0)
    }
}

/// A leaf with a value binding.
#[derive(Component)]
struct Holder<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Counted>,
}

impl Component for Holder<'_> {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = EditResponse<Counted>;

    fn style(_: &StyleCx<'_>, _: HolderStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(10.0))
    }

    fn bind(&mut self) -> Option<BindMut<'_, Counted>> {
        Some(self.value.reborrow())
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<Counted> {
        cx.edit_response()
    }
}

/// Showing and measuring a component reads its binding through a held copy: the value is cloned only when it
/// changed.
#[trial]
fn showing_and_measuring_clone_the_bound_value_only_when_it_changed() {
    let mut frames = Frames::new();
    let mut value = Counted(1);
    let before = CLONES.load(Ordering::SeqCst);
    for _ in 0..3 {
        frames.frame(vec![], |ui| {
            Holder { common: Common::new(), value: Bind::bound(&mut value) }.id_salt("holder").show(ui);
        });
    }
    assert_eq!(CLONES.load(Ordering::SeqCst) - before, 1, "three shows of one value cloned it once");
    frames.frame(vec![], |ui| {
        let mut holder = Holder { common: Common::new(), value: Bind::bound(&mut value) }.id_salt("holder");
        holder.measure_in(ui, craft_behavior::Proposal::Ideal);
    });
    assert_eq!(CLONES.load(Ordering::SeqCst) - before, 1, "measuring an unchanged value clones nothing");
    value.0 = 2;
    frames.frame(vec![], |ui| {
        Holder { common: Common::new(), value: Bind::bound(&mut value) }.id_salt("holder").show(ui);
    });
    assert_eq!(CLONES.load(Ordering::SeqCst) - before, 2, "a changed value is cloned once");
}

static DROPPED: AtomicU32 = AtomicU32::new(0);
static CALLED_CONTEXT: AtomicU32 = AtomicU32::new(0);
static CTX: Mutex<Option<Context>> = Mutex::new(None);

/// A slot whose `Drop` calls a `Context` API, as a slot holding a texture handle does.
#[derive(Clone, Default)]
struct Tracker(Option<Arc<()>>);

impl Drop for Tracker {
    fn drop(&mut self) {
        let Some(token) = self.0.take() else { return };
        if Arc::strong_count(&token) > 1 {
            return;
        }
        DROPPED.fetch_add(1, Ordering::SeqCst);
        if let Ok(guard) = CTX.lock()
            && let Some(ctx) = guard.as_ref()
        {
            let _ = ctx.pixels_per_point();
            CALLED_CONTEXT.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// A leaf that keeps a slot in shown memory.
#[derive(Component)]
struct Keeper {
    #[common]
    common: Common,
}

impl Component for Keeper {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: KeeperStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::splat(10.0))
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let tracker: &mut Tracker = cx.memory();
        tracker.0.get_or_insert_with(|| Arc::new(()));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A composite that measures as the face of a toggle button.
#[derive(Component)]
struct FaceMeasure {
    #[common]
    common: Common,
}

impl Component for FaceMeasure {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FaceMeasureStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.face(ToggleButton::new("Snap to grid", Bind::given(false)))
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// `MeasureCx::face` of a component is that component's own measure.
#[trial]
fn a_measured_face_is_the_size_of_the_component_shown_alone() {
    let mut frames = Frames::new();
    let (mut face, mut alone) = (Vec2::ZERO, Vec2::ZERO);
    for _ in 0..2 {
        frames.frame(vec![], |ui| {
            face = FaceMeasure { common: Common::new() }.show(ui).rect.size();
            alone = ToggleButton::new("Snap to grid", Bind::given(false)).show(ui).response.rect.size();
        });
    }
    assert_eq!(face, alone);
}
