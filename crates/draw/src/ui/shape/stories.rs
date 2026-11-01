//! Stories of the shape panel and options bar (task ST-SH1; task RG2 registers them). A fake shape
//! layer (a `LiveShape` and a `ShapeAppearance`) stands for the document, and the panel and the bar
//! edit it as an app does: each field's `EditEvent` becomes a change of the fake, and a cancelled
//! gesture restores it. Sample words are story words; the plain strings are the status lines.
//!
//! - `patterns/shape-panel/sections`: Appearance and Shape for Rect, Ellipse, Polygon, Star and Line; a locked
//!   layer disables every field.
//! - `patterns/shape-options/bar`: the bar for each Shape tool kind, over the tool settings.
//!
//! RG2 registers `stories()` and the components `ShapePanel` and `ShapeOptionsBar`, and RG1 adds
//! [`ShapeStoryLabels`] to `WORDS`.

use egui::Ui;

use super::{LiveShape, ShapeAppearance, ShapeChange, ShapeOptionsBar, ShapePanel, ShapeToolSettings};
use crate::ui::shape_drag::ShapeKind;
use craft_color::ui::Paint;
use craft_theme::Srgba;
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ShapePanel", "ShapeOptionsBar"];

/// The panel's width: the Properties column an app docks it in, which no token names.
const PANEL_WIDTH: f32 = 280.0;

craft_ui::words! {
    /// The captions of the shape stories, English by default.
    #[non_exhaustive]
    pub(crate) struct ShapeStoryLabels {
        bar: "shapeStory.bar" = "Shape options";
        locked: "shapeStory.locked" = "Layer locked";
        change: "shapeStory.change" = "Last change: {change}";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/shape-panel/sections",
            Level::Patterns,
            "ShapePanel",
            "Panel",
            PanelArgs { shape: PanelShape::Rectangle, locked: false },
            show_panel
        ),
        typed_story!(
            "patterns/shape-options/bar",
            Level::Patterns,
            "ShapeOptionsBar",
            "Options bar",
            BarArgs { kind: BarKind::Rectangle, locked: false },
            show_bar
        ),
    ]
}

// --- the fake layer ---

/// The fake shape layer both stories edit.
#[derive(Clone, Debug, PartialEq)]
struct FakeShape {
    shape: LiveShape,
    look: ShapeAppearance,
    stroke_open: bool,
    /// The layer before the gesture in progress, put back by a cancelled gesture.
    before: Option<(LiveShape, ShapeAppearance)>,
    note: String,
}

impl FakeShape {
    fn new(shape: LiveShape, locked: bool) -> Self {
        let look = ShapeAppearance::new(Paint::None, Paint::None, 0.0).locked(locked);
        Self { shape, look, stroke_open: false, before: None, note: String::new() }
    }

    /// Takes the paints from the theme: the accent for the fill and the text colour for the stroke.
    fn themed(mut self, tokens: &Tokens) -> Self {
        self.look.fill = Paint::Solid(Srgba::from(tokens.color.accent.solid));
        self.look.stroke = Paint::Solid(Srgba::from(tokens.color.text.normal));
        self
    }

    /// Answers one field event as the app answers it: a value changes the layer, a cancel puts the
    /// layer back.
    fn apply(&mut self, event: EditEvent<ShapeChange>) {
        if matches!(event, EditEvent::Begin { .. }) {
            self.before = Some((self.shape.clone(), self.look.clone()));
        }
        if let EditEvent::Cancel { .. } = event {
            if let Some((shape, look)) = self.before.take() {
                self.shape = shape;
                self.look = look;
            }
            return;
        }
        if let Some(change) = event.value() {
            self.note = format!("{change:?}");
            self.change(change.clone());
        }
        if matches!(event, EditEvent::Commit { .. } | EditEvent::Set { .. }) {
            self.before = None;
        }
    }

    fn change(&mut self, change: ShapeChange) {
        match change {
            ShapeChange::Fill(paint) => self.look.fill = paint,
            ShapeChange::FillOn(_) => {}
            ShapeChange::Stroke(paint) => self.look.stroke = paint,
            ShapeChange::StrokeWidth(width) => self.look.stroke_width = width,
            ShapeChange::StrokeStyle(style) => self.look.stroke_style = style,
            ShapeChange::Size { width: w, height: h } => match &mut self.shape {
                LiveShape::Rect { width, height, .. } | LiveShape::Ellipse { width, height } | LiveShape::Polygon { width, height, .. } => {
                    (*width, *height) = (w, h)
                }
                LiveShape::Line { .. } => {}
            },
            ShapeChange::Radii(next) => {
                if let LiveShape::Rect { radii, .. } = &mut self.shape {
                    *radii = next;
                }
            }
            ShapeChange::CornerRadius(r) => {
                if let LiveShape::Polygon { corner_radius, .. } = &mut self.shape {
                    *corner_radius = r;
                }
            }
            ShapeChange::Sides(n) => {
                if let LiveShape::Polygon { sides, .. } = &mut self.shape {
                    *sides = n;
                }
            }
            ShapeChange::StarRatio(r) => {
                if let LiveShape::Polygon { star_ratio, .. } = &mut self.shape {
                    *star_ratio = r;
                }
            }
            ShapeChange::Weight(w) => {
                if let LiveShape::Line { weight } = &mut self.shape {
                    *weight = w;
                }
            }
        }
    }
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

fn kept_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_draw::shape::stories", name))
}

fn show_note(ui: &mut Ui, words: &ShapeStoryLabels, note: &str) {
    if !note.is_empty() {
        status(ui, craft_i18n::fmt(&words.change, &[("change", note)]));
    }
}

// --- panel ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PanelShape {
    Rectangle,
    Ellipse,
    Polygon,
    Star,
    Line,
}

impl PanelShape {
    fn shape(self) -> LiveShape {
        match self {
            PanelShape::Rectangle => LiveShape::Rect { width: 320.0, height: 200.0, radii: [12.0, 12.0, 12.0, 12.0] },
            PanelShape::Ellipse => LiveShape::Ellipse { width: 240.0, height: 180.0 },
            PanelShape::Polygon => LiveShape::Polygon { width: 200.0, height: 200.0, corner_radius: 0.0, sides: 6, star_ratio: 1.0 },
            PanelShape::Star => LiveShape::Polygon { width: 200.0, height: 200.0, corner_radius: 4.0, sides: 5, star_ratio: 0.5 },
            PanelShape::Line => LiveShape::Line { weight: 3.0 },
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PanelArgs {
    shape: PanelShape,
    /// Disables every field, as a layer with `locks.all` or `locks.pixels` does.
    locked: bool,
}

/// The Appearance and Shape sections of the fake layer; a field edits it.
fn show_panel(ui: &mut Ui, args: &mut PanelArgs) {
    let words = ShapeStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let key = kept_key(ui, "panel");
    let tokens = Tokens::of(ui);
    let seed = (args.shape, args.locked);
    let fresh = || FakeShape::new(args.shape.shape(), args.locked).themed(&tokens);
    let mut kept: (PanelArgsSeed, FakeShape) = ctx
        .data(|d| d.get_temp(key))
        .filter(|(s, _): &(PanelArgsSeed, FakeShape)| s.0 == seed.0 && s.1 == seed.1)
        .unwrap_or_else(|| (PanelArgsSeed(seed.0, seed.1), fresh()));
    let fake = &mut kept.1;

    let events = ui
        .scope(|ui| {
            ui.set_max_width(PANEL_WIDTH);
            let FakeShape { shape, look, stroke_open, .. } = fake;
            ShapePanel::new("shape-panel", shape, look).stroke_open(stroke_open).show(ui).events
        })
        .inner;
    for event in events {
        fake.apply(event);
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            *fake = fresh();
        }
        if fake.look.locked {
            status(ui, words.locked.clone());
        }
    });
    show_note(ui, &words, &fake.note);
    ctx.data_mut(|d| d.insert_temp(key, kept));
}

/// What the panel story was built from, to start over when an arg changes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PanelArgsSeed(PanelShape, bool);

// --- options bar ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum BarKind {
    Rectangle,
    #[choice(name = "Rounded Rectangle")]
    RoundedRectangle,
    Ellipse,
    Triangle,
    Polygon,
    Star,
    Line,
    Custom,
}

impl BarKind {
    fn kind(self) -> ShapeKind {
        match self {
            BarKind::Rectangle => ShapeKind::Rectangle,
            BarKind::RoundedRectangle => ShapeKind::RoundedRectangle,
            BarKind::Ellipse => ShapeKind::Ellipse,
            BarKind::Triangle => ShapeKind::Triangle,
            BarKind::Polygon => ShapeKind::Polygon,
            BarKind::Star => ShapeKind::Star,
            BarKind::Line => ShapeKind::Line,
            BarKind::Custom => ShapeKind::Custom,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// The Shape tool; its kind picks the last cell (Radius, Sides, Weight or none).
    kind: BarKind,
    /// Disables every field, as a locked selected layer does.
    locked: bool,
}

/// What the bar story keeps: the tool settings and the paints, which are the app's colours.
#[derive(Clone, Debug, PartialEq)]
struct BarKept {
    settings: ShapeToolSettings,
    fake: FakeShape,
}

/// The bar in an options bar; its events change the tool settings and the paints.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = ShapeStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let key = kept_key(ui, "bar");
    let tokens = Tokens::of(ui);
    let fresh = || BarKept { settings: ShapeToolSettings::default(), fake: FakeShape::new(LiveShape::Line { weight: 3.0 }, false).themed(&tokens) };
    let mut kept: BarKept = ctx.data(|d| d.get_temp(key)).unwrap_or_else(fresh);
    kept.fake.look.locked = args.locked;
    let BarKept { settings, fake } = &mut kept;
    // The bar shows the settings: Fill is on when the paint is, and the width is the tool's.
    let fill = fake.look.fill.clone();
    let mut look = fake.look.clone();
    look.stroke_width = settings.stroke_width;
    if !settings.fill_on {
        look.fill = Paint::None;
    }

    let mut events = Vec::new();
    OptionsBar::new("shape-options").label(words.bar.clone()).show(ui, |slots| {
        slots.body(|ui| {
            events = ShapeOptionsBar::new("shape-options-bar", args.kind.kind(), settings, &look).stroke_open(&mut fake.stroke_open).show(ui).events;
        });
    });
    for event in events {
        if let Some(change) = event.value() {
            fake.note = format!("{change:?}");
            match change.clone() {
                ShapeChange::FillOn(on) => settings.fill_on = on,
                ShapeChange::CornerRadius(r) => settings.corner_radius = r,
                ShapeChange::Sides(n) => settings.sides = n,
                ShapeChange::StarRatio(r) => settings.star_ratio = r,
                ShapeChange::Weight(w) => settings.line_weight = w,
                ShapeChange::StrokeWidth(w) => settings.stroke_width = w,
                other => fake.change(other),
            }
        }
    }
    if !settings.fill_on {
        fake.look.fill = fill;
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            kept = fresh();
        }
    });
    show_note(ui, &words, &kept.fake.note);
    ctx.data_mut(|d| d.insert_temp(key, kept));
}
