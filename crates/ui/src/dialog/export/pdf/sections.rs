//! The seven sections of Save PDF and the rows they share.

use std::ops::RangeInclusive;

use craft_pdf::settings::{Choice, ColorConversion, Compatibility, DOCUMENT_CMYK, Downsample, Encryption, ImageCodec, PdfSettings, Standard};
use egui::Ui;

use super::super::web::{self, caption, check_row};
use super::words::PdfExportLabels;
use crate::composites::{Form, FormGrid, SectionLabel};
use crate::controls::expr::ExprUnits;
use crate::controls::{CheckState, Checkbox, NumberField, RadioGroup, Select, TextField};
use crate::edit::EditEvent;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::localized::{LocaleNumbers, Unit};
use crate::patterns::unit_number_field;
use crate::primitives::Content;

/// Trim mark weights the Trim mark weight list offers, in points.
const WEIGHTS: [f64; 3] = [0.125, 0.25, 0.5];

/// The artboards to write: all, or the typed range ("1-3, 5").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ArtboardChoice {
    pub range_only: bool,
    pub range: String,
}

/// The two check boxes that turn a password on. A password that is set keeps its box checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PasswordSwitches {
    pub open: bool,
    pub permissions: bool,
}

/// What the sections read and edit.
pub(super) struct Sections<'a> {
    pub settings: &'a mut PdfSettings,
    pub labels: &'a PdfExportLabels,
    pub numbers: &'a LocaleNumbers,
    /// The unit lengths in points are shown in.
    pub unit: LengthUnit,
    /// The colour profiles the app knows, for Destination and Output intent profile.
    pub profiles: &'a [String],
    /// The names of the saved flattener presets.
    pub flatteners: &'a [String],
    /// The document's bleed in points: top, bottom, left, right.
    pub document_bleed: [f64; 4],
    pub artboards: &'a mut ArtboardChoice,
    pub switches: &'a mut PasswordSwitches,
    /// The warnings of the Summary, already worded.
    pub warnings: &'a [String],
}

impl Sections<'_> {
    pub(super) fn show(&mut self, ui: &mut Ui, section: usize) {
        match section {
            1 => self.compression(ui),
            2 => self.marks_and_bleeds(ui),
            3 => self.output(ui),
            4 => self.advanced(ui),
            5 => self.security(ui),
            6 => self.summary(ui),
            _ => self.general(ui),
        }
    }

    fn general(&mut self, ui: &mut Ui) {
        let labels = self.labels;
        heading(ui, &labels.options);
        let plain = self.settings.standard == Standard::None;
        // PDF layers need PDF 1.5, and PDF/X-1a and PDF/X-3 have none.
        let layers = self.settings.standard.allows_layers() && self.settings.compatibility.has_layers();
        let settings = &mut *self.settings;
        FormGrid::new("pdf-general").show(ui, |form| {
            flag(form, &labels.preserve_editing, &mut settings.preserve_editing, plain);
            flag(form, &labels.thumbnails, &mut settings.thumbnails, true);
            flag(form, &labels.fast_web_view, &mut settings.fast_web_view, true);
            flag(form, &labels.view_after_saving, &mut settings.view_after_saving, true);
            flag(form, &labels.create_layers, &mut settings.create_layers, layers);
            flag(form, &labels.include_non_printing, &mut settings.include_non_printing, true);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.artboards);
        let mut all = !self.artboards.range_only;
        let edit = RadioGroup::new("pdf-artboards", labels.artboards.as_str(), &mut all)
            .item(true, labels.all.as_str())
            .item(false, labels.range.as_str())
            .show(ui);
        drop(edit);
        self.artboards.range_only = !all;
        let range = &mut self.artboards.range;
        let enabled = !all;
        FormGrid::new("pdf-range").show(ui, |form| {
            form.row(&labels.range, |ui, _| {
                let edit = TextField::new("pdf-range-text", labels.range.as_str(), range.as_str())
                    .placeholder(labels.range_hint.as_str())
                    .enabled(enabled)
                    .show(ui);
                if let Some(text) = edit.event.as_ref().and_then(EditEvent::value) {
                    range.clone_from(text);
                }
                edit
            });
        });
    }

    fn compression(&mut self, ui: &mut Ui) {
        let (labels, numbers) = (self.labels, self.numbers);
        let compression = &mut self.settings.compression;
        for (index, (title, image)) in
            [(&labels.color_images, &mut compression.color), (&labels.gray_images, &mut compression.gray)].into_iter().enumerate()
        {
            heading(ui, title);
            FormGrid::new(("pdf-compression", index)).show(ui, |form| {
                resample_rows(form, labels, numbers, index, &mut image.downsample, &mut image.ppi, &mut image.above_ppi);
                pick(form, ("pdf-codec", index), "imageCodec", &labels.compression, labels, &mut image.compression, |_| true);
                let lossy = !matches!(image.compression, ImageCodec::None | ImageCodec::Zip);
                form.set_disabled(!lossy);
                pick(form, ("pdf-jpeg-quality", index), "jpegQuality", &labels.quality, labels, &mut image.quality, |_| true);
                form.set_disabled(false);
            });
        }
        let mono = &mut compression.mono;
        heading(ui, &labels.mono_images);
        FormGrid::new(("pdf-compression", 2)).show(ui, |form| {
            resample_rows(form, labels, numbers, 2, &mut mono.downsample, &mut mono.ppi, &mut mono.above_ppi);
            pick(form, "pdf-mono-codec", "monoCodec", &labels.compression, labels, &mut mono.compression, |_| true);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        FormGrid::new("pdf-compress-text").show(ui, |form| flag(form, &labels.compress_text, &mut compression.compress_text, true));
    }

    fn marks_and_bleeds(&mut self, ui: &mut Ui) {
        let (labels, numbers, unit) = (self.labels, self.numbers, self.unit);
        heading(ui, &labels.marks);
        let marks = &mut self.settings.marks;
        FormGrid::new("pdf-marks").show(ui, |form| {
            let count = [marks.trim, marks.registration, marks.color_bars, marks.page_info].iter().filter(|on| **on).count();
            let mut all = match count {
                0 => CheckState::Unchecked,
                4 => CheckState::Checked,
                _ => CheckState::Mixed,
            };
            let before = all;
            form.unlabelled(|ui| Checkbox::new(labels.all_marks.as_str(), &mut all).show(ui));
            if all != before {
                let on = all == CheckState::Checked;
                (marks.trim, marks.registration, marks.color_bars, marks.page_info) = (on, on, on, on);
            }
            flag(form, &labels.trim_marks, &mut marks.trim, true);
            flag(form, &labels.registration_marks, &mut marks.registration, true);
            flag(form, &labels.color_bars, &mut marks.color_bars, true);
            flag(form, &labels.page_information, &mut marks.page_info, true);
            pick(form, "pdf-mark-kind", "markKind", &labels.mark_type, labels, &mut marks.kind, |_| true);
            let mut weight = WEIGHTS.iter().position(|w| (w - marks.weight).abs() < 1e-9).unwrap_or(WEIGHTS.len());
            let before = weight;
            let custom = weight == WEIGHTS.len();
            form.row(&labels.mark_weight, |ui, _| {
                let mut select = Select::new("pdf-mark-weight", labels.mark_weight.as_str(), &mut weight);
                for (index, w) in WEIGHTS.iter().enumerate() {
                    select = select.option(index, points_text(labels, *w));
                }
                if custom {
                    select = select.option(WEIGHTS.len(), points_text(labels, marks.weight));
                }
                select.show(ui)
            });
            if weight != before && weight < WEIGHTS.len() {
                marks.weight = WEIGHTS[weight];
            }
            length_row(form, "pdf-mark-offset", &labels.mark_offset, &mut marks.offset, unit, numbers, 0.0..=72.0);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.bleeds);
        let bleed = &mut self.settings.bleed;
        let document = self.document_bleed;
        FormGrid::new("pdf-bleed").show(ui, |form| {
            flag(form, &labels.use_document_bleed, &mut bleed.use_document, true);
            // With the document's bleed the fields show it.
            let use_document = bleed.use_document;
            let mut shown = if use_document { document } else { bleed.values() };
            form.set_disabled(use_document);
            for (index, (id, label)) in [
                ("pdf-bleed-top", &labels.top),
                ("pdf-bleed-bottom", &labels.bottom),
                ("pdf-bleed-left", &labels.left),
                ("pdf-bleed-right", &labels.right),
            ]
            .into_iter()
            .enumerate()
            {
                length_row(form, id, label, &mut shown[index], unit, numbers, 0.0..=72.0);
            }
            form.set_disabled(false);
            if !use_document {
                [bleed.top, bleed.bottom, bleed.left, bleed.right] = shown;
            }
        });
    }

    fn output(&mut self, ui: &mut Ui) {
        let (labels, profiles) = (self.labels, self.profiles);
        let standard = self.settings.standard;
        let settings = &mut *self.settings;
        heading(ui, &labels.color);
        let cmyk_only = standard.cmyk_only();
        let output = &mut settings.output;
        FormGrid::new("pdf-color").show(ui, |form| {
            pick(form, "pdf-conversion", "colorConversion", &labels.color_conversion, labels, &mut output.conversion, |_| true);
            // PDF/X-1a converts to CMYK even without a conversion.
            let converting = output.conversion != ColorConversion::None || cmyk_only;
            let mut destinations = vec![(String::new(), labels.document_profile.clone()), (DOCUMENT_CMYK.to_owned(), DOCUMENT_CMYK.to_owned())];
            destinations.extend(profiles.iter().map(|name| (name.clone(), name.clone())));
            form.set_disabled(!converting);
            name_row(form, "pdf-destination", &labels.destination, &mut output.destination, destinations);
            // A standard decides whether colours are tagged.
            form.set_disabled(standard != Standard::None);
            pick(form, "pdf-profiles", "profileInclusion", &labels.profile_inclusion, labels, &mut output.profiles, |_| true);
            form.set_disabled(false);
        });
        if cmyk_only {
            caption(ui, labels.note_cmyk_only.clone(), Content::Muted);
        } else if standard != Standard::None {
            caption(ui, labels.note_tagged.clone(), Content::Muted);
        }
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.output_intent);
        // PDF/A files carry their own output intent; PDF/X files always have one.
        let own = standard != Standard::PdfA2b;
        let blank = if standard.is_pdfx() { &labels.cmyk_in_effect } else { &labels.no_profile };
        let output = &mut settings.output;
        FormGrid::new("pdf-intent").show(ui, |form| {
            form.set_disabled(!own);
            let mut intents = vec![(String::new(), blank.clone())];
            intents.extend(profiles.iter().map(|name| (name.clone(), name.clone())));
            name_row(form, "pdf-intent-profile", &labels.intent_profile, &mut output.output_intent, intents);
            text_row(form, "pdf-condition", &labels.output_condition, &mut output.output_condition);
            text_row(form, "pdf-condition-id", &labels.condition_identifier, &mut output.output_condition_id);
            text_row(form, "pdf-registry", &labels.registry_name, &mut output.registry);
            let mut trapped = output.trapped;
            form.unlabelled(|ui| Checkbox::bool(labels.trapped.as_str(), &mut trapped).show(ui));
            output.trapped = trapped;
            form.set_disabled(false);
        });
        if !own {
            caption(ui, labels.note_pdfa_intent.clone(), Content::Muted);
        }
    }

    fn advanced(&mut self, ui: &mut Ui) {
        let (labels, numbers) = (self.labels, self.numbers);
        heading(ui, &labels.fonts);
        let settings = &mut *self.settings;
        let advanced = &mut settings.advanced;
        FormGrid::new("pdf-fonts").show(ui, |form| {
            form.set_disabled(advanced.outline_text);
            form.row(&labels.subset_below, |ui, _| {
                let edit = NumberField::new("pdf-subset", labels.subset_below.as_str(), advanced.font_subset_percent, numbers)
                    .range(0.0..=100.0)
                    .decimals(0..=2)
                    .unit(Unit::new(&labels.percent))
                    .show(ui);
                if let Some(value) = web::typed(&edit) {
                    advanced.font_subset_percent = value;
                }
                edit
            });
            form.set_disabled(false);
        });
        caption(ui, labels.of_characters.clone(), Content::Muted);
        FormGrid::new("pdf-outline").show(ui, |form| flag(form, &labels.outline_text, &mut advanced.outline_text, true));
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.overprint_flattener);
        // PDF 1.3 files (PDF/X-1a and PDF/X-3 ones too) have no transparency: it is flattened.
        let flat = settings.pdf13();
        let saved = self.flatteners;
        FormGrid::new("pdf-overprint").show(ui, |form| {
            pick(form, "pdf-overprint-choice", "overprint", &labels.overprint, labels, &mut settings.advanced.overprint, |_| true);
            let mut presets = vec![
                ("High Resolution".to_owned(), labels.flatten_high.clone()),
                ("Medium Resolution".to_owned(), labels.flatten_medium.clone()),
                ("Low Resolution".to_owned(), labels.flatten_low.clone()),
            ];
            presets.extend(saved.iter().map(|name| (name.clone(), name.clone())));
            // The empty name and the ids `high`, `medium` and `low` name the built-in presets.
            let mut current = match settings.flattener_preset.trim() {
                "" => "High Resolution".to_owned(),
                id if ["high", "medium", "low"].contains(&id.to_ascii_lowercase().as_str()) => {
                    presets[["high", "medium", "low"].iter().position(|p| p.eq_ignore_ascii_case(id)).unwrap_or(0)].0.clone()
                }
                name => name.to_owned(),
            };
            let before = current.clone();
            form.set_disabled(!flat);
            name_row(form, "pdf-flattener", &labels.flattener_preset, &mut current, presets);
            form.set_disabled(false);
            if current != before {
                settings.flattener_preset = current;
            }
        });
        if !flat {
            caption(ui, labels.note_flatten.clone(), Content::Muted);
        }
    }

    fn security(&mut self, ui: &mut Ui) {
        let labels = self.labels;
        let plain = self.settings.standard == Standard::None;
        if !plain {
            caption(ui, labels.note_standard_password.clone(), Content::Muted);
        }
        let compatibility = self.settings.compatibility;
        let security = &mut self.settings.security;
        let switches = &mut *self.switches;
        let mut open = false;
        let mut restrict = false;
        heading(ui, &labels.open_password_heading);
        FormGrid::new("pdf-open-password").show(ui, |form| {
            open = password_switch(form, &labels.require_open_password, &mut switches.open, &mut security.open_password, plain);
            form.set_disabled(!open);
            password_row(form, "pdf-open-password-field", &labels.open_password, &mut security.open_password);
            form.set_disabled(false);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.permissions);
        FormGrid::new("pdf-permissions").show(ui, |form| {
            restrict = password_switch(form, &labels.restrict_permissions, &mut switches.permissions, &mut security.permissions_password, plain);
            form.set_disabled(!restrict);
            password_row(form, "pdf-permissions-password-field", &labels.permissions_password, &mut security.permissions_password);
            pick(form, "pdf-printing", "printing", &labels.printing_allowed, labels, &mut security.printing, |_| true);
            pick(form, "pdf-changes", "changes", &labels.changes_allowed, labels, &mut security.changes, |_| true);
            flag(form, &labels.allow_copy, &mut security.copy, true);
            // What can be copied can be read out.
            let mut reader = security.screen_reader || security.copy;
            form.unlabelled(|ui| Checkbox::bool(labels.allow_reader.as_str(), &mut reader).enabled(!security.copy).show(ui));
            if !security.copy {
                security.screen_reader = reader;
            }
            form.set_disabled(false);
            let plaintext_ok = (open || restrict) && !matches!(compatibility, Compatibility::Pdf13 | Compatibility::Pdf14);
            flag(form, &labels.plaintext_metadata, &mut security.plaintext_metadata, plaintext_ok);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.row);
        let name = Encryption::for_compatibility(compatibility).label();
        caption(ui, craft_i18n::fmt(&labels.encryption_note, &[("name", name)]), Content::Muted);
    }

    fn summary(&mut self, ui: &mut Ui) {
        let labels = self.labels;
        heading(ui, &labels.options);
        let settings = &*self.settings;
        let choice = |kind: &str, id: &str, fallback: &str| labels.choice(kind, id, fallback);
        let standard = choice("standard", settings.standard.id(), settings.standard.label());
        let encryption = settings.encryption().map_or_else(|| labels.no_encryption.clone(), |e| e.label().to_owned());
        FormGrid::new("pdf-summary").show(ui, |form| {
            form.value(&labels.standard, standard);
            form.value(&labels.compatibility, settings.compatibility.label());
            form.value(&labels.encryption, encryption);
        });
        ui.add_space(crate::theme::current(ui.ctx()).space.gap.section);
        heading(ui, &labels.summary_warnings);
        if self.warnings.is_empty() {
            caption(ui, labels.no_warnings.clone(), Content::Muted);
        }
        for warning in self.warnings {
            ui.label(warning);
        }
    }
}

fn heading(ui: &mut Ui, text: &str) {
    let _ = SectionLabel::new(text).show(ui);
}

/// A check box row; `enabled` off greys it.
fn flag(form: &mut Form<'_>, label: &str, value: &mut bool, enabled: bool) {
    if enabled {
        check_row(form, label, value);
    } else {
        form.unlabelled(|ui| Checkbox::bool(label, value).enabled(false).show(ui));
    }
}

/// A list of the choices of `T`, worded through `labels`; `usable` greys the choices that don't
/// apply.
fn pick<T: Choice + PartialEq + Send + Sync>(
    form: &mut Form<'_>,
    id: impl std::hash::Hash + std::fmt::Debug,
    kind: &str,
    label: &str,
    labels: &PdfExportLabels,
    value: &mut T,
    usable: impl Fn(T) -> bool,
) {
    form.row(label, |ui, _| {
        let mut select = Select::new(("pdf-pick", id), label, value);
        for (at, choice) in T::ALL.iter().enumerate() {
            select = select.option(*choice, labels.choice(kind, T::IDS[at], T::LABELS[at])).option_enabled(usable(*choice));
        }
        select.show(ui)
    });
}

/// A list of names (`value`, `text`); a current value that is not among them is added.
fn name_row(form: &mut Form<'_>, id: &str, label: &str, current: &mut String, mut names: Vec<(String, String)>) {
    if !names.iter().any(|(value, _)| value == current) {
        names.push((current.clone(), current.clone()));
    }
    form.row(label, |ui, _| {
        let mut select = Select::new(id, label, current);
        for (value, text) in names {
            select = select.option(value, text);
        }
        select.show(ui)
    });
}

fn text_row(form: &mut Form<'_>, id: &str, label: &str, value: &mut String) {
    form.row(label, |ui, _| {
        let edit = TextField::new(id, label, value.as_str()).show(ui);
        if let Some(text) = edit.event.as_ref().and_then(EditEvent::value) {
            value.clone_from(text);
        }
        edit
    });
}

fn password_row(form: &mut Form<'_>, id: &str, label: &str, value: &mut String) {
    form.row(label, |ui, _| {
        let edit = TextField::new(id, label, value.as_str()).password(true).show(ui);
        if let Some(text) = edit.event.as_ref().and_then(EditEvent::value) {
            value.clone_from(text);
        }
        edit
    });
}

/// The check box that turns a password on. Unchecking it clears the password. Returns whether the
/// password applies.
fn password_switch(form: &mut Form<'_>, label: &str, switch: &mut bool, password: &mut String, enabled: bool) -> bool {
    let mut on = *switch || !password.is_empty();
    let before = on;
    form.unlabelled(|ui| Checkbox::bool(label, &mut on).enabled(enabled).show(ui));
    if on != before {
        *switch = on;
        if !on {
            password.clear();
        }
    }
    on && enabled
}

/// Downsampling, to and above rows of one kind of image.
fn resample_rows(
    form: &mut Form<'_>,
    labels: &PdfExportLabels,
    numbers: &LocaleNumbers,
    index: usize,
    downsample: &mut Downsample,
    ppi: &mut f64,
    above: &mut f64,
) {
    pick(form, ("pdf-downsample", index), "downsample", &labels.downsampling, labels, downsample, |_| true);
    form.set_disabled(*downsample == Downsample::None);
    for (id, label, value) in [("pdf-ppi", &labels.to, ppi), ("pdf-above-ppi", &labels.above, above)] {
        form.row(label, |ui, _| {
            let edit = NumberField::new((id, index), label.as_str(), *value, numbers)
                .range(9.0..=2400.0)
                .decimals(0..=2)
                .unit(Unit::new(&labels.ppi))
                .show(ui);
            if let Some(typed) = web::typed(&edit) {
                *value = typed;
            }
            edit
        });
    }
    form.set_disabled(false);
}

/// A length stored in points and shown in `unit`.
fn length_row(form: &mut Form<'_>, id: &str, label: &str, points: &mut f64, unit: LengthUnit, numbers: &LocaleNumbers, range: RangeInclusive<f64>) {
    let unit = if unit.is_pixel_unit() { LengthUnit::Points } else { unit };
    let shown = unit.from_pixels(*points, 1.0, 72.0, 72.0);
    let bounds = unit.from_pixels(*range.start(), 1.0, 72.0, 72.0)..=unit.from_pixels(*range.end(), 1.0, 72.0, 72.0);
    form.row(label, |ui, _| {
        let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context: UnitContext::default() };
        let edit = unit_number_field(id, label, shown, numbers, &units).range(bounds).show(ui);
        if let Some(typed) = web::typed(&edit) {
            *points = unit.to_pixels(typed, 1.0, 72.0, 72.0);
        }
        edit
    });
}

fn points_text(labels: &PdfExportLabels, points: f64) -> String {
    craft_i18n::fmt(&labels.points, &[("n", &points.to_string())])
}
