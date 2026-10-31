//! [`PointPicker`]: a thumbnail with a draggable cross for a centre point, plus X and Y fields.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use egui::load::SizedTexture;
use egui::{AsIdSalt, IdSalt, Ui, Vec2};

use crate::edit::{EditPolicy, EditResponse};
use crate::localized::Unit;
use crate::theme::UiTokens;

/// The style of one picker, from the tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct PointPickerStyle {
    /// The thumbnail's size, from `dialog.viewport_min`.
    pub thumbnail: Vec2,
    /// Cross arm length in points.
    pub cross: f32,
}

/// A 2D point picked on a thumbnail of the image (Radial Blur's Blur Center, Zoom Blur, Lens
/// Flare), with X and Y [`NumberField`](super::NumberField)s beside it.
///
/// The user sees the image fitted in a sunken well (a checkerboard when there is no image), a
/// cross at the point, and two fields. A press on the thumbnail moves the point there; a drag
/// follows; the fields edit the same value in the picker's unit (percent of the image by
/// default). Keyboard: the thumbnail is one Tab stop; arrows move the point by one unit, Shift
/// ten; Esc during a drag restores the value (edit lifecycle). AccessKit: a group named `label`
/// with the fields as its children.
///
/// Reports one edit per gesture through [`EditResponse`], the value in the picker's unit.
#[must_use = "show the picker with `show`"]
pub struct PointPicker<'a> {
    id_salt: IdSalt,
    label: String,
    value: [f64; 2],
    image: Option<SizedTexture>,
    range: RangeInclusive<f64>,
    unit: Unit,
    decimals: usize,
    policy: EditPolicy,
    enabled: bool,
    _numbers: std::marker::PhantomData<&'a ()>,
}

impl<'a> PointPicker<'a> {
    /// A picker named `label` showing `value` (x, y) in `0..=100` percent, over `image` when
    /// there is one.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: [f64; 2], image: Option<SizedTexture>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            image,
            range: 0.0..=100.0,
            unit: Unit::new("{n}%"),
            decimals: 0,
            policy: EditPolicy::default(),
            enabled: true,
            _numbers: std::marker::PhantomData,
        }
    }

    /// The fields' range on each axis and the unit they show (pixels of the image, percent).
    pub fn range(mut self, range: RangeInclusive<f64>, unit: Unit) -> Self {
        self.range = range;
        self.unit = unit;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The style, dumped under `components/point_picker`.
    // new: sizes from `dialog.viewport_min` and the field tokens; no PhotoCraft token.
    pub fn style(tokens: &UiTokens) -> PointPickerStyle {
        todo!()
    }

    /// Draws the thumbnail and the two fields.
    // pc: crates/ui-egui/src/dialogs/filter_dialog/zoom_center.rs:21 `row` (:50 `interact` for
    // the press and drag, :74 `paint` for the cross) with an image in place of the streak diagram.
    pub fn show(self, ui: &mut Ui) -> EditResponse<[f64; 2]> {
        todo!()
    }
}
