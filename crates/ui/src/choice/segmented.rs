//! `Segmented`: one of a few options as adjoining pills. A leaf with a part per segment; each segment wears the face
//! of a toggle button. The row is one Tab stop over its segments: arrows, Home and End move focus between them, Space
//! or a click picks the focused one.

use crate::buttons::ToggleButton;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Entry, Focus as _, Input, MeasureCx, Name, PaintUi, PartsCx, Pointer as _,
    Props as _, Role, SetPosition,
};
use crate::runtime::env::Label;
use crate::runtime::focus::{ActivateKeys, KeyUse};
use crate::style::{ComponentStyle, Flags, FocusLook, StyleCx};
use craft_behavior::{Flow, Metrics, NavShape, PartKey, PartRef, Roving, Sense, Value};
use egui::{Rect, Vec2};

const SEGMENT: PartKey = PartKey::new("segment");

/// The style of a segmented row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SegmentedStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// Between neighbouring segments.
    gap: f32,
    height: f32,
}

/// A row of segments over one value. A `Mixed` binding picks none.
#[derive(crate::contract::Component)]
#[component(probe(Segmented<'static, u8>))]
pub struct Segmented<'a, T: Value + 'static> {
    #[common]
    common: Common,
    value: Bind<'a, T>,
    entries: &'a [Entry<T>],
}

impl<'a, T: Value + 'static> Segmented<'a, T> {
    /// A row over `value` named `label`, one segment per entry. An entry's label is its text, or its tooltip and name
    /// when it has an icon; separators are not segments.
    pub fn new(label: impl Into<Label>, value: Bind<'a, T>, entries: &'a [Entry<T>]) -> Self {
        debug_assert!(entries.iter().all(|entry| !entry.is_separator()), "a `Segmented` has no separators");
        Segmented { common: Common::new(), value, entries }.label(label)
    }

    fn part(at: usize) -> PartRef {
        SEGMENT.at(at as u64)
    }

    fn position_of(&self, value: Option<&T>) -> Option<usize> {
        let value = value?;
        self.entries.iter().position(|entry| entry.value() == Some(value))
    }

    /// The face segment `entry` wears: a toggle button, icon only when the entry has an icon.
    fn face(entry: &Entry<T>, checked: bool) -> ToggleButton<'static> {
        let face = ToggleButton::new(entry.label().clone(), Bind::given(checked)).enabled(entry.enabled());
        match entry.icon() {
            Some(icon) => face.icon(icon.clone()).icon_only(),
            None => face,
        }
    }

    /// Each segment's width: its face's own measure.
    fn widths(&self, cx: &MeasureCx<'_, Self>) -> Vec<f32> {
        self.entries.iter().map(|entry| cx.face(Self::face(entry, false)).x).collect()
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
            focus: cx.focus_look(),
            gap: cx.tokens.space.gap.tight,
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
        for (at, (entry, width)) in self.entries.iter().zip(widths).enumerate() {
            let (cell, rest) = flow.take_start(width);
            flow = rest.take_start(gap).1;
            let mut part = cx.part(Self::part(at), cell);
            if entry.icon().is_some() {
                part = part.tooltip(entry.label().clone());
            }
            if !entry.enabled() {
                continue;
            }
            let part = part.sense(Sense::CLICK | Sense::FOCUS).focusable();
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
        for (at, entry) in self.entries.iter().enumerate() {
            let hit = cx.part(Self::part(at)).press.phase.is_click() || by_key == Some(Self::part(at));
            if let Some(value) = entry.value().filter(|_| entry.enabled() && hit && current != Some(at)) {
                cx.set(value.clone());
            }
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        for (at, entry) in self.entries.iter().enumerate() {
            let node = access.node(Self::part(at), Role::RADIO_BUTTON);
            node.name = Name::Text(cx.env.resolve(entry.label()).as_str().to_owned());
            node.position = Some(SetPosition::new(at).of(self.entries.len()));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let current = self.position_of(cx.value());
        for (at, entry) in self.entries.iter().enumerate() {
            ui.face(Self::part(at), Self::face(entry, current == Some(at)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<T> {
        cx.edit_response()
    }
}

#[cfg(feature = "testing")]
mod stories;
