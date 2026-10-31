//! The catalog the dialog tests and screenshots use.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use craft_host::{AppInfo, Host};
use craft_i18n::{Embedded, Format, I18n, Lang};
use craft_keymap::ui::{COUNTS, WORDS};
use craft_keymap::{Bindable, Binding, Chord, Key, Keymap, KeymapCommands, Kind, Platform, Scope, Scopes};
use craft_ui::direction::{self, Direction};
use craft_ui::localized::words::Words;

fn binding(text: &str) -> Binding {
    Binding::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

pub fn fixture(platform: Platform) -> Keymap {
    let tool = Kind::Tool { group: Some("Marquee".into()) };
    let path = |p: &[&str]| p.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let timeline = Scope::new("Timeline").unwrap_or_else(Scope::app);
    let catalog = vec![
        Bindable::new("file.save", "Save", Scope::app()).with_path(path(&["File", "Save"])).with_default(binding("Cmd+S")),
        Bindable::new("file.saveAs", "Save As…", Scope::app()).with_path(path(&["File", "Save As…"])).with_default(binding("Cmd+Shift+S")),
        Bindable::new("file.export", "Export…", Scope::app()).with_path(path(&["File", "Export…"])).with_default(binding("Cmd+E")),
        Bindable::new("object.front", "Bring to Front", Scope::app()).with_path(path(&["Object", "Arrange", "Bring to Front"])),
        Bindable::new("tool.marquee", "Marquee Tool", Scope::app()).with_kind(tool.clone()).with_default(binding("M")),
        Bindable::new("tool.ellipse", "Elliptical Marquee Tool", Scope::app()).with_kind(tool).with_default(binding("M")),
        Bindable::new("timeline.loop", "Loop", timeline).with_default(binding("L")),
        Bindable::new("type.bold", "Bold", Scope::app()).with_default(binding("Text: Cmd+B")),
    ];
    Keymap::new(catalog, Scopes::new().with_panel("Timeline").with_context("Text"), platform)
}

/// `km` with a command whose label and menu path are far too long for a row.
#[allow(dead_code)]
pub fn with_long_label(km: Keymap) -> Keymap {
    let long = "Distribute Selected Objects Evenly Along the Active Guide Path and Keep Their Spacing".to_string();
    let mut catalog = km.catalog().to_vec();
    catalog.insert(
        3,
        Bindable::new("object.distribute", long.as_str(), Scope::app())
            .with_path(vec!["Object".into(), "Arrange and Distribute Along a Path".into(), long.clone()])
            .with_default(binding("Cmd+Alt+Shift+D"))
            .with_default(binding("Ctrl+Alt+Shift+F12")),
    );
    Keymap::new(catalog, Scopes::new().with_panel("Timeline").with_context("Text"), km.platform())
}

/// The words, command labels and direction a test installs on the egui context, as an app does:
/// `Words` from a craft-i18n catalog, a Host whose `Commands` are `KeymapCommands`, and the
/// interface direction.
#[allow(dead_code)]
pub struct Locale {
    i18n: Option<(Arc<I18n>, Lang)>,
    pub rtl: bool,
}

#[allow(dead_code)]
impl Locale {
    /// English; no Host.
    pub fn english() -> Locale {
        Locale { i18n: None, rtl: false }
    }

    /// English laid out right to left, to look at the mirrored layout.
    pub fn english_rtl() -> Locale {
        Locale { i18n: None, rtl: true }
    }

    /// A catalog whose rows are `rows`, `(context, source, translation)`; plural rows are
    /// `("@plural", "one|other", "one|other")`.
    pub fn with_rows(rows: &[(&str, &str, &str)], rtl: bool) -> Locale {
        let mut text = String::new();
        for (context, source, translation) in rows {
            let _ = writeln!(text, "{context}\t{source}\t{translation}");
        }
        let embedded: &'static Embedded = Box::leak(Box::new([("de", &*Box::leak(text.into_boxed_str()), "")]));
        let (i18n, _) = I18n::new(embedded, &["de"]);
        i18n.set_format(Format::from_tag("en-US"));
        let lang = i18n.from_code("de").unwrap_or(Lang::EN);
        Locale { i18n: Some((Arc::new(i18n), lang)), rtl }
    }

    /// German command labels where `labels` has them.
    pub fn labelled(labels: &[(&str, &str)]) -> Locale {
        let rows: Vec<_> = labels.iter().map(|(id, label)| ("@id", *id, *label)).collect();
        Locale::with_rows(&rows, false)
    }

    /// A pseudo translation: every string about 40 % longer, accented, between right-to-left marks
    /// and in `⟦…⟧`, so text drawn without the marks never went through the lookup. Placeholders
    /// stay as they are. Labels, path parts and key names are marked too.
    pub fn pseudo(rtl: bool) -> Locale {
        let mut owned: Vec<(&'static str, String, String)> = Vec::new();
        let mut add = |context: &'static str, source: &str, translation: String| owned.push((context, source.to_string(), translation));
        for word in WORDS.iter().flat_map(|words| words.iter()) {
            add("", word.english, Pseudo::mark(word.english));
        }
        for count in COUNTS {
            add(
                "@plural",
                &format!("{}|{}", count.one, count.word.english),
                format!("{}|{}", Pseudo::mark(count.one), Pseudo::mark(count.word.english)),
            );
        }
        let mut segments = BTreeSet::new();
        for b in with_long_label(fixture(Platform::Mac)).catalog() {
            add("@id", &b.command.id, Pseudo::mark(&b.label));
            segments.extend(b.paths.iter().flatten().cloned());
            segments.insert(b.scope.as_str().to_string());
            if let Kind::Tool { group: Some(g) } = &b.kind {
                segments.insert(g.clone());
            }
        }
        segments.extend(["Timeline", "Text", "Other"].map(String::from));
        for segment in &segments {
            add("", segment, Pseudo::mark(segment));
        }
        for key in Key::ALL {
            let name = Chord::plain(*key).display(Platform::Windows);
            if name.chars().any(|c| c.is_ascii_lowercase()) {
                add("key", &name, Pseudo::mark(&name));
            }
        }
        let rows: Vec<_> = owned.iter().map(|(c, s, t)| (*c, s.as_str(), t.as_str())).collect();
        Locale::with_rows(&rows, rtl)
    }

    /// Installs this locale on `ctx`.
    pub fn install(&self, ctx: &egui::Context) {
        direction::apply(ctx, if self.rtl { Direction::Rtl } else { Direction::Ltr });
        let Some((i18n, lang)) = &self.i18n else { return };
        Words::from_i18n(Arc::clone(i18n), *lang).install(ctx);
        let app = AppInfo { id: "keymap-tests", name: "Keymap tests", version: "0" };
        let commands = KeymapCommands::new(Arc::clone(i18n), *lang);
        Host::builder(app).i18n(Arc::clone(i18n)).lang(*lang).commands(Arc::new(commands)).build().install(ctx);
    }
}

/// The marks of the pseudo translation.
#[allow(dead_code)]
pub struct Pseudo;

#[allow(dead_code)]
impl Pseudo {
    pub fn mark(source: &str) -> String {
        // A catalog row ending in … must end in … like its source.
        let (source, ellipsis) = source.strip_suffix('…').map_or((source, ""), |rest| (rest, "…"));
        let mut out = String::from("⟦\u{200F}");
        let mut in_placeholder = false;
        let mut letters = 0;
        for c in source.chars() {
            match c {
                '{' => in_placeholder = true,
                '}' => in_placeholder = false,
                _ => {}
            }
            let accented = match c {
                _ if in_placeholder => c,
                'a' => 'á',
                'e' => 'ë',
                'i' => 'ï',
                'o' => 'ö',
                'u' => 'ü',
                'A' => 'Å',
                'E' => 'É',
                'O' => 'Ø',
                _ => c,
            };
            if !in_placeholder && c.is_alphabetic() {
                letters += 1;
            }
            out.push(accented);
        }
        out.extend(std::iter::repeat_n('~', letters * 2 / 5));
        out.push_str("\u{200F}⟧");
        out.push_str(ellipsis);
        out
    }
}
