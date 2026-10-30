//! The app layer the gallery holds: the base overrides and component pins of an app's
//! `sizes.json`, edited from the token browser and applied to every theme the gallery shows.
//!
//! The file moves as text: it is loaded by dropping it on the gallery (desktop and web) or pasting
//! it into the field, and exported by copying the field's text.

use std::sync::{Arc, Mutex};

use craft_theme::{AppSizes, ControlSize, SizeLayer, SizeMap, SizeOrigin, SizeValue, Tone};
use craft_ui::composites::Notice;
use craft_ui::controls::{Button, Checkbox, NumberField, Segmented, TextField};
use craft_ui::localized::LocaleNumbers;

use egui::{Context, Grid, Ui};

use crate::fields;

/// The part of `sizes.json` an edit goes to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Section {
    #[default]
    All,
    Compact,
    Comfortable,
}

/// A dropped file's name and its contents, or why they could not be read.
type Arrived = (String, Result<Vec<u8>, String>);

/// The rows of the JSON field.
const TEXT_ROWS: usize = 10;

/// The app layer, the text of its file, and what the last load or export reported.
#[derive(Default)]
pub(crate) struct AppLayer {
    sizes: AppSizes,
    text: String,
    section: Section,
    /// The outcome of the last load or export: its text, and whether it is an error.
    status: Option<(String, bool)>,
    /// Dropped files and their contents once read: the web reads them in the background.
    inbox: Arc<Mutex<Vec<Arrived>>>,
}

impl AppLayer {
    /// The overrides to resolve with.
    pub(crate) fn sizes(&self) -> &AppSizes {
        &self.sizes
    }

    fn map_mut(&mut self) -> &mut SizeMap {
        match self.section {
            Section::All => &mut self.sizes.all,
            Section::Compact => &mut self.sizes.compact,
            Section::Comfortable => &mut self.sizes.comfortable,
        }
    }

    fn map(&self) -> &SizeMap {
        match self.section {
            Section::All => &self.sizes.all,
            Section::Compact => &self.sizes.compact,
            Section::Comfortable => &self.sizes.comfortable,
        }
    }

    /// Replaces the layer with the file `bytes`, named `name` in the errors.
    pub(crate) fn load(&mut self, name: &str, bytes: &[u8]) {
        match AppSizes::parse(bytes) {
            Ok(sizes) => {
                self.sizes = sizes;
                self.text = String::from_utf8_lossy(bytes).into_owned();
                self.status = Some((format!("Loaded {name}"), false));
            }
            Err(errors) => self.status = Some((format!("{name}: {errors}"), true)),
        }
    }

    /// The text of the current layer as `sizes.json`; the errors if the values break a rule that
    /// the file format checks.
    fn export(&mut self, ctx: &Context) {
        self.text = self.sizes.to_json();
        self.status = Some(match AppSizes::parse(self.text.as_bytes()) {
            Ok(_) => {
                ctx.copy_text(self.text.clone());
                ("Copied sizes.json to the clipboard".to_owned(), false)
            }
            Err(errors) => (format!("sizes.json would not load: {errors}"), true),
        });
    }

    /// Loads the files dropped on the window, and those whose contents have been read since.
    pub(crate) fn take_dropped(&mut self, ctx: &Context) {
        for file in ctx.input(|i| i.raw.dropped_files.clone()) {
            let name = file.path().file_name().map_or_else(|| "sizes.json".to_owned(), |n| n.to_string_lossy().into_owned());
            let inbox = Arc::clone(&self.inbox);
            let deliver = move |contents: Result<Vec<u8>, String>| {
                if let Ok(mut inbox) = inbox.lock() {
                    inbox.push((name, contents));
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            deliver(file.bytes());
            // A browser reads a file asynchronously.
            #[cfg(target_arch = "wasm32")]
            {
                let ctx = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    deliver(file.bytes_async().await);
                    ctx.request_repaint();
                });
            }
        }
        let arrived = self.inbox.lock().map(|mut inbox| std::mem::take(&mut *inbox)).unwrap_or_default();
        for (name, contents) in arrived {
            match contents {
                Ok(bytes) => self.load(&name, &bytes),
                Err(error) => self.status = Some((format!("{name} could not be read: {error}"), true)),
            }
        }
    }

    /// The load and export controls, the file's text and the outcome of the last action.
    pub(crate) fn file(&mut self, ui: &mut Ui) {
        let count = self.sizes.all.len() + self.sizes.compact.len() + self.sizes.comfortable.len();
        ui.label(format!(
            "{count} overrides: {} for all, {} Compact, {} Comfortable",
            self.sizes.all.len(),
            self.sizes.compact.len(),
            self.sizes.comfortable.len()
        ));
        let edit = TextField::new("gallery.app_layer.text", "sizes.json", &self.text)
            .placeholder("Paste an app's sizes.json, or drop the file on the gallery")
            .multiline(TEXT_ROWS)
            .policy(fields::policy())
            .show(ui);
        fields::apply(&edit, &mut self.text);
        ui.horizontal(|ui| {
            if ui.add(Button::new("Load").size(ControlSize::Sm).id_salt("gallery.app_layer.load")).clicked() {
                let text = std::mem::take(&mut self.text);
                self.load("sizes.json", text.as_bytes());
                if self.status.as_ref().is_some_and(|(_, error)| *error) {
                    self.text = text;
                }
            }
            if ui.add(Button::new("Export").size(ControlSize::Sm).id_salt("gallery.app_layer.export")).clicked() {
                self.export(ui.ctx());
            }
            if ui.add(Button::new("Clear").size(ControlSize::Sm).enabled(!self.sizes.is_empty()).id_salt("gallery.app_layer.clear")).clicked() {
                self.sizes = AppSizes::default();
                self.text.clear();
                self.status = None;
            }
        });
        if let Some((message, error)) = &self.status {
            Notice::new(message.clone()).tone(if *error { Tone::Bad } else { Tone::Good }).id_salt("gallery.app_layer.status").show(ui);
        }
    }

    /// The value of the selected size row and its editor: where the app sets it, in the part of
    /// the file the user chose.
    pub(crate) fn editor(&mut self, ui: &mut Ui, origin: &SizeOrigin) {
        Grid::new("gallery.sizes.facts").num_columns(2).show(ui, |ui| {
            ui.label("Value");
            ui.monospace(value_words(origin.value));
            ui.end_row();
            ui.label("Set by");
            ui.label(layer_words(origin.layer));
            ui.end_row();
            if let SizeValue::Number(_) = origin.value {
                ui.label("Range");
                ui.monospace(format!("{} to {}", origin.range.0, origin.range.1));
                ui.end_row();
            }
        });
        let mut section = self.section;
        Segmented::new("gallery.app_layer.section", "Part of sizes.json", &mut section)
            .item(Section::All, "All")
            .item(Section::Compact, "Compact")
            .item(Section::Comfortable, "Comfortable")
            .size(ControlSize::Sm)
            .show(ui);
        self.section = section;
        let path = origin.path.clone();
        let set = self.map().get(&path).copied();
        let numbers = LocaleNumbers::of(ui);
        match origin.value {
            SizeValue::Number(shown) => {
                let current = match set {
                    Some(SizeValue::Number(n)) => n,
                    _ => shown,
                };
                let edit = NumberField::new(("gallery.app_layer.value", &path), "Override", current, &numbers)
                    .range(origin.range.0..=origin.range.1)
                    .decimals(0..=1)
                    .policy(fields::policy())
                    .show(ui);
                if let Some(&value) = edit.event.as_ref().and_then(|e| e.value()) {
                    self.map_mut().insert(path.clone(), SizeValue::Number(value as f32));
                }
            }
            SizeValue::Switch(shown) => {
                let mut on = match set {
                    Some(SizeValue::Switch(b)) => b,
                    _ => shown,
                };
                let before = on;
                let _ = Checkbox::bool("Override", &mut on).show(ui);
                if on != before {
                    self.map_mut().insert(path.clone(), SizeValue::Switch(on));
                }
            }
        }
        if ui.add(Button::new("Remove override").size(ControlSize::Sm).enabled(set.is_some()).id_salt("gallery.app_layer.remove")).clicked() {
            self.map_mut().remove(&path);
        }
    }
}

/// A value as shown in the list and the detail.
pub(crate) fn value_words(value: SizeValue) -> String {
    match value {
        SizeValue::Number(n) => n.to_string(),
        SizeValue::Switch(true) => "on".to_owned(),
        SizeValue::Switch(false) => "off".to_owned(),
    }
}

/// The layer that set a size, in words.
pub(crate) fn layer_words(layer: SizeLayer) -> &'static str {
    match layer {
        SizeLayer::SizeSet => "size set",
        SizeLayer::App => "app",
        SizeLayer::Derived => "derived",
        SizeLayer::AppPin => "app pin",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loaded_file_is_exported_unchanged_and_a_bad_one_names_its_path() {
        let mut layer = AppLayer::default();
        layer.load("sizes.json", br#"{ "comfortable": { "size.control.md": 28 } }"#);
        assert_eq!(layer.sizes().comfortable.get("size.control.md"), Some(&SizeValue::Number(28.0)));
        let exported = layer.sizes().to_json();
        let mut again = AppLayer::default();
        again.load("sizes.json", exported.as_bytes());
        assert_eq!(again.sizes(), layer.sizes());

        layer.load("bad.json", br#"{ "all": { "size.control.md": 99 } }"#);
        let (message, error) = layer.status.clone().unwrap();
        assert!(error && message.contains("bad.json") && message.contains("all.size.control.md"), "{message}");
        assert_eq!(layer.sizes().comfortable.len(), 1, "a file that fails to load leaves the layer as it was");
    }
}
