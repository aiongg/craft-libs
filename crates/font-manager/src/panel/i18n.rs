//! Interface strings through craft-i18n: the English text is the lookup key.

use std::cell::RefCell;
use std::sync::{Arc, LazyLock};

use craft_host::Host;
use craft_i18n::{Embedded, FileSize, I18n, Lang};

/// The embedded translation catalogs: the panel's own Japanese and Arabic rows, until its rows
/// are part of craft-i18n.
static CATALOGS: &Embedded = &[("ja", "", include_str!("catalogs/ja.tsv")), ("ar", "", include_str!("catalogs/ar.tsv"))];

/// The interface languages offered, in menu order.
pub const LANGUAGES: &[&str] = &["en", "ja", "ar"];

pub static I18N: LazyLock<I18n> = LazyLock::new(|| {
    let (i18n, problems) = I18n::new(CATALOGS, LANGUAGES);
    for problem in problems {
        log::warn!("translation catalog: {problem}; that string shows in English");
    }
    i18n
});

/// What the frame being drawn follows: the Host's language, as one of the panel's, and the Host's
/// number format.
struct Frame {
    lang: Lang,
    host: Option<Arc<Host>>,
}

thread_local! {
    static FRAME: RefCell<Frame> = const { RefCell::new(Frame { lang: Lang::EN, host: None }) };
}

/// Makes the strings and numbers of the frame about to be drawn follow the Host installed in `ctx`
/// (`host.lang()` in its catalogs); English and the panel's own number format without a Host or
/// when the Host has no catalog for the language.
pub fn follow(ctx: &egui::Context) -> Option<Arc<Host>> {
    let host = Host::try_of_ctx(ctx);
    let lang = host.as_deref().and_then(|h| h.i18n().map(|i| i.code(h.lang()))).and_then(|code| I18N.match_tag(code)).unwrap_or(Lang::EN);
    FRAME.with_borrow_mut(|f| *f = Frame { lang, host: host.clone() });
    host
}

/// The interface language of the frame.
pub fn current() -> Lang {
    FRAME.with_borrow(|f| f.lang)
}

/// The Host's language code (`ja`, `pt-br`) as the panel's catalogs know it; `en` without one.
pub fn code_in(ctx: &egui::Context) -> String {
    follow(ctx);
    I18N.code(current()).to_owned()
}

/// Translates a literal.
macro_rules! tl {
    ($s:expr) => {
        $crate::panel::i18n::I18N.t($crate::panel::i18n::current(), $s)
    };
}
pub(crate) use tl;

/// Translates `template` and fills its `{name}` placeholders.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    craft_i18n::fmt(I18N.t(current(), template), args)
}

/// A count with its noun, `{n}` in the English forms.
pub fn tn(n: u64, one: &str, other: &str) -> String {
    I18N.trn(current(), n, one, other)
}

/// A download size, `1.5 MB`.
pub fn size(bytes: u64) -> String {
    match with_host_catalogs(|i18n, lang| i18n.file_size(lang, bytes, FileSize::DECIMAL)) {
        Some(text) => text,
        None => I18N.file_size(current(), bytes, FileSize::DECIMAL),
    }
}

/// A count on its own, in the region's digits and grouping.
pub fn number(n: u64) -> String {
    with_host_catalogs(|i18n, _| i18n.format().integer(n)).unwrap_or_else(|| I18N.format().integer(n))
}

/// Runs `f` with the Host's catalogs and language, when the frame has a Host with catalogs. Sizes
/// and numbers are written in the Host's region.
fn with_host_catalogs<T>(f: impl FnOnce(&I18n, Lang) -> T) -> Option<T> {
    let host = FRAME.with_borrow(|frame| frame.host.clone())?;
    host.i18n().map(|i18n| f(i18n, host.lang()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row reads. The strings the panel shows without a row are listed (run with
    /// `--nocapture`); they show in English, so a missing row never fails.
    #[test]
    fn catalogs_load_and_missing_rows_are_listed() {
        let (_, problems) = I18n::new(CATALOGS, LANGUAGES);
        assert!(problems.is_empty(), "{problems:#?}");
        // English-only by design: style and weight names, licence and product names.
        let english = [
            "Thin",
            "ExtraLight",
            "Light",
            "Regular",
            "Medium",
            "SemiBold",
            "Bold",
            "ExtraBold",
            "Black",
            "Italic",
            "Variable {from}–{to}",
            "SIL Open Font License",
            "Apache License 2.0",
            "Ubuntu Font Licence",
            "Google Fonts",
            "{source}: {error}",
        ];
        let sources = [include_str!("app.rs"), include_str!("browse.rs"), include_str!("details.rs"), include_str!("sources.rs")];
        let mut keys = Vec::new();
        for source in sources {
            for marker in ["tl!(", "fmt("] {
                for part in source.split(marker).skip(1) {
                    if let Some(key) = part.trim_start().strip_prefix('"').and_then(|p| p.find('"').map(|end| &p[..end])) {
                        keys.push(key.to_owned());
                    }
                }
            }
        }
        keys.retain(|k| !english.contains(&k.as_str()));
        assert!(keys.len() > 60, "{}", keys.len());
        for lang in ["ja", "ar"] {
            let lang = I18N.match_tag(lang).unwrap_or(Lang::EN);
            let missing: Vec<_> = keys.iter().filter(|k| I18N.t(lang, k) == k.as_str()).collect();
            if !missing.is_empty() {
                println!("{} is missing {} strings: {missing:#?}", I18N.code(lang), missing.len());
            }
        }
    }
}
