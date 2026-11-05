//! The Blending Options page.

use egui::{Ui, UiBuilder};

use craft_theme::TypeRole;
use craft_ui::composites::{SectionLabel, SliderRow, SliderRowResponse};
use craft_ui::controls::{BlendChoice, Checkbox};
use craft_ui::edit::parts::Edits;
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Divider, Gap, Stack, Text};

use super::BlendIfBar;
use super::labels::{BlendIfBarLabels, LayerStyleLabels};
use super::page::{StyleOp, apply_op, blend_options, caption_row, pick};
use crate::layer_style::{BlendIfRange, Blending, Knockout, LayerStyle};

/// What the Blending Options page reads (the values come from the edit queue's style).
pub(super) struct BlendingInput<'a> {
    /// The Blend If entry shown.
    pub blend_if_channel: usize,
    /// The document's colour channels.
    pub channel_names: &'a [String],
    pub blend_modes: &'a [BlendChoice],
    pub labels: &'a LayerStyleLabels,
    pub bar_labels: &'a BlendIfBarLabels,
    pub numbers: &'a LocaleNumbers,
    pub enabled: bool,
}

/// What the Blending Options page reports in one pass besides its edits.
#[derive(Default)]
pub(super) struct BlendingOutput {
    /// The Blend If select picked another entry (view state).
    pub blend_if_channel: Option<usize>,
}

/// What a Blending Options control did to the [`Blending`]: the operation a held event replays.
#[derive(Clone, Debug)]
pub(super) enum BlendingOp {
    Blend(String),
    Opacity(f32),
    FillOpacity(f32),
    /// Channel `index` of `of` channels blends or not.
    Channel {
        index: usize,
        of: usize,
        on: bool,
    },
    Knockout(Knockout),
    /// Advanced check `index` (Photoshop's order, [`advanced_checks`]).
    Check {
        index: usize,
        on: bool,
    },
    /// The range of This Layer or Underlying Layer of Blend If entry `entry`.
    BlendIf {
        entry: usize,
        underlying: bool,
        range: BlendIfRange,
    },
}

impl BlendingOp {
    /// Applies the operation to `blending`.
    pub(super) fn apply(&self, blending: &mut Blending) {
        match self {
            BlendingOp::Blend(blend) => blending.blend.clone_from(blend),
            BlendingOp::Opacity(opacity) => blending.opacity = *opacity,
            BlendingOp::FillOpacity(opacity) => blending.fill_opacity = *opacity,
            BlendingOp::Channel { index, of, on } => {
                blending.channels.resize((*of).max(*index + 1), true);
                blending.channels[*index] = *on;
            }
            BlendingOp::Knockout(knockout) => blending.knockout = *knockout,
            BlendingOp::Check { index, on } => (advanced_set(*index))(blending, *on),
            BlendingOp::BlendIf { entry, underlying, range } => {
                if let Some(entry) = blending.blend_if.get_mut(*entry) {
                    if *underlying {
                        entry.underlying = *range;
                    } else {
                        entry.this_layer = *range;
                    }
                }
            }
        }
    }
}

/// Pushes `edit` of a Blending Options control; `op` makes the operation of a control value.
/// `typing` for a number field whose focus is a typing run's.
fn push_blending<T: Clone + Send + Sync + 'static>(
    edits: &mut Edits<LayerStyle, StyleOp>,
    edit: &EditResponse<T>,
    typing: bool,
    op: impl Fn(&T) -> BlendingOp,
) {
    let ops = edit.event.as_ref().and_then(EditEvent::value).map(|v| StyleOp::Blending(op(v)));
    let map = |v: T, base: &LayerStyle| {
        let mut next = base.clone();
        op(&v).apply(&mut next.blending);
        next
    };
    if typing {
        edits.typing(edit, ops, apply_op, map);
    } else {
        edits.push(edit, ops, apply_op, map);
    }
}

/// A slider row's two parts as edits of `op`.
fn push_row(edits: &mut Edits<LayerStyle, StyleOp>, row: &SliderRowResponse, op: impl Fn(f32) -> BlendingOp + Copy) {
    let whole = move |v: &f64| op(v.round() as f32);
    push_blending(edits, &row.field, true, whole);
    push_blending(edits, &row.slider, false, whole);
}

/// Draws Photoshop's Blending Options page:
///
/// - General Blending: Blend Mode (`Select` over `input.blend_modes`; a mode the list lacks, such as an
///   imported Paint.NET mode, is added at the end so it still shows), Opacity 0..=100 %.
/// - Advanced Blending: Fill Opacity 0..=100 %; "Channels:" with one check per name in
///   `input.channel_names` (checked: the channel blends); Knockout (None, Shallow, Deep); the five
///   checks in Photoshop's order (Blend Interior Effects as Group, Blend Clipped Layers as Group,
///   Transparency Shapes Layer, Layer Mask Hides Effects, Vector Mask Hides Effects).
/// - Blend If (hidden with no entries): the "Blend If:" select ([`blend_if_entry_label`] names
///   the entries), then for This Layer and Underlying Layer a caption and a
///   [`BlendIfBar`](super::BlendIfBar) with its readout. A bar's id holds the shown entry, so
///   each entry has its own gesture and focus.
///
/// Sections are divided by hairlines and headed by section labels (`type.label`). The controls
/// draw from `edits`' style and push their edits to it in drawing order; the dialog names the
/// page's AccessKit group.
pub(super) fn blending_page(ui: &mut Ui, input: &BlendingInput<'_>, edits: &mut Edits<LayerStyle, StyleOp>) -> BlendingOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1252-1350 `blending_page`, :1197 `KNOCKOUT`,
    // :1200-1206 `ADVANCED_CHECKS`.
    let mut out = BlendingOutput::default();
    let b = edits.value().blending.clone();
    let (labels, enabled) = (input.labels, input.enabled);
    let id = ui.make_persistent_id("blendingOptionsPage");
    let mut builder = UiBuilder::new().scope_id(id);
    if !enabled {
        builder = builder.disabled();
    }
    ui.scope_builder(builder, |ui| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            SectionLabel::new(labels.general_blending.as_str()).show(ui);
            let response = caption_row(ui, &labels.blend_mode, |ui| {
                let options = blend_options(input.blend_modes, &b.blend);
                pick(ui, (id, "blend"), &labels.blend_mode, b.blend.clone(), options, enabled)
            });
            push_blending(edits, &response, false, |v: &String| BlendingOp::Blend(v.clone()));
            let row = percent_row(ui, (id, "opacity"), &labels.opacity, b.opacity, input.numbers, enabled);
            push_row(edits, &row, BlendingOp::Opacity);
            Divider::horizontal().show(ui);

            SectionLabel::new(labels.advanced_blending.as_str()).show(ui);
            let row = percent_row(ui, (id, "fillOpacity"), &labels.fill_opacity, b.fill_opacity, input.numbers, enabled);
            push_row(edits, &row, BlendingOp::FillOpacity);

            let of = input.channel_names.len();
            let mut channels = b.channels.clone();
            channels.resize(of, true);
            caption_row(ui, &labels.channels, |ui| {
                for (index, name) in input.channel_names.iter().enumerate() {
                    let mut on = channels[index];
                    let response = Checkbox::bool(name.as_str(), &mut on).id_salt((id, "channel", index)).enabled(enabled).show(ui);
                    push_blending(edits, &response, false, |v: &bool| BlendingOp::Channel { index, of, on: *v });
                }
            });

            let knockouts =
                [(Knockout::None, &labels.knockout_none), (Knockout::Shallow, &labels.knockout_shallow), (Knockout::Deep, &labels.knockout_deep)];
            let options = knockouts.iter().map(|(k, text)| (*k, (*text).clone(), false)).collect();
            let response = caption_row(ui, &labels.knockout, |ui| pick(ui, (id, "knockout"), &labels.knockout, b.knockout, options, enabled));
            push_blending(edits, &response, false, |v: &Knockout| BlendingOp::Knockout(*v));

            for (index, (text, get)) in advanced_labels(labels).into_iter().zip(ADVANCED_GETTERS).enumerate() {
                let mut on = get(&b);
                let response = Checkbox::bool(text, &mut on).id_salt((id, "advanced", index)).enabled(enabled).show(ui);
                push_blending(edits, &response, false, |v: &bool| BlendingOp::Check { index, on: *v });
            }
            Divider::horizontal().show(ui);

            let entries = b.blend_if.len();
            if entries == 0 {
                return;
            }
            let shown = input.blend_if_channel.min(entries - 1);
            let names: Vec<_> = (0..entries).map(|i| blend_if_entry_label(i, entries, input.channel_names, &labels.gray)).collect();
            let options = names.iter().enumerate().map(|(i, name)| (i, name.clone(), false)).collect();
            let response = caption_row(ui, &labels.blend_if, |ui| pick(ui, (id, "blendIfChannel"), &labels.blend_if, shown, options, enabled));
            if let Some(&index) = response.event.as_ref().and_then(EditEvent::value) {
                out.blend_if_channel = Some(index);
            }
            let entry = b.blend_if[shown];
            for (caption, underlying) in [(&labels.this_layer, false), (&labels.underlying_layer, true)] {
                ui.add(Text::new(caption.as_str()).role(TypeRole::Label).content(Content::Muted));
                let range = if underlying { entry.underlying } else { entry.this_layer };
                let bar = BlendIfBar::new((id, "blendIf", shown, underlying), caption.as_str(), range)
                    .labels(input.bar_labels.clone())
                    .numbers(input.numbers)
                    .enabled(enabled);
                let response = bar.show(ui);
                push_blending(edits, &response, false, |v: &BlendIfRange| BlendingOp::BlendIf { entry: shown, underlying, range: *v });
            }
        });
    });
    out
}

/// A 0..=100 % slider row, whole percent.
fn percent_row(ui: &mut Ui, id_salt: impl egui::AsIdSalt, label: &str, value: f32, numbers: &LocaleNumbers, enabled: bool) -> SliderRowResponse {
    let row = SliderRow::new(id_salt, label, value, 0.0..=100.0, numbers)
        .unit(Unit::new("{n} %"))
        .decimals(0)
        .step(1.0)
        .default_value(100.0)
        .enabled(enabled);
    row.show(ui)
}

/// The Advanced Blending checks' values, in Photoshop's order.
const ADVANCED_GETTERS: [fn(&Blending) -> bool; 5] = [
    |b| b.blend_interior_effects_as_group,
    |b| b.blend_clipped_layers_as_group,
    |b| b.transparency_shapes_layer,
    |b| b.layer_mask_hides_effects,
    |b| b.vector_mask_hides_effects,
];

/// The setter of Advanced Blending check `index`.
fn advanced_set(index: usize) -> fn(&mut Blending, bool) {
    match index {
        0 => |b, v| b.blend_interior_effects_as_group = v,
        1 => |b, v| b.blend_clipped_layers_as_group = v,
        2 => |b, v| b.transparency_shapes_layer = v,
        3 => |b, v| b.layer_mask_hides_effects = v,
        _ => |b, v| b.vector_mask_hides_effects = v,
    }
}

/// The Advanced Blending checks' labels, in Photoshop's order.
fn advanced_labels(labels: &LayerStyleLabels) -> [&str; 5] {
    [
        &labels.blend_interior_effects,
        &labels.blend_clipped_layers,
        &labels.transparency_shapes_layer,
        &labels.layer_mask_hides_effects,
        &labels.vector_mask_hides_effects,
    ]
}

/// The name of Blend If entry `index` when the style has `entries` entries and the document's
/// channels are `channel_names`: with as many entries as names (a one-channel document) the
/// name itself; otherwise entry 0 is `gray` and entry `i` is name `i − 1`.
pub(super) fn blend_if_entry_label(index: usize, entries: usize, channel_names: &[String], gray: &str) -> String {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1317-1325 (the entry labels).
    let name = |i: usize| channel_names.get(i).cloned().unwrap_or_default();
    if entries == channel_names.len() {
        name(index)
    } else if index == 0 {
        gray.to_owned()
    } else {
        name(index - 1)
    }
}
