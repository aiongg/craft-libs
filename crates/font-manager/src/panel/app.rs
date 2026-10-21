//! The panel: installed fonts, the Google Fonts catalogue, and the details of the selected family.

use std::collections::HashMap;
use std::sync::Arc;

use crate::craft_font_catalog::{FontEnv, Limits, Os};
use crate::filter::{Query, Search};
use crate::index::{Category, Index, categories, script_name};
use crate::installed::{InstalledFamily, group};
use crate::store::{Installer, RemoveRequest, StoreError};
use crate::{Catalog, FaceInfo, Source};
use craft_i18n::Folder;
use craft_ui::direction;
use craft_ui::fonts::{CraftFace, I18nScripts, UiFonts};
use egui::{
    Align, CentralPanel, Color32, ComboBox, Context, FontId, Id, Label, Layout, Modal, Panel, ProgressBar, RichText, ScrollArea, Slider, TextEdit,
    Ui, vec2,
};

use crate::panel::faces::{FaceFonts, Owner, Shown};
use crate::panel::i18n::{self, I18N, LANGUAGES, fmt, tl, tn};
use crate::panel::jobs::{Finished, Jobs, Kind, Request, Runner};

/// Installs the standalone app's interface fonts in `ctx` and sets the interface direction: craft-ui's
/// font stack for the panel's interface language, with `craft_faces` (craft-fonts' faces the app
/// embeds with craft-fonts-build, or none) and the system's fonts for every other character. A host
/// with its own fonts doesn't call it.
pub fn install_interface_fonts(ctx: &Context, craft_faces: &[CraftFace]) {
    let language = I18N.code(i18n::current()).to_owned();
    UiFonts::new(&I18nScripts).languages(LANGUAGES).ui_language(&language).craft_faces(craft_faces).install(ctx);
    direction::apply(ctx, direction::resolve(&language, direction::Setting::Language));
}

/// Preview downloads: at most this much per file, and this many faces in a session.
const PREVIEW_LIMITS: Limits = Limits { max_file_bytes: 64 << 20, max_faces_per_file: 512, max_added_faces: 20_000 };

/// What the app keeps between runs.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The sample text; empty for the default sentence.
    pub sample: String,
    pub size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { sample: String::new(), size: 28.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Installed,
    Catalogue,
}

/// What the app is built from; tests pass their own catalog, store and index.
pub struct Options {
    pub catalog: Arc<Catalog>,
    pub installer: Arc<dyn Installer>,
    /// The Google Fonts index, or why it couldn't be read.
    pub index: Result<Index, String>,
    pub runner: Runner,
}

/// A download waiting for the user's consent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Install(usize),
    Preview(usize),
}

struct Catalogue {
    index: Arc<Index>,
    search: Search,
}

pub struct FontsPanel {
    settings: Settings,
    tab: Tab,
    catalog: Arc<Catalog>,
    installer: Arc<dyn Installer>,
    catalogue: Result<Catalogue, String>,
    query: Query,
    results: Vec<usize>,
    results_for: Option<Query>,
    language_text: String,
    installed: Vec<InstalledFamily>,
    /// Installed faces by lower-case family name, every source together.
    installed_by_name: HashMap<String, Vec<FaceInfo>>,
    installed_generation: Option<u64>,
    installed_text: String,
    selected_installed: Option<(String, Source)>,
    selected_family: Option<usize>,
    /// Downloaded preview files, never installed.
    previews: Arc<Catalog>,
    previewed: HashMap<String, Vec<FaceInfo>>,
    faces: FaceFonts,
    jobs: Jobs,
    asking: Option<Pending>,
    message: Option<(bool, String)>,
    focused: bool,
    folder: Folder,
}

fn category_name(c: Category) -> &'static str {
    match c {
        Category::SansSerif => tl!("Sans serif"),
        Category::Serif => tl!("Serif"),
        Category::Display => tl!("Display"),
        Category::Handwriting => tl!("Handwriting"),
        Category::Monospace => tl!("Monospace"),
    }
}

fn source_name(s: Source) -> &'static str {
    match s {
        Source::Store => tl!("Font store"),
        _ => tl!("System"),
    }
}

/// The face closest to an upright regular.
fn regular(faces: &[FaceInfo]) -> Option<&FaceInfo> {
    faces.iter().min_by_key(|f| (f.italic, ((f.weight - 400.0).abs() * 10.0) as i64, ((f.width - 100.0).abs() * 10.0) as i64))
}

fn style_name(face: &FaceInfo) -> &str {
    if face.style.is_empty() { tl!("Regular") } else { &face.style }
}

impl FontsPanel {
    pub fn new(options: Options, settings: Settings) -> FontsPanel {
        let catalogue = options.index.map(|index| Catalogue { search: Search::new(&index), index: Arc::new(index) });
        let previews = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
        FontsPanel {
            settings,
            tab: Tab::Installed,
            catalog: options.catalog,
            installer: options.installer,
            catalogue,
            query: Query::default(),
            results: Vec::new(),
            results_for: None,
            language_text: String::new(),
            installed: Vec::new(),
            installed_by_name: HashMap::new(),
            installed_generation: None,
            installed_text: String::new(),
            selected_installed: None,
            selected_family: None,
            previews,
            previewed: HashMap::new(),
            faces: FaceFonts::default(),
            jobs: Jobs::new(options.runner),
            asking: None,
            message: None,
            focused: true,
            folder: Folder::new("en"),
        }
    }

    /// The panel's own settings, for the host to save.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// The installed families, as the Installed tab lists them.
    pub fn installed(&self) -> &[InstalledFamily] {
        &self.installed
    }

    /// The catalogue's families for the current search and filters.
    pub fn results(&self) -> Vec<String> {
        let Ok(c) = &self.catalogue else { return Vec::new() };
        self.results.iter().filter_map(|&i| c.index.families().get(i)).map(|f| f.family.clone()).collect()
    }

    /// Shows the interface in `language` (a tag such as `de` or `pt-BR`), the host's interface
    /// language, when its catalog is embedded; English otherwise.
    pub fn set_language(&mut self, language: &str) {
        i18n::set_current(I18N.match_tag(language).unwrap_or(craft_i18n::Lang::EN));
    }

    /// Draws the panel into `ui`, which it fills.
    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.faces.begin_frame(&ctx);
        self.poll_jobs();
        let focused = ctx.input(|i| i.focused);
        if focused && !self.focused && self.catalog.is_ready() {
            self.catalog.rescan_if_changed();
        }
        self.focused = focused;
        self.refresh_installed();
        self.refresh_results();

        Panel::top("craft-fonts.header").show(ui, |ui| self.header(ui));
        Panel::top("craft-fonts.controls").show(ui, |ui| self.controls(ui));
        Panel::bottom("craft-fonts.status").show(ui, |ui| self.status(ui));
        direction::end_panel(ui, "craft-fonts.details").resizable(true).default_size(420.0).min_size(320.0).show(ui, |ui| self.details(ui));
        CentralPanel::default_margins().show(ui, |ui| self.list(ui));
        self.consent(&ctx);
    }

    fn poll_jobs(&mut self) {
        for (job, result) in self.jobs.poll() {
            let family = job.family.as_str();
            self.message = Some(match result {
                Ok(Finished::Installed) => (false, fmt("{family} is installed.", &[("family", family)])),
                Ok(Finished::Removed) => {
                    self.selected_installed = None;
                    (false, fmt("{family} is removed.", &[("family", family)]))
                }
                Ok(Finished::Previewed(files)) => {
                    let mut faces = Vec::new();
                    for file in files {
                        match self.previews.add_bytes(file.bytes, Source::User, &PREVIEW_LIMITS) {
                            Ok(added) => faces.extend(added),
                            Err(e) => log::warn!("preview of {}: {e}", file.path),
                        }
                    }
                    self.previewed.insert(family.to_lowercase(), faces);
                    continue;
                }
                Err(e) => (
                    true,
                    match job.kind {
                        Kind::Install => fmt("{family} couldn't be installed: {error}", &[("family", family), ("error", &e.0)]),
                        Kind::Remove => fmt("{family} couldn't be removed: {error}", &[("family", family), ("error", &e.0)]),
                        Kind::Preview => fmt("{family} couldn't be downloaded for a preview: {error}", &[("family", family), ("error", &e.0)]),
                    },
                ),
            });
        }
    }

    fn refresh_installed(&mut self) {
        if !self.catalog.is_ready() {
            return;
        }
        let generation = self.catalog.generation();
        if self.installed_generation == Some(generation) {
            return;
        }
        self.installed_generation = Some(generation);
        let faces = self.catalog.faces();
        self.installed = group(&faces);
        self.installed_by_name.clear();
        for family in &self.installed {
            self.installed_by_name.entry(family.name.to_lowercase()).or_default().extend(family.faces.iter().cloned());
        }
    }

    fn refresh_results(&mut self) {
        let Ok(c) = &self.catalogue else { return };
        if self.results_for.as_ref() != Some(&self.query) {
            self.results = c.search.families(&c.index, &self.query);
            self.results_for = Some(self.query.clone());
        }
    }

    fn sample_text(&self) -> String {
        if self.settings.sample.trim().is_empty() {
            tl!("The quick brown fox jumps over the lazy dog").to_owned()
        } else {
            self.settings.sample.clone()
        }
    }

    /// Has the user allowed downloads from Google Fonts? A suite-wide setting of the font store,
    /// off until they turn it on or confirm an install or preview.
    pub fn downloads_allowed(&self) -> bool {
        self.installer.downloads_allowed()
    }

    /// Turns downloads on or off, as the user's own action (a host's settings page, say). A
    /// failure is also shown in the panel's status line.
    pub fn set_downloads_allowed(&mut self, allowed: bool) -> Result<(), StoreError> {
        let result = self.installer.allow_downloads(allowed);
        if let Err(e) = &result {
            self.message = Some((true, fmt("The download setting couldn't be saved: {error}", &[("error", &e.0)])));
        }
        result
    }

    fn header(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        direction::horizontal(ui, |ui| {
            ui.selectable_value(&mut self.tab, Tab::Installed, tl!("Installed"));
            ui.selectable_value(&mut self.tab, Tab::Catalogue, tl!("Catalogue"));
        });
        direction::horizontal(ui, |ui| {
            let mut allowed = self.downloads_allowed();
            if ui.checkbox(&mut allowed, tl!("Allow downloads")).changed() {
                let _ = self.set_downloads_allowed(allowed);
            }
            ui.weak(tl!("Fonts download from Google Fonts' GitHub repository through jsDelivr, only when you install or preview one."));
        });
        ui.add_space(2.0);
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            let label = ui.label(tl!("Search"));
            let text = match self.tab {
                Tab::Installed => &mut self.installed_text,
                Tab::Catalogue => &mut self.query.text,
            };
            ui.add(TextEdit::singleline(text).desired_width(180.0).hint_text(tl!("Family name"))).labelled_by(label.id);
            if self.tab == Tab::Catalogue
                && let Ok(c) = &self.catalogue
            {
                filters(ui, &mut self.query, &mut self.language_text, &c.index, &c.search);
            }
            ui.separator();
            let label = ui.label(tl!("Sample text"));
            let hint = tl!("The quick brown fox jumps over the lazy dog");
            ui.add(TextEdit::singleline(&mut self.settings.sample).desired_width(220.0).hint_text(hint)).labelled_by(label.id);
            ui.add(Slider::new(&mut self.settings.size, 8.0..=120.0).text(tl!("Size")));
        });
        ui.add_space(2.0);
    }
}

fn filters(ui: &mut Ui, query: &mut Query, language_text: &mut String, index: &Index, search: &Search) {
    {
        let any = tl!("Any category");
        ComboBox::from_label(tl!("Category")).selected_text(query.category.map_or(any, category_name)).show_ui(ui, |ui| {
            ui.selectable_value(&mut query.category, None, any);
            for c in Category::ALL {
                ui.selectable_value(&mut query.category, Some(c), category_name(c));
            }
        });

        let language_label = |i: usize| {
            index.languages().get(i).map_or_else(String::new, |l| match l.autonym.as_deref() {
                Some(autonym) if autonym != l.name => format!("{} · {autonym}", l.name),
                _ => l.name.clone(),
            })
        };
        let selected = query.language.map_or_else(|| tl!("Any language").to_owned(), language_label);
        ComboBox::from_label(tl!("Language")).selected_text(selected).height(420.0).width(260.0).show_ui(ui, |ui| {
            let label = ui.label(tl!("Find a language"));
            let field = ui.add(TextEdit::singleline(language_text).hint_text(tl!("Name in English or in the language"))).labelled_by(label.id);
            if field.gained_focus() || ui.memory(|m| m.focused().is_none()) {
                field.request_focus();
            }
            ui.selectable_value(&mut query.language, None, tl!("Any language"));
            let matches = search.languages(language_text);
            const SHOWN: usize = 300;
            for &i in matches.iter().take(SHOWN) {
                ui.selectable_value(&mut query.language, Some(i), language_label(i));
            }
            if matches.len() > SHOWN {
                ui.weak(tn((matches.len() - SHOWN) as u64, "{n} more; type to narrow the list", "{n} more; type to narrow the list"));
            }
        });

        let any = tl!("Any script");
        let selected = query.script.as_deref().and_then(|s| script_name(index, s)).unwrap_or(any);
        ComboBox::from_label(tl!("Script")).selected_text(selected).height(420.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut query.script, None, any);
            for &i in search.scripts() {
                if let Some(s) = index.scripts().get(i) {
                    ui.selectable_value(&mut query.script, Some(s.id.clone()), &s.name);
                }
            }
        });
    }
}

impl FontsPanel {
    fn status(&mut self, ui: &mut Ui) {
        ui.add_space(2.0);
        for job in self.jobs.running() {
            direction::horizontal(ui, |ui| {
                let what = match job.kind {
                    Kind::Install => fmt("Installing {family}", &[("family", &job.family)]),
                    Kind::Remove => fmt("Removing {family}", &[("family", &job.family)]),
                    Kind::Preview => fmt("Downloading {family} for a preview", &[("family", &job.family)]),
                };
                ui.label(what);
                if job.progress.total > 0 {
                    let done = job.progress.done as f32 / job.progress.total as f32;
                    ui.add(ProgressBar::new(done).desired_width(200.0).show_percentage());
                } else {
                    ui.spinner();
                }
            });
        }
        match &self.message {
            Some((true, text)) => {
                ui.colored_label(ui.visuals().error_fg_color, text);
            }
            Some((false, text)) => {
                ui.label(text);
            }
            None if self.jobs.running().next().is_none() => {
                let count = tn(self.installed.len() as u64, "{n} installed family", "{n} installed families");
                ui.weak(count);
            }
            None => {}
        }
        ui.add_space(2.0);
    }

    /// `text` in `face`, on one line or wrapped, or a note where the face can't be drawn.
    fn sample(&mut self, ui: &mut Ui, owner: Owner, face: &FaceInfo, text: &str, wrap: bool) {
        let catalog = match owner {
            Owner::Installed => Arc::clone(&self.catalog),
            Owner::Preview => Arc::clone(&self.previews),
        };
        let size = self.settings.size;
        match self.faces.family(ui.ctx(), &catalog, owner, face) {
            Shown::Ready(family) => {
                let label = Label::new(RichText::new(text).font(FontId::new(size, family)));
                ui.add(if wrap { label.wrap() } else { label.truncate() });
            }
            Shown::Loading => {
                ui.allocate_space(vec2(ui.available_width(), size * 1.2));
            }
            Shown::Unavailable => {
                ui.weak(tl!("This font can't be shown."));
            }
        }
    }

    fn row_height(&self, ui: &Ui) -> f32 {
        ui.text_style_height(&egui::TextStyle::Body) + self.settings.size * 1.35 + 12.0
    }

    fn list(&mut self, ui: &mut Ui) {
        match self.tab {
            Tab::Installed => self.installed_list(ui),
            Tab::Catalogue => self.catalogue_list(ui),
        }
    }

    fn installed_list(&mut self, ui: &mut Ui) {
        if !self.catalog.is_ready() {
            direction::horizontal(ui, |ui| {
                ui.spinner();
                ui.label(tl!("Looking for installed fonts…"));
            });
            ui.ctx().request_repaint();
            return;
        }
        let text = self.folder.key(self.installed_text.trim());
        let shown: Vec<usize> =
            (0..self.installed.len()).filter(|&i| text.is_empty() || self.folder.key(&self.installed[i].name).contains(&text)).collect();
        if shown.is_empty() {
            ui.weak(tl!("No installed fonts match."));
            return;
        }
        let sample = self.sample_text();
        let row = self.row_height(ui);
        ScrollArea::vertical().auto_shrink(false).show_rows(ui, row, shown.len(), |ui, range| {
            for &i in shown.get(range).unwrap_or_default() {
                let Some(family) = self.installed.get(i).cloned() else { continue };
                ui.allocate_ui_with_layout(vec2(ui.available_width(), row), Layout::top_down(Align::Min), |ui| {
                    ui.set_min_height(row);
                    direction::horizontal(ui, |ui| {
                        let key = (family.name.clone(), family.source);
                        let selected = self.selected_installed.as_ref() == Some(&key);
                        if ui.selectable_label(selected, RichText::new(&family.name).strong()).clicked() {
                            self.selected_installed = Some(key);
                        }
                        let styles = tn(family.faces.len() as u64, "{n} style", "{n} styles");
                        ui.weak(format!("{} · {styles}", source_name(family.source)));
                    });
                    if let Some(face) = regular(&family.faces) {
                        self.sample(ui, Owner::Installed, face, &sample, false);
                    }
                });
            }
        });
    }

    fn catalogue_list(&mut self, ui: &mut Ui) {
        let index = match &self.catalogue {
            Ok(c) => Arc::clone(&c.index),
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, fmt("The Google Fonts index can't be read: {error}", &[("error", e)]));
                return;
            }
        };
        if self.results.is_empty() {
            ui.weak(tl!("No fonts match."));
            return;
        }
        let sample = self.sample_text();
        let row = self.row_height(ui);
        let results = self.results.clone();
        ScrollArea::vertical().auto_shrink(false).show_rows(ui, row, results.len(), |ui, range| {
            for &i in results.get(range).unwrap_or_default() {
                let Some(family) = index.families().get(i) else { continue };
                let key = family.family.to_lowercase();
                ui.allocate_ui_with_layout(vec2(ui.available_width(), row), Layout::top_down(Align::Min), |ui| {
                    ui.set_min_height(row);
                    let installed = self.installed_by_name.get(&key).and_then(|f| regular(f)).cloned();
                    let previewed = self.previewed.get(&key).and_then(|f| regular(f)).cloned();
                    direction::horizontal(ui, |ui| {
                        let selected = self.selected_family == Some(i);
                        if ui.selectable_label(selected, RichText::new(&family.family).strong()).clicked() {
                            self.selected_family = Some(i);
                        }
                        let categories: Vec<&str> = categories(family).map(category_name).collect();
                        let styles = tn(family.files.len() as u64, "{n} file", "{n} files");
                        ui.weak(format!("{} · {styles}", categories.join(", ")));
                        if installed.is_some() {
                            ui.label(RichText::new(tl!("Installed")).color(Color32::from_rgb(0x3a, 0xa0, 0x5a)));
                        }
                    });
                    match (installed, previewed) {
                        (Some(face), _) => self.sample(ui, Owner::Installed, &face, &sample, false),
                        (None, Some(face)) => self.sample(ui, Owner::Preview, &face, &sample, false),
                        (None, None) => {
                            ui.weak(tl!("Not installed. Select it to preview."));
                        }
                    }
                });
            }
        });
    }

    fn details(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        match self.tab {
            Tab::Installed => self.installed_details(ui),
            Tab::Catalogue => self.catalogue_details(ui),
        }
    }

    fn styles(&mut self, ui: &mut Ui, owner: Owner, faces: &[FaceInfo]) {
        let sample = self.sample_text();
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for face in faces {
                ui.weak(style_name(face));
                self.sample(ui, owner, face, &sample, true);
                ui.add_space(6.0);
            }
        });
    }

    fn installed_details(&mut self, ui: &mut Ui) {
        let Some((name, source)) = self.selected_installed.clone() else {
            ui.weak(tl!("Select a family to see its styles."));
            return;
        };
        let Some(family) = self.installed.iter().find(|f| f.name == name && f.source == source).cloned() else {
            ui.weak(tl!("Select a family to see its styles."));
            return;
        };
        ui.heading(&family.name);
        ui.label(format!("{} · {}", source_name(family.source), tn(family.faces.len() as u64, "{n} style", "{n} styles")));
        let busy = self.jobs.is_running(&family.name);
        let remove = ui.add_enabled(family.removable() && !busy, egui::Button::new(tl!("Remove")));
        let remove = if family.removable() {
            remove
        } else {
            remove.on_disabled_hover_text(tl!("System fonts are removed with your system's own font tools."))
        };
        if remove.clicked() {
            let mut files: Vec<_> = family.faces.iter().filter_map(|f| f.path.clone()).collect();
            files.sort();
            files.dedup();
            let request = RemoveRequest { family: family.name.clone(), files };
            self.jobs.start(ui.ctx(), Arc::clone(&self.installer), Arc::clone(&self.catalog), Request::Remove(request));
        }
        ui.separator();
        self.styles(ui, Owner::Installed, &family.faces);
    }

    fn catalogue_details(&mut self, ui: &mut Ui) {
        let Ok(c) = &self.catalogue else { return };
        let index = Arc::clone(&c.index);
        let Some(i) = self.selected_family else {
            ui.weak(tl!("Select a family to see its styles."));
            return;
        };
        let Some(family) = index.families().get(i) else { return };
        let key = family.family.to_lowercase();
        ui.heading(&family.family);
        if !family.designer.is_empty() {
            ui.label(fmt("By {designer}", &[("designer", &family.designer)]));
        }
        let categories: Vec<&str> = categories(family).map(category_name).collect();
        ui.weak(format!("{} · {} · {}", categories.join(", "), family.license, tn(family.languages.len() as u64, "{n} language", "{n} languages")));
        let scripts: Vec<&str> = family.scripts.iter().map(|s| script_name(&index, s).unwrap_or(s)).collect();
        ui.add(Label::new(RichText::new(scripts.join(", ")).weak()).wrap());
        let size = i18n::size(family.size());
        let installed = self.installed_by_name.get(&key).cloned();
        let busy = self.jobs.is_running(&family.family);
        direction::horizontal(ui, |ui| {
            if installed.is_some() {
                ui.label(tl!("Installed"));
            } else if ui.add_enabled(!busy, egui::Button::new(tl!("Install"))).clicked() {
                self.ask(ui.ctx(), Pending::Install(i));
            }
            ui.weak(fmt("{size} download", &[("size", &size)]));
        });
        ui.separator();
        if let Some(faces) = installed {
            self.styles(ui, Owner::Installed, &faces);
        } else if let Some(faces) = self.previewed.get(&key).cloned() {
            self.styles(ui, Owner::Preview, &faces);
        } else if busy {
            ui.spinner();
        } else if ui.button(tl!("Preview downloads this font")).clicked() {
            self.ask(ui.ctx(), Pending::Preview(i));
        }
    }

    /// Runs `pending` if downloads are allowed, or asks first.
    fn ask(&mut self, ctx: &Context, pending: Pending) {
        if self.downloads_allowed() {
            self.download(ctx, pending);
        } else {
            self.asking = Some(pending);
        }
    }

    fn download(&mut self, ctx: &Context, pending: Pending) {
        if !self.downloads_allowed() {
            return;
        }
        let Ok(c) = &self.catalogue else { return };
        let index = Arc::clone(&c.index);
        let request = match pending {
            Pending::Install(i) => Request::Install(index, i),
            Pending::Preview(i) => Request::Preview(index, i),
        };
        self.message = None;
        self.jobs.start(ctx, Arc::clone(&self.installer), Arc::clone(&self.catalog), request);
    }

    fn consent(&mut self, ctx: &Context) {
        let Some(pending) = self.asking else { return };
        let Ok(c) = &self.catalogue else { return };
        let (Pending::Install(i) | Pending::Preview(i)) = pending;
        let Some(family) = c.index.families().get(i) else { return };
        let text = fmt(
            "Craft Font Manager downloads {family} ({size}) from Google Fonts' GitHub repository through jsDelivr, and checks every file before using it.",
            &[("family", &family.family), ("size", &i18n::size(family.size()))],
        );
        let mut allow = false;
        let mut cancel = false;
        let response = Modal::new(Id::unique("craft-font-manager.consent")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.heading(tl!("Download from Google Fonts?"));
            ui.label(text);
            ui.weak(tl!("Downloads stay allowed until you turn them off."));
            ui.add_space(8.0);
            direction::horizontal(ui, |ui| {
                allow = ui.button(tl!("Download")).clicked();
                cancel = ui.button(tl!("Cancel")).clicked();
            });
        });
        if allow {
            self.asking = None;
            if self.set_downloads_allowed(true).is_ok() {
                self.download(ctx, pending);
            }
        } else if cancel || response.should_close() {
            self.asking = None;
        }
    }
}
