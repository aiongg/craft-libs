//! Variable font axes (F14.09): the axes and named instances of a face, and the row of sliders
//! that edits them (Photoshop's Properties › Character axis sliders; FigCraft's axis panel).

use craft_ui::composites::SliderRow;
use craft_ui::controls::Select;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use egui::Ui;
use skrifa::raw::types::Tag;
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider as _};

craft_ui::words! {
    /// The words of the axis sliders, English by default.
    #[non_exhaustive]
    pub struct AxesLabels {
        weight: "axes.weight" = "Weight";
        width: "axes.width" = "Width";
        slant: "axes.slant" = "Slant";
        italic: "axes.italic" = "Italic";
        optical_size: "axes.opticalSize" = "Optical size";
        style: "axes.style" = "Style";
        custom: "axes.custom" = "Custom";
    }
}

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
pub fn read_axes(data: &[u8], index: u32) -> Vec<FontAxis> {
    let Ok(font) = FontRef::from_index(data, index) else { return Vec::new() };
    font.axes()
        .iter()
        .map(|axis| FontAxis {
            tag: axis.tag().to_be_bytes(),
            name: name(&font, axis.name_id()).unwrap_or_default(),
            min: axis.min_value(),
            default: axis.default_value(),
            max: axis.max_value(),
            hidden: axis.is_hidden(),
        })
        .collect()
}

/// The named instances of the face at `index` in `data`, in font order.
pub fn read_named_instances(data: &[u8], index: u32) -> Vec<NamedInstance> {
    let Ok(font) = FontRef::from_index(data, index) else { return Vec::new() };
    let tags: Vec<[u8; 4]> = font.axes().iter().map(|axis| axis.tag().to_be_bytes()).collect();
    font.named_instances()
        .iter()
        .map(|instance| NamedInstance {
            name: name(&font, instance.subfamily_name_id()).unwrap_or_default(),
            coordinates: tags.iter().copied().zip(instance.user_coords()).collect(),
        })
        .collect()
}

fn name(font: &FontRef, id: StringId) -> Option<String> {
    font.localized_strings(id).english_or_first().map(|s| s.to_string()).filter(|s| !s.is_empty())
}

/// How close two coordinates are to count as equal: well under the 0.01 the sliders step by.
const SAME: f32 = 0.005;

/// The instance whose coordinates equal `values` on every axis (the Style field shows its name),
/// else `None` (the field shows "Custom").
// pc: crates/text/src/fonts.rs:512 (instance matching by axis values, #8797).
pub fn matching_instance<'a>(instances: &'a [NamedInstance], values: &[([u8; 4], f32)]) -> Option<&'a NamedInstance> {
    instances.iter().find(|instance| {
        !instance.coordinates.is_empty()
            && instance.coordinates.iter().all(|(tag, at)| values.iter().any(|(t, v)| t == tag && (v - at).abs() <= SAME))
    })
}

/// The label of an axis: its name when the font gives one, else the registered tag's
/// (`Weight`, `Width`, `Slant`, `Italic`, `Optical size`), else the tag itself.
pub fn axis_label(axis: &FontAxis) -> String {
    label_in(axis, &AxesLabels::default())
}

fn label_in(axis: &FontAxis, labels: &AxesLabels) -> String {
    if !axis.name.is_empty() {
        return axis.name.clone();
    }
    match &axis.tag {
        b"wght" => labels.weight.clone(),
        b"wdth" => labels.width.clone(),
        b"slnt" => labels.slant.clone(),
        b"ital" => labels.italic.clone(),
        b"opsz" => labels.optical_size.clone(),
        tag => Tag::new(tag).to_string(),
    }
}

/// What changed in the sliders this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxisSlidersResponse {
    /// (tag, new value) for every axis the user moved this pass, with the values of a drag in
    /// progress (live preview).
    pub changed: Vec<([u8; 4], f32)>,
    /// (tag, value) of the axes whose gesture ended this pass (a release, a typed value, a key
    /// step, an instance pick): one edit for the host to record per gesture.
    pub settled: Vec<([u8; 4], f32)>,
    /// A named instance was picked from the list.
    pub instance: Option<NamedInstance>,
}

/// One `SliderRow` per visible axis (label, slider, number field with the axis's range), plus a
/// `Select` of named instances above them when the face has any. Hidden axes and axes with an
/// empty or non-finite range are left out. Values are the current coordinates; an axis without
/// one shows its default. Axes whose range is 10 wide or more step by whole units, the others by
/// hundredths.
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
        let labels = AxesLabels::of(ui);
        let numbers = LocaleNumbers::of(ui);
        let mut out = AxisSlidersResponse::default();
        let AxisSliders { axes, instances, values } = self;

        if !instances.is_empty() {
            let current: Vec<([u8; 4], f32)> = axes.iter().map(|a| (a.tag, value_of(values, a))).collect();
            let mut picked = matching_instance(instances, &current)
                .map_or(usize::MAX, |m| instances.iter().position(|i| std::ptr::eq(i, m)).unwrap_or(usize::MAX));
            let before = picked;
            let mut select = Select::new("axes.instances", labels.style.clone(), &mut picked).placeholder(labels.custom.clone());
            for (i, instance) in instances.iter().enumerate() {
                select = select.option(i, instance.name.clone());
            }
            let response = select.show(ui);
            ui.add_space(ui.spacing().item_spacing.y);
            if let Some(EditEvent::Commit { changed: true, .. } | EditEvent::Set { .. }) = response.event
                && picked != before
                && let Some(instance) = instances.get(picked)
            {
                for &(tag, value) in &instance.coordinates {
                    set_value(values, tag, value);
                    out.changed.push((tag, value));
                    out.settled.push((tag, value));
                }
                out.instance = Some(instance.clone());
            }
        }

        for axis in axes.iter().filter(|a| !a.hidden && a.min.is_finite() && a.max.is_finite() && a.min < a.max) {
            let wide = axis.max - axis.min >= 10.0;
            let value = value_of(values, axis);
            let row = SliderRow::new(("axis", axis.tag), label_in(axis, &labels), value, f64::from(axis.min)..=f64::from(axis.max), &numbers)
                .step(if wide { 1.0 } else { 0.01 })
                .decimals(if wide { 0 } else { 2 })
                .default_value(axis.default.clamp(axis.min, axis.max))
                .show(ui);
            for event in row.edits() {
                let (v, settled) = match &event {
                    EditEvent::Update { value, .. } => (*value, false),
                    EditEvent::Commit { value, changed, .. } => (*value, *changed),
                    EditEvent::Set { value, .. } => (*value, true),
                    _ => continue,
                };
                let v = (v as f32).clamp(axis.min, axis.max);
                set_value(values, axis.tag, v);
                out.changed.push((axis.tag, v));
                if settled {
                    out.settled.push((axis.tag, v));
                }
            }
        }
        out
    }
}

/// The value an axis shows: the stored one, else the axis default, within the axis range.
fn value_of(values: &[([u8; 4], f32)], axis: &FontAxis) -> f32 {
    let default = if axis.default.is_finite() { axis.default } else { axis.min };
    let stored = values.iter().find(|(t, _)| *t == axis.tag).map(|(_, v)| *v).filter(|v| v.is_finite());
    let (lo, hi) = if axis.min <= axis.max { (axis.min, axis.max) } else { (axis.max, axis.min) };
    stored.unwrap_or(default).clamp(lo, hi)
}

fn set_value(values: &mut Vec<([u8; 4], f32)>, tag: [u8; 4], value: f32) {
    match values.iter_mut().find(|(t, _)| *t == tag) {
        Some(slot) => slot.1 = value,
        None => values.push((tag, value)),
    }
}
