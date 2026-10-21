//! Interface strings through craft-i18n: the English text is the lookup key.

use std::cell::Cell;
use std::sync::LazyLock;

use craft_i18n::{Embedded, FileSize, I18n, Lang};

/// The embedded translation catalogs. The interface is English until the app's catalog rows
/// exist in craft-i18n.
static CATALOGS: &Embedded = &[];

/// The interface languages offered, in menu order.
pub const LANGUAGES: &[&str] = &["en"];

pub static I18N: LazyLock<I18n> = LazyLock::new(|| {
    let (i18n, problems) = I18n::new(CATALOGS, LANGUAGES);
    for problem in problems {
        log::warn!("translation catalog: {problem}; that string shows in English");
    }
    i18n
});

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// The interface language.
pub fn current() -> Lang {
    CURRENT.get()
}

pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
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
    I18N.file_size(current(), bytes, FileSize::DECIMAL)
}
