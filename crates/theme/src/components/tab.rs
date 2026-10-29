//! Tabs: one tab of a document or panel strip. Sizes come from the Size preset; the colours are
//! the surface, state and text tokens, so the active tab is the panel colour and joins the panel
//! or canvas below it.
//!
//! Labels and details are text on the strip, on the active tab and on a hovered tab (4.5:1, 7:1
//! with high contrast); an inactive label is measured on the strip it rests on. The × is the
//! control's only mark, and the drop marker the only cue for the drop slot, so each is measured as
//! a non-text mark (3:1, 4.5:1): the × on the strip and on the active tab, the marker on its halo.
//! The fills themselves, the active tab against the strip among them, are design targets with no
//! minimum, as row selection fills are.

use super::{ComponentField, ContrastPair, Family, FieldKind, FloatField, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::derive::{mix, over};
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// Tab tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TabTokens {
    /// Height of the strip: `size.bar.tab`.
    pub height: f32,
    /// Height of the strip with Large Tabs.
    pub height_large: f32,
    /// Between a tab's edge and its contents: twice `space.pad.control`.
    pub pad_x: f32,
    /// Between the parts of a tab: `space.gap.inline`.
    pub gap: f32,
    /// Least width of a panel tab before the strip overflows.
    pub min_width: f32,
    /// Side of the × hit box: `tab.close_box`.
    pub close_box: f32,
    /// Side of the × glyph: `size.icon.sm`.
    pub close_icon: f32,
    /// Width of the overflow button: `size.control.sm`.
    pub overflow: f32,
    /// Corner radius of the active and hovered tab's top corners: 0 in Compact,
    /// `shape.radius.control` in Comfortable.
    pub radius: f32,
    /// A divider at each document tab's trailing edge.
    pub dividers: bool,
    /// Width of the drop marker line and outline.
    pub drop_width: f32,
    /// Width of the halo on each side of the drop marker.
    pub drop_halo_width: f32,
    /// Height of the progress bar.
    pub progress_height: f32,
    /// How far outside the strip a tab drag still resolves targets.
    pub tear_off: f32,
    /// Type role of the label.
    pub label: TypeRole,
    /// Type role of the label with Large Tabs.
    pub label_large: TypeRole,
    /// Type role of the detail text.
    pub detail_role: TypeRole,
    /// Behind the strip: `surface.background`.
    pub strip: Rgba,
    /// Behind the active tab: `surface.panel`.
    pub active: Rgba,
    /// Behind a tab under the pointer: the strip moved toward `state.hover`.
    pub hover: Rgba,
    /// The label of the active and of a hovered tab: `text.normal`.
    pub text: Rgba,
    /// The label of an inactive tab: `text.muted`.
    pub inactive_text: Rgba,
    /// The detail text: `text.faint`.
    pub detail: Rgba,
    /// The divider at a tab's trailing edge: `line.divider`.
    pub divider: Rgba,
    /// The ×: `text.muted`.
    pub close_text: Rgba,
    /// The × under the pointer: `text.normal`.
    pub close_hover: Rgba,
    /// The drop marker: `accent.solid`.
    pub drop: Rgba,
    /// The outline under the drop marker: `surface.panel`.
    pub drop_halo: Rgba,
}

impl TabTokens {
    /// The strip's height, with Large Tabs if `large`.
    pub fn strip_height(&self, large: bool) -> f32 {
        if large { self.height_large } else { self.height }
    }

    /// The label's type role, with Large Tabs if `large`.
    pub fn label_role(&self, large: bool) -> TypeRole {
        if large { self.label_large } else { self.label }
    }

    fn color_mut(&mut self, f: TabField) -> Option<&mut Rgba> {
        Some(match f {
            TabField::Radius => return None,
            TabField::Strip => &mut self.strip,
            TabField::Active => &mut self.active,
            TabField::Hover => &mut self.hover,
            TabField::Text => &mut self.text,
            TabField::InactiveText => &mut self.inactive_text,
            TabField::Detail => &mut self.detail,
            TabField::Divider => &mut self.divider,
            TabField::Close => &mut self.close_text,
            TabField::CloseHover => &mut self.close_hover,
            TabField::Drop => &mut self.drop,
            TabField::DropHalo => &mut self.drop_halo,
        })
    }
}

/// An overridable tab token. Paths: `radius`, `strip`, `active`, `hover`, `text`,
/// `text.inactive`, `detail`, `divider`, `close`, `close.hover`, `drop`, `drop.halo`, e.g.
/// `tab.text.inactive`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TabField {
    /// `tab.radius`
    Radius,
    /// `tab.strip`
    Strip,
    /// `tab.active`
    Active,
    /// `tab.hover`
    Hover,
    /// `tab.text`
    Text,
    /// `tab.text.inactive`
    InactiveText,
    /// `tab.detail`
    Detail,
    /// `tab.divider`
    Divider,
    /// `tab.close`
    Close,
    /// `tab.close.hover`
    CloseHover,
    /// `tab.drop`
    Drop,
    /// `tab.drop.halo`
    DropHalo,
}

impl TabField {
    const fn path(self) -> &'static str {
        match self {
            TabField::Radius => "radius",
            TabField::Strip => "strip",
            TabField::Active => "active",
            TabField::Hover => "hover",
            TabField::Text => "text",
            TabField::InactiveText => "text.inactive",
            TabField::Detail => "detail",
            TabField::Divider => "divider",
            TabField::Close => "close",
            TabField::CloseHover => "close.hover",
            TabField::Drop => "drop",
            TabField::DropHalo => "drop.halo",
        }
    }
}

impl Family for TabTokens {
    type Field = TabField;
    const FIELDS: &'static [TabField] = &[
        TabField::Radius,
        TabField::Strip,
        TabField::Active,
        TabField::Hover,
        TabField::Text,
        TabField::InactiveText,
        TabField::Detail,
        TabField::Divider,
        TabField::Close,
        TabField::CloseHover,
        TabField::Drop,
        TabField::DropHalo,
    ];

    fn defaults(s: &Semantic<'_>) -> TabTokens {
        let (c, tab) = (s.color, &s.size.tab);
        let strip = c.surface.background;
        let hover = mix(strip, over(c.state.hover, strip), values::TAB_HOVER_MIX);
        TabTokens {
            height: s.size.bar.tab,
            height_large: tab.height_large,
            pad_x: 2.0 * s.space.pad.control,
            gap: s.space.gap.inline,
            min_width: tab.min_width,
            close_box: tab.close,
            close_icon: s.size.icon.sm,
            overflow: s.size.control.sm,
            radius: if tab.rounded { s.shape.radius.control } else { 0.0 },
            dividers: tab.dividers,
            drop_width: values::TAB_DROP_WIDTH,
            drop_halo_width: values::TAB_DROP_HALO,
            progress_height: values::TAB_PROGRESS_HEIGHT,
            tear_off: values::TAB_TEAR_OFF,
            label: TypeRole::Label,
            label_large: TypeRole::Body,
            detail_role: TypeRole::Caption,
            strip,
            active: c.surface.panel,
            hover,
            text: c.text.normal,
            inactive_text: c.text.muted,
            detail: c.text.faint,
            divider: c.line.divider,
            close_text: c.text.muted,
            close_hover: c.text.normal,
            drop: c.accent.solid,
            drop_halo: c.surface.panel,
        }
    }

    fn path(f: TabField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<TabField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: TabField) -> FieldKind {
        match f {
            TabField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: TabField) -> OverrideValue {
        match f {
            TabField::Radius => OverrideValue::Length(self.radius),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: TabField, v: OverrideValue) {
        match (f, v) {
            (TabField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Tab;
        let component = |f| TokenRef::Component(field(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: component(fg), bg: component(bg), minimum, alternate: None };
        let (text, mark) = (values::TEXT_MIN, values::NON_TEXT_MIN);
        vec![
            pair(TabField::Text, TabField::Active, text),
            pair(TabField::Text, TabField::Hover, text),
            pair(TabField::InactiveText, TabField::Strip, text),
            pair(TabField::Detail, TabField::Strip, text),
            pair(TabField::Detail, TabField::Hover, text),
            pair(TabField::Detail, TabField::Active, text),
            pair(TabField::Close, TabField::Strip, mark),
            pair(TabField::Close, TabField::Active, mark),
            pair(TabField::Drop, TabField::DropHalo, mark),
            // The strip's keyboard focus stroke is `float.focus`.
            ContrastPair {
                fg: TokenRef::Component(ComponentField::Float(FloatField::Focus)),
                bg: component(TabField::Strip),
                minimum: mark,
                alternate: None,
            },
        ]
    }
}
