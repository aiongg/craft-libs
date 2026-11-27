//! The panel's look: theme `craft.pro`, dark, compact, resolved by craft-theme, and the row
//! geometry of look sheet sections 1.3 and 3. Components read their values here.

use craft_theme::{Appearance, Contrast, Mode, Rgba, Size, TextSize, ThemeId, Tokens, builtins, resolve};
use craft_ui_rt::{PaintCx, Stroke};
use craft_ui_rt::{Pos2, Rect, Vec2};

/// The edge of a rect a rule runs along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// A kind tile's glyph share of the tile's shorter side (`KIND_GLYPH_SHARE`, craft-layers).
pub const KIND_GLYPH_SHARE: f32 = 0.64;
/// A type layer's tile: ink on paper in every theme.
pub const TYPE_PAPER: Rgba = Rgba::rgb(0xfa, 0xfa, 0xfa);
pub const TYPE_INK: Rgba = Rgba::rgb(0x28, 0x28, 0x28);
/// Rows built beyond the view on each side.
pub const OVERSCAN_ROWS: usize = 4;

/// The tokens every v4 run uses: `craft.pro`, dark, compact, standard contrast.
pub fn pro_dark_tokens() -> Tokens {
    let id = ThemeId::parse("craft.pro").ok();
    let theme = builtins().iter().find(|t| Some(&t.id) == id.as_ref()).unwrap_or(&builtins()[0]);
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false };
    resolve(theme, appearance, None).tokens
}

#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub panel: Rgba,
    /// The window's background surface, which the list's scroll fades take (see `scroll::EdgeFades`).
    pub background: Rgba,
    pub sunken: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub text_on_selected: Rgba,
    pub hover: Rgba,
    pub pressed: Rgba,
    pub selected: Rgba,
    pub divider: Rgba,
    pub border: Rgba,
    pub control_edge: Rgba,
    pub frame: Rgba,
    pub focus: Rgba,
    pub accent: Rgba,
    pub checker_light: Rgba,
    pub checker_dark: Rgba,
    pub accent_soft: Rgba,
    pub row_height: f32,
    pub toggle_width: f32,
    pub icon: f32,
    pub glyph: f32,
    pub pad: f32,
    pub gap: f32,
    pub trailing_pad: f32,
    pub min_label: f32,
    pub thumb: f32,
    pub tight: f32,
    pub label_line: f32,
    pub detail_line: f32,
    pub body_size: f32,
    pub caption_size: f32,
    pub control: f32,
    pub radius_control: f32,
    pub thin: f32,
    pub medium: f32,
    pub thick: f32,
    pub focus_width: f32,
    pub icon_stroke: f32,
    pub checker: f32,
    pub footer_pad: f32,
    pub bar: f32,
    pub bar_min_thumb: f32,
}

impl Look {
    pub fn of(t: &Tokens) -> Look {
        let c = &t.color;
        let control = t.size.control.md;
        let thumb = t.size.thumb.md;
        let thumb_pad = t.space.gap.stack;
        let pad_regular = t.space.pad.regular;
        Look {
            panel: c.surface.panel,
            background: c.surface.background,
            sunken: c.surface.sunken,
            text: c.text.normal,
            text_muted: c.text.muted,
            text_faint: c.text.faint,
            text_on_selected: c.text.on_selected,
            hover: c.state.hover,
            pressed: c.state.pressed,
            selected: c.state.selected,
            divider: c.line.divider,
            border: c.line.border,
            control_edge: c.line.control_edge,
            frame: c.line.frame,
            focus: c.line.focus,
            accent: c.accent.solid,
            checker_light: c.overlay.checker_light,
            checker_dark: c.overlay.checker_dark,
            accent_soft: c.accent.soft,
            row_height: (thumb + thumb_pad).max(control + pad_regular),
            toggle_width: control + t.space.gap.inline,
            icon: t.size.icon.md,
            glyph: t.size.icon.glyph_md,
            pad: t.space.pad.control,
            gap: t.space.gap.inline,
            trailing_pad: pad_regular,
            min_label: 3.0 * control,
            thumb,
            tight: t.space.gap.tight,
            label_line: t.typography.body.line_height,
            detail_line: t.typography.caption.line_height,
            body_size: t.typography.body.size,
            caption_size: t.typography.caption.size,
            control,
            radius_control: t.shape.radius.control,
            thin: t.stroke.thin,
            medium: t.stroke.medium,
            thick: t.stroke.thick,
            focus_width: t.stroke.focus,
            icon_stroke: t.stroke.icon,
            checker: t.size.checker,
            footer_pad: t.space.pad.control,
            bar: t.size.handle.sm,
            bar_min_thumb: t.size.handle.md * 2.0,
        }
    }

    /// The x where the name band starts: after the eye and lock cells.
    pub fn band_x(&self) -> f32 {
        2.0 * self.toggle_width
    }

    /// The thumbnail slot's x, row-relative.
    pub fn thumb_x(&self) -> f32 {
        self.band_x() + self.pad
    }

    /// The label's start, row-relative (look sheet 3.5).
    pub fn label_x(&self) -> f32 {
        self.thumb_x() + self.thumb + self.gap
    }

    pub fn footer_height(&self) -> f32 {
        self.control + 2.0 * self.footer_pad
    }

    /// Paints the divider along `edge` of `rect` on the physical pixels just inside it, as craft's
    /// `Rule::edge`: `thin` rounded to whole pixels (at least one), its centre line snapped so both
    /// sides fall on pixel edges, drawn as a line segment. In f32, as craft computes it.
    pub fn edge_rule(&self, cx: &mut PaintCx<'_>, rect: Rect, edge: Edge) {
        let ppp = cx.scale();
        let physical = (self.thin as f32 * ppp).round().max(1.0);
        let half = physical / ppp.max(f32::EPSILON) / 2.0;
        let snap = |v: f32| {
            let px = v * ppp;
            let snapped = if physical % 2.0 == 1.0 { px.floor() + 0.5 } else { px.round() };
            snapped / ppp
        };
        let (l, t, r, b) = (rect.min.x as f32, rect.min.y as f32, rect.max.x as f32, rect.max.y as f32);
        let [x0, y0, x1, y1] = match edge {
            Edge::Top => [l, snap(t + half), r, snap(t + half)],
            Edge::Bottom => [l, snap(b - half), r, snap(b - half)],
            Edge::Left => [snap(l + half), t, snap(l + half), b],
            Edge::Right => [snap(r - half), t, snap(r - half), b],
        };
        let p = |x: f32, y: f32| Pos2::new(x, y);
        cx.line(p(x0, y0), p(x1, y1), Stroke::new(physical / ppp, self.divider));
    }
}

/// `rect` with every edge on a whole physical pixel (`round_to_pixels`).
pub fn snap(rect: Rect, scale: f32) -> Rect {
    let s = scale;
    let r = |v: f32| (v * s).round() / s;
    crate::geo::rect(r(rect.min.x), r(rect.min.y), r(rect.max.x), r(rect.max.y))
}

/// The square an icon of `side` points is drawn in when centred on `center`: its side in whole
/// pixels, its corner on a whole pixel.
pub fn icon_square(center: Pos2, side: f32, scale: f32) -> (Rect, u32) {
    let s = scale;
    let px = (side * s).round().max(1.0) as u32;
    let side = px as f32 / s;
    let min = center - Vec2::new(side / 2.0, side / 2.0);
    let min = Pos2::new((min.x * s).round() / s, (min.y * s).round() / s);
    (Rect::from_min_size(min, Vec2::new(side, side)), px)
}

/// Thumbnail textures come in sides of whole steps of this many pixels and are drawn scaled into
/// the picture rect, as craft's `ThumbRequest::for_cell` asks for them.
pub const THUMB_SIZE_STEP: u32 = 16;

/// The texture side for a picture `pixels` wide on screen.
pub fn thumb_texture_side(pixels: f32) -> u32 {
    ((pixels / THUMB_SIZE_STEP as f32).ceil().max(1.0) as u32) * THUMB_SIZE_STEP
}
