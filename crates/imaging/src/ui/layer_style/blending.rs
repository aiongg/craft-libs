//! The Blending Options page (task AD-9).

use egui::accesskit::Role;
use egui::{Ui, UiBuilder};

use craft_theme::TypeRole;
use craft_ui::composites::{SectionLabel, SliderRow};
use craft_ui::controls::{BlendChoice, Checkbox};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Divider, Gap, Stack, Text};

use super::BlendIfBar;
use super::labels::{BlendIfBarLabels, LayerStyleLabels};
use super::page::{blend_options, caption_row, pick};
use crate::layer_style::{Blending, Knockout};

/// What the Blending Options page reads.
pub(super) struct BlendingInput<'a> {
    pub blending: &'a Blending,
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

/// What the Blending Options page reports in one pass.
#[derive(Default)]
pub(super) struct BlendingOutput {
    /// The page's values after an edit (slider drags as a gesture; every other control a `Set`).
    pub edit: Option<EditEvent<Blending>>,
    /// The Blend If select picked another entry (view state).
    pub blend_if_channel: Option<usize>,
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
///   the entries), then for This Layer and Underlying Layer a caption with the readout and a
///   [`BlendIfBar`](super::BlendIfBar).
///
/// Sections are divided by hairlines and headed by section labels (`type.label`).
pub(super) fn blending_page(ui: &mut Ui, input: &BlendingInput<'_>) -> BlendingOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1252-1350 `blending_page`, :1197 `KNOCKOUT`,
    // :1200-1206 `ADVANCED_CHECKS`.
    let mut out = BlendingOutput::default();
    let (b, labels, enabled) = (input.blending, input.labels, input.enabled);
    let id = ui.make_persistent_id("blendingOptionsPage");
    let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(labels.blending_options.as_str());
    if !enabled {
        builder = builder.disabled();
    }
    // Keeps the first edit of the pass.
    let mut report = |event: Option<EditEvent<Blending>>| {
        if out.edit.is_none() {
            out.edit = event;
        }
    };
    let mut channel_pick = None;
    ui.scope_builder(builder, |ui| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            SectionLabel::new(labels.general_blending.as_str()).show(ui);
            let event = caption_row(ui, &labels.blend_mode, |ui| {
                let options = blend_options(input.blend_modes, &b.blend);
                pick(ui, (id, "blend"), &labels.blend_mode, b.blend.clone(), options, enabled)
            });
            report(event.map(|e| e.map(|v| edited(b, |n| n.blend = v))));
            report(
                percent_row(ui, (id, "opacity"), &labels.opacity, b.opacity, input.numbers, enabled).map(|e| e.map(|v| edited(b, |n| n.opacity = v))),
            );
            Divider::horizontal().show(ui);

            SectionLabel::new(labels.advanced_blending.as_str()).show(ui);
            let event = percent_row(ui, (id, "fillOpacity"), &labels.fill_opacity, b.fill_opacity, input.numbers, enabled);
            report(event.map(|e| e.map(|v| edited(b, |n| n.fill_opacity = v))));

            let mut channels = b.channels.clone();
            channels.resize(input.channel_names.len(), true);
            caption_row(ui, &labels.channels, |ui| {
                for (i, name) in input.channel_names.iter().enumerate() {
                    let mut on = channels[i];
                    let event = Checkbox::bool(name.as_str(), &mut on).id_salt((id, "channel", i)).enabled(enabled).show(ui).event;
                    report(event.map(|e| {
                        e.map(|v| {
                            let mut next = channels.clone();
                            next[i] = v;
                            edited(b, |n| n.channels = next)
                        })
                    }));
                }
            });

            let knockouts =
                [(Knockout::None, &labels.knockout_none), (Knockout::Shallow, &labels.knockout_shallow), (Knockout::Deep, &labels.knockout_deep)];
            let options = knockouts.iter().map(|(k, text)| (*k, (*text).clone(), false)).collect();
            let event = caption_row(ui, &labels.knockout, |ui| pick(ui, (id, "knockout"), &labels.knockout, b.knockout, options, enabled));
            report(event.map(|e| e.map(|v| edited(b, |n| n.knockout = v))));

            for (i, (text, get, set)) in advanced_checks(labels).into_iter().enumerate() {
                let mut on = get(b);
                let event = Checkbox::bool(text, &mut on).id_salt((id, "advanced", i)).enabled(enabled).show(ui).event;
                report(event.map(|e| e.map(|v| edited(b, |n| set(n, v)))));
            }
            Divider::horizontal().show(ui);

            let entries = b.blend_if.len();
            if entries == 0 {
                return;
            }
            let shown = input.blend_if_channel.min(entries - 1);
            let names: Vec<_> = (0..entries).map(|i| blend_if_entry_label(i, entries, input.channel_names, &labels.gray)).collect();
            let options = names.iter().enumerate().map(|(i, name)| (i, name.clone(), false)).collect();
            let event = caption_row(ui, &labels.blend_if, |ui| pick(ui, (id, "blendIfChannel"), &labels.blend_if, shown, options, enabled));
            if let Some(&index) = event.as_ref().and_then(EditEvent::value) {
                channel_pick = Some(index);
            }
            let entry = b.blend_if[shown];
            for (caption, underlying) in [(&labels.this_layer, false), (&labels.underlying_layer, true)] {
                ui.add(Text::new(caption.as_str()).role(TypeRole::Label).content(Content::Muted));
                let layer = caption.trim_end_matches([':', '\u{ff1a}']);
                let range = if underlying { entry.underlying } else { entry.this_layer };
                let bar = BlendIfBar::new((id, "blendIf", underlying), layer, range).labels(input.bar_labels.clone()).enabled(enabled);
                let event = bar.show(ui).event;
                report(event.map(|e| {
                    e.map(|v| {
                        edited(b, |n| {
                            if let Some(entry) = n.blend_if.get_mut(shown) {
                                if underlying {
                                    entry.underlying = v;
                                } else {
                                    entry.this_layer = v;
                                }
                            }
                        })
                    })
                }));
            }
        });
    });
    out.blend_if_channel = channel_pick;
    out
}

/// `blending` after `change`.
fn edited(blending: &Blending, change: impl FnOnce(&mut Blending)) -> Blending {
    let mut next = blending.clone();
    change(&mut next);
    next
}

/// A 0..=100 % slider row; the first event of the pass, rounded to whole percent.
fn percent_row(ui: &mut Ui, id_salt: impl egui::AsIdSalt, label: &str, value: f32, numbers: &LocaleNumbers, enabled: bool) -> Option<EditEvent<f32>> {
    let row = SliderRow::new(id_salt, label, value, 0.0..=100.0, numbers)
        .unit(Unit::new("{n} %"))
        .decimals(0)
        .step(1.0)
        .default_value(100.0)
        .enabled(enabled);
    row.show(ui).edits().next().map(|e| e.map(|v| v.round() as f32))
}

type AdvancedCheck<'a> = (&'a str, fn(&Blending) -> bool, fn(&mut Blending, bool));

/// The Advanced Blending checks in Photoshop's order: label, getter, setter.
fn advanced_checks(labels: &LayerStyleLabels) -> [AdvancedCheck<'_>; 5] {
    [
        (&labels.blend_interior_effects, |b| b.blend_interior_effects_as_group, |b, v| b.blend_interior_effects_as_group = v),
        (&labels.blend_clipped_layers, |b| b.blend_clipped_layers_as_group, |b, v| b.blend_clipped_layers_as_group = v),
        (&labels.transparency_shapes_layer, |b| b.transparency_shapes_layer, |b, v| b.transparency_shapes_layer = v),
        (&labels.layer_mask_hides_effects, |b| b.layer_mask_hides_effects, |b, v| b.layer_mask_hides_effects = v),
        (&labels.vector_mask_hides_effects, |b| b.vector_mask_hides_effects, |b, v| b.vector_mask_hides_effects = v),
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
