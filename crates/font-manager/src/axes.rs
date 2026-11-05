//! Variable font axes (F14.09): the axes and named instances of a face, and the row of sliders
//! that edits them (Photoshop's Properties › Character axis sliders; FigCraft's axis panel).

use craft_ui::composites::SliderRow;
use craft_ui::controls::Select;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use egui::{AsIdSalt, IdSalt, Modifiers, Ui};
use skrifa::raw::types::Tag;
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider as _};

craft_ui::words! {
    /// The words of the axis sliders, English by default.
    #[non_exhaustive]
    pub struct AxisSlidersLabels {
        weight: "axisSliders.weight" = "Weight";
        width: "axisSliders.width" = "Width";
        slant: "axisSliders.slant" = "Slant";
        italic: "axisSliders.italic" = "Italic";
        style: "axisSliders.style" = "Style";
        custom: "axisSliders.custom" = "Custom";
    }
}

/// One variation axis of a face.
#[derive(Clone, Debug, PartialEq)]
pub struct FontAxis {
    /// The OpenType tag (`wght`, `wdth`, `slnt`, `ital`, `opsz`, or a custom one).
    pub tag: [u8; 4],
    /// The axis's name from the font's name table (`Weight`), English when there is one.
    pub name: String,
    /// Lowest value, in the axis's own units.
    pub min: f32,
    /// Value the face uses when no coordinate is set.
    pub default: f32,
    /// Highest value, in the axis's own units.
    pub max: f32,
    /// Hidden from the user by the font (`HIDDEN_AXIS`).
    pub hidden: bool,
}

/// A named instance of a variable face (`Bold Condensed`): its coordinates per axis.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedInstance {
    /// The instance's name from the font's name table.
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
/// (`Weight`, `Width`, `Slant`, `Italic`), else the tag itself.
pub fn axis_label(axis: &FontAxis) -> String {
    label_in(axis, &AxisSlidersLabels::default())
}

fn label_in(axis: &FontAxis, labels: &AxisSlidersLabels) -> String {
    if !axis.name.is_empty() {
        return axis.name.clone();
    }
    match &axis.tag {
        b"wght" => labels.weight.clone(),
        b"wdth" => labels.width.clone(),
        b"slnt" => labels.slant.clone(),
        b"ital" => labels.italic.clone(),
        tag => Tag::new(tag).to_string(),
    }
}

/// What changed in the sliders this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxisSlidersResponse {
    /// The gesture events of this pass, in drawing order, keyed by axis tag: a slider drag or a
    /// field edit reports `Begin`, `Update` and `Commit`; a key step, a typed value and an
    /// instance pick report `Set` (one per axis of the instance). The sliders have already stored
    /// the values the events carry.
    pub edits: Vec<([u8; 4], EditEvent<f32>)>,
    /// A named instance was picked from the list.
    pub instance: Option<NamedInstance>,
}

/// One `SliderRow` per visible axis (label, slider, number field with the axis's range), plus a
/// `Select` of named instances above them when the face has any. Hidden axes, `opsz` (the text
/// layout sets the optical size from the font size, as in PhotoCraft) and axes with an
/// empty or non-finite range are left out. Values are the current coordinates; an axis without
/// one shows what the layout draws ([`AxisSliders::drawn`]; else its default). Axes whose range is 10 wide or more step by whole units, the others by
/// hundredths.
pub struct AxisSliders<'a> {
    axes: &'a [FontAxis],
    instances: &'a [NamedInstance],
    values: &'a mut Vec<([u8; 4], f32)>,
    drawn: Drawn,
    salt: IdSalt,
}

/// The style the sliders' unset axes show.
#[derive(Clone, Copy, Default)]
struct Drawn {
    weight: Option<f32>,
    italic: bool,
}

impl<'a> AxisSliders<'a> {
    /// `id_salt` names the sliders to egui; give each instance on a screen its own, such as the
    /// layer or style they edit.
    pub fn new(id_salt: impl AsIdSalt, axes: &'a [FontAxis], values: &'a mut Vec<([u8; 4], f32)>) -> Self {
        AxisSliders { axes, instances: &[], values, drawn: Drawn::default(), salt: IdSalt::new(id_salt) }
    }

    /// What the layout draws for an axis `values` has no coordinate for: the style's `weight` on
    /// `wght` and the axis maximum on `ital` for an italic style, as PhotoCraft's `axis_value`.
    /// Without it an unset axis shows its default.
    pub fn drawn(mut self, weight: f32, italic: bool) -> Self {
        self.drawn = Drawn { weight: Some(weight), italic };
        self
    }

    /// Show a named-instance list above the sliders.
    pub fn instances(mut self, instances: &'a [NamedInstance]) -> Self {
        self.instances = instances;
        self
    }

    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:189 `axis_sliders` and `:166 axis_value`.
    /// Draw the sliders (and the instance list, if any) and report the edits made this frame.
    pub fn show(self, ui: &mut Ui) -> AxisSlidersResponse {
        let labels = AxisSlidersLabels::of(ui);
        let numbers = LocaleNumbers::of(ui);
        let mut out = AxisSlidersResponse::default();
        let AxisSliders { axes, instances, values, drawn, salt } = self;

        if !instances.is_empty() {
            let current: Vec<([u8; 4], f32)> = axes.iter().map(|a| (a.tag, value_of(values, a, drawn))).collect();
            let mut picked = matching_instance(instances, &current)
                .map_or(usize::MAX, |m| instances.iter().position(|i| std::ptr::eq(i, m)).unwrap_or(usize::MAX));
            let before = picked;
            let mut select = Select::new((salt, "instances"), labels.style.clone(), &mut picked).placeholder(labels.custom.clone());
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
                    out.edits.push((tag, EditEvent::set(value, Modifiers::NONE)));
                }
                out.instance = Some(instance.clone());
            }
        }

        for axis in axes.iter().filter(|a| !a.hidden && a.tag != *b"opsz" && a.min.is_finite() && a.max.is_finite() && a.min < a.max) {
            let wide = axis.max - axis.min >= 10.0;
            let value = value_of(values, axis, drawn);
            let row = SliderRow::new((salt, axis.tag), label_in(axis, &labels), value, f64::from(axis.min)..=f64::from(axis.max), &numbers)
                .step(if wide { 1.0 } else { 0.01 })
                .decimals(if wide { 0 } else { 2 })
                .default_value(axis.default.clamp(axis.min, axis.max))
                .show(ui);
            for event in row.edits() {
                let event = event.map(|v| (v as f32).clamp(axis.min, axis.max));
                if let Some(&v) = event.value() {
                    set_value(values, axis.tag, v);
                }
                out.edits.push((axis.tag, event));
            }
        }
        out
    }
}

/// The value an axis shows: the stored one, else what the style draws, else the axis default,
/// within the axis range.
fn value_of(values: &[([u8; 4], f32)], axis: &FontAxis, drawn: Drawn) -> f32 {
    let default = if axis.default.is_finite() { axis.default } else { axis.min };
    let stored = values.iter().find(|(t, _)| *t == axis.tag).map(|(_, v)| *v).filter(|v| v.is_finite());
    let styled = match &axis.tag {
        b"wght" => drawn.weight.filter(|w| w.is_finite()),
        b"ital" if drawn.weight.is_some() && drawn.italic => Some(axis.max).filter(|m| m.is_finite()),
        _ => None,
    };
    let (lo, hi) = if axis.min <= axis.max { (axis.min, axis.max) } else { (axis.max, axis.min) };
    stored.or(styled).unwrap_or(default).clamp(lo, hi)
}

fn set_value(values: &mut Vec<([u8; 4], f32)>, tag: [u8; 4], value: f32) {
    match values.iter_mut().find(|(t, _)| *t == tag) {
        Some(slot) => slot.1 = value,
        None => values.push((tag, value)),
    }
}
