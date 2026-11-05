//! Stories of the Brush Settings panel and its shared rows.
//! Fake section bodies and a fake stroke image drawn from [`PreviewStroke`] with software stamps,
//! so every state shows without an engine; every string goes through story words. The sections,
//! the tip angle and the control rows keep their state in the story, as an app keeps its brush
//! settings, and apply the events the components report.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/brush-section-list/sections` | The list with enable boxes, locks, switch rows and the link row; every section off |
//! | `patterns/brush-settings-panel/panel` | The whole panel with fake bodies for three sections and the strip; an off section greyed |
//! | `composites/tip-ellipse/ellipse` | The angle and roundness widget with its value readout |
//! | `composites/dynamic-control-row/dynamics` | `DynamicControlRow` with and without Fade, the curve toggle lit and plain; `BrushSizeField` with and without restore |
//! | `composites/brush-stroke-strip/strip` | The strip with the S-curve of a fake stroke at three widths |

use std::sync::{Arc, Mutex};

use egui::{Color32, ColorImage, Ui, vec2};

use super::{
    BrushPanelAction, BrushSection, BrushSectionEvent, BrushSectionList, BrushSettingsPanel, BrushSettingsStyle, BrushSizeField, BrushStrokeStrip,
    DynamicControl, DynamicControlRow, PreviewStroke, TipAngle, TipEllipse,
};
use craft_ui::canvas::textures::Revision;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BrushSectionList", "BrushSettingsPanel", "TipEllipse", "DynamicControlRow", "BrushStrokeStrip"];

craft_ui::words! {
    /// The section names, captions and control names the Brush Settings stories show (the app's
    /// words, not the panel's).
    #[non_exhaustive]
    pub struct BrushSettingsStoryLabels {
        tip_shape: "brushSettingsStory.tipShape" = "Brush Tip Shape";
        shape_dynamics: "brushSettingsStory.shapeDynamics" = "Shape Dynamics";
        scattering: "brushSettingsStory.scattering" = "Scattering";
        texture: "brushSettingsStory.texture" = "Texture";
        dual_brush: "brushSettingsStory.dualBrush" = "Dual Brush";
        color_dynamics: "brushSettingsStory.colorDynamics" = "Color Dynamics";
        transfer: "brushSettingsStory.transfer" = "Transfer";
        brush_pose: "brushSettingsStory.brushPose" = "Brush Pose";
        noise: "brushSettingsStory.noise" = "Noise";
        wet_edges: "brushSettingsStory.wetEdges" = "Wet Edges";
        build_up: "brushSettingsStory.buildUp" = "Build-up";
        smoothing: "brushSettingsStory.smoothing" = "Smoothing";
        protect_texture: "brushSettingsStory.protectTexture" = "Protect Texture";
        control_off: "brushSettingsStory.controlOff" = "Off";
        control_fade: "brushSettingsStory.controlFade" = "Fade";
        control_pen_pressure: "brushSettingsStory.controlPenPressure" = "Pen Pressure";
        control_pen_tilt: "brushSettingsStory.controlPenTilt" = "Pen Tilt";
        size: "brushSettingsStory.size" = "Size";
        size_jitter: "brushSettingsStory.sizeJitter" = "Size Jitter";
        angle_note: "brushSettingsStory.angleNote" = "Angle {angle}°, roundness {roundness} %";
        link_opened: "brushSettingsStory.linkOpened" = "Brush presets opened";
        action_new: "brushSettingsStory.actionNew" = "Pressed: New";
        action_reset: "brushSettingsStory.actionReset" = "Pressed: Reset";
        action_update: "brushSettingsStory.actionUpdate" = "Pressed: Update";
        no_body: "brushSettingsStory.noBody" = "This section has no sample settings.";
        selected_note: "brushSettingsStory.selectedNote" = "Selected: {name}";
        with_restore: "brushSettingsStory.withRestore" = "With restore";
        without_restore: "brushSettingsStory.withoutRestore" = "Without restore";
        curve_open: "brushSettingsStory.curveOpen" = "Response curve open";
        narrow: "brushSettingsStory.narrow" = "Narrow";
        medium: "brushSettingsStory.medium" = "Medium";
        wide: "brushSettingsStory.wide" = "Wide";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/brush-section-list/sections",
            Level::Composites,
            "BrushSectionList",
            "Sections",
            SectionsArgs { link: true, all_off: false },
            show_sections
        ),
        typed_story!(
            "patterns/brush-settings-panel/panel",
            Level::Patterns,
            "BrushSettingsPanel",
            "Panel",
            PanelArgs { link: true, can_update: true, height: 420.0, diameter: 24.0 },
            show_panel
        ),
        typed_story!(
            "composites/tip-ellipse/ellipse",
            Level::Composites,
            "TipEllipse",
            "Ellipse",
            EllipseArgs { degrees: 30.0, roundness: 60.0, enabled: true },
            show_ellipse
        ),
        typed_story!(
            "composites/dynamic-control-row/dynamics",
            Level::Composites,
            "DynamicControlRow",
            "Dynamics",
            DynamicsArgs { curve_shaped: false, diameter: 120.0, original: 80.0, max: 2500.0 },
            show_dynamics
        ),
        typed_story!(
            "composites/brush-stroke-strip/strip",
            Level::Composites,
            "BrushStrokeStrip",
            "Strip",
            StripArgs { diameter: 18.0, narrow: 160.0, medium: 280.0, wide: 420.0 },
            show_strip
        ),
    ]
}

// --- args ---

#[derive(StoryArgs, Debug, PartialEq)]
struct SectionsArgs {
    /// Shows the link row above the sections.
    link: bool,
    /// Clears every enable box and switch.
    #[arg(name = "all off")]
    all_off: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PanelArgs {
    /// Shows the link row above the sections.
    link: bool,
    /// Enables the Update button.
    #[arg(name = "can update")]
    can_update: bool,
    /// The panel's height; the pane scrolls inside it. A story-local extent: an app docks the
    /// panel in a bounded area.
    #[arg(range = 200.0..=700.0)]
    height: f64,
    /// The fake stroke's diameter.
    #[arg(range = 1.0..=60.0)]
    diameter: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct EllipseArgs {
    /// The tip's angle in degrees.
    #[arg(range = -180.0..=180.0)]
    degrees: f64,
    /// The tip's roundness in percent.
    #[arg(range = 1.0..=100.0)]
    roundness: f64,
    enabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DynamicsArgs {
    /// Marks the second row's response curve as shaped, so its toggle is lit while closed.
    #[arg(name = "curve shaped")]
    curve_shaped: bool,
    /// The brush diameter of both size fields.
    #[arg(range = 1.0..=2500.0)]
    diameter: f64,
    /// The size the restore button puts back.
    #[arg(range = 1.0..=2500.0)]
    original: f64,
    /// The largest diameter.
    #[arg(range = 1.0..=5000.0)]
    max: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StripArgs {
    /// The fake stroke's diameter.
    #[arg(range = 1.0..=60.0)]
    diameter: f64,
    /// The three strips' widths.
    #[arg(range = 80.0..=600.0)]
    narrow: f64,
    #[arg(range = 80.0..=600.0)]
    medium: f64,
    #[arg(range = 80.0..=600.0)]
    wide: f64,
}

// --- the fake brush ---

/// The section keys: indexes into `section_names`.
type Key = u8;

const TIP_SHAPE: Key = 0;
const SHAPE_DYNAMICS: Key = 1;
const SCATTERING: Key = 2;

fn section_names(words: &BrushSettingsStoryLabels) -> [&str; 13] {
    [
        &words.tip_shape,
        &words.shape_dynamics,
        &words.scattering,
        &words.texture,
        &words.dual_brush,
        &words.color_dynamics,
        &words.transfer,
        &words.brush_pose,
        &words.noise,
        &words.wet_edges,
        &words.build_up,
        &words.smoothing,
        &words.protect_texture,
    ]
}

/// Photoshop's list: Brush Tip Shape without a box, seven panes with a box and a lock, five
/// switch rows. Texture stays off and Noise on, so a mixed list shows when `on` is set.
fn sections(words: &BrushSettingsStoryLabels, on: bool) -> Vec<BrushSection<Key>> {
    section_names(words)
        .into_iter()
        .zip(0..)
        .map(|(name, key)| match key {
            TIP_SHAPE => BrushSection::pane(key, name),
            1..=7 => BrushSection::pane(key, name).enabled(on && key != 3).locked(false),
            _ => BrushSection::switch(key, name, on && key == 8),
        })
        .collect()
}

/// Applies a list event as an app applies it to its brush settings.
fn apply(brush: &mut Brush, event: BrushSectionEvent<Key>, words: &BrushSettingsStoryLabels) {
    let position = |brush: &Brush, key| brush.sections.iter().position(|section| section.key == key);
    match event {
        BrushSectionEvent::Select(key) => {
            brush.selected = key;
            let name = position(brush, key).map(|at| brush.sections[at].name.clone()).unwrap_or_default();
            brush.note = craft_i18n::fmt(&words.selected_note, &[("name", &name)]);
        }
        BrushSectionEvent::Enable { key, on } | BrushSectionEvent::Switch { key, on } => {
            if let Some(at) = position(brush, key) {
                brush.sections[at].enabled = Some(on);
            }
        }
        BrushSectionEvent::Lock { key, on } => {
            if let Some(at) = position(brush, key) {
                brush.sections[at].locked = Some(on);
            }
        }
        BrushSectionEvent::OpenLink => brush.note = words.link_opened.clone(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Off,
    Fade,
    PenPressure,
    PenTilt,
}

fn controls(words: &BrushSettingsStoryLabels) -> Vec<DynamicControl<Control>> {
    vec![
        DynamicControl::new(Control::Off, words.control_off.as_str()),
        DynamicControl::new(Control::Fade, words.control_fade.as_str()).fade(),
        DynamicControl::new(Control::PenPressure, words.control_pen_pressure.as_str()).curve(),
        DynamicControl::new(Control::PenTilt, words.control_pen_tilt.as_str()).curve(),
    ]
}

/// One control row's state: its control, its fade steps and whether its curve editor is open.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RowState {
    control: Control,
    steps: f64,
    curve_open: bool,
}

impl RowState {
    fn new(control: Control) -> Self {
        Self { control, steps: 25.0, curve_open: false }
    }

    /// Shows the row and applies what it reports.
    fn show(&mut self, ui: &mut Ui, salt: &str, list: &[DynamicControl<Control>], shaped: bool) {
        let out = DynamicControlRow::new(salt, list, &self.control).fade_steps(self.steps as u32).curve(self.curve_open, shaped).show(ui);
        if let Some(control) = out.control {
            self.control = control;
            self.curve_open = false;
        }
        if let Some(value) = out.fade_steps.as_ref().and_then(|event| event.value()) {
            self.steps = *value;
        }
        if out.curve_toggled {
            self.curve_open = !self.curve_open;
        }
    }
}

/// The settings the fake panel bodies edit.
#[derive(Clone, Debug)]
struct Brush {
    sections: Vec<BrushSection<Key>>,
    selected: Key,
    angle: TipAngle,
    jitter: RowState,
    scatter: RowState,
    note: String,
    all_off: bool,
}

impl Brush {
    fn new(words: &BrushSettingsStoryLabels, all_off: bool) -> Self {
        Self {
            sections: sections(words, !all_off),
            selected: SHAPE_DYNAMICS,
            angle: TipAngle { degrees: 30.0, roundness: 0.6 },
            jitter: RowState::new(Control::PenPressure),
            scatter: RowState::new(Control::Fade),
            note: String::new(),
            all_off,
        }
    }
}

/// A fake stroke: the S-curve of [`PreviewStroke`] stamped with round dabs of `diameter` points,
/// scaled down to fit the strip as the app's engine does.
fn fake_stroke(size: [usize; 2], diameter: f64, ppp: f32, color: Color32) -> ColorImage {
    let [w, h] = size;
    let mut image = ColorImage::filled(size, Color32::TRANSPARENT);
    let stroke = PreviewStroke::new(w as u32, h as u32);
    let diameter = diameter * f64::from(ppp);
    let fit = stroke.fit(diameter);
    let samples = stroke.samples();
    let mut stamp = |x: f64, y: f64, radius: f64| {
        let (x0, x1) = ((x - radius - 1.0).floor().max(0.0) as usize, ((x + radius + 1.0).ceil().max(0.0) as usize).min(w));
        let (y0, y1) = ((y - radius - 1.0).floor().max(0.0) as usize, ((y + radius + 1.0).ceil().max(0.0) as usize).min(h));
        for py in y0..y1 {
            for px in x0..x1 {
                let reach = (px as f64 + 0.5 - x).hypot(py as f64 + 0.5 - y);
                let cover = (radius + 0.5 - reach).clamp(0.0, 1.0) as f32;
                let dab = color.gamma_multiply(cover);
                let pixel = &mut image.pixels[py * w + px];
                if dab.a() > pixel.a() {
                    *pixel = dab;
                }
            }
        }
    };
    const SUBSTEPS: usize = 6;
    for pair in samples.windows(2) {
        for step in 0..=SUBSTEPS {
            let t = step as f64 / SUBSTEPS as f64;
            let at = |a: f64, b: f64| a + (b - a) * t;
            let pressure = at(f64::from(pair[0].pressure), f64::from(pair[1].pressure));
            stamp(at(pair[0].x, pair[1].x), at(pair[0].y, pair[1].y), diameter * fit * pressure / 2.0);
        }
    }
    image
}

fn stroke_color(ui: &Ui) -> Color32 {
    Tokens::of(ui).color.accent.solid.to_egui()
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// The brush kept for `name`, rebuilt (keeping the selection) when the all-off arg changes.
fn load_brush(ui: &Ui, name: &str, words: &BrushSettingsStoryLabels, all_off: bool) -> Arc<Mutex<Brush>> {
    let kept = shared(ui, name, || Brush::new(words, all_off));
    {
        let mut brush = lock(&kept);
        if brush.all_off != all_off {
            let selected = brush.selected;
            *brush = Brush::new(words, all_off);
            brush.selected = selected;
        }
    }
    kept
}

// --- sections ---

fn show_sections(ui: &mut Ui, args: &mut SectionsArgs) {
    let words = BrushSettingsStoryLabels::of(ui);
    let kept = load_brush(ui, "sections", &words, args.all_off);
    let mut brush = lock(&kept);
    let out = BrushSectionList::new("brush-sections", &brush.sections).selected(Some(&brush.selected)).link(args.link).show(ui);
    for event in out.events {
        apply(&mut brush, event, &words);
    }
    status(ui, brush.note.clone());
}

// --- panel ---

/// The fake body of a section; three sections have controls.
fn show_body(ui: &mut Ui, section: Key, brush: &mut Brush, words: &BrushSettingsStoryLabels, diameter: &mut f64) {
    match section {
        TIP_SHAPE => {
            let edit = TipEllipse::new("panel-ellipse", brush.angle).show(ui);
            if let Some(angle) = edit.event.as_ref().and_then(|event| event.value()) {
                brush.angle = *angle;
            }
            let edit = BrushSizeField::new("panel-size", words.size.as_str(), *diameter, 2500.0).restore(Some(80.0)).show(ui);
            if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                *diameter = *value;
            }
        }
        SHAPE_DYNAMICS => {
            ui.add(Text::new(words.size_jitter.as_str()).content(Content::Muted));
            brush.jitter.show(ui, "panel-jitter", &controls(words), false);
        }
        SCATTERING => brush.scatter.show(ui, "panel-scatter", &controls(words), false),
        _ => {
            ui.add(Text::new(words.no_body.as_str()).content(Content::Muted));
        }
    }
}

fn show_panel(ui: &mut Ui, args: &mut PanelArgs) {
    let words = BrushSettingsStoryLabels::of(ui);
    let kept = load_brush(ui, "panel", &words, false);
    let mut brush = lock(&kept);
    let style = BrushSettingsStyle::of(&Tokens::of(ui));
    let color = stroke_color(ui);
    let ppp = ui.pixels_per_point();
    let diameter = args.diameter;
    let revision = Revision::of(&(diameter.to_bits(), brush.angle.degrees.to_bits(), brush.angle.roundness.to_bits()));
    let mut new_diameter = diameter;
    let (sections, selected) = (brush.sections.clone(), brush.selected);
    let out = ui
        .allocate_ui(vec2(style.panel_width, args.height as f32), |ui| {
            BrushSettingsPanel::new("brush-settings")
                .sections(&sections)
                .selected(Some(&selected))
                .link(args.link)
                .can_update(args.can_update)
                .strip(revision, |size| fake_stroke(size, diameter, ppp, color))
                .show(ui, |ui, section| show_body(ui, *section, &mut brush, &words, &mut new_diameter))
        })
        .inner;
    args.diameter = new_diameter;
    for event in out.sections {
        apply(&mut brush, event, &words);
    }
    match out.action {
        Some(BrushPanelAction::NewPreset) => brush.note = words.action_new.clone(),
        Some(BrushPanelAction::Reset) => brush.note = words.action_reset.clone(),
        Some(BrushPanelAction::UpdatePreset) => brush.note = words.action_update.clone(),
        _ => {}
    }
    status(ui, brush.note.clone());
}

// --- ellipse ---

fn show_ellipse(ui: &mut Ui, args: &mut EllipseArgs) {
    let words = BrushSettingsStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let value = TipAngle { degrees: args.degrees, roundness: args.roundness / 100.0 };
    let edit = ui.add_enabled_ui(args.enabled, |ui| TipEllipse::new("tip-ellipse", value).show(ui)).inner;
    let shown = match edit.event.as_ref().and_then(|event| event.value()) {
        Some(angle) => {
            args.degrees = angle.degrees;
            args.roundness = (angle.roundness * 100.0).round();
            *angle
        }
        None => value,
    };
    status(
        ui,
        craft_i18n::fmt(
            &words.angle_note,
            &[("angle", &numbers.format(shown.degrees, 0..=0)), ("roundness", &numbers.format((shown.roundness * 100.0).round(), 0..=0))],
        ),
    );
}

// --- dynamics ---

fn show_dynamics(ui: &mut Ui, args: &mut DynamicsArgs) {
    let words = BrushSettingsStoryLabels::of(ui);
    let list = controls(&words);
    let kept = shared(ui, "dynamics", || (RowState::new(Control::Fade), RowState::new(Control::PenPressure)));
    {
        let mut rows = lock(&kept);
        let (fade, curve) = &mut *rows;
        fade.show(ui, "dynamics-fade", &list, false);
        curve.show(ui, "dynamics-curve", &list, args.curve_shaped);
        status(ui, if curve.curve_open { words.curve_open.clone() } else { String::new() });
    }

    ui.add_space(ui.spacing().item_spacing.y * 2.0);
    ui.add(Text::new(words.with_restore.as_str()).content(Content::Muted));
    let edit = BrushSizeField::new("dynamics-size-restore", words.size.as_str(), args.diameter, args.max).restore(Some(args.original)).show(ui);
    if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
        args.diameter = *value;
    }
    ui.add(Text::new(words.without_restore.as_str()).content(Content::Muted));
    let edit = BrushSizeField::new("dynamics-size-plain", words.size.as_str(), args.diameter, args.max).show(ui);
    if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
        args.diameter = *value;
    }
}

// --- strip ---

fn show_strip(ui: &mut Ui, args: &mut StripArgs) {
    let words = BrushSettingsStoryLabels::of(ui);
    let style = BrushSettingsStyle::of(&Tokens::of(ui));
    let color = stroke_color(ui);
    let ppp = ui.pixels_per_point();
    let diameter = args.diameter;
    let revision = Revision::of(&diameter.to_bits());
    let widths = [(&words.narrow, args.narrow), (&words.medium, args.medium), (&words.wide, args.wide)];
    for (index, (caption, width)) in widths.into_iter().enumerate() {
        status(ui, caption.clone());
        ui.allocate_ui(vec2(width as f32, style.strip_height), |ui| {
            BrushStrokeStrip::new(("brush-strip", index), revision).show(ui, |size| fake_stroke(size, diameter, ppp, color));
        });
    }
}
