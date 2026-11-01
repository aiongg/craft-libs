//! Stories of the preset library, the named preset row and the picker.
//!
//! Every story uses fakes: a fake `Files` as the step 4 stories use, so Load…, Replace…, Save…
//! and Import… show with no platform; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/preset-browser/footer-in-panel` | a browser with `footer(..)` in a short dock group: the list scrolls, the footer stays; size slider, count, Delete, New, New Group |
//! | `composites/preset-browser/drop-on-canvas` | `drop_zone` over a fake canvas; the status line names the dropped preset and the position |
//! | `composites/preset-browser/hover-preview` | `out.hovered` drives a preview tile |
//! | `composites/preset-library/groups-and-folders` | `PresetLibrary<T>` with nested folders (nesting 3): rename, delete, move, drag into folders, a name clash message |
//! | `composites/preset-picker/pattern-grid` | `Square` field, 6 columns, 30 patterns, no groups |
//! | `composites/preset-picker/shape-groups` | `Wide` field, 7 columns, group headers |
//! | `composites/preset-picker/contour-with-files` | `Square` field, 4 columns, built-in and user contours, Delete, Load, Replace, Save through the fake `Files` |
//! | `composites/preset-picker/names-list` | `PickerField::Name`, List view, groups (LightCraft's curve presets, SoundCraft's menu) |
//! | `composites/preset-picker/keeps-open` | `OnActivate`: a click selects, a double-click closes |
//! | `composites/named-preset/contour-buttons` | style `Buttons`: New… with the suggested name, a name clash error, Load…, Save… |
//! | `composites/named-preset/trace-menu` | style `Menu`: Save as New Preset…, Rename…, Delete, "Custom" when nothing matches |
//! | `composites/named-preset/rtl` | the row and the picker in right to left |

use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{AppInfo, FileError, Files, Host, PickAnswer, PickId, PickKind, PickRequest, Picked};
use egui::ecolor::Hsva;
use egui::epaint::Shape;
use egui::{Color32, Context, Painter, Pos2, Rect, Sense, StrokeKind, Ui, Vec2, pos2, vec2};

use super::menu::{NamedPresetEvent, NamedPresetRow, NamedPresetStyle, PresetExport, PresetFileEvent, PresetFiles};
use super::model::{GroupPath, PresetError, PresetId, PresetKey, PresetLibrary};
use super::picker::{PickerClose, PickerField, PickerThumb, PresetPicker};
use super::{FooterEvent, PresetBrowser, PresetEvent, PresetFooterOptions, PresetReorder, PresetRow, PresetState, PresetView};
use crate::controls::slider::with_values;
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};
use crate::story::{Level, NoArgs, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

/// The components this module has stories for. "Preset browser" is listed in `presets/stories.rs`.
pub const COMPONENTS: &[&str] = &["Preset library", "Preset picker", "Named preset"];

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/preset-browser/footer-in-panel",
            Level::Composites,
            "Preset browser",
            "Footer in panel",
            FooterArgs { height: 180.0, slider: true, new_group: true, count: true },
            show_footer
        ),
        typed_story!(
            "composites/preset-browser/drop-on-canvas",
            Level::Composites,
            "Preset browser",
            "Drop on canvas",
            DropArgs { canvas_height: 120.0 },
            show_drop
        ),
        typed_story!(
            "composites/preset-browser/hover-preview",
            Level::Composites,
            "Preset browser",
            "Hover preview",
            PreviewArgs { preview_height: 80.0 },
            show_preview
        ),
        typed_story!(
            "composites/preset-library/groups-and-folders",
            Level::Composites,
            "Preset library",
            "Groups and folders",
            LibraryArgs { height: 260.0, drag_groups: true },
            show_library
        ),
        typed_story!(
            "composites/preset-picker/pattern-grid",
            Level::Composites,
            "Preset picker",
            "Pattern grid",
            GridArgs { columns: 6 },
            show_pattern_grid
        ),
        typed_story!(
            "composites/preset-picker/shape-groups",
            Level::Composites,
            "Preset picker",
            "Shape groups",
            ShapeArgs { columns: 7 },
            show_shape_groups
        ),
        typed_story!(
            "composites/preset-picker/contour-with-files",
            Level::Composites,
            "Preset picker",
            "Contour with files",
            ContourArgs { columns: 4, split: false },
            show_contour_files
        ),
        typed_story!(
            "composites/preset-picker/names-list",
            Level::Composites,
            "Preset picker",
            "Names list",
            NamesArgs { editable: true },
            show_names_list
        ),
        typed_story!(
            "composites/preset-picker/keeps-open",
            Level::Composites,
            "Preset picker",
            "Keeps open",
            KeepsOpenArgs { columns: 5, mode: FieldArg::Thumbnail },
            show_keeps_open
        ),
        typed_story!(
            "composites/named-preset/contour-buttons",
            Level::Composites,
            "Named preset",
            "Contour buttons",
            ButtonsArgs { clash: true },
            show_contour_buttons
        ),
        typed_story!(
            "composites/named-preset/trace-menu",
            Level::Composites,
            "Named preset",
            "Trace menu",
            TraceArgs { custom: true },
            show_trace_menu
        ),
        typed_story!("composites/named-preset/rtl", Level::Composites, "Named preset", "Right to left", NoArgs {}, show_rtl),
    ]
}

crate::words! {
    /// The sample text of the preset library, picker and named preset stories, English by default.
    #[non_exhaustive]
    pub struct PresetLibraryStoryLabels {
        pattern: "presetLibraryStory.pattern" = "Pattern {n}";
        shape: "presetLibraryStory.shape" = "Shape {n}";
        contour: "presetLibraryStory.contour" = "Contour {n}";
        preset: "presetLibraryStory.preset" = "Preset {n}";
        group: "presetLibraryStory.group" = "Group";
        panel_label: "presetLibraryStory.panelLabel" = "Presets";
        library_label: "presetLibraryStory.libraryLabel" = "Preset library";
        pattern_label: "presetLibraryStory.patternLabel" = "Pattern";
        shape_label: "presetLibraryStory.shapeLabel" = "Shape";
        contour_label: "presetLibraryStory.contourLabel" = "Contour";
        curve_label: "presetLibraryStory.curveLabel" = "Curve preset";
        plugin_label: "presetLibraryStory.pluginLabel" = "Plugin preset";
        trace_label: "presetLibraryStory.traceLabel" = "Trace preset";
        group_basic: "presetLibraryStory.groupBasic" = "Basic";
        group_arrows: "presetLibraryStory.groupArrows" = "Arrows";
        group_symbols: "presetLibraryStory.groupSymbols" = "Symbols";
        group_warm: "presetLibraryStory.groupWarm" = "Warm";
        group_cool: "presetLibraryStory.groupCool" = "Cool";
        group_pastel: "presetLibraryStory.groupPastel" = "Pastel";
        group_deep: "presetLibraryStory.groupDeep" = "Deep";
        group_standard: "presetLibraryStory.groupStandard" = "Standard";
        group_film: "presetLibraryStory.groupFilm" = "Film";
        curve_linear: "presetLibraryStory.curveLinear" = "Linear";
        curve_medium: "presetLibraryStory.curveMedium" = "Medium Contrast";
        curve_strong: "presetLibraryStory.curveStrong" = "Strong Contrast";
        curve_lighten: "presetLibraryStory.curveLighten" = "Lighten Shadows";
        curve_darken: "presetLibraryStory.curveDarken" = "Darken Highlights";
        curve_faded: "presetLibraryStory.curveFaded" = "Faded";
        curve_cross: "presetLibraryStory.curveCross" = "Cross Process";
        sound_init: "presetLibraryStory.soundInit" = "Init";
        sound_pad: "presetLibraryStory.soundPad" = "Warm Pad";
        sound_bass: "presetLibraryStory.soundBass" = "Plucked Bass";
        sound_bell: "presetLibraryStory.soundBell" = "Glass Bell";
        sound_lead: "presetLibraryStory.soundLead" = "Wide Lead";
        trace_default: "presetLibraryStory.traceDefault" = "Default";
        trace_high: "presetLibraryStory.traceHigh" = "High Fidelity Photo";
        trace_low: "presetLibraryStory.traceLow" = "Low Fidelity Photo";
        trace_three: "presetLibraryStory.traceThree" = "3 Colors";
        trace_sketch: "presetLibraryStory.traceSketch" = "Sketched Art";
        trace_line: "presetLibraryStory.traceLine" = "Line Art";
        canvas: "presetLibraryStory.canvas" = "Canvas";
        drop_hint: "presetLibraryStory.dropHint" = "Drag a preset onto the canvas.";
        dropped: "presetLibraryStory.dropped" = "Dropped {name} at {x}, {y}.";
        hover_hint: "presetLibraryStory.hoverHint" = "Hover a preset to preview it.";
        previewing: "presetLibraryStory.previewing" = "Preview: {name}";
        selected: "presetLibraryStory.selected" = "Selected: {name}";
        nothing_selected: "presetLibraryStory.nothingSelected" = "Nothing selected.";
        last_change: "presetLibraryStory.lastChange" = "Last change: {change}";
        activated: "presetLibraryStory.activated" = "Activated: {name}";
        name_taken: "presetLibraryStory.nameTaken" = "That name is already in use.";
        too_deep: "presetLibraryStory.tooDeep" = "That group would be nested too deep.";
        files_note: "presetLibraryStory.filesNote" = "Files are fake: Load… reads two contours, Save… writes nothing.";
        rtl_note: "presetLibraryStory.rtlNote" = "With the direction set to right to left the row, the field and its popup open from the right. Thumbnails keep their geometry.";
    }
}

fn numbered(template: &str, n: u32) -> String {
    craft_i18n::fmt(template, &[("n", &n.to_string())])
}

// ---- Fake files -----------------------------------------------------------------------------

/// A file service for stories: Open answers at once with one file of two contour names, Save
/// answers with a path in a made-up `/stories` folder and the write is kept in memory.
#[derive(Default)]
struct StoryFiles {
    state: Mutex<StoryFilesState>,
}

#[derive(Default)]
struct StoryFilesState {
    next: u64,
    answered: Vec<PickAnswer>,
}

impl Files for StoryFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.next += 1;
        let id = PickId(state.next);
        let picked = match request.kind {
            PickKind::Save => {
                Picked::Paths(vec![PathBuf::from("/stories").join(request.file_name.clone().unwrap_or_else(|| request.purpose.clone()))])
            }
            _ => Picked::Contents(vec![("Imported.shc".to_owned(), b"Ripple\nSawtooth".to_vec())]),
        };
        state.answered.push(PickAnswer { id, request, picked });
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let at = state.answered.iter().position(|answer| answer.id == id)?;
        Some(state.answered.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).answered.retain(|answer| answer.id != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Ok(())
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

/// Gives the installed Host the fake file service (the gallery's Host answers every picker with
/// Cancelled), or installs a Host with it when the story runs without one. A Host the app or the
/// gallery installs again afterwards is replaced again on the next pass.
fn ensure_host(ctx: &Context) {
    let key = egui::Id::unique("craft_ui::presets::stories_library::host");
    let current = Host::try_of_ctx(ctx);
    let ours = ctx.data(|d| d.get_temp::<usize>(key));
    if current.as_ref().is_some_and(|host| Some(Arc::as_ptr(host) as usize) == ours) {
        return;
    }
    let files = Arc::new(StoryFiles::default());
    match current {
        Some(host) => host.with_files(files).install(ctx),
        None => Host::builder(AppInfo { id: "craft-ui-stories", name: "Craft Stories", version: "0" }).files(files).install(ctx),
    }
    if let Some(host) = Host::try_of_ctx(ctx) {
        ctx.data_mut(|d| d.insert_temp(key, Arc::as_ptr(&host) as usize));
    }
}

/// The names in a loaded file, one per line.
fn names_in(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes).lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect()
}

// ---- Thumbnails -----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Art {
    Swatch,
    Pattern,
    Shape,
    Contour,
}

impl Art {
    fn paint(self, painter: &Painter, rect: Rect, n: u32) {
        match self {
            Art::Swatch => {
                painter.rect_filled(rect, 0.0, color_of(n));
            }
            Art::Pattern => paint_pattern(painter, rect, n),
            Art::Shape => paint_shape(painter, rect, n),
            Art::Contour => paint_contour(painter, rect, n),
        }
    }
}

fn color_of(n: u32) -> Color32 {
    Hsva::new((n as f32 * 0.137).fract(), 0.55 + 0.3 * ((n % 3) as f32 / 2.0), 0.85, 1.0).into()
}

fn ink(painter: &Painter) -> egui::Stroke {
    painter.ctx().global_style().visuals.widgets.noninteractive.fg_stroke
}

fn paint_pattern(painter: &Painter, rect: Rect, n: u32) {
    let painter = painter.with_clip_rect(rect);
    let stroke = ink(&painter);
    let step = rect.height() / (3.0 + (n % 5) as f32 * 2.0);
    let mut at = 0.0;
    while at < rect.width() + rect.height() {
        let (a, b) = if n.is_multiple_of(2) {
            (rect.left_top() + vec2(at, 0.0), rect.left_top() + vec2(at - rect.height(), rect.height()))
        } else {
            (rect.left_top() + vec2(at, 0.0), rect.left_top() + vec2(at, rect.height()))
        };
        painter.line_segment([a, b], stroke);
        at += step;
    }
}

fn paint_shape(painter: &Painter, rect: Rect, n: u32) {
    let (center, radius) = (rect.center(), rect.width().min(rect.height()) * 0.45);
    let sides = 3 + n % 6;
    let star = n.is_multiple_of(2) && sides >= 5;
    let count = if star { sides * 2 } else { sides };
    let points: Vec<Pos2> = (0..count)
        .map(|i| {
            let angle = TAU * i as f32 / count as f32 - FRAC_PI_2;
            let r = if star && i % 2 == 1 { radius * 0.45 } else { radius };
            center + r * vec2(angle.cos(), angle.sin())
        })
        .collect();
    painter.add(Shape::closed_line(points, ink(painter)));
}

fn paint_contour(painter: &Painter, rect: Rect, n: u32) {
    let area = rect.shrink(rect.width() * 0.1);
    let power = 0.6 + (n % 5) as f32 * 0.35;
    let points: Vec<Pos2> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.0;
            let y = if n.is_multiple_of(2) { t.powf(power) } else { 0.5 - 0.5 * (t * PI * (1.0 + (n % 3) as f32)).cos() };
            pos2(area.left() + t * area.width(), area.bottom() - y * area.height())
        })
        .collect();
    painter.add(Shape::line(points, ink(painter)));
}

// ---- A library on a shelf ---------------------------------------------------------------------

type Key = PresetKey<PresetId>;

/// The seed a thumbnail draws from: the preset's id.
fn seed(key: &Key) -> u32 {
    match key {
        PresetKey::Item(id) => id.get() as u32,
        PresetKey::Group(path) => path.depth() as u32,
    }
}

/// A library with the view settings, selection and a note on the last change, as an app keeps them.
#[derive(Clone)]
struct Shelf {
    library: PresetLibrary<u32>,
    state: PresetState<Key>,
    selected: Option<Key>,
    note: String,
    /// Drops on the canvas: the position relative to the canvas and the preset's seed.
    dropped: Vec<(Vec2, u32)>,
}

impl Shelf {
    fn new(library: PresetLibrary<u32>, thumb: f32) -> Self {
        let mut state = PresetState::new();
        state.thumb = thumb;
        Self { library, state, selected: None, note: String::new(), dropped: Vec::new() }
    }

    fn exists(&self, key: &Key) -> bool {
        match key {
            PresetKey::Item(id) => self.library.get(*id).is_some(),
            PresetKey::Group(path) => self.library.group(path).is_some(),
        }
    }

    fn name_of(&self, key: &Key) -> String {
        match key {
            PresetKey::Item(id) => self.library.get(*id).map(|i| i.name.clone()).unwrap_or_default(),
            PresetKey::Group(path) => path.name().to_owned(),
        }
    }

    /// Applies the browser's events to the library.
    fn handle(&mut self, events: &[PresetEvent<Key>], words: &PresetLibraryStoryLabels) {
        for event in events {
            if let PresetEvent::Select { key, .. } = event {
                self.selected = Some(key.clone());
            }
            let changes = self.library.apply(event);
            if matches!(event, PresetEvent::Rename { .. }) && changes.is_empty() {
                self.note = words.name_taken.clone();
            } else if let Some(change) = changes.last() {
                self.note = craft_i18n::fmt(&words.last_change, &[("change", &format!("{change:?}"))]);
            }
        }
        if let Some(key) = self.selected.take().filter(|k| self.exists(k)) {
            self.selected = Some(key);
        }
    }

    /// The group New and New Group act in: the selected group, or the selected preset's.
    fn target_group(&self) -> GroupPath {
        match &self.selected {
            Some(PresetKey::Group(path)) => path.clone(),
            Some(PresetKey::Item(id)) => self.library.group_of(*id).unwrap_or_else(GroupPath::root),
            None => GroupPath::root(),
        }
    }

    fn footer(&mut self, events: Vec<FooterEvent>, words: &PresetLibraryStoryLabels, nested: bool) {
        for event in events {
            match event {
                FooterEvent::New => {
                    let name = numbered(&words.preset, self.library.total() as u32 + 1);
                    match self.library.add(&self.target_group(), &name, 0) {
                        Ok(id) => self.selected = Some(PresetKey::Item(id)),
                        Err(_) => self.note = words.name_taken.clone(),
                    }
                }
                FooterEvent::NewGroup => {
                    let parent = if nested { self.target_group() } else { GroupPath::root() };
                    match self.library.new_group(&parent, &words.group) {
                        Ok(path) => self.selected = Some(PresetKey::Group(path)),
                        Err(PresetError::TooDeep) => self.note = words.too_deep.clone(),
                        Err(_) => {}
                    }
                }
                FooterEvent::Delete => {
                    if let Some(key) = self.selected.clone() {
                        self.library.apply(&PresetEvent::Delete(key));
                    }
                }
                _ => {}
            }
        }
        if let Some(key) = self.selected.take().filter(|k| self.exists(k)) {
            self.selected = Some(key);
        }
    }

    fn status(&self, ui: &mut Ui, words: &PresetLibraryStoryLabels) {
        let line = match &self.selected {
            Some(key) => craft_i18n::fmt(&words.selected, &[("name", &self.name_of(key))]),
            None => words.nothing_selected.clone(),
        };
        ui.add(Text::new(line).content(Content::Muted));
        if !self.note.is_empty() {
            ui.add(Text::new(self.note.clone()).content(Content::Muted));
        }
    }
}

/// Twelve presets in two groups and a few ungrouped ones.
fn sample_library(words: &PresetLibraryStoryLabels) -> PresetLibrary<u32> {
    let mut library = PresetLibrary::new();
    let root = GroupPath::root();
    let mut n = 0;
    for _ in 0..3 {
        n += 1;
        let _ = library.add(&root, &numbered(&words.preset, n), n);
    }
    for name in [&words.group_warm, &words.group_cool] {
        if let Ok(group) = library.new_group(&root, name) {
            for _ in 0..12 {
                n += 1;
                let _ = library.add(&group, &numbered(&words.preset, n), n);
            }
        }
    }
    library
}

// ---- Browser stories ----------------------------------------------------------------------------

#[derive(StoryArgs, Debug, PartialEq)]
struct FooterArgs {
    /// The browser's height with its footer, in points.
    #[arg(range = 100.0..=400.0)]
    height: f32,
    slider: bool,
    #[arg(name = "new group")]
    new_group: bool,
    count: bool,
}

/// The list scrolls and the footer stays at the bottom of a short dock group.
fn show_footer(ui: &mut Ui, args: &mut FooterArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let start = Shelf::new(sample_library(&words), 32.0);
    with_values(ui, "preset-footer", [start], |ui, values| {
        let shelf = &mut values[0];
        let rows = shelf.library.rows();
        let mut options = PresetFooterOptions::new().slider(args.slider).new_group(args.new_group).can_delete(shelf.selected.is_some());
        if args.count {
            options = options.count(shelf.library.total());
        }
        let Shelf { state, selected, .. } = shelf;
        let out = PresetBrowser::new("preset-footer", &words.panel_label, state, &rows)
            .selected(selected.as_ref())
            .max_height(args.height)
            .thumb_range(16.0..=64.0)
            .footer(options)
            .show(ui, |painter, rect, info| Art::Swatch.paint(painter, rect, seed(info.key)));
        shelf.handle(&out.events, &words);
        shelf.footer(out.footer, &words, false);
        shelf.status(ui, &words);
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DropArgs {
    /// The canvas height in points.
    #[arg(range = 60.0..=240.0, name = "canvas height")]
    canvas_height: f32,
}

/// A preset dragged onto the canvas reports `DropAt`; the canvas keeps a dot per drop.
fn show_drop(ui: &mut Ui, args: &mut DropArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let start = Shelf::new(sample_library(&words), 32.0);
    with_values(ui, "preset-drop", [start], |ui, values| {
        let shelf = &mut values[0];
        let (canvas, _) = ui.allocate_exact_size(vec2(ui.available_width(), args.canvas_height), Sense::hover());
        let rows = shelf.library.rows();
        let Shelf { state, selected, .. } = shelf;
        let out = PresetBrowser::new("preset-drop", &words.panel_label, state, &rows)
            .selected(selected.as_ref())
            .max_height(200.0)
            .drop_zone(canvas)
            .show(ui, |painter, rect, info| Art::Swatch.paint(painter, rect, seed(info.key)));
        let mut note = None;
        for event in &out.events {
            if let PresetEvent::DropAt { key, pos, .. } = event {
                shelf.dropped.push((*pos - canvas.min, seed(key)));
                note = Some(craft_i18n::fmt(
                    &words.dropped,
                    &[("name", &shelf.name_of(key)), ("x", &format!("{:.0}", pos.x - canvas.min.x)), ("y", &format!("{:.0}", pos.y - canvas.min.y))],
                ));
            }
        }
        shelf.handle(&out.events, &words);
        if let Some(note) = note {
            shelf.note = note;
        }
        let painter = ui.painter_at(canvas);
        let visuals = ui.visuals();
        painter.rect_filled(canvas, 0.0, visuals.extreme_bg_color);
        let border = if out.drag.is_some() { visuals.selection.stroke } else { visuals.widgets.noninteractive.bg_stroke };
        painter.rect_stroke(canvas, 0.0, border, StrokeKind::Inside);
        let radius = ui.spacing().interact_size.y * 0.3;
        for (offset, n) in &shelf.dropped {
            painter.circle_filled(canvas.min + *offset, radius, color_of(*n));
        }
        let label = if shelf.dropped.is_empty() { words.drop_hint.clone() } else { words.canvas.clone() };
        painter.text(
            canvas.left_top() + vec2(radius, radius),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(radius * 2.0),
            visuals.weak_text_color(),
        );
        shelf.status(ui, &words);
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PreviewArgs {
    /// The preview tile's height in points.
    #[arg(range = 40.0..=200.0, name = "preview height")]
    preview_height: f32,
}

/// The preview tile shows the preset under the pointer, else the selected one.
fn show_preview(ui: &mut Ui, args: &mut PreviewArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let start = Shelf::new(sample_library(&words), 32.0);
    with_values(ui, "preset-preview", [start], |ui, values| {
        let shelf = &mut values[0];
        let rows = shelf.library.rows();
        let Shelf { state, selected, .. } = shelf;
        let out = PresetBrowser::new("preset-preview", &words.panel_label, state, &rows)
            .selected(selected.as_ref())
            .max_height(200.0)
            .show(ui, |painter, rect, info| Art::Swatch.paint(painter, rect, seed(info.key)));
        shelf.handle(&out.events, &words);
        let shown = out.hovered.clone().or_else(|| shelf.selected.clone());
        let (tile, _) = ui.allocate_exact_size(vec2(ui.available_width(), args.preview_height), Sense::hover());
        let painter = ui.painter_at(tile);
        let visuals = ui.visuals();
        painter.rect_filled(tile, 0.0, visuals.extreme_bg_color);
        match shown.filter(|k| matches!(k, PresetKey::Item(_))) {
            Some(key) => {
                painter.rect_filled(tile.shrink(ui.spacing().item_spacing.x), 0.0, color_of(seed(&key)));
                ui.add(Text::new(craft_i18n::fmt(&words.previewing, &[("name", &shelf.name_of(&key))])).content(Content::Muted));
            }
            None => {
                ui.add(Text::new(words.hover_hint.clone()).content(Content::Muted));
            }
        }
        shelf.status(ui, &words);
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct LibraryArgs {
    /// The browser's height in points.
    #[arg(range = 120.0..=400.0)]
    height: f32,
    #[arg(name = "drag groups")]
    drag_groups: bool,
}

/// A library with folders three levels deep. The footer's New Group adds a folder in the selected
/// group, and refuses past the nesting; renaming a preset to a used name shows the refusal.
fn nested_library(words: &PresetLibraryStoryLabels) -> PresetLibrary<u32> {
    let mut library = PresetLibrary::new().with_nesting(3);
    let root = GroupPath::root();
    let mut n = 0;
    let mut fill = |library: &mut PresetLibrary<u32>, group: &GroupPath, count: u32| {
        for _ in 0..count {
            n += 1;
            let _ = library.add(group, &numbered(&words.preset, n), n);
        }
    };
    fill(&mut library, &root, 2);
    for name in [&words.group_warm, &words.group_cool] {
        if let Ok(top) = library.new_group(&root, name) {
            fill(&mut library, &top, 3);
            for inner in [&words.group_pastel, &words.group_deep] {
                if let Ok(folder) = library.new_group(&top, inner) {
                    fill(&mut library, &folder, 3);
                    if let Ok(deeper) = library.new_group(&folder, &words.group_standard) {
                        fill(&mut library, &deeper, 2);
                    }
                }
            }
        }
    }
    library
}

fn show_library(ui: &mut Ui, args: &mut LibraryArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let start = Shelf::new(nested_library(&words), 28.0);
    with_values(ui, "preset-library", [start], |ui, values| {
        let shelf = &mut values[0];
        let rows = shelf.library.rows();
        let nesting = shelf.library.nesting();
        let reorder = if args.drag_groups { PresetReorder::items_and_groups().nest(nesting) } else { PresetReorder::items() };
        let options = PresetFooterOptions::new().count(shelf.library.total()).can_delete(shelf.selected.is_some());
        let Shelf { state, selected, .. } = shelf;
        let out = PresetBrowser::new("preset-library", &words.library_label, state, &rows)
            .selected(selected.as_ref())
            .toolbar(true)
            .max_height(args.height)
            .reorder(reorder)
            .footer(options)
            .show(ui, |painter, rect, info| Art::Swatch.paint(painter, rect, seed(info.key)));
        shelf.handle(&out.events, &words);
        shelf.footer(out.footer, &words, true);
        shelf.status(ui, &words);
    });
}

// ---- Picker stories -----------------------------------------------------------------------------

/// A picker's host state: the selection and a line about the last pick.
#[derive(Clone, Default)]
struct Pick {
    selected: Option<u32>,
    activated: Option<u32>,
}

fn pick_status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

fn selected_line(ui: &Ui, name: Option<String>) -> String {
    let words = PresetLibraryStoryLabels::of(ui);
    match name {
        Some(name) => craft_i18n::fmt(&words.selected, &[("name", &name)]),
        None => words.nothing_selected.clone(),
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct GridArgs {
    #[arg(range = 2..=10)]
    columns: u8,
}

fn show_pattern_grid(ui: &mut Ui, args: &mut GridArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let rows: Vec<PresetRow<u32>> = (1..=30).map(|n| PresetRow::item(n, numbered(&words.pattern, n), 0)).collect();
    with_values(ui, "preset-pattern-grid", [Pick { selected: Some(3), activated: None }], |ui, values| {
        let pick = &mut values[0];
        let out = PresetPicker::new("preset-pattern-grid", &words.pattern_label, &rows)
            .selected(pick.selected.as_ref())
            .columns(args.columns as usize)
            .field(PickerField::Thumbnail(PickerThumb::Square))
            .show(ui, |painter, rect, info| Art::Pattern.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            pick.selected = out.picked;
        }
        let name = pick.selected.map(|n| numbered(&words.pattern, n));
        pick_status(ui, selected_line(ui, name));
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ShapeArgs {
    #[arg(range = 3..=10)]
    columns: u8,
}

/// Three groups of shapes in a wide field.
fn show_shape_groups(ui: &mut Ui, args: &mut ShapeArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let mut rows = Vec::new();
    let mut n = 0;
    for (g, name) in [&words.group_basic, &words.group_arrows, &words.group_symbols].into_iter().enumerate() {
        rows.push(PresetRow::group(1000 + g as u32, name.clone(), 0));
        for _ in 0..7 {
            n += 1;
            rows.push(PresetRow::item(n, numbered(&words.shape, n), 1));
        }
    }
    with_values(ui, "preset-shape-groups", [Pick { selected: Some(2), activated: None }], |ui, values| {
        let pick = &mut values[0];
        let out = PresetPicker::new("preset-shape-groups", &words.shape_label, &rows)
            .selected(pick.selected.as_ref())
            .columns(args.columns as usize)
            .field(PickerField::Thumbnail(PickerThumb::Wide))
            .show(ui, |painter, rect, info| Art::Shape.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            pick.selected = out.picked;
        }
        let name = pick.selected.map(|n| numbered(&words.shape, n));
        pick_status(ui, selected_line(ui, name));
    });
}

/// The contour list of the file story: built-in contours 1 to 6 and the user's after them.
#[derive(Clone)]
struct Contours {
    /// `(key, name, user)`.
    items: Vec<(u32, String, bool)>,
    next: u32,
    selected: Option<u32>,
    status: Option<String>,
}

impl Contours {
    fn new(words: &PresetLibraryStoryLabels) -> Self {
        let mut items: Vec<(u32, String, bool)> = (1..=6).map(|n| (n, numbered(&words.contour, n), false)).collect();
        items.extend((7..=8).map(|n| (n, numbered(&words.contour, n), true)));
        Self { items, next: 9, selected: Some(1), status: None }
    }

    fn rows(&self) -> Vec<PresetRow<u32>> {
        self.items.iter().map(|(key, name, _)| PresetRow::item(*key, name.clone(), 0)).collect()
    }

    fn is_user(&self, key: u32) -> bool {
        self.items.iter().any(|(k, _, user)| *k == key && *user)
    }

    fn bytes(&self) -> Vec<u8> {
        self.items.iter().filter(|(_, _, user)| *user).map(|(_, name, _)| name.as_str()).collect::<Vec<_>>().join("\n").into_bytes()
    }

    fn loaded(&mut self, bytes: &[u8], replace: bool) {
        if replace {
            self.items.retain(|(_, _, user)| !*user);
        }
        for name in names_in(bytes) {
            self.items.push((self.next, name, true));
            self.next += 1;
        }
        if self.selected.is_some_and(|k| !self.items.iter().any(|(key, ..)| *key == k)) {
            self.selected = None;
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ContourArgs {
    #[arg(range = 2..=8)]
    columns: u8,
    /// A click on the thumbnail reports `field_clicked`; only the chevron opens the popup.
    split: bool,
}

/// Delete works on the user's contours only; Load…, Replace… and Save… go through the fake `Files`.
fn show_contour_files(ui: &mut Ui, args: &mut ContourArgs) {
    ensure_host(ui.ctx());
    let words = PresetLibraryStoryLabels::of(ui);
    let start = (Contours::new(&words), 0u32);
    with_values(ui, "preset-contour-files", [start], |ui, values| {
        let (contours, clicks) = &mut values[0];
        let rows = contours.rows();
        let bytes = contours.bytes();
        let files = PresetFiles::new("contours", "Contours", &["shc"])
            .replace(true)
            .export(move |_| Some(PresetExport { file_name: "Contours.shc".to_owned(), bytes: bytes.clone() }));
        let user: HashSet<u32> = contours.items.iter().filter(|(_, _, user)| *user).map(|(key, ..)| *key).collect();
        let out = PresetPicker::new("preset-contour-files", &words.contour_label, &rows)
            .selected(contours.selected.as_ref())
            .columns(args.columns as usize)
            .split(args.split)
            .can_delete(|key| user.contains(key))
            .files(&files)
            .show(ui, |painter, rect, info| Art::Contour.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            contours.selected = out.picked;
        }
        if out.field_clicked {
            *clicks += 1;
        }
        for event in &out.events {
            if let PresetEvent::Delete(key) = event
                && contours.is_user(*key)
            {
                contours.items.retain(|(k, ..)| k != key);
                if contours.selected == Some(*key) {
                    contours.selected = None;
                }
            }
        }
        for event in &out.files.events {
            if let PresetFileEvent::Loaded { bytes, replace, .. } = event {
                contours.loaded(bytes, *replace);
            }
        }
        if let Some(status) = &out.files.status {
            contours.status = Some(status.text.clone());
        }
        let name = contours.selected.and_then(|k| contours.items.iter().find(|(key, ..)| *key == k)).map(|(_, name, _)| name.clone());
        pick_status(ui, selected_line(ui, name));
        if let Some(status) = &contours.status {
            pick_status(ui, status.clone());
        }
        pick_status(ui, words.files_note.clone());
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct NamesArgs {
    /// Rename and Delete in the popup's context menu.
    editable: bool,
}

/// A list of names in a `Name` field: grouped curve presets and a flat plugin menu.
fn show_names_list(ui: &mut Ui, args: &mut NamesArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let mut curves = Vec::new();
    let mut key = 0;
    let groups = [
        (&words.group_standard, vec![&words.curve_linear, &words.curve_medium, &words.curve_strong]),
        (&words.group_film, vec![&words.curve_lighten, &words.curve_darken, &words.curve_faded, &words.curve_cross]),
    ];
    for (g, (name, presets)) in groups.into_iter().enumerate() {
        curves.push(PresetRow::group(1000 + g as u32, name.clone(), 0));
        for preset in presets {
            key += 1;
            curves.push(PresetRow::item(key, preset.clone(), 1));
        }
    }
    let sounds: Vec<PresetRow<u32>> = [&words.sound_init, &words.sound_pad, &words.sound_bass, &words.sound_bell, &words.sound_lead]
        .into_iter()
        .enumerate()
        .map(|(i, name)| PresetRow::item(i as u32 + 1, name.clone(), 0))
        .collect();
    let start = (curves, Pick { selected: Some(2), activated: None }, sounds, Pick { selected: Some(1), activated: None });
    with_values(ui, "preset-names-list", [start], |ui, values| {
        let (curve_rows, curve_pick, sound_rows, sound_pick) = &mut values[0];
        let editable = args.editable;
        let out = PresetPicker::new("preset-names-curves", &words.curve_label, curve_rows)
            .selected(curve_pick.selected.as_ref())
            .field(PickerField::Name)
            .view(PresetView::List)
            .can_rename(move |_| editable)
            .can_delete(move |_| editable)
            .show(ui, |painter, rect, info| Art::Contour.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            curve_pick.selected = out.picked;
        }
        for event in out.events {
            match event {
                PresetEvent::Rename { key, text } => {
                    if let Some(row) = curve_rows.iter_mut().find(|r| r.key == key) {
                        row.label = text;
                    }
                }
                PresetEvent::Delete(key) => curve_rows.retain(|r| r.key != key),
                _ => {}
            }
        }
        let out = PresetPicker::new("preset-names-sounds", &words.plugin_label, sound_rows)
            .selected(sound_pick.selected.as_ref())
            .field(PickerField::Name)
            .view(PresetView::List)
            .show(ui, |_, _, _| {});
        if out.picked.is_some() {
            sound_pick.selected = out.picked;
        }
        let find = |rows: &[PresetRow<u32>], key: Option<u32>| key.and_then(|k| rows.iter().find(|r| r.key == k)).map(|r| r.label.clone());
        let line = selected_line(ui, find(curve_rows, curve_pick.selected));
        pick_status(ui, line);
        let line = selected_line(ui, find(sound_rows, sound_pick.selected));
        pick_status(ui, line);
    });
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum FieldArg {
    Thumbnail,
    Name,
    #[choice(name = "Thumbnail and name")]
    Both,
}

impl FieldArg {
    fn field(self) -> PickerField {
        match self {
            FieldArg::Thumbnail => PickerField::Thumbnail(PickerThumb::Square),
            FieldArg::Name => PickerField::Name,
            FieldArg::Both => PickerField::ThumbnailAndName(PickerThumb::Square),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct KeepsOpenArgs {
    #[arg(range = 2..=8)]
    columns: u8,
    mode: FieldArg,
}

/// `OnActivate`: a click selects and the popup stays; a double-click or Enter activates and closes.
fn show_keeps_open(ui: &mut Ui, args: &mut KeepsOpenArgs) {
    let words = PresetLibraryStoryLabels::of(ui);
    let rows: Vec<PresetRow<u32>> = (1..=20).map(|n| PresetRow::item(n, numbered(&words.pattern, n), 0)).collect();
    with_values(ui, "preset-keeps-open", [Pick { selected: Some(1), activated: None }], |ui, values| {
        let pick = &mut values[0];
        let out = PresetPicker::new("preset-keeps-open", &words.pattern_label, &rows)
            .selected(pick.selected.as_ref())
            .columns(args.columns as usize)
            .field(args.mode.field())
            .close(PickerClose::OnActivate)
            .show(ui, |painter, rect, info| Art::Pattern.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            pick.selected = out.picked;
        }
        if out.activated.is_some() {
            pick.activated = out.activated;
        }
        let name = pick.selected.map(|n| numbered(&words.pattern, n));
        pick_status(ui, selected_line(ui, name));
        if let Some(n) = pick.activated {
            pick_status(ui, craft_i18n::fmt(&words.activated, &[("name", &numbered(&words.pattern, n))]));
        }
    });
}

// ---- Named preset stories -------------------------------------------------------------------------

/// A named-preset host: a library whose first presets are built in.
#[derive(Clone)]
struct Named {
    library: PresetLibrary<u32>,
    built_in: HashSet<PresetId>,
    selected: Option<Key>,
    status: Option<String>,
}

impl Named {
    fn new(names: &[String]) -> Self {
        let mut library = PresetLibrary::new();
        let built_in = names.iter().enumerate().filter_map(|(i, name)| library.add(&GroupPath::root(), name, i as u32 + 1).ok()).collect();
        Self { library, built_in, selected: None, status: None }
    }

    fn name_of(&self, key: &Key) -> Option<String> {
        match key {
            PresetKey::Item(id) => self.library.get(*id).map(|i| i.name.clone()),
            PresetKey::Group(_) => None,
        }
    }

    fn user_bytes(&self) -> Vec<u8> {
        let user = self.library.items().into_iter().filter(|i| !self.built_in.contains(&i.id));
        user.map(|i| i.name.as_str()).collect::<Vec<_>>().join("\n").into_bytes()
    }

    fn apply(&mut self, events: Vec<NamedPresetEvent<Key>>) {
        let root = GroupPath::root();
        for event in events {
            match event {
                NamedPresetEvent::Apply(key) => self.selected = Some(key),
                NamedPresetEvent::Create { name } => {
                    if let Ok(id) = self.library.add(&root, &name, 0) {
                        self.selected = Some(PresetKey::Item(id));
                    }
                }
                NamedPresetEvent::Rename { key: PresetKey::Item(id), text } => {
                    let _ = self.library.rename(id, &text);
                }
                NamedPresetEvent::Delete(PresetKey::Item(id)) => {
                    let _ = self.library.delete(id);
                    self.selected = self.selected.take().filter(|k| matches!(k, PresetKey::Item(i) if self.library.get(*i).is_some()));
                }
                NamedPresetEvent::File(PresetFileEvent::Loaded { bytes, replace, .. }) => {
                    if replace {
                        let user: Vec<PresetId> = self.library.items().into_iter().map(|i| i.id).filter(|id| !self.built_in.contains(id)).collect();
                        for id in user {
                            let _ = self.library.delete(id);
                        }
                    }
                    for name in names_in(&bytes) {
                        let _ = self.library.add(&root, &name, 0);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ButtonsArgs {
    /// The suggested name is one a preset already has, so New… shows the clash error.
    clash: bool,
}

fn named_status(ui: &mut Ui, host: &Named) {
    let words = PresetLibraryStoryLabels::of(ui);
    let name = host.selected.as_ref().and_then(|k| host.name_of(k));
    pick_status(ui, selected_line(ui, name));
    if let Some(status) = &host.status {
        pick_status(ui, status.clone());
    }
    pick_status(ui, words.files_note.clone());
}

/// The contour editor's row: a thumbnail field, New…, Load… and Save….
fn show_contour_buttons(ui: &mut Ui, args: &mut ButtonsArgs) {
    ensure_host(ui.ctx());
    let words = PresetLibraryStoryLabels::of(ui);
    let names: Vec<String> = (1..=4).map(|n| numbered(&words.contour, n)).collect();
    with_values(ui, "named-contour-buttons", [Named::new(&names)], |ui, values| {
        let host = &mut values[0];
        let suggestion = if args.clash { numbered(&words.contour, 1) } else { numbered(&words.contour, host.library.total() as u32 + 1) };
        let rows = host.library.rows();
        let bytes = host.user_bytes();
        let files = PresetFiles::new("contours", "Contours", &["shc"])
            .export(move |_| Some(PresetExport { file_name: "Contours.shc".to_owned(), bytes: bytes.clone() }));
        let out = NamedPresetRow::new("named-contour-buttons", &words.contour_label, &rows)
            .selected(host.selected.as_ref())
            .style(NamedPresetStyle::Buttons)
            .field(PickerField::Thumbnail(PickerThumb::Square))
            .suggest_name(suggestion)
            .check_name(|name| host.library.name_error(name))
            .built_in(|key| matches!(key, PresetKey::Item(id) if host.built_in.contains(id)))
            .files(&files)
            .show(ui, |painter, rect, info| Art::Contour.paint(painter, rect, seed(info.key)));
        if let Some(status) = &out.status {
            host.status = Some(status.text.clone());
        }
        host.apply(out.events);
        named_status(ui, host);
    });
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TraceArgs {
    /// No preset matches the settings: the field reads "Custom". Picking a preset clears it.
    custom: bool,
}

/// Image Trace's row: a `Name` field and one Manage menu.
fn show_trace_menu(ui: &mut Ui, args: &mut TraceArgs) {
    ensure_host(ui.ctx());
    let words = PresetLibraryStoryLabels::of(ui);
    let names: Vec<String> = [&words.trace_default, &words.trace_high, &words.trace_low, &words.trace_three, &words.trace_sketch, &words.trace_line]
        .into_iter()
        .cloned()
        .collect();
    with_values(ui, "named-trace-menu", [Named::new(&names)], |ui, values| {
        let host = &mut values[0];
        if host.selected.is_none() {
            host.selected = host.library.items().first().map(|i| PresetKey::Item(i.id));
        }
        let rows = host.library.rows();
        let bytes = host.user_bytes();
        let files = PresetFiles::new("trace-presets", "Trace presets", &["json"])
            .export(move |_| Some(PresetExport { file_name: "Trace.json".to_owned(), bytes: bytes.clone() }));
        let selected = if args.custom { None } else { host.selected.as_ref() };
        let out = NamedPresetRow::new("named-trace-menu", &words.trace_label, &rows)
            .selected(selected)
            .style(NamedPresetStyle::Menu)
            .suggest_name(numbered(&words.preset, host.library.total() as u32 + 1))
            .check_name(|name| host.library.name_error(name))
            .built_in(|key| matches!(key, PresetKey::Item(id) if host.built_in.contains(id)))
            .files(&files)
            .show(ui, |_, _, _| {});
        if out.events.iter().any(|e| matches!(e, NamedPresetEvent::Apply(_) | NamedPresetEvent::Create { .. })) {
            args.custom = false;
        }
        if let Some(status) = &out.status {
            host.status = Some(status.text.clone());
        }
        host.apply(out.events);
        named_status(ui, host);
    });
}

/// The row and a picker take the interface direction of the gallery: choose right to left there.
fn show_rtl(ui: &mut Ui, _args: &mut NoArgs) {
    ensure_host(ui.ctx());
    let words = PresetLibraryStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let names: Vec<String> = (1..=4).map(|n| numbered(&words.contour, n)).collect();
    with_values(ui, "named-rtl", [(Named::new(&names), Pick { selected: Some(2), activated: None })], |ui, values| {
        let (host, pick) = &mut values[0];
        ui.add(Text::new(words.rtl_note.clone()).content(Content::Muted));
        let rows = host.library.rows();
        let out = NamedPresetRow::new("named-rtl", &words.contour_label, &rows)
            .selected(host.selected.as_ref())
            .suggest_name(numbered(&words.contour, host.library.total() as u32 + 1))
            .check_name(|name| host.library.name_error(name))
            .built_in(|key| matches!(key, PresetKey::Item(id) if host.built_in.contains(id)))
            .show(ui, |painter, rect, info| Art::Contour.paint(painter, rect, seed(info.key)));
        host.apply(out.events);
        let patterns: Vec<PresetRow<u32>> = (1..=12).map(|n| PresetRow::item(n, numbered(&words.pattern, n), 0)).collect();
        let out = PresetPicker::new("named-rtl-picker", &words.pattern_label, &patterns)
            .selected(pick.selected.as_ref())
            .field(PickerField::ThumbnailAndName(PickerThumb::Square))
            .show(ui, |painter, rect, info| Art::Pattern.paint(painter, rect, *info.key));
        if out.picked.is_some() {
            pick.selected = out.picked;
        }
        let name = pick.selected.map(|n| numbered(&words.pattern, n));
        pick_status(ui, craft_i18n::fmt(&story.picked, &[("action", &name.unwrap_or_default())]));
    });
}
