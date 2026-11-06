//! The Summary's list of options that differ from the chosen preset, and the artboard range.

use craft_pdf::settings::PdfSettings;
use serde_json::Value;

use super::words::PdfExportLabels;

/// The section (0-based, in the section list's order) an option belongs to.
fn section_of(path: &str) -> usize {
    match path.split('.').next().unwrap_or_default() {
        "compression" => 1,
        "marks" | "bleed" => 2,
        "output" => 3,
        "advanced" => 4,
        "security" => 5,
        _ => 0,
    }
}

/// The words of one setting, by its JSON path (`compression.color.ppi`); `None` for a setting
/// without a row.
fn leaf_label<'a>(labels: &'a PdfExportLabels, path: &str) -> Option<&'a str> {
    let leaf = path.rsplit('.').next().unwrap_or(path);
    let word = match (path, leaf) {
        (_, "standard") => &labels.standard,
        (_, "compatibility") => &labels.compatibility,
        (_, "preserveEditing") => &labels.preserve_editing,
        (_, "thumbnails") => &labels.thumbnails,
        (_, "fastWebView") => &labels.fast_web_view,
        (_, "viewAfterSaving") => &labels.view_after_saving,
        (_, "createLayers") => &labels.create_layers,
        (_, "includeNonPrinting") => &labels.include_non_printing,
        (_, "flattenerPreset") => &labels.flattener_preset,
        (_, "downsample") => &labels.downsampling,
        (_, "ppi") => &labels.to,
        (_, "abovePpi") => &labels.above,
        (_, "compression") => &labels.compression,
        (_, "quality") => &labels.quality,
        (_, "compressText") => &labels.compress_text,
        (_, "trim") => &labels.trim_marks,
        (_, "registration") => &labels.registration_marks,
        (_, "colorBars") => &labels.color_bars,
        (_, "pageInfo") => &labels.page_information,
        (_, "kind") => &labels.mark_type,
        (_, "weight") => &labels.mark_weight,
        (_, "offset") => &labels.mark_offset,
        (_, "useDocument") => &labels.use_document_bleed,
        (_, "top") => &labels.top,
        (_, "bottom") => &labels.bottom,
        (_, "left") => &labels.left,
        (_, "right") => &labels.right,
        (_, "conversion") => &labels.color_conversion,
        (_, "destination") => &labels.destination,
        (_, "profiles") => &labels.profile_inclusion,
        (_, "outputIntent") => &labels.intent_profile,
        (_, "outputCondition") => &labels.output_condition,
        (_, "outputConditionId") => &labels.condition_identifier,
        (_, "registry") => &labels.registry_name,
        (_, "trapped") => &labels.trapped,
        (_, "fontSubsetPercent") => &labels.subset_below,
        (_, "outlineText") => &labels.outline_text,
        (_, "overprint") => &labels.overprint,
        (_, "printing") => &labels.printing_allowed,
        (_, "changes") => &labels.changes_allowed,
        (_, "copy") => &labels.allow_copy,
        (_, "screenReader") => &labels.allow_reader,
        (_, "plaintextMetadata") => &labels.plaintext_metadata,
        _ => return None,
    };
    Some(word)
}

/// `compression.color.abovePpi` as "Compression › Color images › above": the section, the group
/// (`color`, `gray`, `mono` images), then the setting.
pub(super) fn option_label(labels: &PdfExportLabels, path: &str) -> String {
    let names = [
        &labels.section_general,
        &labels.section_compression,
        &labels.section_marks,
        &labels.section_output,
        &labels.section_advanced,
        &labels.section_security,
    ];
    let mut parts = vec![names[section_of(path)].clone()];
    let keys: Vec<&str> = path.split('.').collect();
    if let [_, group, _] = keys.as_slice() {
        parts.push(match *group {
            "color" => labels.color_images.clone(),
            "gray" => labels.gray_images.clone(),
            _ => labels.mono_images.clone(),
        });
    }
    parts.push(leaf_label(labels, path).map_or_else(|| humanize(keys.last().copied().unwrap_or(path)), str::to_owned));
    parts.join(" › ")
}

/// `abovePpi` as "Above Ppi", for a setting without words.
fn humanize(key: &str) -> String {
    let mut out = String::new();
    for (at, c) in key.chars().enumerate() {
        if at == 0 {
            out.extend(c.to_uppercase());
        } else {
            if c.is_uppercase() {
                out.push(' ');
            }
            out.push(c);
        }
    }
    out
}

fn value_text(labels: &PdfExportLabels, path: &str, value: &Value) -> String {
    match value {
        Value::Bool(true) => labels.on.clone(),
        Value::Bool(false) => labels.off.clone(),
        Value::String(id) => {
            let kind = match path.rsplit('.').next().unwrap_or(path) {
                "downsample" => "downsample",
                "quality" => "jpegQuality",
                "kind" => "markKind",
                "conversion" => "colorConversion",
                "profiles" => "profileInclusion",
                "overprint" => "overprint",
                "printing" => "printing",
                "changes" => "changes",
                "standard" => "standard",
                "compression" if path.contains("mono") => "monoCodec",
                "compression" => "imageCodec",
                _ => "",
            };
            labels.choice(kind, id, id)
        }
        other => other.to_string(),
    }
}

fn leaves(prefix: &str, value: &Value, base: &Value, out: &mut Vec<(String, Value)>) {
    match (value, base) {
        (Value::Object(map), Value::Object(base)) => {
            for (key, value) in map {
                let path = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
                leaves(&path, value, base.get(key).unwrap_or(&Value::Null), out);
            }
        }
        _ if value != base => out.push((prefix.to_owned(), value.clone())),
        _ => {}
    }
}

/// One "Section › Setting: value" line for every setting of `settings` that differs from `base`,
/// in the order of the sections. Passwords are never part of the comparison.
pub(super) fn changes(labels: &PdfExportLabels, settings: &PdfSettings, base: &PdfSettings) -> Vec<String> {
    let (Ok(value), Ok(base)) = (serde_json::to_value(settings), serde_json::to_value(base)) else { return Vec::new() };
    let mut diff = Vec::new();
    leaves("", &value, &base, &mut diff);
    diff.sort_by_key(|(path, _)| section_of(path));
    diff.iter().map(|(path, value)| format!("{}: {}", option_label(labels, path), value_text(labels, path, value))).collect()
}

/// Why an artboard range was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RangeError {
    /// `part` is not a number from 1 to the artboard count.
    NotANumber {
        part: String,
    },
    /// A part of only a hyphen.
    NoNumber {
        part: String,
    },
    Backwards {
        part: String,
    },
    Empty,
}

/// Whether `text` (`"1-3, 5"`, 1-based; `all` for every artboard) names artboards of a document
/// of `count`: each number lies in `1..=count`, `3-` runs to the last, `-2` from the first, no
/// part runs backwards.
// vc: crates/doc/src/range.rs:6 parse_range
pub(super) fn check_range(text: &str, count: usize) -> Result<(), RangeError> {
    if text.trim().eq_ignore_ascii_case("all") {
        return Ok(());
    }
    let number = |part: &str, open: usize| -> Result<usize, RangeError> {
        let part = part.trim();
        if part.is_empty() {
            return Ok(open);
        }
        part.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
            .filter(|n| (1..=count).contains(n))
            .ok_or_else(|| RangeError::NotANumber { part: part.to_owned() })
    };
    let mut any = false;
    for part in text.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        let (from, to) = match part.replace('\u{2013}', "-").split_once('-') {
            Some((a, b)) if a.trim().is_empty() && b.trim().is_empty() => return Err(RangeError::NoNumber { part: part.to_owned() }),
            Some((a, b)) => (number(a, 1)?, number(b, count)?),
            None => {
                let n = number(part, 0)?;
                (n, n)
            }
        };
        if from > to {
            return Err(RangeError::Backwards { part: part.to_owned() });
        }
        any = true;
    }
    if any { Ok(()) } else { Err(RangeError::Empty) }
}

impl PdfExportLabels {
    pub(super) fn range_error_text(&self, text: &str, count: usize, error: &RangeError) -> String {
        let count = count.to_string();
        let fill = |word: &str, part: &str| craft_i18n::fmt(word, &[("range", text), ("part", part), ("count", &count)]);
        match error {
            RangeError::NotANumber { part } => fill(&self.range_not_a_number, part),
            RangeError::NoNumber { part } => fill(&self.range_no_number, part),
            RangeError::Backwards { part } => fill(&self.range_backwards, part),
            RangeError::Empty => fill(&self.range_empty, ""),
        }
    }
}
