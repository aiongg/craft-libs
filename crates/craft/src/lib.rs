//! The craft libraries in one dependency. Turn on one feature per crate and `ui` for their
//! egui side; import through `craft::<crate>`.

pub mod egui_fork;

#[cfg(feature = "ai-core")]
pub use craft_ai_core as ai_core;
#[cfg(feature = "atomic")]
pub use craft_atomic as atomic;
#[cfg(feature = "automation")]
pub use craft_automation as automation;
#[cfg(feature = "browse")]
pub use craft_browse as browse;
#[cfg(feature = "cms")]
pub use craft_cms as cms;
#[cfg(feature = "color")]
pub use craft_color as color;
#[cfg(feature = "crw")]
pub use craft_crw as crw;
#[cfg(feature = "dialogs")]
pub use craft_dialogs as dialogs;
#[cfg(feature = "docpath")]
pub use craft_docpath as docpath;
#[cfg(feature = "draw")]
pub use craft_draw as draw;
#[cfg(feature = "ecproj-format")]
pub use craft_ecproj_format as ecproj_format;
#[cfg(feature = "font-catalog")]
pub use craft_font_catalog as font_catalog;
#[cfg(feature = "font-manager")]
pub use craft_font_manager as font_manager;
#[cfg(feature = "font-store")]
pub use craft_font_store as font_store;
pub use craft_host as host;
pub use craft_i18n as i18n;
#[cfg(feature = "imaging")]
pub use craft_imaging as imaging;
#[cfg(feature = "jpeg")]
pub use craft_jpeg as jpeg;
#[cfg(feature = "keymap")]
pub use craft_keymap as keymap;
#[cfg(feature = "layers")]
pub use craft_layers as layers;
#[cfg(feature = "media")]
pub use craft_media as media;
#[cfg(feature = "menu")]
pub use craft_menu as menu;
#[cfg(feature = "pages")]
pub use craft_pages as pages;
#[cfg(feature = "pdf")]
pub use craft_pdf as pdf;
#[cfg(feature = "platform")]
pub use craft_platform as platform;
#[cfg(feature = "prepress")]
pub use craft_prepress as prepress;
#[cfg(feature = "print")]
pub use craft_print as print;
#[cfg(feature = "quit")]
pub use craft_quit as quit;
#[cfg(feature = "review")]
pub use craft_review as review;
#[cfg(feature = "settings")]
pub use craft_settings as settings;
#[cfg(feature = "table")]
pub use craft_table as table;
#[cfg(feature = "text")]
pub use craft_text as text;
pub use craft_theme as theme;
#[cfg(feature = "theme-store")]
pub use craft_theme_store as theme_store;
#[cfg(feature = "timeline")]
pub use craft_timeline as timeline;
#[cfg(feature = "ui")]
pub use craft_ui as ui;
#[cfg(feature = "update")]
pub use craft_update as update;
#[cfg(feature = "vectorcraft-format")]
pub use craft_vectorcraft_format as vectorcraft_format;
#[cfg(feature = "window")]
pub use craft_window as window;
#[cfg(feature = "ui")]
pub use egui;

pub mod prelude {
    pub use craft_host::{AppInfo, Host, Section};
    #[cfg(feature = "ui")]
    pub use craft_ui::{
        dialog::{DialogStack, Outcome},
        edit::EditEvent,
        theme::Tokens,
    };
}

/// Every story of every enabled crate.
#[cfg(feature = "ui")]
pub mod story {
    use craft_ui::story::Story;

    /// Every story: the kernel's first, then the family crates alphabetically, each only when its
    /// feature is on.
    pub fn all() -> Vec<Story> {
        let mut all = craft_ui::story::registry();
        #[cfg(feature = "automation")]
        all.extend(craft_automation::ui::stories());
        #[cfg(feature = "browse")]
        all.extend(craft_browse::ui::stories());
        #[cfg(feature = "color")]
        all.extend(craft_color::ui::stories());
        #[cfg(feature = "dialogs")]
        all.extend(craft_dialogs::ui::stories());
        #[cfg(feature = "draw")]
        all.extend(craft_draw::ui::stories());
        #[cfg(feature = "font-manager")]
        all.extend(craft_font_manager::ui::stories());
        #[cfg(feature = "imaging")]
        all.extend(craft_imaging::ui::stories());
        #[cfg(feature = "keymap")]
        all.extend(craft_keymap::ui::stories());
        #[cfg(feature = "layers")]
        all.extend(craft_layers::ui::stories());
        #[cfg(feature = "pages")]
        all.extend(craft_pages::ui::stories());
        #[cfg(feature = "review")]
        all.extend(craft_review::ui::stories());
        #[cfg(feature = "table")]
        all.extend(craft_table::ui::stories());
        #[cfg(feature = "text")]
        all.extend(craft_text::ui::stories());
        #[cfg(feature = "theme-store")]
        all.extend(craft_theme_store::ui::stories());
        #[cfg(feature = "timeline")]
        all.extend(craft_timeline::ui::stories());
        all
    }
}

/// Every labels table of every enabled crate, for the catalog extraction and the missing list.
#[cfg(feature = "ui")]
pub mod words {
    use craft_ui::localized::Word;
    use craft_ui::localized::words::Counted;

    /// Every labels table: the kernel's, then the family crates' alphabetically, each only when
    /// its feature is on. craft-font-manager's fonts panel keeps its words in a catalog of its own;
    /// the picker, Glyphs panel, Missing Fonts dialog and axis sliders are listed.
    pub fn all() -> Vec<&'static [Word]> {
        let mut all: Vec<&'static [Word]> = craft_ui::localized::words::tables().to_vec();
        #[cfg(feature = "automation")]
        all.extend(craft_automation::ui::WORDS);
        #[cfg(feature = "browse")]
        all.extend(craft_browse::ui::WORDS);
        #[cfg(feature = "color")]
        all.extend(craft_color::ui::WORDS);
        #[cfg(feature = "dialogs")]
        all.extend(craft_dialogs::ui::WORDS);
        #[cfg(feature = "draw")]
        all.extend(craft_draw::ui::WORDS);
        #[cfg(feature = "font-manager")]
        all.extend(craft_font_manager::ui::WORDS);
        #[cfg(feature = "imaging")]
        all.extend(craft_imaging::ui::WORDS);
        #[cfg(feature = "keymap")]
        all.extend(craft_keymap::ui::WORDS);
        #[cfg(feature = "layers")]
        all.extend(craft_layers::ui::WORDS);
        #[cfg(feature = "pages")]
        all.extend(craft_pages::ui::WORDS);
        #[cfg(feature = "print")]
        all.extend(craft_print::ui::WORDS);
        #[cfg(feature = "review")]
        all.extend(craft_review::ui::WORDS);
        #[cfg(feature = "table")]
        all.extend(craft_table::ui::WORDS);
        #[cfg(feature = "text")]
        all.extend(craft_text::ui::WORDS);
        #[cfg(feature = "theme-store")]
        all.extend(craft_theme_store::ui::WORDS);
        #[cfg(feature = "timeline")]
        all.extend(craft_timeline::ui::WORDS);
        #[cfg(any(feature = "platform", feature = "dialogs"))]
        all.push(service_messages());
        all
    }

    /// The messages the platform services and the save flow word with `PlatformText`, as one table
    /// of words (the services have no catalog; the app looks the ids up in its own).
    #[cfg(any(feature = "platform", feature = "dialogs"))]
    fn service_messages() -> &'static [Word] {
        static TABLE: std::sync::OnceLock<Vec<Word>> = std::sync::OnceLock::new();
        TABLE.get_or_init(|| {
            let mut words: Vec<Word> = Vec::new();
            #[cfg(feature = "platform")]
            words.extend(craft_platform::texts().iter().map(|t| Word::keyed(t.id, t.english)));
            #[cfg(feature = "dialogs")]
            words.extend(craft_dialogs::save::TEXTS.iter().map(|t| Word::keyed(t.id, t.english)));
            words
        })
    }

    /// The counted words (singular and plural forms) of every enabled crate that has them.
    #[allow(unused_mut, reason = "each push is behind its crate's feature")]
    pub fn counts() -> Vec<&'static [Counted]> {
        let mut all: Vec<&'static [Counted]> = vec![craft_ui::localized::words::COUNTS];
        #[cfg(feature = "automation")]
        all.push(craft_automation::ui::COUNTS);
        #[cfg(feature = "browse")]
        all.push(craft_browse::ui::COUNTS);
        #[cfg(feature = "color")]
        all.push(craft_color::ui::COUNTS);
        #[cfg(feature = "dialogs")]
        all.push(craft_dialogs::ui::COUNTS);
        #[cfg(feature = "draw")]
        all.push(craft_draw::ui::COUNTS);
        #[cfg(feature = "font-manager")]
        all.push(craft_font_manager::ui::COUNTS);
        #[cfg(feature = "imaging")]
        all.push(craft_imaging::ui::COUNTS);
        #[cfg(feature = "keymap")]
        all.push(craft_keymap::ui::COUNTS);
        #[cfg(feature = "pages")]
        all.push(craft_pages::ui::COUNTS);
        #[cfg(feature = "text")]
        all.push(craft_text::ui::COUNTS);
        all
    }
}
