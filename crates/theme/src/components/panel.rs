//! Panels: the body of a docked or floating panel, with an optional footer bar. A floating panel's
//! frame reads `float.*` and its header `panel_header.*`.
//!
//! The lines are one physical pixel and are design targets with no minimum.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;

/// Panel tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelTokens {
    /// Padding on every side of the body: `space.pad.panel`.
    pub pad: f32,
    /// Height of the footer bar under its line: `size.control.md + space.pad.control` (PhotoCraft's
    /// bar is 30).
    pub footer_height: f32,
    /// Between footer buttons: half the inline gap (PhotoCraft's `item_spacing.x` is 2).
    pub footer_gap: f32,
    /// Behind the body: `surface.panel`.
    pub fill: Rgba,
    /// Under a floating panel's header: `line.divider`.
    pub header_line: Rgba,
    /// Above the footer, edge to edge: `line.divider`.
    pub footer_line: Rgba,
}

impl PanelTokens {
    fn color_mut(&mut self, f: PanelField) -> &mut Rgba {
        match f {
            PanelField::Fill => &mut self.fill,
            PanelField::HeaderLine => &mut self.header_line,
            PanelField::FooterLine => &mut self.footer_line,
        }
    }
}

/// An overridable panel token. Paths: `fill`, `header.line`, `footer.line`, e.g. `panel.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelField {
    /// `panel.fill`
    Fill,
    /// `panel.header.line`
    HeaderLine,
    /// `panel.footer.line`
    FooterLine,
}

impl PanelField {
    const fn path(self) -> &'static str {
        match self {
            PanelField::Fill => "fill",
            PanelField::HeaderLine => "header.line",
            PanelField::FooterLine => "footer.line",
        }
    }
}

impl Family for PanelTokens {
    type Field = PanelField;
    const FIELDS: &'static [PanelField] = &[PanelField::Fill, PanelField::HeaderLine, PanelField::FooterLine];

    fn defaults(s: &Semantic<'_>) -> PanelTokens {
        let c = s.color;
        PanelTokens {
            pad: s.space.pad.panel,
            footer_height: s.size.control.md + s.space.pad.control,
            footer_gap: s.space.gap.inline / 2.0,
            fill: c.surface.panel,
            header_line: c.line.divider,
            footer_line: c.line.divider,
        }
    }

    fn path(f: PanelField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<PanelField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(_: PanelField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: PanelField) -> OverrideValue {
        let mut copy = *self;
        OverrideValue::Color(*copy.color_mut(f))
    }

    fn set(&mut self, f: PanelField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
