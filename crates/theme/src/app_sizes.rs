//! An app's size overrides: layer 2 (base values) and layer 4 (component pins) of the size
//! model. The app ships them as a JSON block read by [`AppSizes::parse`] and hands them to the
//! resolver in [`AppDefaults`](crate::AppDefaults).

use std::collections::BTreeMap;

use super::components::{ComponentTokens, SizeField};
use super::format::{self, FormatErrors};
use super::sizes::{ComponentSizes, Leaf, Preset, Slot, leaves};
use super::theme::Size;

/// The value of one override: a length, or a switch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeValue {
    /// A length in logical pixels; a corner radius of 999 gives pills.
    Number(f32),
    /// A switch, such as `size.list.rounded`.
    Switch(bool),
}

/// Overrides by path, such as `size.control.md` or `toolbar.button`.
pub type SizeMap = BTreeMap<String, SizeValue>;

/// An app's size overrides. A path is either a base value (layer 2: `size.*`, `space.*`,
/// `radius.*`, `typography.<role>.size` and `.line_height`), which the size set's derived
/// component sizes follow, or a component size (layer 4: [`SizeField`]), which changes that one
/// token. A size set's own map wins over `all`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppSizes {
    /// For every size set.
    pub all: SizeMap,
    /// For Compact.
    pub compact: SizeMap,
    /// For Comfortable.
    pub comfortable: SizeMap,
}

/// Where a path leads.
pub(crate) enum Target {
    /// A base value: its path in a size set file.
    Base(String),
    /// A component size.
    Pin(SizeField),
}

/// The base paths an app may override: sizes, spacing, corner radii and type sizes, not the
/// edge, fonts or the set's component values.
pub(crate) fn base_path(path: &str) -> bool {
    ["size.", "space.", "radius."].iter().any(|p| path.starts_with(p))
        || path.strip_prefix("typography.").is_some_and(|rest| rest.ends_with(".size") || rest.ends_with(".line_height"))
}

/// The place `path` leads to, or `None` when it names no override.
pub(crate) fn target(path: &str) -> Option<Target> {
    if base_path(path) {
        let (mut base, mut components) = (Preset::default(), ComponentSizes::default());
        let file_path = format!("base.{path}");
        let found = leaves(&mut base, &mut components).iter().any(|l| l.path == file_path);
        return found.then_some(Target::Base(file_path));
    }
    SizeField::from_path(path).map(Target::Pin)
}

impl AppSizes {
    /// Reads and checks a `sizes.json` block: `{"all": {...}, "compact": {...}, "comfortable":
    /// {...}}`, each a map from path to a number or switch. Rejects an unknown path, a value of
    /// the wrong kind and one out of range, naming the path; apps call it in a test so a bad
    /// file fails their CI.
    pub fn parse(bytes: &[u8]) -> Result<AppSizes, FormatErrors> {
        format::parse_app_sizes(bytes)
    }

    /// The block as `sizes.json` text, which [`AppSizes::parse`] reads back.
    pub fn to_json(&self) -> String {
        format::write_app_sizes(self)
    }

    /// Whether no path is overridden.
    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.compact.is_empty() && self.comfortable.is_empty()
    }

    /// The overrides for `size`: `all`, with the set's own map over it.
    pub(crate) fn merged(&self, size: Size) -> SizeMap {
        let own = match size {
            Size::Compact => &self.compact,
            Size::Comfortable => &self.comfortable,
        };
        let mut out = self.all.clone();
        out.extend(own.iter().map(|(k, v)| (k.clone(), *v)));
        out
    }

    /// Layer 2: the base overrides for `size` on `base`, before anything derives from it.
    pub(crate) fn apply_base(&self, size: Size, base: &mut Preset) {
        let merged = self.merged(size);
        if merged.is_empty() {
            return;
        }
        let mut components = ComponentSizes::default();
        for Leaf { path, slot, .. } in leaves(base, &mut components) {
            let Some(value) = path.strip_prefix("base.").and_then(|p| merged.get(p)) else { continue };
            match (slot, value) {
                (Slot::Num(x) | Slot::Length(x), SizeValue::Number(n)) => *x = *n,
                (Slot::Bool(b), SizeValue::Switch(v)) => *b = *v,
                // `parse` rejects a value of the wrong kind.
                _ => {}
            }
        }
    }

    /// Layer 4: the component pins for `size` on the derived `tokens`.
    pub(crate) fn apply_pins(&self, size: Size, tokens: &mut ComponentTokens) {
        for (path, value) in &self.merged(size) {
            if let (Some(Target::Pin(field)), SizeValue::Number(n)) = (target(path), value) {
                field.set(tokens, *n);
            }
        }
    }
}
