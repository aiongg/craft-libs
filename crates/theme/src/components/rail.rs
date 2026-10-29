//! Rails: the tool rail and the panel rail, and the flyout of a rail button that stands for a
//! group of choices.
//!
//! A rail button at rest shows its glyph in `rail.icon` (the tool rail) or `rail.icon.panel` (the
//! panel rail); under the pointer, `rail.icon.hover` on `rail.hover`. A tool-rail button that
//! holds the current tool is a toggled-on icon button (`icon_button.selected.*`); a panel-rail
//! button whose panel is open is quiet: `rail.selected` inside a `rail.selected.border` outline,
//! the glyph in `rail.selected.text`. The flyout list is a menu: its frame, rows and active row
//! come from the popup surface and `menu.*`.
//!
//! The glyph is the button's only mark (its name is in the tooltip), so each glyph colour is held
//! to the non-text minimum on what it sits on. The flyout indicator is a hint beside the
//! accessible `has_popup` name, so it has no pair. The edge line is a design target.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Rail tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RailTokens {
    /// Width of the tool rail: `size.rail.width`.
    pub width: f32,
    /// Square tool-rail button: `size.rail.button`.
    pub button: f32,
    /// Width of the panel rail: `size.rail.panel_width`.
    pub panel_width: f32,
    /// Square panel-rail button: `size.rail.panel_button`.
    pub panel_button: f32,
    /// Glyph of a tool-rail button: two thirds of the button, rounded (PhotoCraft's Pro toolbar).
    pub icon: f32,
    /// Glyph of a panel-rail button: 0.52 of the button, rounded (PhotoCraft's rail).
    pub panel_icon: f32,
    /// Between the two columns of a tool rail: half the inline gap.
    pub gap: f32,
    /// Between the rows of a tool rail: three quarters of the inline gap, rounded.
    pub row_gap: f32,
    /// Between the buttons of a panel rail: the inline gap.
    pub panel_gap: f32,
    /// Above and below the buttons, and how far the edge line runs past the rail at each end:
    /// `space.pad.panel`.
    pub pad_y: f32,
    /// Height of the column toggle at the top of a tool rail.
    pub toggle_height: f32,
    /// Glyph of the column toggle.
    pub toggle_icon: f32,
    /// Height of a flyout row: `size.control.md` and a little more.
    pub flyout_row: f32,
    /// Between a button and its flyout: `space.gap.stack`.
    pub flyout_gap: f32,
    /// Glyph in a flyout row: half way between `size.icon.sm` and `size.icon.md`.
    pub flyout_icon: f32,
    /// Legs of the flyout indicator triangle.
    pub indicator: f32,
    /// Distance of the indicator from its button's corner.
    pub indicator_inset: f32,
    /// Radius of the open-panel marker dot.
    pub marker_radius: f32,
    /// Width of the outline of a selected panel-rail button.
    pub selected_border_width: f32,
    /// Corner radius of a button's fill and outline: `radius.control`.
    pub radius: f32,
    /// Behind a rail: `surface.chrome`.
    pub fill: Rgba,
    /// Behind a panel-rail button whose panel is open: `state.selected`.
    pub selected: Rgba,
    /// Outline of that button: `accent.border`.
    pub selected_border: Rgba,
    /// Glyph on that button: `text.on_selected`.
    pub selected_text: Rgba,
    /// Behind a button under the pointer: `state.hover`.
    pub hover: Rgba,
    /// Glyph of a tool-rail button at rest: `text.muted`.
    pub icon_color: Rgba,
    /// Glyph of a panel-rail button at rest: `text.faint` (PhotoCraft's `text_faint`).
    pub panel_icon_color: Rgba,
    /// Glyph of a button under the pointer: `text.normal`.
    pub icon_hover: Rgba,
    /// The flyout indicator: `text.faint`.
    pub indicator_color: Rgba,
    /// The open-panel marker dot: `accent.solid`.
    pub marker: Rgba,
    /// The line on the canvas side of a panel rail: `line.divider`.
    pub edge: Rgba,
}

impl RailTokens {
    fn color_mut(&mut self, f: RailField) -> Option<&mut Rgba> {
        Some(match f {
            RailField::Radius => return None,
            RailField::Fill => &mut self.fill,
            RailField::Selected => &mut self.selected,
            RailField::SelectedBorder => &mut self.selected_border,
            RailField::SelectedText => &mut self.selected_text,
            RailField::Hover => &mut self.hover,
            RailField::Icon => &mut self.icon_color,
            RailField::PanelIcon => &mut self.panel_icon_color,
            RailField::IconHover => &mut self.icon_hover,
            RailField::Indicator => &mut self.indicator_color,
            RailField::Marker => &mut self.marker,
            RailField::Edge => &mut self.edge,
        })
    }
}

/// An overridable rail token. Paths: `radius`, `fill`, `selected`, `selected.border`,
/// `selected.text`, `hover`, `icon`, `icon.panel`, `icon.hover`, `indicator`, `marker`, `edge`,
/// e.g. `rail.selected.text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RailField {
    /// `rail.radius`
    Radius,
    /// `rail.fill`
    Fill,
    /// `rail.selected`
    Selected,
    /// `rail.selected.border`
    SelectedBorder,
    /// `rail.selected.text`
    SelectedText,
    /// `rail.hover`
    Hover,
    /// `rail.icon`
    Icon,
    /// `rail.icon.panel`
    PanelIcon,
    /// `rail.icon.hover`
    IconHover,
    /// `rail.indicator`
    Indicator,
    /// `rail.marker`
    Marker,
    /// `rail.edge`
    Edge,
}

impl RailField {
    const fn path(self) -> &'static str {
        match self {
            RailField::Radius => "radius",
            RailField::Fill => "fill",
            RailField::Selected => "selected",
            RailField::SelectedBorder => "selected.border",
            RailField::SelectedText => "selected.text",
            RailField::Hover => "hover",
            RailField::Icon => "icon",
            RailField::PanelIcon => "icon.panel",
            RailField::IconHover => "icon.hover",
            RailField::Indicator => "indicator",
            RailField::Marker => "marker",
            RailField::Edge => "edge",
        }
    }
}

impl Family for RailTokens {
    type Field = RailField;
    const FIELDS: &'static [RailField] = &[
        RailField::Radius,
        RailField::Fill,
        RailField::Selected,
        RailField::SelectedBorder,
        RailField::SelectedText,
        RailField::Hover,
        RailField::Icon,
        RailField::PanelIcon,
        RailField::IconHover,
        RailField::Indicator,
        RailField::Marker,
        RailField::Edge,
    ];

    fn defaults(s: &Semantic<'_>) -> RailTokens {
        let (c, rail) = (s.color, s.size.rail);
        RailTokens {
            width: rail.width,
            button: rail.button,
            panel_width: rail.panel_width,
            panel_button: rail.panel_button,
            icon: (rail.button * values::RAIL_TOOL_ICON_RATIO).round(),
            panel_icon: (rail.panel_button * values::RAIL_PANEL_ICON_RATIO).round(),
            gap: s.space.gap.inline / 2.0,
            row_gap: (s.space.gap.inline * values::RAIL_ROW_GAP_RATIO).round(),
            panel_gap: s.space.gap.inline,
            pad_y: s.space.pad.panel,
            toggle_height: values::RAIL_TOGGLE_HEIGHT,
            toggle_icon: values::RAIL_TOGGLE_ICON,
            flyout_row: s.size.control.md + values::RAIL_FLYOUT_ROW_EXTRA,
            flyout_gap: s.space.gap.stack,
            flyout_icon: (s.size.icon.sm + s.size.icon.md) / 2.0,
            indicator: values::RAIL_INDICATOR,
            indicator_inset: values::RAIL_INDICATOR_INSET,
            marker_radius: values::RAIL_MARKER_RADIUS,
            selected_border_width: values::RAIL_SELECTED_BORDER_WIDTH,
            radius: s.shape.radius.control,
            fill: c.surface.chrome,
            selected: c.state.selected,
            selected_border: c.accent.border,
            selected_text: c.text.on_selected,
            hover: c.state.hover,
            icon_color: c.text.muted,
            panel_icon_color: c.text.faint,
            icon_hover: c.text.normal,
            indicator_color: c.text.faint,
            marker: c.accent.solid,
            edge: c.line.divider,
        }
    }

    fn path(f: RailField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<RailField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: RailField) -> FieldKind {
        match f {
            RailField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: RailField) -> OverrideValue {
        match f {
            RailField::Radius => OverrideValue::Length(self.radius),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: RailField, v: OverrideValue) {
        match (f, v) {
            (RailField::Radius, OverrideValue::Length(r)) => self.radius = r,
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
        let component = |f| TokenRef::Component(ComponentField::Rail(f));
        let pair = |fg, bg| ContrastPair { fg: component(fg), bg: component(bg), minimum: values::NON_TEXT_MIN, alternate: None };
        vec![
            pair(RailField::Icon, RailField::Fill),
            pair(RailField::PanelIcon, RailField::Fill),
            pair(RailField::IconHover, RailField::Hover),
            pair(RailField::SelectedText, RailField::Selected),
            pair(RailField::Marker, RailField::Fill),
        ]
    }
}
