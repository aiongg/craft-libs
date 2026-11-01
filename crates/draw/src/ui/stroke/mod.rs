//! The stroke editor and its button (F09.01): one editor body ([`StrokeEditor`]) that a popover
//! ([`StrokeButton`], PhotoCraft's shape options bar) or a docked panel (VectorCraft's Stroke
//! panel, DesignCraft's Properties) hosts, with live line samples ([`StrokePreview`]), mixed
//! values ([`StrokeMixed`]) and app-filled rows ([`StrokeChoiceRow`]) for arrowheads, profiles
//! and named stroke types. The editor edits geometry only; width, colour and paint are the host's
//! controls. The value is [`StrokeStyle`], re-exported here.

mod button;
mod editor;
mod labels;
mod preview;
#[cfg(test)]
mod preview_tests;
pub(crate) mod stories;
mod style;
#[cfg(test)]
mod tests;

use std::ops::BitOr;

use egui::{Painter, Rect};

pub use crate::stroke::{DashUnit, LineCap, LineJoin, LineStyle, StrokeAlign, StrokeError, StrokeStyle};
pub use button::{StrokeButton, StrokeButtonResponse};
pub use editor::{StrokeEditor, StrokeEditorResponse};
pub use labels::StrokeLabels;
pub use preview::StrokePreview;
pub use style::StrokeEditorStyle;

/// Which parts of the editor show. Combine with `|`.
///
/// The Alignment, Caps and Corners rows and the Miter limit row always show; a section adds the
/// rest. [`POPOVER`](Self::POPOVER) is PhotoCraft's popover, [`PANEL`](Self::PANEL) VectorCraft's
/// docked panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StrokeSections(u8);

impl StrokeSections {
    /// The Solid, Dashed and Dotted preset rows with their line samples.
    pub const PRESETS: Self = Self(1);
    /// The Opacity row, 0–100 % (lead call 18: PhotoCraft has it, VectorCraft and DesignCraft do
    /// not).
    pub const OPACITY: Self = Self(1 << 1);
    /// The "Dashed line" check, the Dash and Gap pairs, Add pair, Remove pair and Dash offset.
    pub const DASHES: Self = Self(1 << 2);
    /// The tall line sample at the bottom.
    pub const SAMPLE: Self = Self(1 << 3);
    /// Alignment, Caps and Corners as rows of icon toggle buttons (`Segmented` with icons) in
    /// place of selects (VectorCraft's panel).
    pub const ICONS: Self = Self(1 << 4);
    /// PhotoCraft's popover: presets, opacity, dashes and the sample, choices as selects.
    pub const POPOVER: Self = Self(Self::PRESETS.0 | Self::OPACITY.0 | Self::DASHES.0 | Self::SAMPLE.0);
    /// A docked panel: dashes, choices as icon buttons.
    pub const PANEL: Self = Self(Self::DASHES.0 | Self::ICONS.0);

    /// Whether every section of `other` is in `self`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl Default for StrokeSections {
    fn default() -> Self {
        Self::POPOVER
    }
}

impl BitOr for StrokeSections {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// The fields whose values differ across the host's selection: such a field shows the mixed
/// placeholder (`NumberField::mixed`, a select's "Mixed") and lights no button; the first edit
/// sets the field for every item. Default: nothing mixed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StrokeMixed {
    pub align: bool,
    pub cap: bool,
    pub join: bool,
    pub miter_limit: bool,
    /// The pattern: no preset row is selected, the "Dashed line" check shows its mixed state and
    /// the pair rows are hidden until the check is set.
    pub dashes: bool,
    pub dash_offset: bool,
    pub opacity: bool,
}

impl StrokeMixed {
    /// The first of `styles` and which of its fields differ in the others (VectorCraft's panel
    /// over a multi-selection). `None` for no styles.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B3)")]
    pub fn of<'s>(styles: impl IntoIterator<Item = &'s StrokeStyle>) -> Option<(StrokeStyle, StrokeMixed)> {
        // vectorcraft: crates/ui-egui/src/panels/stroke.rs:169-172 (mixed values light no button), :225 (mixed dashes).
        todo!()
    }
}

/// The control of the editor an edit came from, so the host can coalesce edits by field
/// ([`StrokeEditorResponse::field`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrokeField {
    /// A preset row.
    Preset,
    Align,
    Cap,
    Join,
    MiterLimit,
    Opacity,
    /// The "Dashed line" check.
    Dashed,
    /// The dash or gap at this index of [`StrokeStyle::dashes`].
    Dash(usize),
    AddPair,
    RemovePair,
    DashOffset,
}

/// One choice of a [`StrokeChoiceRow`]: the app's id and the name the select shows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StrokeChoice {
    pub id: String,
    pub label: String,
}

impl StrokeChoice {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into() }
    }
}

/// The painter of a choice's preview in a [`StrokeChoiceRow`]: the painter, the preview's rect
/// and the choice.
pub(crate) type PaintChoice<'a> = Box<dyn Fn(&Painter, Rect, &StrokeChoice) + 'a>;

/// A row the app fills: a label and a `Select` of [`StrokeChoice`]s with painted previews
/// (arrowheads at the start and end, a width profile, DesignCraft's named stroke types). The
/// editor owns no arrowhead or profile data (lead call 19); picking reports
/// [`StrokeEditorResponse::choice`] with the row's and the choice's ids. S7-shapes' connector bar
/// uses the same rows for its caps.
pub struct StrokeChoiceRow<'a> {
    pub id: String,
    pub label: String,
    pub choices: &'a [StrokeChoice],
    /// The picked choice's id, `MixedValue::Mixed` across a selection, `None` for none.
    pub selected: Option<craft_ui::controls::MixedValue<&'a str>>,
    /// The preview painter; read by the editor and the connector bar's cap menus.
    pub(crate) paint: Option<PaintChoice<'a>>,
}

impl<'a> StrokeChoiceRow<'a> {
    /// A row `id` labelled `label` offering `choices`, nothing picked, previews as text only.
    pub fn new(id: impl Into<String>, label: impl Into<String>, choices: &'a [StrokeChoice]) -> Self {
        Self { id: id.into(), label: label.into(), choices, selected: None, paint: None }
    }

    /// The picked choice.
    pub fn selected(mut self, selected: craft_ui::controls::MixedValue<&'a str>) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Paints each choice's preview beside its label, in the select's button and its list
    /// (`Select::row_content`).
    pub fn paint(mut self, paint: impl Fn(&Painter, Rect, &StrokeChoice) + 'a) -> Self {
        self.paint = Some(Box::new(paint));
        self
    }
}
