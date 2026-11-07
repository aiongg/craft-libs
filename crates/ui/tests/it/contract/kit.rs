//! Hand-written components for the contract trials: a leaf, a container and a collection with no derives.

use craft_behavior::{
    Intent, ItemSource, Metrics, PartKey, PartRef, PressPhase, Selection, Sense, SpanLayout,
};
use craft_ui::contract::{
    Collection, CollectionProps, Common, Component, Container, ContainerProps, Cx, Focus as _, HasProps, Internal, MeasureCx, Meta, PartsCx,
    Pointer as _, Role, shape,
};
use craft_ui::style::{ComponentStyle, StyleCx, StyleKey};
use egui::{Id, Rect, Response, Vec2};
use std::cell::RefCell;

macro_rules! style_key {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name;

        impl StyleKey for $name {}
    };
}

style_key!(ProbeKey);
style_key!(PanelKey);
style_key!(TableKey);
style_key!(RowProbeKey);

/// A style with a minimum size.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlainStyle {
    pub min: Vec2,
}

impl ComponentStyle for PlainStyle {
    fn metrics(&self) -> Metrics {
        Metrics::new().min_size(self.min)
    }
}

/// A part a probe declares: its rect relative to the probe's own, what it senses and whether Tab reaches it.
#[derive(Clone, Copy)]
pub struct PartSpec {
    pub key: PartKey,
    pub at: Rect,
    pub sense: Sense,
    pub focusable: bool,
}

/// What a probe saw while it ran.
pub struct Seen {
    pub rect: Rect,
    pub id: Option<Id>,
    /// The press phases of the probe's own shapes, one per pass that had a press.
    pub press: Vec<PressPhase>,
    pub parts: Vec<(PartKey, PressPhase)>,
    pub activations: u32,
    pub cancels: u32,
    pub active: Option<PartRef>,
    pub updates: u32,
}

/// Records what the pipeline gave the component in `update`.
fn record<C: Component>(seen: &RefCell<Seen>, parts: &[PartSpec], cx: &mut Cx<'_, C>) {
    let mut seen = seen.borrow_mut();
    seen.updates += 1;
    seen.rect = cx.rect;
    seen.id = Some(cx.id);
    let phase = cx.press().phase;
    if phase.is_press() {
        seen.press.push(phase);
    }
    for spec in parts {
        let phase = cx.part(spec.key).press.phase;
        if phase.is_press() {
            seen.parts.push((spec.key, phase));
        }
    }
    seen.activations += u32::from(cx.activation().is_some());
    seen.cancels += u32::from(cx.intents().take(Intent::Cancel));
    seen.active = cx.active_part();
}

impl Default for Seen {
    fn default() -> Self {
        Seen {
            rect: Rect::NOTHING,
            id: None,
            press: Vec::new(),
            parts: Vec::new(),
            activations: 0,
            cancels: 0,
            active: None,
            updates: 0,
        }
    }
}

/// A leaf of a given size that records what the pipeline gives it.
pub struct Probe<'a> {
    common: Common,
    size: Vec2,
    sense: Option<Sense>,
    parts: &'a [PartSpec],
    seen: &'a RefCell<Seen>,
}

impl<'a> Probe<'a> {
    pub fn new(seen: &'a RefCell<Seen>) -> Self {
        Probe { common: Common::new(), size: Vec2::new(40.0, 20.0), sense: None, parts: &[], seen }
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }

    pub fn parts(mut self, parts: &'a [PartSpec]) -> Self {
        self.parts = parts;
        self
    }
}

impl Meta for Probe<'_> {
    type Shape = shape::Leaf;
    type StyleKey = ProbeKey;

    fn style_key(&self) -> ProbeKey {
        ProbeKey
    }

    fn common(&self, _: Internal) -> &Common {
        &self.common
    }

    fn common_mut(&mut self, _: Internal) -> &mut Common {
        &mut self.common
    }
}

impl Component for Probe<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: ProbeKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        self.size
    }

    fn sense(&self) -> Sense {
        self.sense.unwrap_or(Sense::CLICK | Sense::FOCUS)
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        for spec in self.parts {
            let at = spec.at.translate(rect.min.to_vec2());
            let part = cx.part(spec.key, at).sense(spec.sense);
            if spec.focusable {
                part.focusable();
            }
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        record(self.seen, self.parts, cx);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A container whose body is the caller's closure.
pub struct Panel<'a> {
    common: Common,
    props: ContainerProps,
    sense: Sense,
    seen: Option<&'a RefCell<Seen>>,
}

impl<'a> Panel<'a> {
    pub fn new() -> Self {
        Panel { common: Common::new(), props: ContainerProps::new(), sense: Sense::empty(), seen: None }
    }

    pub fn seen(mut self, seen: &'a RefCell<Seen>) -> Self {
        self.seen = Some(seen);
        self
    }

    pub fn arena(mut self, on: bool) -> Self {
        self.props = self.props.arena(on);
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl Meta for Panel<'_> {
    type Shape = shape::Body;
    type StyleKey = PanelKey;

    fn style_key(&self) -> PanelKey {
        PanelKey
    }

    fn common(&self, _: Internal) -> &Common {
        &self.common
    }

    fn common_mut(&mut self, _: Internal) -> &mut Common {
        &mut self.common
    }
}

impl HasProps for Panel<'_> {
    fn props(&self, _: Internal) -> &ContainerProps {
        &self.props
    }
}

impl Component for Panel<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: PanelKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn sense(&self) -> Sense {
        self.sense
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(seen) = self.seen {
            record(seen, &[], cx);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Panel<'_> {}

/// A list of `rows` rows of fixed height.
pub struct Rows(pub usize);

impl ItemSource<usize> for Rows {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

/// A collection of rows of fixed height.
pub struct Table {
    common: Common,
    props: ContainerProps,
    items: CollectionProps,
    row_h: f32,
}

impl Table {
    pub fn new(row_h: f32) -> Self {
        Table { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new(), row_h }
    }
}

impl Meta for Table {
    type Shape = shape::Items;
    type StyleKey = TableKey;

    fn style_key(&self) -> TableKey {
        TableKey
    }

    fn common(&self, _: Internal) -> &Common {
        &self.common
    }

    fn common_mut(&mut self, _: Internal) -> &mut Common {
        &mut self.common
    }
}

impl HasProps for Table {
    fn props(&self, _: Internal) -> &ContainerProps {
        &self.props
    }
}

impl Component for Table {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TableKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Collection for Table {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = RowProbe;
    type ItemEvent = ();

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(self.row_h, source.len(), width)
    }
}

/// One row of a `Table`: it records its key and the arena it joined.
pub struct RowProbe {
    common: Common,
    pub log: &'static std::sync::Mutex<Vec<(usize, Rect)>>,
    key: usize,
}

impl RowProbe {
    pub fn new(key: usize, log: &'static std::sync::Mutex<Vec<(usize, Rect)>>) -> Self {
        RowProbe { common: Common::new(), log, key }
    }
}

impl Meta for RowProbe {
    type Shape = shape::Leaf;
    type StyleKey = RowProbeKey;

    fn style_key(&self) -> RowProbeKey {
        RowProbeKey
    }

    fn common(&self, _: Internal) -> &Common {
        &self.common
    }

    fn common_mut(&mut self, _: Internal) -> &mut Common {
        &mut self.common
    }
}

impl Component for RowProbe {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: RowProbeKey) -> PlainStyle {
        PlainStyle { min: Vec2::new(10.0, 10.0) }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Ok(mut log) = self.log.lock() {
            log.push((self.key, cx.rect));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
