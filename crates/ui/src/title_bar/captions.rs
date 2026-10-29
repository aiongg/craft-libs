//! Caption buttons: minimize, maximize or restore, and close, in four desktop families.

use std::sync::Arc;

use craft_i18n::{I18n, Lang};
use craft_window::Button;
use craft_window::policy::{CaptionFamily, Captions};
use egui::{Color32, Id, Painter, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, ViewportCommand, pos2, vec2};

use super::TitleBarStyle;
use super::resize::start_os_move;
use crate::direction::is_rtl;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// The width of one Windows caption button, as the native ones.
pub const WINDOWS_BUTTON_WIDTH: f32 = 46.0;

/// A caption button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Caption {
    Minimize,
    /// Maximize, or Restore while the window is maximized.
    Maximize,
    Close,
}

impl From<Button> for Caption {
    fn from(b: Button) -> Self {
        match b {
            Button::Minimize => Self::Minimize,
            Button::Maximize => Self::Maximize,
            Button::Close => Self::Close,
        }
    }
}

/// The id every widget and piece of state of the bar in a window derives from. `window` is what
/// the bar's `id_salt` was given, or the window's viewport id without one.
pub(crate) fn window_scope(window: impl std::hash::Hash + std::fmt::Debug) -> Id {
    Id::unique(("title_bar", window))
}

/// The stable id of a caption button's widget in the window `window` (the bar's `id_salt`, or the
/// window's viewport id without one), so tests and agents can find it.
pub fn caption_id(window: impl std::hash::Hash + std::fmt::Debug, c: Caption) -> Id {
    caption_id_in(window_scope(window), c)
}

pub(crate) fn caption_id_in(scope: Id, c: Caption) -> Id {
    scope.with(("caption", c))
}

/// Tooltips and accessibility labels of the caption buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptionLabels {
    pub minimize: String,
    pub maximize: String,
    pub restore: String,
    pub close: String,
}

impl Default for CaptionLabels {
    fn default() -> Self {
        Self { minimize: "Minimize".into(), maximize: "Maximize".into(), restore: "Restore".into(), close: "Close".into() }
    }
}

impl CaptionLabels {
    /// The labels in `lang`; a missing translation is the English.
    pub fn from_i18n(i18n: &I18n, lang: Lang) -> Self {
        let d = Self::default();
        let t = |s: &str| i18n.t(lang, s).to_owned();
        Self { minimize: t(&d.minimize), maximize: t(&d.maximize), restore: t(&d.restore), close: t(&d.close) }
    }

    /// Replace each label with `translate(key)`, keeping the English where it returns `None`.
    ///
    /// | Key | English |
    /// |---|---|
    /// | `window.minimize` | Minimize |
    /// | `window.maximize` | Maximize |
    /// | `window.restore` | Restore |
    /// | `window.close` | Close |
    #[must_use]
    pub fn localized(mut self, translate: impl Fn(&str) -> Option<String>) -> Self {
        for (key, label) in [
            ("window.minimize", &mut self.minimize),
            ("window.maximize", &mut self.maximize),
            ("window.restore", &mut self.restore),
            ("window.close", &mut self.close),
        ] {
            if let Some(text) = translate(key) {
                *label = text;
            }
        }
        self
    }

    fn of(&self, c: Caption, maximized: bool) -> &str {
        match c {
            Caption::Minimize => &self.minimize,
            Caption::Maximize if maximized => &self.restore,
            Caption::Maximize => &self.maximize,
            Caption::Close => &self.close,
        }
    }
}

/// A caption glyph, as icon themes name them (`window-restore-symbolic`, …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Renders an icon as a square coverage mask (`side` × `side` bytes, row by row), or `None`.
pub type RenderIconFn = dyn Fn(CaptionIcon, u32) -> Option<Vec<u8>> + Send + Sync;

/// The round buttons' glyphs from the desktop's icon theme (Linux: `window-*-symbolic`), drawn
/// in the glyph colour as symbolic icons are. An icon the theme lacks draws the family's own.
#[derive(Clone)]
pub struct CaptionIcons {
    /// The theme's name; the cached textures follow it.
    pub theme: String,
    pub render: Arc<RenderIconFn>,
}

impl PartialEq for CaptionIcons {
    fn eq(&self, other: &Self) -> bool {
        self.theme == other.theme && Arc::ptr_eq(&self.render, &other.render)
    }
}

impl std::fmt::Debug for CaptionIcons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptionIcons").field("theme", &self.theme).finish_non_exhaustive()
    }
}

/// The button side, the gap on each side of it, and the gap between the outermost button and the
/// window's edge. libadwaita: 24 pt buttons 13 pt apart, 10 pt from the edge (the gap is split
/// between neighbours, so the hit areas touch). Breeze sizes buttons by the font (about 1.3 em,
/// 18 pt at KDE's default 10 pt font), half a button apart, 8 pt from the edge.
pub(crate) fn metrics(family: CaptionFamily) -> (f32, f32, f32) {
    match family {
        CaptionFamily::Windows => (WINDOWS_BUTTON_WIDTH, 0.0, 0.0),
        CaptionFamily::Adwaita | CaptionFamily::Generic => (24.0, 6.5, 10.0),
        CaptionFamily::Breeze => (18.0, 4.5, 8.0),
    }
}

/// The buttons at the (left, right) ends, left to right. A right-to-left UI mirrors the Windows
/// captions; the Linux desktops' layouts are the user's and stay as set.
pub(crate) fn arranged(captions: &Captions, rtl: bool) -> (Vec<Caption>, Vec<Caption>) {
    let side = |s: &[Button]| s.iter().map(|&b| Caption::from(b)).collect::<Vec<_>>();
    let (mut left, mut right) = (side(&captions.layout.left), side(&captions.layout.right));
    if rtl && captions.family == CaptionFamily::Windows {
        std::mem::swap(&mut left, &mut right);
        left.reverse();
        right.reverse();
    }
    (left, right)
}

/// Each button's horizontal extent and face centre on a side starting at `from`, in order.
/// `outer_left` puts the edge gap on the first button's left (the left end) instead of the last
/// button's right.
fn slots(family: CaptionFamily, side: &[Caption], from: f32, outer_left: bool) -> Vec<(Caption, (f32, f32), f32)> {
    let (size, gap, end) = metrics(family);
    let mut x = from;
    let n = side.len();
    side.iter()
        .enumerate()
        .map(|(i, &c)| {
            let before = if outer_left && i == 0 { end } else { gap };
            let after = if !outer_left && i + 1 == n { end } else { gap };
            let slot = (x, x + before + size + after);
            let centre = x + before + size / 2.0;
            x = slot.1;
            (c, slot, centre)
        })
        .collect()
}

/// The width the buttons of one side take.
pub(crate) fn side_width(family: CaptionFamily, side: &[Caption]) -> f32 {
    slots(family, side, 0.0, true).last().map_or(0.0, |(_, (_, r), _)| *r)
}

/// Every button's hit rect (full bar height) and face centre, for a bar `bar` between the
/// window's `left` and `right` edges.
pub(crate) fn place(
    family: CaptionFamily,
    (left_side, right_side): &(Vec<Caption>, Vec<Caption>),
    bar: Rect,
    left: f32,
    right: f32,
) -> Vec<(Caption, Rect, Pos2)> {
    let rect = |(c, (l, r), centre): (Caption, (f32, f32), f32)| {
        (c, Rect::from_min_max(pos2(l, bar.top()), pos2(r, bar.bottom())), pos2(centre, bar.center().y))
    };
    let lefts = slots(family, left_side, left, true);
    let rights = slots(family, right_side, right - side_width(family, right_side), false);
    lefts.into_iter().chain(rights).map(rect).collect()
}

/// How a button looks this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Look {
    pub hovered: bool,
    pub pressed: bool,
    /// The window has keyboard focus.
    pub active: bool,
    pub maximized: bool,
}

impl Look {
    /// The round buttons' disc: 0 at rest, 1 hovered, 2 pressed.
    pub fn level(self) -> f32 {
        if self.pressed && self.hovered {
            2.0
        } else if self.hovered {
            1.0
        } else {
            0.0
        }
    }
}

/// The caption buttons of `captions` at the ends of `bar` (the title bar's outer rect), full bar
/// height and flush with the window's edges even where the bar's contents run wider, so a
/// maximized window's corner hits the outermost one. Minimize and maximize send their
/// [`ViewportCommand`]s; Close only returns [`Caption::Close`]: the app runs its own quit path.
pub fn caption_buttons(ui: &mut Ui, bar: Rect, captions: &Captions, style: &TitleBarStyle) -> Option<Caption> {
    CaptionButtons::new(captions, style).show(ui, bar)
}

/// [`caption_buttons`] with translated labels or icon-theme glyphs.
pub struct CaptionButtons<'a> {
    captions: &'a Captions,
    style: &'a TitleBarStyle,
    scope: Option<Id>,
    labels: Option<&'a CaptionLabels>,
    icons: Option<&'a CaptionIcons>,
}

impl<'a> CaptionButtons<'a> {
    pub fn new(captions: &'a Captions, style: &'a TitleBarStyle) -> Self {
        Self { captions, style, scope: None, labels: None, icons: None }
    }

    /// The window the buttons belong to, as the bar's `id_salt` (default: the viewport id), so
    /// each window's buttons have their own ids.
    #[must_use]
    pub fn window(mut self, window: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.scope = Some(window_scope(window));
        self
    }

    #[must_use]
    pub(crate) fn window_scope(mut self, scope: Id) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Tooltips and accessibility labels; English without.
    #[must_use]
    pub fn labels(mut self, labels: &'a CaptionLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Glyphs from the desktop's icon theme for the round families.
    #[must_use]
    pub fn icons(mut self, icons: Option<&'a CaptionIcons>) -> Self {
        self.icons = icons;
        self
    }

    pub fn show(self, ui: &mut Ui, bar: Rect) -> Option<Caption> {
        self.show_rects(ui, bar).0
    }

    /// The clicked button and every button's rect.
    pub(crate) fn show_rects(self, ui: &mut Ui, bar: Rect) -> (Option<Caption>, Vec<(Caption, Rect)>) {
        let ctx = ui.ctx().clone();
        let family = self.captions.family;
        let windows = family == CaptionFamily::Windows;
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        let active = ctx.input(|i| i.viewport().focused) != Some(false);
        let english = CaptionLabels::default();
        let labels = self.labels.unwrap_or(&english);
        let window = ctx.content_rect();
        let scope = self.scope.unwrap_or_else(|| window_scope(ctx.viewport_id()));
        let motion = Tokens::of(ui).motion.hover;
        let (fade, easing) = (motion.millis as f32 / 1000.0, motion.easing.to_egui());
        let mut clicked = None;
        let mut rects = Vec::new();
        for (c, r, centre) in place(family, &arranged(self.captions, is_rtl(ui)), bar, bar.left().max(window.left()), bar.right().min(window.right()))
        {
            // GTK's and KDE's caption buttons move the window when dragged; Windows' don't. Like the
            // OS's own, they are no Tab stops; screen readers still reach and press them.
            let sense = if windows { Sense::CLICK } else { Sense::CLICK | Sense::DRAG };
            let id = caption_id_in(scope, c);
            let resp = ui.interact(r, id, sense);
            if !windows && resp.drag_started_by(PointerButton::Primary) {
                start_os_move(&ctx);
            }
            let look = Look { hovered: resp.hovered(), pressed: resp.is_pointer_button_down_on(), active, maximized };
            let level = match family {
                // libadwaita eases the disc; `motion.hover` sets how, and reduced motion removes it.
                CaptionFamily::Adwaita | CaptionFamily::Generic => {
                    let animate = |salt: &str, on: bool| ctx.animate_bool_with_time_and_easing(id.with(salt), on, fade, easing);
                    animate("hover", look.hovered) + animate("pressed", look.pressed && look.hovered)
                }
                _ => look.level(),
            };
            paint(ui, family, c, r, centre, look, level, self.style, self.icons);
            let label = labels.of(c, maximized).to_owned();
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            if resp.on_hover_text(label).clicked() {
                clicked = Some(c);
            }
            rects.push((c, r));
        }
        match clicked {
            Some(Caption::Minimize) => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
            Some(Caption::Maximize) => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
            Some(Caption::Close) | None => {}
        }
        (clicked, rects)
    }
}

/// Paints one button: `r` is its hit rect, `centre` its face's centre, `level` the round disc's
/// (eased) state between 0 (rest) and 2 (pressed).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint(
    ui: &Ui,
    family: CaptionFamily,
    c: Caption,
    r: Rect,
    centre: Pos2,
    look: Look,
    level: f32,
    style: &TitleBarStyle,
    icons: Option<&CaptionIcons>,
) {
    let lit = look.hovered || look.pressed;
    let glyph = if lit || look.active { style.glyph } else { style.glyph_inactive };
    if family == CaptionFamily::Windows {
        let close = c == Caption::Close;
        let fill = match (look.pressed, look.hovered) {
            (true, _) if close => style.windows_close.gamma_multiply(0.9),
            (_, true) if close => style.windows_close,
            (true, _) => style.pressed,
            (_, true) => style.hover,
            _ => Color32::TRANSPARENT,
        };
        let p = ui.painter();
        p.rect_filled(r, 0.0, fill);
        let ink = if close && lit { style.windows_close_glyph } else { glyph };
        paint_windows_glyph(p, c, look.maximized, r.center(), Stroke::new(1.0, ink));
    } else {
        paint_round(ui, family, c, centre, look, level, glyph, style, icons);
    }
}

/// The 10 pt line glyphs of the Windows caption buttons.
fn paint_windows_glyph(p: &Painter, c: Caption, maximized: bool, at: Pos2, s: Stroke) {
    let g = Rect::from_center_size(at, vec2(10.0, 10.0));
    match c {
        Caption::Minimize => {
            p.line_segment([g.left_center(), g.right_center()], s);
        }
        // Restore: a front square with the corner of the one behind it.
        Caption::Maximize if maximized => {
            let front = Rect::from_min_max(g.min + vec2(0.0, 2.0), g.max - vec2(2.0, 0.0));
            p.rect_stroke(front, 0.0, s, StrokeKind::Middle);
            p.line(vec![g.min + vec2(2.0, 2.0), g.min + vec2(2.0, 0.0), g.right_top(), g.max - vec2(0.0, 2.0), g.max - vec2(2.0, 2.0)], s);
        }
        Caption::Maximize => {
            p.rect_stroke(g, 0.0, s, StrokeKind::Middle);
        }
        Caption::Close => {
            p.line_segment([g.left_top(), g.right_bottom()], s);
            p.line_segment([g.right_top(), g.left_bottom()], s);
        }
    }
}

/// A round Linux caption button centred at `centre`: its disc, then its glyph on a 16 pt grid.
/// Adwaita's disc rests at the glyph colour at 10 % (libadwaita's `alpha(currentColor)`), so it
/// suits light and dark bars alike; Generic buttons are bare at rest. Breeze's are bare too and
/// invert under the pointer, as KWin's: the disc in the glyph colour (Close in Breeze's reds)
/// and the glyph in the bar's.
#[allow(clippy::too_many_arguments)]
fn paint_round(
    ui: &Ui,
    family: CaptionFamily,
    c: Caption,
    centre: Pos2,
    look: Look,
    level: f32,
    glyph: Color32,
    style: &TitleBarStyle,
    icons: Option<&CaptionIcons>,
) {
    let (size, _, _) = metrics(family);
    // Whole pixels, so the glyphs' edges stay sharp.
    let ppp = ui.ctx().pixels_per_point();
    let snap = |p: Pos2| pos2((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
    let centre = snap(centre);
    let close = c == Caption::Close;
    let (fill, ink) = if family == CaptionFamily::Breeze {
        match (look.hovered, look.pressed && look.hovered) {
            (_, true) if close => (style.breeze_close_pressed, style.breeze_close_pressed_glyph),
            (true, _) if close => (style.breeze_close_hover, style.breeze_close_hover_glyph),
            (_, true) => (style.glyph.gamma_multiply(0.5), style.bar),
            (true, _) => (style.glyph, style.bar),
            _ => (Color32::TRANSPARENT, glyph),
        }
    } else {
        let rest = if family == CaptionFamily::Adwaita { style.glyph.gamma_multiply(0.10) } else { Color32::TRANSPARENT };
        let fill = if level <= 1.0 { rest.lerp_to_gamma(style.hover, level) } else { style.hover.lerp_to_gamma(style.pressed, level - 1.0) };
        (fill, glyph)
    };
    let p = ui.painter();
    if fill.a() > 0 {
        p.circle_filled(centre, size / 2.0, fill);
    }
    // The glyph grid: 16 pt, or 2 pt less than a smaller button.
    let k = (size - 2.0).min(16.0) / 16.0;
    let icon = match c {
        Caption::Minimize => CaptionIcon::Minimize,
        Caption::Maximize if look.maximized => CaptionIcon::Restore,
        Caption::Maximize => CaptionIcon::Maximize,
        Caption::Close => CaptionIcon::Close,
    };
    if let Some(icons) = icons
        && let Some(tex) = theme_icon(ui.ctx(), icons, icon, (16.0 * k * ppp).round() as u32)
    {
        let side = tex.size_vec2() / ppp;
        let rect = Rect::from_min_size(snap(centre - side / 2.0), side);
        p.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), ink);
        return;
    }
    let at = |x: f32, y: f32| centre + vec2(x - 8.0, y - 8.0) * k;
    if family == CaptionFamily::Breeze {
        // Breeze: 1 pt chevrons (down to minimize, up to maximize), a diamond to restore, an X.
        let s = Stroke::new(1.0, ink);
        match c {
            Caption::Minimize => {
                p.line(vec![at(2.5, 5.5), at(8.0, 11.0), at(13.5, 5.5)], s);
            }
            Caption::Maximize if look.maximized => {
                p.add(egui::Shape::closed_line(vec![at(8.0, 3.0), at(13.0, 8.0), at(8.0, 13.0), at(3.0, 8.0)], s));
            }
            Caption::Maximize => {
                p.line(vec![at(2.5, 10.5), at(8.0, 5.0), at(13.5, 10.5)], s);
            }
            Caption::Close => {
                let s = Stroke::new(1.2, ink);
                p.line_segment([at(3.6, 3.6), at(12.4, 12.4)], s);
                p.line_segment([at(12.4, 3.6), at(3.6, 12.4)], s);
            }
        }
    } else {
        // libadwaita: a 2 pt bar low down, a 2 pt square outline (smaller to restore), an X.
        let frame = |a: f32, b: f32| Rect::from_min_max(at(a, a), at(b, b));
        match c {
            Caption::Minimize => {
                p.rect_filled(Rect::from_min_max(at(4.0, 10.0), at(12.0, 12.0)), 0.0, ink);
            }
            Caption::Maximize => {
                let square = if look.maximized { frame(5.0, 11.0) } else { frame(4.0, 12.0) };
                p.rect_stroke(square, 0.0, Stroke::new(2.0 * k, ink), StrokeKind::Inside);
            }
            Caption::Close => {
                let s = Stroke::new(1.5 * k, ink);
                p.line_segment([at(4.5, 4.5), at(11.5, 11.5)], s);
                p.line_segment([at(11.5, 4.5), at(4.5, 11.5)], s);
            }
        }
    }
}

/// `icon` from the theme at `px` device pixels, uploaded once per theme and size; `None` (also
/// remembered) when the theme lacks it or it doesn't render.
pub(crate) fn theme_icon(ctx: &egui::Context, icons: &CaptionIcons, icon: CaptionIcon, px: u32) -> Option<TextureHandle> {
    let id = Id::unique(("title_bar_caption_icon", &icons.theme, icon, px));
    if let Some(cached) = ctx.data(|d| d.get_temp::<Option<TextureHandle>>(id)) {
        return cached;
    }
    let side = usize::try_from(px).ok().filter(|&s| (1..=256).contains(&s));
    let tex = side.and_then(|side| {
        let mask = (icons.render)(icon, px).filter(|m| m.len() == side * side)?;
        // White at the mask's coverage, tinted with the ink when drawn.
        let pixels = mask.into_iter().map(Color32::from_white_alpha).collect();
        let image = egui::ColorImage { size: [side, side], source_size: vec2(px as f32, px as f32), pixels };
        Some(ctx.load_texture(format!("caption-{icon:?}-{px}"), image, egui::TextureOptions::LINEAR))
    });
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}
