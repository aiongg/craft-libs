//! Variable font axes (F14.09): the axes and named instances of a face, and the row of sliders
//! that edits them (Photoshop's Properties › Character axis sliders; FigCraft's axis panel).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

/// One variation axis of a face.
#[derive(Clone, Debug, PartialEq)]
pub struct FontAxis {
    /// The OpenType tag (`wght`, `wdth`, `slnt`, `ital`, `opsz`, or a custom one).
    pub tag: [u8; 4],
    /// The axis's name from the font's name table (`Weight`), English when there is one.
    pub name: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
    /// Hidden from the user by the font (`HIDDEN_AXIS`).
    pub hidden: bool,
}

/// A named instance of a variable face (`Bold Condensed`): its coordinates per axis.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedInstance {
    pub name: String,
    /// (tag, value) for every axis of the face, in axis order.
    pub coordinates: Vec<([u8; 4], f32)>,
}

/// The axes of the face at `index` in `data`, in font order; empty for a static font.
// pc: crates/text/src/fonts.rs:476 (`f.axes()` through the text engine); here through skrifa's
// `axes()` and `named_instances()`.
pub fn read_axes(data: &[u8], index: u32) -> Vec<FontAxis> {
    todo!()
}

/// The named instances of the face at `index` in `data`, in font order.
pub fn read_named_instances(data: &[u8], index: u32) -> Vec<NamedInstance> {
    todo!()
}

/// The instance whose coordinates equal `values` on every axis (the Style field shows its name),
/// else `None` (the field shows "Custom").
// pc: crates/text/src/fonts.rs:512 (instance matching by axis values, #8797).
pub fn matching_instance<'a>(instances: &'a [NamedInstance], values: &[([u8; 4], f32)]) -> Option<&'a NamedInstance> {
    todo!()
}

/// The label of an axis: its name when the font gives one, else the registered tag's
/// (`Weight`, `Width`, `Slant`, `Italic`, `Optical size`), else the tag itself.
pub fn axis_label(axis: &FontAxis) -> String {
    todo!()
}

/// What changed in the sliders this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxisSlidersResponse {
    /// (tag, new value) for every axis the user moved; one edit per gesture (the value settles on
    /// release, with live preview while dragging).
    pub changed: Vec<([u8; 4], f32)>,
    /// A named instance was picked from the list.
    pub instance: Option<NamedInstance>,
}

/// One `SliderRow` per visible axis (label, slider, number field with the axis's range), plus a
/// `Select` of named instances above them when the face has any. Hidden axes are left out.
/// Values are the current coordinates; an axis without one shows its default.
pub struct AxisSliders<'a> {
    axes: &'a [FontAxis],
    instances: &'a [NamedInstance],
    values: &'a mut Vec<([u8; 4], f32)>,
}

impl<'a> AxisSliders<'a> {
    pub fn new(axes: &'a [FontAxis], values: &'a mut Vec<([u8; 4], f32)>) -> Self {
        AxisSliders { axes, instances: &[], values }
    }

    /// Show a named-instance list above the sliders.
    pub fn instances(mut self, instances: &'a [NamedInstance]) -> Self {
        self.instances = instances;
        self
    }

    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:189 `axis_sliders` and `:166 axis_value`.
    pub fn show(self, ui: &mut Ui) -> AxisSlidersResponse {
        todo!()
    }
}
