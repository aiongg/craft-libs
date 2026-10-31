//! Command words for the Host: labels from a craft-i18n catalog.

use std::sync::Arc;

use craft_host::Commands;
use craft_i18n::{I18n, Lang};

type HintSource = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// [`Commands`] over a craft-i18n catalog: a command's label is the catalog's `@id` row for its id
/// in `lang`, the English label where there is none. Build one with the language the Host is
/// built for. It does not hold the [`Keymap`](crate::Keymap).
#[derive(Clone)]
pub struct KeymapCommands {
    i18n: Arc<I18n>,
    lang: Lang,
    hints: Option<HintSource>,
}

impl KeymapCommands {
    pub fn new(i18n: Arc<I18n>, lang: Lang) -> Self {
        KeymapCommands { i18n, lang, hints: None }
    }

    /// Tooltip lines for a command id, from the app's own source; without one there are none.
    #[must_use]
    pub fn with_hints(mut self, hints: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Self {
        self.hints = Some(Arc::new(hints));
        self
    }
}

impl Commands for KeymapCommands {
    fn label(&self, id: &str, english: &str) -> String {
        self.i18n.tid(self.lang, id, english).to_owned()
    }

    fn hint(&self, id: &str) -> Option<String> {
        self.hints.as_ref()?(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static CATALOG: &craft_i18n::Embedded = &[("de", "@id\tfile.save\tSichern\n", "")];

    #[test]
    fn labels_come_from_the_id_rows_and_hints_from_the_source() {
        let (i18n, _) = I18n::new(CATALOG, &["de"]);
        let lang = i18n.from_code("de").unwrap_or(Lang::EN);
        let commands = KeymapCommands::new(Arc::new(i18n), lang);
        assert_eq!(commands.label("file.save", "Save"), "Sichern");
        assert_eq!(commands.label("file.open", "Open"), "Open");
        assert_eq!(commands.hint("file.save"), None);
        let commands = commands.with_hints(|id| (id == "file.save").then(|| "Writes the file".to_string()));
        assert_eq!(commands.hint("file.save").as_deref(), Some("Writes the file"));
        assert_eq!(commands.hint("file.open"), None);
    }
}
