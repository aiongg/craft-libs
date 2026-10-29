//! Dock areas: the panel groups, column bars and iconic columns of a docking surface, and the
//! previews of a drop. The tab strip, the active tab and the tab labels are the `tab` family's;
//! this family holds what surrounds them. Sizes follow the Size preset, the colours are the
//! surface, line, accent and state tokens.
//!
//! The drop outline is the only cue for where a drop lands, so it is measured as a non-text mark
//! on the dock background (3:1, 4.5:1); the label of the selected icon cell is text on that cell.
//! The borders and fills are design targets with no minimum.

use super::{ComponentField, ContrastPair, Family, FieldKind, Identify, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Dock area tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DockTokens {
    /// Between a column's panes and around a group's parts: `space.gap.inline`.
    pub gap: f32,
    /// Width of a group's border.
    pub border_width: f32,
    /// Height of a floating window's title bar: 14 Compact, 18 Comfortable (14 scaled by
    /// `size.control.md` over 24, rounded).
    pub title_height: f32,
    /// Height of a collapsed column's bar, by the same rule.
    pub column_bar_height: f32,
    /// Side of an iconic column's icon: `size.icon.lg`.
    pub icon_size: f32,
    /// Added to the icon size for the height of an iconic column's row.
    pub icon_pad: f32,
    /// Width of an iconic column: `size.rail.panel_width`.
    pub iconic_width: f32,
    /// Width of a drop outline and of the line under a dragged column.
    pub drop_width: f32,
    /// Width of a keyboard focus stroke.
    pub focus_width: f32,
    /// Corner radius of the insertion marker.
    pub marker_radius: f32,
    /// Alpha factor of the filled drop preview.
    pub drop_fill: f32,
    /// Alpha factor of the highlighted zone under a dragged tab.
    pub ghost_fill: f32,
    /// Alpha factor of the outline of every zone under a dragged tab.
    pub ghost_stroke: f32,
    /// Alpha factor of the light fill inside a tab drop's outline.
    pub tab_fill: f32,
    /// Alpha factor of the resize handle's stroke at hover and focus.
    pub grip_alpha: f32,
    /// Corner radius of the current icon cell and of the letter box: `radius.control`.
    pub icon_radius: f32,
    /// Corner radius of the float hint box: `radius.popup`.
    pub hint_radius: f32,
    /// Behind the panes: `surface.panel`.
    pub background: Rgba,
    /// A group's border: `line.border`.
    pub border: Rgba,
    /// A resize handle at hover and drag: `accent.solid`.
    pub resize: Rgba,
    /// Drop previews, the insertion marker and drop outlines: `accent.solid`.
    pub drop: Rgba,
    /// Behind the current icon cell: `state.selected`, moved to reach the non-text minimum on
    /// `background`.
    pub icon_selected: Rgba,
    /// The label and icon on the current icon cell: `text.on_selected`, moved to read on the
    /// moved fill.
    pub icon_selected_text: Rgba,
}

impl DockTokens {
    fn color_mut(&mut self, f: DockField) -> Option<&mut Rgba> {
        Some(match f {
            DockField::IconRadius | DockField::HintRadius => return None,
            DockField::Background => &mut self.background,
            DockField::Border => &mut self.border,
            DockField::Resize => &mut self.resize,
            DockField::Drop => &mut self.drop,
            DockField::IconSelected => &mut self.icon_selected,
            DockField::IconSelectedText => &mut self.icon_selected_text,
        })
    }
}

/// An overridable dock token. Paths: `background`, `border`, `resize`, `drop`, `icon.selected`,
/// `icon.selected.text`, `icon.radius`, `hint.radius`, e.g. `dock.icon.selected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DockField {
    /// `dock.background`
    Background,
    /// `dock.border`
    Border,
    /// `dock.resize`
    Resize,
    /// `dock.drop`
    Drop,
    /// `dock.icon.selected`
    IconSelected,
    /// `dock.icon.selected.text`
    IconSelectedText,
    /// `dock.icon.radius`
    IconRadius,
    /// `dock.hint.radius`
    HintRadius,
}

impl DockField {
    const fn path(self) -> &'static str {
        match self {
            DockField::Background => "background",
            DockField::Border => "border",
            DockField::Resize => "resize",
            DockField::Drop => "drop",
            DockField::IconSelected => "icon.selected",
            DockField::IconSelectedText => "icon.selected.text",
            DockField::IconRadius => "icon.radius",
            DockField::HintRadius => "hint.radius",
        }
    }
}

impl Family for DockTokens {
    type Field = DockField;
    const FIELDS: &'static [DockField] = &[
        DockField::Background,
        DockField::Border,
        DockField::Resize,
        DockField::Drop,
        DockField::IconSelected,
        DockField::IconSelectedText,
        DockField::IconRadius,
        DockField::HintRadius,
    ];

    /// The Compact bar heights are the docking code's own 14, scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> DockTokens {
        let c = s.color;
        let k = s.size.control.md / values::DOCK_BASE_CONTROL;
        let (icon_selected, icon_selected_text) = Identify::on(&[c.surface.panel], s.high).fill(c.state.selected, c.text.on_selected);
        DockTokens {
            gap: s.space.gap.inline,
            border_width: values::DOCK_BORDER_WIDTH,
            title_height: (values::DOCK_BAR_HEIGHT * k).round(),
            column_bar_height: (values::DOCK_BAR_HEIGHT * k).round(),
            icon_size: s.size.icon.lg,
            icon_pad: values::DOCK_ICON_PAD,
            iconic_width: s.size.rail.panel_width,
            drop_width: values::DOCK_DROP_WIDTH,
            focus_width: values::DOCK_FOCUS_WIDTH,
            marker_radius: values::DOCK_MARKER_RADIUS,
            drop_fill: values::DOCK_DROP_FILL,
            ghost_fill: values::DOCK_GHOST_FILL,
            ghost_stroke: values::DOCK_GHOST_STROKE,
            tab_fill: values::DOCK_TAB_FILL,
            grip_alpha: values::DOCK_GRIP_ALPHA,
            icon_radius: s.shape.radius.control,
            hint_radius: s.shape.radius.popup,
            background: c.surface.panel,
            border: c.line.border,
            resize: c.accent.solid,
            drop: c.accent.solid,
            icon_selected,
            icon_selected_text,
        }
    }

    fn path(f: DockField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<DockField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: DockField) -> FieldKind {
        match f {
            DockField::IconRadius | DockField::HintRadius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: DockField) -> OverrideValue {
        match f {
            DockField::IconRadius => OverrideValue::Length(self.icon_radius),
            DockField::HintRadius => OverrideValue::Length(self.hint_radius),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: DockField, v: OverrideValue) {
        match (f, v) {
            (DockField::IconRadius, OverrideValue::Length(r)) => self.icon_radius = r,
            (DockField::HintRadius, OverrideValue::Length(r)) => self.hint_radius = r,
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
        let component = |f| TokenRef::Component(ComponentField::Dock(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: component(fg), bg: component(bg), minimum, alternate: None };
        vec![
            pair(DockField::IconSelected, DockField::Background, values::STATE_CUE_MIN),
            pair(DockField::IconSelectedText, DockField::IconSelected, values::TEXT_MIN),
            pair(DockField::Drop, DockField::Background, values::NON_TEXT_MIN),
        ]
    }
}
