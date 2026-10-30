//! Renders gallery stories to PNG under a chosen appearance, without a window.
//!
//! The `capture` binary reads a spec: a JSON list of [`Shot`]s. Each shot draws one story the way
//! the gallery's canvas does (the story on its surface, or the shot's, in the dock colour)
//! with `egui_kittest` and wgpu, through `craft_ui::testing`'s renderer. The faces come from
//! `CRAFT_FONTS_DIR`, installed by [`Appearance::configure`] as in the gallery.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use craft_theme::{ContrastChoice, ModeChoice, Size, SystemAppearance, builtins};
use craft_ui::story::{ArgValue, Args, Level, Story, Surface};
use craft_ui::testing::{symbols, wgpu_renderer};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf, install};
use egui::accesskit::{Action, Role};
use egui::{Pos2, Rect, Vec2};
use egui_kittest::kittest::NodeT as _;
use egui_kittest::{Harness, LazyRenderer};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::appearance::{Appearance, builtin};
use crate::faces;
use crate::gallery::{Tile, Tiles, args_of, canvas, canvas_frame, default_story_of, nav, stories, tiles_view};

/// The width, in points, of a shot that gives none. The story's frame fills the width it is
/// given, so a story has no intrinsic width to fit.
pub const DEFAULT_WIDTH: f32 = 640.0;

/// The tallest a shot that gives no height can be, in points.
const MAX_FIT_HEIGHT: f32 = 8192.0;

/// A widget state to show on the story's first interactive widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// The pointer rests on the widget.
    Hover,
    /// The primary button is held down on the widget.
    Pressed,
}

/// One image of the spec.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shot {
    /// The file name without `.png`.
    pub name: String,
    /// The component's display name or id slug, ignoring case: `Button` or `button`. Give this
    /// or `level`.
    #[serde(default)]
    pub component: Option<String>,
    /// A level's slug, ignoring case (`controls`): the whole Overview of that level, every
    /// component's default story in tiles, as wide as `width`. Takes no `story`, `args` or
    /// `state`.
    #[serde(default)]
    pub level: Option<String>,
    /// The story's display name or last id segment, ignoring case; the component's first story
    /// when absent.
    #[serde(default)]
    pub story: Option<String>,
    /// Overrides of the story's args by name: a boolean, an option name, a string or a number,
    /// by the arg's kind.
    #[serde(default)]
    pub args: Map<String, Value>,
    /// The surface the story is drawn on: `panel`, `canvas`, `chrome` or `own`; the story's own when
    /// absent. An overview shot takes none: its tiles keep their stories' surfaces.
    #[serde(default)]
    pub surface: Option<String>,
    /// A built-in theme id such as `craft.artcraft`; the suite default when absent.
    #[serde(default)]
    pub theme: Option<String>,
    /// `light`, `dark` or `system`; light when absent.
    #[serde(default)]
    pub mode: Option<ModeChoice>,
    /// `standard`, `high` or `system`; standard when absent.
    #[serde(default)]
    pub contrast: Option<ContrastChoice>,
    /// `compact` or `comfortable`; compact when absent.
    #[serde(default)]
    pub size: Option<Size>,
    /// Defaults to 1.
    #[serde(default = "one")]
    pub pixels_per_point: f32,
    /// In points. Without a height the image is as tall as the content; without a width it is
    /// [`DEFAULT_WIDTH`] wide. A height needs a width.
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
    /// The state of the story's first interactive widget.
    #[serde(default)]
    pub state: Option<State>,
}

fn one() -> f32 {
    1.0
}

/// What a shot draws.
enum Content {
    Story(Story, Args),
    /// The default stories of one level's components, in the Overview's order.
    Overview(Vec<Story>),
}

/// A shot with its story, args and appearance resolved.
pub struct Resolved {
    name: String,
    content: Content,
    appearance: Appearance,
    pixels_per_point: f32,
    /// The image's width in points; `size` holds it with the height when both are given.
    width: f32,
    size: Option<Vec2>,
    state: Option<State>,
}

/// The shots of a spec file's text.
pub fn parse_spec(text: &str) -> Result<Vec<Shot>, String> {
    let shots: Vec<Shot> = serde_json::from_str(text).map_err(|e| format!("spec: {e}"))?;
    for (i, shot) in shots.iter().enumerate() {
        let name = &shot.name;
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(format!("spec: shot {i}: name `{name}` is not a file name"));
        }
        if shots[..i].iter().any(|s| s.name == *name) {
            return Err(format!("spec: shot name `{name}` is used twice"));
        }
    }
    Ok(shots)
}

/// Every component with its stories, one component per line.
pub fn list() -> String {
    let stories = stories();
    let mut out = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for story in &stories {
        if seen.contains(&story.id_component()) {
            continue;
        }
        seen.push(story.id_component());
        let names: Vec<&str> = stories.iter().filter(|s| s.component == story.component && s.level == story.level).map(|s| s.slug()).collect();
        out.push_str(&format!("{} ({}): {}\n", story.component, story.level.slug(), names.join(", ")));
    }
    out
}

trait StoryNames {
    fn id_component(&self) -> &'static str;
    fn slug(&self) -> &'static str;
}

impl StoryNames for Story {
    /// The component's slug, the id's second segment.
    fn id_component(&self) -> &'static str {
        self.id.split('/').nth(1).unwrap_or(self.id)
    }

    /// The story's slug, the id's last segment.
    fn slug(&self) -> &'static str {
        self.id.rsplit('/').next().unwrap_or(self.id)
    }
}

/// The story of `component` the shot names, with its args applied.
fn story_content(shot: &Shot, component: &str, stories: &[Story]) -> Result<Content, String> {
    let wanted = component.to_lowercase();
    let in_component: Vec<Story> = stories.iter().filter(|s| s.component.to_lowercase() == wanted || s.id_component() == wanted).copied().collect();
    let Some(first) = in_component.first().and_then(|s| default_story_of(&in_component, s.component)) else {
        let mut components: Vec<&str> = stories.iter().map(|s| s.component).collect();
        components.dedup();
        return Err(format!("unknown component `{component}`; components: {}", components.join(", ")));
    };
    let story = match &shot.story {
        None => *first,
        Some(name) => {
            let name = name.to_lowercase();
            match in_component.iter().find(|s| s.name.to_lowercase() == name || s.slug() == name) {
                Some(story) => *story,
                None => {
                    let names: Vec<&str> = in_component.iter().map(|s| s.slug()).collect();
                    return Err(format!("{} has no story `{name}`; stories: {}", first.component, names.join(", ")));
                }
            }
        }
    };
    let story = match &shot.surface {
        None => story,
        Some(name) => {
            let wanted = name.to_lowercase();
            let Some(surface) = Surface::ALL.into_iter().find(|s| s.slug() == wanted) else {
                let names: Vec<&str> = Surface::ALL.into_iter().map(Surface::slug).collect();
                return Err(format!("unknown surface `{name}`; surfaces: {}", names.join(", ")));
            };
            story.on(surface)
        }
    };
    let mut args = story.defaults();
    apply_args(&mut args, &shot.args)?;
    Ok(Content::Story(story, args))
}

/// The default stories of the components of the level `slug`.
fn overview_content(shot: &Shot, slug: &str, stories: &[Story]) -> Result<Content, String> {
    if shot.story.is_some() || !shot.args.is_empty() || shot.state.is_some() {
        return Err("an overview shot takes no `story`, `args` or `state`".to_owned());
    }
    if shot.surface.is_some() {
        return Err("an overview shot takes no `surface`: its tiles keep their stories' surfaces".to_owned());
    }
    let wanted = slug.to_lowercase();
    let Some(level) = Level::ALL.into_iter().find(|l| l.slug() == wanted) else {
        let slugs: Vec<&str> = Level::ALL.into_iter().map(Level::slug).collect();
        return Err(format!("unknown level `{slug}`; levels: {}", slugs.join(", ")));
    };
    let tiles = nav(stories, "")
        .into_iter()
        .find(|(l, _)| *l == level)
        .map(|(_, components)| components.iter().filter_map(|c| default_story_of(stories, c)).copied().collect::<Vec<_>>())
        .filter(|tiles| !tiles.is_empty())
        .ok_or_else(|| format!("level `{slug}` has no components"))?;
    Ok(Content::Overview(tiles))
}

/// Finds the shot's story and applies its args and appearance. The error names the valid values.
pub fn resolve(shot: &Shot) -> Result<Resolved, String> {
    let located = |e: String| format!("shot `{}`: {e}", shot.name);
    let stories = stories();
    let content = match (&shot.component, &shot.level) {
        (Some(component), None) => story_content(shot, component, &stories).map_err(located)?,
        (None, Some(level)) => overview_content(shot, level, &stories).map_err(located)?,
        _ => return Err(located("give either `component` or `level`".to_owned())),
    };
    let mut appearance = Appearance::default();
    if let Some(theme) = &shot.theme {
        if builtin(theme).is_none() {
            let ids: Vec<&str> = builtins().iter().map(|t| t.id.as_str()).collect();
            return Err(located(format!("unknown theme `{theme}`; themes: {}", ids.join(", "))));
        }
        appearance.theme.clone_from(theme);
    }
    appearance.mode = shot.mode.unwrap_or(ModeChoice::Light);
    appearance.contrast = shot.contrast.unwrap_or(ContrastChoice::Standard);
    appearance.comfortable = shot.size == Some(Size::Comfortable);

    if !(shot.pixels_per_point.is_finite() && shot.pixels_per_point > 0.0) {
        return Err(located(format!("pixels_per_point must be positive, not {}", shot.pixels_per_point)));
    }
    if [shot.width, shot.height].into_iter().flatten().any(|v| v.is_nan() || v <= 0.0) {
        return Err(located("width and height must be positive".to_owned()));
    }
    let width = shot.width.unwrap_or(DEFAULT_WIDTH);
    let size = match (shot.width, shot.height) {
        (Some(w), Some(h)) => Some(Vec2::new(w, h)),
        (None, None) | (Some(_), None) => None,
        (None, Some(_)) => return Err(located("give `width` with `height`".to_owned())),
    };
    Ok(Resolved { name: shot.name.clone(), content, appearance, pixels_per_point: shot.pixels_per_point, width, size, state: shot.state })
}

fn apply_args(args: &mut Args, overrides: &Map<String, Value>) -> Result<(), String> {
    for (name, value) in overrides {
        let known: Vec<&str> = args.iter().map(|a| a.name).collect();
        let Some(arg) = args.iter_mut().find(|a| a.name == name) else {
            return Err(format!("unknown arg `{name}`; args: {}", known.join(", ")));
        };
        let mismatch = |kind: &str| format!("arg `{name}` is {kind}, not {value}");
        match &mut arg.value {
            ArgValue::Bool(b) => *b = value.as_bool().ok_or_else(|| mismatch("a boolean"))?,
            ArgValue::Text(t) => *t = value.as_str().ok_or_else(|| mismatch("a string"))?.to_owned(),
            ArgValue::Number { value: n, min, max } => {
                let wanted = value.as_f64().ok_or_else(|| mismatch("a number"))?;
                if !(*min..=*max).contains(&wanted) {
                    return Err(format!("arg `{name}` takes {min} to {max}, not {wanted}"));
                }
                *n = wanted;
            }
            ArgValue::Choice { options, index } => {
                let wanted = value.as_str().ok_or_else(|| mismatch("one of the options"))?;
                *index = options
                    .iter()
                    .position(|o| *o == wanted)
                    .ok_or_else(|| format!("arg `{name}` has no option `{wanted}`; options: {}", options.join(", ")))?;
            }
        }
    }
    Ok(())
}

/// Draws the shot and returns its image.
pub fn render(shot: &Resolved) -> Result<image::RgbaImage, String> {
    let system = SystemAppearance::default();
    let base = shot.appearance.base(system);
    let appearance = shot.appearance.clone();
    let fixed = shot.size;
    let setup = shot.appearance.clone();
    let mut builder = Harness::builder()
        .with_size(fixed.unwrap_or(Vec2::new(shot.width, MAX_FIT_HEIGHT)))
        .with_pixels_per_point(shot.pixels_per_point)
        .with_accessibility_check(false)
        .with_render_cursor(false)
        .renderer(LazyRenderer::new(wgpu_renderer))
        .with_context_setup(move |ctx| {
            symbols::install(ctx);
            setup.configure(ctx, None, false, system);
        });
    if !faces::available() {
        builder = builder.allow_missing_glyphs();
    }
    let used = Rc::new(Cell::new(Rect::NOTHING));
    let measured = Rc::clone(&used);
    let (tiles, initial) = match &shot.content {
        Content::Story(story, args) => (Vec::new(), HashMap::from([(story.id.to_owned(), args.clone())])),
        Content::Overview(tiles) => (
            tiles.iter().map(|s| Tile { key: s.component, story: *s, title: s.component, note: None, open: false }).collect::<Vec<_>>(),
            HashMap::new(),
        ),
    };
    let story = match &shot.content {
        Content::Story(story, _) => Some(*story),
        Content::Overview(_) => None,
    };
    let mut harness = builder.build_ui_state(
        move |ui, args: &mut HashMap<String, Args>| {
            install(ui.ctx(), &base);
            let tokens = Tokens::of(ui);
            // The harness leaves a transparent margin around its panel; the image is all dock colour.
            ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, tokens.color.surface.background.to_egui());
            let frame = canvas_frame(&tokens).show(ui, |ui| {
                if fixed.is_some() {
                    ui.set_min_size(ui.available_size());
                }
                match &story {
                    Some(story) => {
                        let args = args_of(args, &HashMap::new(), story);
                        canvas(ui, story, args, &appearance, system, None);
                    }
                    None => {
                        let mut scroll_to = None;
                        let config = Tiles {
                            one_column: false,
                            selectable: false,
                            selected: None,
                            buttons: false,
                            scale_of_selected: false,
                            scroll_to: &mut scroll_to,
                            inspector: None,
                        };
                        tiles_view(ui, &tiles, config, args, &HashMap::new(), &appearance, system);
                    }
                }
            });
            measured.set(frame.response.rect);
        },
        initial,
    );
    if fixed.is_none() {
        // The frame hugs the story at the tall initial size; the image is as tall as the frame.
        let _ = harness.run_ok();
        harness.set_size(Vec2::new(shot.width, used.get().height().ceil()));
    }
    // A spinner repaints every frame, so a story may never settle completely.
    let _ = harness.run_ok();
    if let Some(state) = shot.state {
        let at = first_interactive(&harness).ok_or_else(|| format!("shot `{}`: the story has no interactive widget for `state`", shot.name))?;
        harness.hover_at(at);
        let _ = harness.run_ok();
        if state == State::Pressed {
            harness.drag_at(at);
            let _ = harness.run_ok();
        }
    }
    harness.render().map_err(|e| format!("shot `{}`: {e}", shot.name))
}

/// The roles of widgets a pointer acts on. A selectable label also takes clicks but is no
/// widget to show a state on.
const INTERACTIVE: [Role; 9] =
    [Role::Button, Role::CheckBox, Role::RadioButton, Role::Switch, Role::Slider, Role::SpinButton, Role::ComboBox, Role::TextInput, Role::Tab];

/// The centre of the first widget in tree order that takes focus or a click.
fn first_interactive(harness: &Harness<'_, HashMap<String, Args>>) -> Option<Pos2> {
    harness
        .root()
        .children_recursive()
        .find(|node| {
            let node = node.accesskit_node();
            let data = node.data();
            !node.is_hidden() && INTERACTIVE.contains(&node.role()) && (data.supports_action(Action::Focus) || data.supports_action(Action::Click))
        })
        .map(|node| node.rect().center())
}

/// Renders every shot of the spec file to `<out_dir>/<name>.png`. Every shot is resolved before
/// the first is drawn, so a mistake in the spec costs no rendering.
pub fn run(spec: &Path, out_dir: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(spec).map_err(|e| format!("{}: {e}", spec.display()))?;
    let shots = parse_spec(&text)?;
    let resolved: Vec<Resolved> = shots.iter().map(resolve).collect::<Result<_, _>>()?;
    std::fs::create_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    for shot in &resolved {
        let image = render(shot)?;
        let path = out_dir.join(format!("{}.png", shot.name));
        image.save(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{} ({}x{})", path.display(), image.width(), image.height());
    }
    Ok(())
}
