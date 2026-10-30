//! The Preferences body story, and the fake host the tests share.

use std::collections::BTreeMap;

use craft_theme::Rgba;
use egui::{Id, Ui, vec2};

use super::{GroupSpec, PageSpec, PrefValue, PreferencesBody, PreferencesHost, PreferencesTexts, RowKind, RowSpec};
use crate::direction::{self, Direction};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PreferencesWindow"];

#[derive(StoryArgs, Debug, PartialEq)]
struct WindowArgs {
    search: String,
    #[arg(range = 560.0..=1040.0)]
    width: f32,
    #[arg(range = 400.0..=800.0)]
    height: f32,
    rtl: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/preferences/window",
        Level::Patterns,
        "PreferencesWindow",
        "Window",
        WindowArgs { search: String::new(), width: 760.0, height: 480.0, rtl: false },
        show_window
    )]
}

/// A host of five pages with every row kind: General, Interface (grouped), Tools, an empty page
/// and Advanced (a custom row and a custom page area).
pub(crate) struct FakeHost {
    pub saved: BTreeMap<String, PrefValue>,
    pub draft: BTreeMap<String, PrefValue>,
    pub log: Vec<String>,
    pub fail: Option<String>,
    pub can_ok: bool,
}

impl FakeHost {
    pub fn new() -> Self {
        let mut saved = BTreeMap::new();
        for (k, v) in [
            ("general.language", PrefValue::Choice("en".into())),
            ("general.name", PrefValue::Text("Studio".into())),
            ("general.autosave", PrefValue::Bool(true)),
            ("interface.accent", PrefValue::Color(Rgba([0x33, 0x66, 0x99, 255]))),
            ("interface.show_tips", PrefValue::Bool(false)),
            ("interface.show_rulers", PrefValue::Bool(true)),
            ("tools.size", PrefValue::Int(12)),
            ("tools.smoothing", PrefValue::Float(0.5)),
        ] {
            saved.insert(k.to_owned(), v);
        }
        Self { draft: saved.clone(), saved, log: Vec::new(), fail: None, can_ok: true }
    }

    fn text(&self, key: &str) -> &str {
        match self.draft.get(key) {
            Some(PrefValue::Text(t) | PrefValue::Choice(t)) => t,
            _ => "",
        }
    }

    fn flag(&self, key: &str) -> bool {
        matches!(self.draft.get(key), Some(PrefValue::Bool(true)))
    }

    #[cfg(test)]
    pub fn logged(&self, what: &str) -> usize {
        self.log.iter().filter(|l| l.as_str() == what).count()
    }
}

impl PreferencesHost for FakeHost {
    fn texts(&self) -> PreferencesTexts {
        let mut t = PreferencesTexts::english();
        if self.text("general.language") == "de" {
            t.title = "Einstellungen".into();
            t.cancel = "Abbrechen".into();
            t.no_matches = "Keine passenden Einstellungen".into();
        }
        t
    }

    fn pages(&self) -> Vec<PageSpec<'_>> {
        let color = match self.draft.get("interface.accent") {
            Some(PrefValue::Color(c)) => *c,
            _ => Rgba([0, 0, 0, 255]),
        };
        let int = match self.draft.get("tools.size") {
            Some(PrefValue::Int(v)) => *v,
            _ => 0,
        };
        let float = match self.draft.get("tools.smoothing") {
            Some(PrefValue::Float(v)) => *v,
            _ => 0.0,
        };
        let general = PageSpec::new(
            "general",
            "General",
            vec![
                RowSpec::new(
                    "general.language",
                    "Language:",
                    RowKind::Choice { value: self.text("general.language"), options: vec![("en", "English"), ("de", "Deutsch")] },
                )
                .keywords("locale")
                .help("Applies at next launch."),
                RowSpec::new("general.name", "Workspace name:", RowKind::Text(self.text("general.name"))),
                RowSpec::new("general.autosave", "Save automatically", RowKind::Bool(self.flag("general.autosave"))),
            ],
        );
        let interface = PageSpec {
            id: "interface",
            title: "Interface",
            groups: vec![
                GroupSpec::new(Some("Appearance"), vec![RowSpec::new("interface.accent", "Accent colour:", RowKind::Color(color)).keywords("theme")]),
                GroupSpec::new(
                    Some("Options"),
                    vec![
                        RowSpec::new("interface.show_tips", "Show tips", RowKind::Bool(self.flag("interface.show_tips"))),
                        RowSpec::new("interface.show_rulers", "Show rulers", RowKind::Bool(self.flag("interface.show_rulers"))),
                    ],
                )
                .checks(),
            ],
            footnote: Some("Some options apply to new windows only."),
            custom: false,
        };
        let tools = PageSpec::new(
            "tools",
            "Tools",
            vec![
                RowSpec::new("tools.size", "Brush size:", RowKind::Int { value: int, range: 1..=100, unit: Some("px") }),
                RowSpec::new("tools.smoothing", "Smoothing:", RowKind::Float { value: float, range: 0.0..=1.0, decimals: 2, unit: None }),
                RowSpec::new("tools.hidden", "Hidden tool option", RowKind::Bool(false)).visible(false),
            ],
        );
        let plugins = PageSpec::new("plugins", "Plug-ins", vec![RowSpec::new("plugins.none", "Not shown", RowKind::Bool(false)).visible(false)]);
        let advanced = PageSpec::new("advanced", "Advanced", vec![RowSpec::new("advanced.disks", "Scratch disks:", RowKind::Custom)]);
        vec![general, interface, tools, plugins, advanced]
    }

    fn set(&mut self, key: &str, value: PrefValue) {
        self.log.push(format!("set {key}"));
        self.draft.insert(key.to_owned(), value);
    }

    fn custom_row(&mut self, ui: &mut Ui, _key: &str) {
        ui.label("Startup disk");
    }

    fn custom_page(&mut self, ui: &mut Ui, _page: &str) {
        ui.label("Custom page");
    }

    fn differs(&self) -> bool {
        self.draft != self.saved
    }

    fn can_ok(&self) -> bool {
        self.can_ok
    }

    fn apply(&mut self) -> Result<(), String> {
        self.log.push("apply".into());
        match &self.fail {
            Some(m) => Err(m.clone()),
            None => {
                self.saved = self.draft.clone();
                Ok(())
            }
        }
    }

    fn cancel(&mut self) {
        self.log.push("cancel".into());
        self.draft = self.saved.clone();
    }

    fn reset_page(&mut self, page: &str) {
        self.log.push(format!("reset {page}"));
    }
}

fn show_window(ui: &mut Ui, args: &mut WindowArgs) {
    let ctx = ui.ctx().clone();
    let key = Id::unique("craft_ui::preferences::story");
    let last: Option<(String, bool)> = ctx.data(|d| d.get_temp(key));
    if last.as_ref().map(|l| &l.0) != Some(&args.search) {
        let mut state = PreferencesBody::state(&ctx, "story");
        state.search = args.search.clone();
        PreferencesBody::set_state(&ctx, "story", state);
    }
    if last.as_ref().map(|l| l.1) != Some(args.rtl) {
        direction::apply(&ctx, if args.rtl { Direction::Rtl } else { Direction::Ltr });
    }
    ctx.data_mut(|d| d.insert_temp(key, (args.search.clone(), args.rtl)));
    let host_id = key.with("host");
    let mut host: FakeHost = ctx.data_mut(|d| d.get_temp::<FakeHostBox>(host_id)).map(|b| b.0).unwrap_or_else(FakeHost::new);
    let size = vec2(args.width.min(ui.available_width()), args.height);
    ui.allocate_ui(size, |ui| {
        ui.set_min_size(size);
        let _ = PreferencesBody::new("story").show(ui, &mut host);
    });
    ctx.data_mut(|d| d.insert_temp(host_id, FakeHostBox(host)));
}

/// `FakeHost` for egui's temp memory, which needs `Clone`.
struct FakeHostBox(FakeHost);

impl Clone for FakeHostBox {
    fn clone(&self) -> Self {
        Self(FakeHost {
            saved: self.0.saved.clone(),
            draft: self.0.draft.clone(),
            log: self.0.log.clone(),
            fail: self.0.fail.clone(),
            can_ok: self.0.can_ok,
        })
    }
}
