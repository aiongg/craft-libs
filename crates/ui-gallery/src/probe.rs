//! The paint probe: which theme tokens a drawing paints with, and where.
//!
//! The drawing runs headless in its own context under a probe theme in which every colour token
//! (pass [`Pass::Semantic`]) or every colour token and component colour field (pass
//! [`Pass::Component`]) has a distinct opaque colour. Reading the painted shapes back, each
//! colour that is a probe colour names the token the drawing painted it with, and the rect it
//! painted. Colours that are not probe colours (blends, scaled colours, constants) count as
//! computed. States a drawing does not show, and closed popups, are not painted and so not found.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use craft_theme::{ColorToken, ComponentField, Contrast, Mode, ModePins, Pins, Rgba, SystemAppearance, ThemeDef, TokenRef, resolve};
use craft_ui::story::{Args, Story};
use craft_ui::theme::{ToEgui, install, scope};
use egui::StrokeKind;
use egui::epaint::{ColorMode, Shape, TextShape};
use egui::{Color32, Context, Pos2, RawInput, Rect, Ui, Vec2};

use crate::appearance::Appearance;

/// Which colours the probe theme pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pass {
    /// Semantic tokens only: a component field that takes a token's colour unchanged paints with
    /// the token's probe colour, so a button fill is found as `accent.solid`.
    Semantic,
    /// Semantic tokens and every component colour field: a button fill is found as
    /// `button.primary.fill.rest`.
    Component,
}

/// One painted colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Use {
    /// The token or field the colour is the probe colour of; `None` for a computed colour.
    pub token: Option<TokenRef>,
    /// Where it was painted, in the drawing's coordinates, clipped to the clip rect it was
    /// painted under.
    pub rect: Rect,
    /// For the stroke of a rectangle: its width. Only that band inside `rect` was painted.
    pub border: Option<f32>,
}

#[allow(dead_code, reason = "Pick from preview uses it")]
impl Use {
    /// Whether `pos` is on what was painted.
    pub fn contains(&self, pos: Pos2) -> bool {
        self.rect.contains(pos) && self.border.is_none_or(|width| !self.rect.shrink(width).contains(pos))
    }
}

/// What a drawing painted, in paint order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub uses: Vec<Use>,
}

#[allow(dead_code, reason = "Pick from preview uses the lookups by position")]
impl Usage {
    /// Every token or field painted with its own colour.
    pub fn tokens(&self) -> BTreeSet<TokenRef> {
        self.uses.iter().filter_map(|u| u.token).collect()
    }

    /// How many colours were painted that are not a probe colour.
    pub fn computed(&self) -> usize {
        self.uses.iter().filter(|u| u.token.is_none()).count()
    }

    /// The token painted last at `pos`; `None` when nothing was painted there or the last paint
    /// there was a computed colour.
    pub fn at(&self, pos: Pos2) -> Option<TokenRef> {
        self.uses.iter().rev().find(|u| u.contains(pos)).and_then(|u| u.token)
    }

    /// Where `token` was painted.
    pub fn rects(&self, token: impl Into<TokenRef>) -> Vec<Rect> {
        let token = token.into();
        self.uses.iter().filter(|u| u.token == Some(token)).map(|u| u.rect).collect()
    }
}

/// Runs drawings under probe themes.
pub struct Probe;

impl Probe {
    /// Draws `draw` at the origin of a headless context `size` large, with the fonts, direction,
    /// scale and size of `appearance` and the colours of `theme` replaced by probe colours
    /// (Standard contrast, `mode`), and reads back what it painted. `draw` runs twice, so the
    /// second pass has settled layout; the usage is that of the second.
    pub fn run(theme: &ThemeDef, appearance: &Appearance, mode: Mode, pass: Pass, size: Vec2, mut draw: impl FnMut(&mut Ui)) -> Usage {
        let (probe, names) = probe_theme(theme, appearance, mode, pass);
        let ctx = Context::default();
        let base = appearance.configure(&ctx, None, false, SystemAppearance::default());
        // design: allow[style-mutation](the probe context mirrors the gallery's interface scale, which craft-ui has no setting for)
        ctx.set_zoom_factor(appearance.zoom());
        let mut output = None;
        for _ in 0..2 {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..RawInput::default() };
            if let Some(previous) = output.take() {
                let previous: egui::FullOutput = previous;
                previous.drop_without_applying_deltas();
            }
            output = Some(ctx.run_ui(raw, |ui| {
                install(ui.ctx(), &base);
                let overrides = |o: &mut craft_ui::theme::Overrides| {
                    appearance.overrides(o);
                    o.theme = Some(probe.clone());
                    o.mode = Some(mode);
                    o.contrast = Some(Contrast::Standard);
                };
                scope(ui, overrides, |ui| draw(ui));
            }));
        }
        let output = output.expect("two passes ran");
        let mut reader = Reader { names: &names, clip: Rect::NOTHING, uses: Vec::new() };
        for clipped in &output.shapes {
            reader.clip = clipped.clip_rect;
            reader.shape(&clipped.shape);
        }
        output.drop_without_applying_deltas();
        Usage { uses: reader.uses }
    }

    /// A story with `args` on its surface frame, as the gallery's canvas draws it.
    pub fn story(theme: &ThemeDef, appearance: &Appearance, mode: Mode, pass: Pass, size: Vec2, story: &Story, args: &mut Args) -> Usage {
        let rtl = appearance.rtl;
        Self::run(theme, appearance, mode, pass, size, |ui| {
            crate::gallery::framed(ui, story, args, None, "", rtl);
        })
    }
}

/// What a cached usage was probed for. The theme's edited values are not part of it: they never
/// change which token a shape paints with.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// The drawing: a story id, or the name of another draw function.
    pub what: String,
    /// What the drawing's arguments were; empty for none.
    pub args: String,
    pub appearance: Appearance,
    pub mode: Mode,
    pub pass: Pass,
    /// The size drawn at, in whole points.
    pub size: [u32; 2],
}

/// The most recent usages, so a view that asks every frame probes once.
#[derive(Default)]
pub struct Cache {
    entries: Vec<(Key, Arc<Usage>)>,
}

impl Cache {
    /// Most usages kept.
    const CAPACITY: usize = 256;

    /// The usage cached for `key`.
    pub fn get(&self, key: &Key) -> Option<Arc<Usage>> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, usage)| Arc::clone(usage))
    }

    /// The usage for `key`, probed with `run` when none is cached.
    pub fn get_or_run(&mut self, key: Key, run: impl FnOnce() -> Usage) -> Arc<Usage> {
        if let Some((_, usage)) = self.entries.iter().find(|(k, _)| *k == key) {
            return Arc::clone(usage);
        }
        let usage = Arc::new(run());
        if self.entries.len() >= Self::CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push((key, Arc::clone(&usage)));
        usage
    }
}

/// `theme` with probe colours pinned in both modes, and the token each probe colour stands for.
fn probe_theme(theme: &ThemeDef, appearance: &Appearance, mode: Mode, pass: Pass) -> (ThemeDef, HashMap<Color32, TokenRef>) {
    let mut probe = theme.clone();
    probe.high_contrast = ModePins::default();
    probe.apps.clear();
    let mut refs: Vec<TokenRef> = ColorToken::ALL.iter().map(|&t| TokenRef::Color(t)).collect();
    let pin_all = |probe: &mut ThemeDef, refs: &[TokenRef]| {
        let pins: Pins = refs.iter().enumerate().map(|(i, &t)| (t, probe_color(i))).collect();
        probe.pins = ModePins { light: pins.clone(), dark: pins };
    };
    pin_all(&mut probe, &refs);
    if pass == Pass::Component {
        // Which fields are colours does not depend on the colours, so one resolve lists them.
        let resolved = resolve(
            &probe,
            craft_theme::Appearance {
                mode,
                contrast: Contrast::Standard,
                size: appearance.size(),
                text_size: appearance.text_size(),
                reduced_motion: false,
            },
            None,
        );
        refs.extend(ComponentField::ALL.iter().filter(|&&f| resolved.tokens.component.color(f).is_some()).map(|&f| TokenRef::Component(f)));
        pin_all(&mut probe, &refs);
    }
    let names = refs.iter().enumerate().map(|(i, &t)| (probe_color(i).to_egui(), t)).collect();
    (probe, names)
}

/// The `i`th probe colour: opaque, and distinct for every `i` below 2^24 (an odd multiplier is a
/// bijection modulo 2^24).
fn probe_color(i: usize) -> Rgba {
    let n = (i as u32 + 1).wrapping_mul(0x9E37_79B1) & 0x00FF_FFFF;
    Rgba([(n >> 16) as u8, (n >> 8) as u8, n as u8, 0xff])
}

/// Collects the painted colours of shapes.
struct Reader<'a> {
    names: &'a HashMap<Color32, TokenRef>,
    clip: Rect,
    uses: Vec<Use>,
}

impl Reader<'_> {
    fn shape(&mut self, shape: &Shape) {
        match shape {
            Shape::Noop | Shape::Callback(_) => {}
            Shape::Vec(shapes) => shapes.iter().for_each(|s| self.shape(s)),
            Shape::Circle(c) => {
                let rect = Rect::from_center_size(c.center, Vec2::splat(2.0 * c.radius));
                self.fill(c.fill, rect);
                self.stroke(c.stroke.color, c.stroke.width, rect);
            }
            Shape::Ellipse(e) => {
                let rect = Rect::from_center_size(e.center, 2.0 * e.radius);
                self.fill(e.fill, rect);
                self.stroke(e.stroke.color, e.stroke.width, rect);
            }
            Shape::LineSegment { points, stroke } => self.stroke(stroke.color, stroke.width, Rect::from_two_pos(points[0], points[1])),
            Shape::Path(p) => {
                let rect = Rect::from_points(&p.points);
                self.fill(p.fill, rect);
                self.path_stroke(&p.stroke, rect);
            }
            Shape::Band(b) => {
                let mut rect = Rect::NOTHING;
                for point in &b.points {
                    rect.extend_with(Pos2::new(point.x, point.y.min));
                    rect.extend_with(Pos2::new(point.x, point.y.max));
                }
                self.fill(b.fill, rect);
                self.stroke(b.stroke.color, b.stroke.width, rect);
            }
            Shape::Rect(r) => {
                self.fill(r.fill, r.rect);
                let width = r.stroke.width;
                let outer = match r.stroke_kind {
                    StrokeKind::Inside => r.rect,
                    StrokeKind::Middle => r.rect.expand(width / 2.0),
                    StrokeKind::Outside => r.rect.expand(width),
                };
                if width > 0.0 {
                    self.add_with(r.stroke.color, outer, Some(width));
                }
            }
            Shape::Text(t) => self.text(t),
            Shape::Mesh(mesh) => {
                let mut groups: Vec<(Color32, Rect)> = Vec::new();
                for v in &mesh.vertices {
                    group(&mut groups, v.color, v.pos);
                }
                for (color, rect) in groups {
                    self.fill(color, rect);
                }
            }
            Shape::QuadraticBezier(b) => {
                let rect = Rect::from_points(&b.points);
                self.fill(b.fill, rect);
                self.path_stroke(&b.stroke, rect);
            }
            Shape::CubicBezier(b) => {
                let rect = Rect::from_points(&b.points);
                self.fill(b.fill, rect);
                self.path_stroke(&b.stroke, rect);
            }
        }
    }

    fn path_stroke(&mut self, stroke: &egui::epaint::PathStroke, rect: Rect) {
        if let ColorMode::Solid(color) = stroke.color {
            self.stroke(color, stroke.width, rect);
        } else {
            self.record(None, rect, None);
        }
    }

    fn stroke(&mut self, color: Color32, width: f32, rect: Rect) {
        if width > 0.0 {
            self.add(color, rect.expand(width / 2.0));
        }
    }

    fn fill(&mut self, color: Color32, rect: Rect) {
        self.add(color, rect);
    }

    /// One use per row and colour, over the row's vertices of that colour. The row's mesh
    /// carries the colour of each glyph already, and the text shape's override and fallback
    /// colours apply to it as the tessellator applies them.
    fn text(&mut self, text: &TextShape) {
        if text.angle != 0.0 || text.opacity_factor < 1.0 {
            // Rotated rows have no axis-aligned vertex bounds, and a faded colour is a blend.
            self.record(None, text.galley.rect.translate(text.pos.to_vec2()), None);
            return;
        }
        for row in &text.galley.rows {
            let offset = text.pos.to_vec2() + row.pos.to_vec2();
            let mut groups: Vec<(Color32, Rect)> = Vec::new();
            for (i, v) in row.visuals.mesh.vertices.iter().enumerate() {
                let color = match text.override_text_color {
                    Some(over) if row.visuals.glyph_vertex_range.contains(&i) => over,
                    Some(_) => v.color,
                    None if v.color == Color32::PLACEHOLDER => text.fallback_color,
                    None => v.color,
                };
                group(&mut groups, color, v.pos + offset);
            }
            for (color, rect) in groups {
                self.add(color, rect);
            }
        }
        if text.underline.width > 0.0 {
            let rect = text.galley.rect.translate(text.pos.to_vec2());
            self.stroke(text.underline.color, text.underline.width, Rect::from_min_max(rect.left_bottom(), rect.right_bottom()));
        }
    }

    /// Records `color` painted over `rect`. Invisible colours and rects outside the clip rect
    /// paint nothing.
    fn add(&mut self, color: Color32, rect: Rect) {
        self.add_with(color, rect, None);
    }

    fn add_with(&mut self, color: Color32, rect: Rect, border: Option<f32>) {
        if color == Color32::TRANSPARENT || color == Color32::PLACEHOLDER {
            return;
        }
        self.record(self.names.get(&color).copied(), rect, border);
    }

    /// Records a paint of `token` (`None`: computed, such as a gradient or a rotated text) over
    /// `rect`, unless `rect` is outside the clip rect.
    fn record(&mut self, token: Option<TokenRef>, rect: Rect, border: Option<f32>) {
        let rect = rect.intersect(self.clip);
        if rect.min.x > rect.max.x || rect.min.y > rect.max.y {
            return;
        }
        self.uses.push(Use { token, rect, border });
    }
}

/// Extends the bounds of `color` in `groups` with `pos`.
fn group(groups: &mut Vec<(Color32, Rect)>, color: Color32, pos: Pos2) {
    match groups.iter_mut().find(|(c, _)| *c == color) {
        Some((_, rect)) => rect.extend_with(pos),
        None => groups.push((color, Rect::from_min_max(pos, pos))),
    }
}

#[cfg(test)]
mod tests;
