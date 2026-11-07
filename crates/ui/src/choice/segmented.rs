//! `Segmented`: one of a few options as adjoining pills. A leaf with a part per segment; each segment wears the face
//! of a toggle button. The row is one Tab stop over its segments: arrows, Home and End move focus between them, Space
//! or a click picks the focused one.

use crate::buttons::ToggleButton;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, Input, MeasureCx, Name, PaintUi, PartsCx, Pointer as _,
    Props as _, Role, SetPosition,
};
use crate::primitives::TextRun;
use crate::runtime::env::Label;
use crate::runtime::focus::{ActivateKeys, KeyUse};
use crate::style::{ComponentStyle, Flags, FocusLook, StyleCx};
use craft_behavior::{Flow, IconId, Metrics, NavShape, PartKey, PartRef, Roving, Sense, Value};
use egui::{Rect, Vec2};

const SEGMENT: PartKey = PartKey::new("segment");

/// One option of a `Segmented`: the value it stands for, its label and an optional icon.
#[derive(Clone, Debug, crate::contract::Props)]
#[non_exhaustive]
pub struct Segment<T> {
    value: T,
    label: Label,
    /// An icon shown in place of the label's text; the label stays the segment's name and tooltip.
    #[prop]
    icon: Option<IconId>,
    /// Whether the segment can be picked.
    #[prop]
    enabled: bool,
}

impl<T> Segment<T> {
    /// A segment standing for `value`, named `label`.
    pub fn new(value: T, label: impl Into<Label>) -> Self {
        Segment { value, label: label.into(), icon: None, enabled: true }
    }
}

/// The style of a segmented row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SegmentedStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// Between a segment's edge and its content.
    pad_x: f32,
    /// Between neighbouring segments.
    gap: f32,
    icon: f32,
    height: f32,
}

/// A row of segments over one value. A `Mixed` binding picks none.
#[derive(crate::contract::Component)]
#[component(probe(Segmented<'static, u8>))]
pub struct Segmented<'a, T: Value + 'static> {
    #[common]
    common: Common,
    value: Bind<'a, T>,
    segments: &'a [Segment<T>],
}

impl<'a, T: Value + 'static> Segmented<'a, T> {
    /// A row of `segments` over `value`.
    pub fn new(value: Bind<'a, T>, segments: &'a [Segment<T>]) -> Self {
        Segmented { common: Common::new(), value, segments }
    }

    fn part(at: usize) -> PartRef {
        SEGMENT.at(at as u64)
    }

    fn position_of(&self, value: Option<&T>) -> Option<usize> {
        let value = value?;
        self.segments.iter().position(|segment| segment.value == *value)
    }

    /// Each segment's width: its text and padding, or a square for an icon.
    fn widths(&self, cx: &MeasureCx<'_, Self>) -> Vec<f32> {
        let style = cx.style;
        self.segments
            .iter()
            .map(|segment| match segment.icon {
                Some(_) => (style.icon + style.pad_x * 2.0).max(style.height),
                None => cx.layout_at(&TextRun::new(segment.label.as_str()), f32::INFINITY).galley.size().x + style.pad_x * 2.0,
            })
            .collect()
    }
}

impl<T: Value + 'static> Component for Segmented<'_, T> {
    const ROLE: Role = Role::RADIO_GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse::of(Input::Pointer).with_activate(ActivateKeys::Space);
    type Style = SegmentedStyle;
    type Output = EditResponse<T>;

    fn style(cx: &StyleCx<'_>, _: SegmentedStyleKey<T>) -> SegmentedStyle {
        let control = cx.control();
        SegmentedStyle {
            metrics: Metrics::new().min_size(Vec2::new(0.0, control.height)),
            focus: crate::controls::focus_look(cx),
            pad_x: control.pad_x,
            gap: cx.tokens.space.gap.tight,
            icon: control.icon,
            height: control.height,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let widths = self.widths(cx);
        let gaps = widths.len().saturating_sub(1) as f32;
        Vec2::new(widths.iter().sum::<f32>() + cx.style.gap * gaps, cx.style.height)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let widths = self.widths(cx);
        let gap = cx.style.gap;
        let current = self.position_of(cx.value());
        let mut flow = Flow::new(rect, cx.flow.is_rtl());
        for (at, (segment, width)) in self.segments.iter().zip(widths).enumerate() {
            let (cell, rest) = flow.take_start(width);
            flow = rest.take_start(gap).1;
            let part = cx.part(Self::part(at), cell);
            if !segment.enabled {
                continue;
            }
            let mut part = part.sense(Sense::CLICK | Sense::FOCUS).focusable();
            if segment.icon.is_some() {
                part = part.tooltip(segment.label.clone());
            }
            if current == Some(at) {
                part.flags(Flags::CHECKED).selected();
            }
        }
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn bind(&mut self) -> Option<BindMut<'_, T>> {
        Some(self.value.reborrow())
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let current = self.position_of(cx.value());
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        for (at, segment) in self.segments.iter().enumerate() {
            let hit = cx.part(Self::part(at)).press.phase.is_click() || by_key == Some(Self::part(at));
            if segment.enabled && hit && current != Some(at) {
                cx.set(segment.value.clone());
            }
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        for (at, segment) in self.segments.iter().enumerate() {
            let node = access.node(Self::part(at), Role::RADIO_BUTTON);
            node.name = Name::Text(segment.label.as_str().to_owned());
            node.position = Some(SetPosition::new(at).of(self.segments.len()));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let current = self.position_of(cx.value());
        for (at, segment) in self.segments.iter().enumerate() {
            let mut face = ToggleButton::new(segment.label.clone(), Bind::given(current == Some(at))).enabled(segment.enabled);
            if let Some(icon) = &segment.icon {
                face = face.icon(icon.clone()).icon_only();
            }
            ui.face(Self::part(at), face);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<T> {
        cx.edit_response()
    }
}
