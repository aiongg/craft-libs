//! [`Remembered`]: last-used options of a dialog, panel or tool, kept in the app's settings
//! through the `Host` (craft-host).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::marker::PhantomData;

use craft_host::{Host, Section};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The settings section every remembered value lives in: one TOML table per scope key.
///
/// ```toml
/// [remembered."filter.gaussianBlur"]
/// radius = 2.5
/// [remembered."prompt.layer.delete"]
/// skip = true
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, serde::Deserialize)]
pub struct RememberedSection {
    #[serde(flatten)]
    pub scopes: toml::Table,
}

impl Section for RememberedSection {
    const KEY: &'static str = "remembered";
}

/// A value remembered under a scope key: a dialog's last-used fields (`"filter.gaussianBlur"`),
/// a panel's view options (`"panel.layers"`), a tool's settings (`"tool.crop"`), a prompt's
/// "Don't show again" (`"prompt.layer.delete"`). The value is any serde type; a dialog's is its
/// `DialogFields`.
///
/// ```ignore
/// let last = Remembered::<DialogFields>::new("filter.gaussianBlur");
/// let fields = last.recall(&host).unwrap_or_default();
/// // … the dialog runs; on OK:
/// last.store(&host, dialog.fields());
/// ```
///
/// - Stored on the dialog's OK only; Cancel keeps the previous value (Photoshop, PhotoCraft).
/// - A value that no longer deserializes (the type changed) reads as `None`.
/// - Preferences' "Reset All Warning Dialogs" calls [`forget_prefix`](Remembered::forget_prefix)
///   with `"prompt."`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remembered<T> {
    scope: String,
    _value: PhantomData<fn() -> T>,
}

impl<T: Serialize + DeserializeOwned> Remembered<T> {
    pub fn new(scope: impl Into<String>) -> Self {
        Self { scope: scope.into(), _value: PhantomData }
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The stored value, or `None` when there is none or it does not fit `T`.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:345 `restore_remembered`
    // (`prefs.dialogs[command]`); here through `Host::setting::<RememberedSection>`.
    pub fn recall(&self, host: &Host) -> Option<T> {
        todo!()
    }

    /// Stores `value` under the scope, replacing the previous one.
    // pc: crates/ui-egui/src/dialogs/filter_dialog.rs:363 `remember` (`prefs.edit(|p| p.dialogs.insert(..))`).
    pub fn store(&self, host: &Host, value: &T) {
        todo!()
    }

    /// Removes the stored value.
    pub fn forget(&self, host: &Host) {
        todo!()
    }

    /// Removes every scope that starts with `prefix` (`"prompt."` resets all warning dialogs).
    // new: Photoshop's Preferences › General › Reset All Warning Dialogs.
    pub fn forget_prefix(host: &Host, prefix: &str) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    // `store` then `recall` through a `MemorySections` host returns the value; another scope is
    // untouched; `forget` makes it `None`.
    #[test]
    #[ignore = "skeleton"]
    fn a_stored_value_is_recalled_under_its_scope_only() {
        todo!()
    }

    // A table that does not fit `T` recalls as `None` instead of failing.
    #[test]
    #[ignore = "skeleton"]
    fn a_value_of_another_shape_reads_as_none() {
        todo!()
    }

    // `forget_prefix("prompt.")` removes the prompt skips and keeps the filter values.
    #[test]
    #[ignore = "skeleton"]
    fn forget_prefix_removes_only_the_matching_scopes() {
        todo!()
    }
}
