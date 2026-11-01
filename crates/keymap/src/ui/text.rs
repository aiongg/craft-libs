//! The dialog's words: its own strings and counts, command labels, menu path parts, key names and
//! text direction, read from the context's [`Words`] and [`Host`].

use std::borrow::Cow;
use std::sync::Arc;

use craft_host::Host;
use craft_ui::localized::words::{Counted, Word, Words};

use crate::chord::{Key, Platform, Trigger};
use crate::keymap::Bindable;
use crate::parity::ReturnGlyph;

/// What the dialog says in one frame: the words installed on the context, the Host's command
/// labels and the interface direction. English where nothing is installed.
pub(crate) struct Say {
    words: Arc<Words>,
    ctx: egui::Context,
    host: Option<Arc<Host>>,
    rtl: bool,
}

impl Say {
    pub(crate) fn of_ctx(ctx: &egui::Context) -> Say {
        Say { words: Words::of_ctx(ctx), ctx: ctx.clone(), host: Host::try_of_ctx(ctx), rtl: craft_ui::direction::is_rtl(ctx) }
    }

    /// One of the dialog's own strings, by English source text (listed in [`WORDS`]). Placeholders
    /// (`{chord}`, `{label}`, `{set}`, `{n}`) are filled after the lookup.
    pub(crate) fn ui(&self, source: &'static str) -> Cow<'static, str> {
        self.words.get(Word::plain(source))
    }

    /// A count's wording with `{n}` filled in, in the locale's plural forms and digits.
    pub(crate) fn count(&self, counted: Counted, n: u64) -> String {
        self.words.counted(&self.ctx, counted, &self.words.get(counted.word), n)
    }

    /// A command's label.
    pub(crate) fn command(&self, b: &Bindable) -> String {
        match &self.host {
            Some(host) => host.commands().label(&b.command.id, &b.label),
            None => b.label.clone(),
        }
    }

    /// A menu path part or a scope name: the Host's catalog row for it, if any.
    pub(crate) fn segment<'a>(&'a self, s: &'a str) -> Cow<'a, str> {
        match self.host.as_deref().and_then(|host| Some(host.i18n()?.t(host.lang(), s))) {
            Some(text) => text.into(),
            None => s.into(),
        }
    }

    /// Extra lines for a command's tooltip.
    pub(crate) fn hint(&self, b: &Bindable) -> Option<String> {
        self.host.as_ref()?.commands().hint(&b.command.id)
    }

    /// A key cap's name on `platform` (`Strg`, `Umschalt`); `None` keeps the crate's name.
    pub(crate) fn key(&self, k: Key, p: Platform) -> Option<String> {
        key_name(&self.words, k, p)
    }

    /// A trigger as the platform shows it, with this language's key names.
    pub(crate) fn trigger(&self, trigger: &Trigger, platform: Platform) -> String {
        trigger_text(&self.words, trigger, platform)
    }

    /// A key's spoken name: the `key` row for its Windows and Linux name, on every platform.
    pub(crate) fn key_word(&self, k: Key) -> String {
        self.words.get(Word::in_context("key", k.text_name())).into_owned()
    }

    /// Whether the interface is drawn right to left.
    pub(crate) fn rtl(&self) -> bool {
        self.rtl
    }

    #[cfg(test)]
    pub(crate) fn with_host(host: Host) -> Say {
        Say { words: Arc::new(Words::english()), ctx: egui::Context::default(), host: Some(Arc::new(host)), rtl: false }
    }

    #[cfg(test)]
    pub(crate) fn english() -> &'static Say {
        static ENGLISH: std::sync::LazyLock<Say> =
            std::sync::LazyLock::new(|| Say { words: Arc::new(Words::english()), ctx: egui::Context::default(), host: None, rtl: false });
        &ENGLISH
    }
}

fn key_name(words: &Words, k: Key, p: Platform) -> Option<String> {
    // Glyphs (⌫, ↑) need no translation.
    if p.is_mac() && k.mac_symbol(ReturnGlyph::default()) != k.text_name() {
        return None;
    }
    let english = k.text_name();
    let name = words.get(Word::in_context("key", english));
    (name != english).then(|| name.into_owned())
}

/// A trigger as `platform` shows it, with key names from the `key` rows of `words`. Modifiers come
/// in the order of [`Chord::display`](crate::Chord::display).
pub fn trigger_text(words: &Words, trigger: &Trigger, platform: Platform) -> String {
    let key = trigger.chord.key;
    let glyph = ReturnGlyph::default();
    let one = trigger.chord.display_with_return(platform, glyph);
    let Some(name) = key_name(words, key, platform) else {
        return if trigger.double { format!("{one} {one}") } else { one };
    };
    let base = if platform.is_mac() { key.mac_symbol(glyph) } else { key.text_name() };
    let one = match one.strip_suffix(base) {
        Some(mods) => format!("{mods}{name}"),
        None => one,
    };
    if trigger.double { format!("{one} {one}") } else { one }
}

/// `s` with `chord`, text inside it, left as it is. Right to left, the parts before and after the
/// chord change sides.
pub(crate) fn around_chord(rtl: bool, s: &str, chord: &str) -> String {
    let Some(i) = s.find(chord).filter(|_| !chord.is_empty()) else { return s.to_string() };
    let (before, rest) = s.split_at(i);
    let after = &rest[chord.len()..];
    if rtl { format!("{after}{chord}{before}") } else { format!("{before}{chord}{after}") }
}

/// The dialog's counts: `one` and `word` (the plural) are the English forms, the key of
/// craft-i18n's `@plural` rows.
pub const COUNTS: &[Counted] = &[
    counts::CLASHES,
    counts::OTHER_SCOPE,
    counts::UNUSABLE,
    counts::IGNORED,
    counts::WILL_CHANGE,
    counts::NOT_MATCHED,
    counts::WITHOUT_SHORTCUT,
    counts::UNREADABLE,
    counts::APPROXIMATE,
    counts::CUT,
    counts::CHANGED,
];

const COUNT_WORDS: &[Word] = &[
    counts::CLASHES.word,
    counts::OTHER_SCOPE.word,
    counts::UNUSABLE.word,
    counts::IGNORED.word,
    counts::WILL_CHANGE.word,
    counts::NOT_MATCHED.word,
    counts::WITHOUT_SHORTCUT.word,
    counts::UNREADABLE.word,
    counts::APPROXIMATE.word,
    counts::CUT.word,
    counts::CHANGED.word,
];

/// The count words, written with [`Say::count`].
pub(crate) mod counts {
    use super::Counted;

    pub(crate) const CLASHES: Counted =
        Counted::new("shortcuts.clashes", "{n} shortcut is assigned more than once", "{n} shortcuts are assigned more than once");
    pub(crate) const OTHER_SCOPE: Counted =
        Counted::new("shortcuts.otherScope", "{n} shortcut also works in another scope", "{n} shortcuts also work in another scope");
    pub(crate) const UNUSABLE: Counted =
        Counted::new("shortcuts.unusable", "{n} saved shortcut could not be used", "{n} saved shortcuts could not be used");
    pub(crate) const IGNORED: Counted =
        Counted::new("shortcuts.ignored", "{n} entry was ignored: too many commands", "{n} entries were ignored: too many commands");
    pub(crate) const WILL_CHANGE: Counted = Counted::new("shortcuts.willChange", "{n} command will change", "{n} commands will change");
    pub(crate) const NOT_MATCHED: Counted = Counted::new("shortcuts.notMatched", "{n} item not matched", "{n} items not matched");
    pub(crate) const WITHOUT_SHORTCUT: Counted =
        Counted::new("shortcuts.withoutShortcut", "{n} item without a shortcut", "{n} items without a shortcut");
    pub(crate) const UNREADABLE: Counted = Counted::new("shortcuts.unreadable", "{n} item could not be read", "{n} items could not be read");
    pub(crate) const APPROXIMATE: Counted = Counted::new("shortcuts.approximate", "{n} key read approximately", "{n} keys read approximately");
    pub(crate) const CUT: Counted = Counted::new("shortcuts.cut", "{n} entry cut at a limit", "{n} entries cut at a limit");
    pub(crate) const CHANGED: Counted = Counted::new("shortcuts.changed", "{n} command changed", "{n} commands changed");
}

/// Every source string the dialog looks up with [`Say::ui`].
const STRING_WORDS: &[Word] = &[
    // Window and footer.
    Word::plain("Keyboard Shortcuts"),
    Word::plain("Custom"),
    Word::plain("OK"),
    Word::plain("Cancel"),
    Word::plain("Close"),
    Word::plain("Undo"),
    Word::plain("Redo"),
    Word::plain("Reset All…"),
    Word::plain("Restore every default shortcut in this set?"),
    Word::plain("Reset All"),
    // Header.
    Word::plain("Show:"),
    Word::plain("Filter:"),
    Word::plain("Search:"),
    Word::plain("Search by key"),
    Word::plain("All Commands"),
    Word::plain("{menu} Menu"),
    Word::plain("Tools"),
    Word::plain("{panel} Panel"),
    Word::plain("Other"),
    Word::plain("Fixed Keys"),
    Word::plain("All"),
    Word::plain("With Shortcut"),
    Word::plain("Changed"),
    Word::plain("Conflicts"),
    Word::plain("No matching commands"),
    Word::plain("(cycles)"),
    Word::plain("{label} (cycles)"),
    Word::plain("{label} (Changed)"),
    // Table and detail pane.
    Word::plain("Command"),
    Word::plain("Commands"),
    Word::plain("Detail pane size"),
    Word::plain("Select a command to see and change its shortcuts"),
    Word::plain("Shortcut"),
    Word::plain("Default"),
    Word::plain("{chord} (Default)"),
    Word::plain("{chord} ({scope})"),
    Word::plain("{label} ({scope})"),
    Word::plain("Application"),
    Word::plain("Current shortcuts:"),
    Word::plain("New shortcut:"),
    Word::plain("New shortcut: {chord}"),
    Word::plain("press keys"),
    Word::plain("{chord} (press again for a double press)"),
    Word::plain("Type…"),
    Word::plain("Context:"),
    Word::plain("Assign"),
    Word::plain("Use Default"),
    Word::plain("Clear"),
    Word::plain("Default: {chord}"),
    Word::plain("None"),
    Word::plain("Currently assigned to:"),
    Word::plain("Not assigned"),
    Word::plain("{label} ({scope}, both work)"),
    Word::plain("Add a shortcut to {label}"),
    Word::plain("Remove {chord} from {label}"),
    Word::plain("Restore the default for {label}"),
    Word::plain("{label}: {chord}"),
    Word::plain("{label}: press keys"),
    Word::plain("Now used by {label}"),
    Word::plain("{label}: {chord} (Now used by {holder})"),
    // Typed shortcuts and saved texts that do not parse.
    Word::plain("the shortcut has no key"),
    Word::plain("unknown modifier {key}"),
    Word::plain("modifier {key} given twice"),
    Word::plain("key sequences are not supported"),
    Word::plain("the scope name is not valid"),
    // On-screen keyboard.
    Word::plain("Keyboard"),
    Word::plain("Keyboard: US"),
    Word::plain("Keyboard: US (macOS)"),
    Word::plain("Control"),
    Word::plain("Option"),
    Word::plain("Shift"),
    Word::plain("Ctrl"),
    Word::plain("Alt"),
    Word::plain("Caps Lock"),
    Word::plain("Win"),
    Word::plain("Super"),
    Word::plain("Panel or context"),
    Word::plain("Both"),
    Word::plain("Modifier on"),
    Word::plain("{chord}: {label}"),
    Word::plain("Click a key to see its shortcuts"),
    Word::plain("Key: {key}"),
    Word::plain("Assign to Selected"),
    Word::plain("Assign {chord} to {label}"),
    Word::plain("Select a command first"),
    Word::plain("Nothing uses this key"),
    // Messages and conflicts.
    Word::plain("⚠ {chord} is used by {label}"),
    Word::plain("{chord} was removed from {label}"),
    Word::plain("Also used by {label}"),
    Word::plain("{chord} is reserved by the system"),
    Word::plain("A command can have at most {n} shortcuts"),
    Word::plain("The scope {scope} is unknown"),
    Word::plain("Too many shortcuts are assigned"),
    Word::plain("This command is unknown"),
    Word::plain("Take It"),
    Word::plain("Accept and Go to Conflict"),
    Word::plain("Keep Both"),
    Word::plain("Go to Conflict"),
    Word::plain("Cancel the new shortcut"),
    Word::plain("Dismiss"),
    // Saved shortcuts that could not be used.
    Word::plain("{key}: no command has this name"),
    Word::plain("{key}: “{text}” is not a shortcut ({error})"),
    Word::plain("{key}: {text} is reserved by the system"),
    Word::plain("{key}: the scope in “{text}” is unknown"),
    Word::plain("{key}: listed more than once"),
    Word::plain("{key}: part of the entry was dropped"),
    // Sets.
    Word::plain("Set:"),
    Word::plain("(modified)"),
    Word::plain("New Set…"),
    Word::plain("Rename…"),
    Word::plain("Delete"),
    Word::plain("Save"),
    Word::plain("Revert Set"),
    Word::plain("Import…"),
    Word::plain("Export"),
    Word::plain("Export…"),
    Word::plain("Summarize…"),
    Word::plain("{title}: {set}"),
    Word::plain("Create"),
    Word::plain("Rename"),
    Word::plain("More"),
    Word::plain("Name:"),
    Word::plain("Based on:"),
    Word::plain("Current shortcuts"),
    Word::plain("Save changes to “{set}”?"),
    Word::plain("Don't Save"),
    Word::plain("Delete the set “{set}”?"),
    Word::plain("Keyboard Shortcuts (JSON)"),
    Word::plain("The name is empty, too long or has control characters"),
    Word::plain("A set with this name exists"),
    Word::plain("The set was not found"),
    Word::plain("Built-in sets cannot be changed"),
    Word::plain("There are too many sets"),
    Word::plain("This is not a shortcut set file"),
    Word::plain("This is not a craft-keymap file"),
    Word::plain("This file is from a newer version (format {n})"),
    Word::plain("The file is too large"),
    // Import review.
    Word::plain("Import “{set}”"),
    Word::plain("Set name:"),
    Word::plain("Import"),
    Word::plain("Cancel Import"),
    Word::plain("line {n}"),
    Word::plain("Will change"),
    Word::plain("Not matched"),
    Word::plain("Unreadable"),
    Word::plain("Approximated"),
    Word::plain("{title} ({n})"),
    Word::plain("Suggestions ({n})"),
    Word::plain("numeric keypad key read as main key"),
    Word::plain("Matched without change"),
    Word::plain("Command for {item}"),
    Word::plain("Choose a command…"),
    Word::plain("no command has this name"),
    Word::plain("more than one command fits"),
    Word::plain("in a context that is not imported"),
    Word::plain("only a command of the other kind fits (tool or menu command)"),
    Word::plain("empty shortcut"),
    Word::plain("the shortcut is too long"),
    Word::plain("unknown key {key}"),
    Word::plain("unsupported modifier {key}"),
    Word::plain("{label} keeps its shortcuts (matched by a panel menu name)"),
    Word::plain("Import as New Set"),
    Word::plain("Imported"),
    Word::plain("Imported as “{set}”"),
    Word::plain("Skipped"),
    Word::plain("Taken from other commands"),
    Word::plain("Not changed"),
    Word::plain("taken by {label}"),
    Word::plain("taken by a later entry"),
    Word::plain("reserved by the system"),
    Word::plain("the scope is unknown"),
    Word::plain("the set's shortcuts could not be added"),
    Word::plain("the shortcuts could not be removed"),
    Word::plain("Copy Report"),
    Word::plain("Done"),
    // Why a `.kys` file is refused (`KysError`), and the tooltip of a saved text that is not a shortcut.
    Word::plain("The file is larger than a keyboard shortcut set can be."),
    Word::plain("The file is not a Photoshop keyboard shortcut set: {message}"),
    Word::plain("This saved text is not a shortcut and is not used."),
];

impl crate::import::kys::KysError {
    /// The error's sentence in the installed language: [`Display`](std::fmt::Display) in English.
    pub fn text(&self, words: &Words) -> String {
        use crate::import::kys::KysError;
        match self {
            KysError::TooLarge => words.get(Word::plain("The file is larger than a keyboard shortcut set can be.")).into_owned(),
            KysError::NotASet(message) => {
                craft_i18n::fmt(&words.get(Word::plain("The file is not a Photoshop keyboard shortcut set: {message}")), &[("message", message)])
            }
        }
    }
}

/// Every word the dialog looks up, for the shared catalog.
pub const WORDS: &[&[Word]] = &[STRING_WORDS, COUNT_WORDS, <super::import_offer::ImportOfferLabels as craft_ui::localized::Labels>::WORDS];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::Chord;

    fn german() -> Words {
        Words::new(|w| (w.context == "key" && w.english == "Delete").then(|| "Entf".to_string()))
    }

    #[test]
    fn trigger_text_uses_localized_key_names() -> Result<(), crate::ParseError> {
        let t = Trigger::parse("Cmd+Shift+Delete")?;
        assert_eq!(trigger_text(&german(), &t, Platform::Windows), "Ctrl+Shift+Entf");
        assert_eq!(trigger_text(&german(), &Trigger::parse("F7 F7")?, Platform::Windows), "F7 F7");
        assert_eq!(trigger_text(&Words::english(), &Trigger::single(Chord::parse("⇧⌘Z")?), Platform::Mac), "⇧⌘Z");
        Ok(())
    }

    /// Every `ui("…")` literal in the dialog's sources is listed.
    #[test]
    fn every_looked_up_string_is_listed() {
        let sources = [
            include_str!("mod.rs"),
            include_str!("state.rs"),
            include_str!("rows.rs"),
            include_str!("capture.rs"),
            include_str!("view.rs"),
            include_str!("keyboard.rs"),
            include_str!("style.rs"),
            include_str!("sets.rs"),
            include_str!("review.rs"),
            include_str!("summary.rs"),
        ];
        let mut missing = Vec::new();
        for src in sources {
            let src = src.split("#[cfg(test)]").next().unwrap_or(src);
            for (i, open) in src.match_indices(".ui(\"") {
                let rest = &src[i + open.len()..];
                if let Some(end) = rest.find('"') {
                    let s = &rest[..end];
                    if !STRING_WORDS.contains(&Word::plain(s)) {
                        missing.push(s.to_string());
                    }
                }
            }
        }
        assert!(missing.is_empty(), "not in STRING_WORDS: {missing:?}");
        let mut sorted: Vec<&str> = STRING_WORDS.iter().map(|w| w.english).collect();
        let len = sorted.len();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), len, "STRING_WORDS has duplicates");
    }
}
