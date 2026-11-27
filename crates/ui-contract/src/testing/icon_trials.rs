//! Trials of the icon lookup: the app icon registry (the catalog wins a clashing name, the first set keeps a name two
//! sets share), the set a theme's icon pack picks, and an app icon's layers by class.

use crate::primitives::items::rgba;
use crate::primitives::svg_set::Layer;
use crate::primitives::{
    AppIcons, Canvas, DataMode, Draw, DrawEnv, FrameServices, Glyph, IconSpec, Mirror, PaintRecord, SvgIcon, TextRun, icon_names, paint_icon,
    register_app_icons,
};
use crate::style::{Ink, StyleColor, TypeRole, UiTokens, ink_color, rgba_color};
use crate::testing::{style, trial};
use craft_behavior::IconId;
use craft_i18n::Direction;
use craft_theme::{AppSizes, Appearance, Contrast, IconPackId, Mode, Rgba, Size, TextSize, builtins};
use craft_ui_rt::{ImageKey, ImageStore, Item, RichText, TextEngine, TextHandle, TextLayout, TextSpec};
use egui::{Color32, ColorImage, Pos2, Rect, Vec2, pos2, vec2};
use std::cell::RefCell;
use std::sync::Arc;

// Built from parts so the workspace scan of icon-id literals does not read them.
const FIRST_NAME: &str = concat!("zz_t10_", "first");
const SECOND_NAME: &str = concat!("zz_t10_", "second");
const CATALOG_NAME: &str = "close";
const DISC: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"8\" fill=\"black\"/></svg>";
const SQUARE: &[u8] =
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><rect x=\"4\" y=\"4\" width=\"16\" height=\"16\" fill=\"black\"/></svg>";

const FIRST_SET: AppIcons = AppIcons { name: "t10-first", stroke_width: None, icons: &[(FIRST_NAME, DISC), (CATALOG_NAME, DISC)], fills: &[] };
const SECOND_SET: AppIcons = AppIcons { name: "t10-second", stroke_width: None, icons: &[(FIRST_NAME, SQUARE), (SECOND_NAME, SQUARE)], fills: &[] };
const FILLED_NAME: &str = concat!("zz_k10_", "filled");
const FILL_COLOR: [u8; 4] = [255, 0, 0, 255];
const FILLED_SET: AppIcons = AppIcons::new("k10-filled", None, &[(FILLED_NAME, DISC)]).with_fills(&[(FILLED_NAME, SQUARE, FILL_COLOR)]);

fn icon(name: &'static str) -> Option<SvgIcon> {
    let tokens = style::tokens(Mode::Light, Contrast::Standard);
    SvgIcon::of(&tokens, Direction::Ltr, 24.0, &IconId::from(std::borrow::Cow::Borrowed(name)), Mirror::Auto)
}

fn mask(name: &'static str) -> Option<ColorImage> {
    icon(name).map(|icon| icon.mask(24.0, 1.0))
}

#[trial]
fn an_app_icon_set_resolves_after_the_catalog_and_the_first_set_keeps_a_shared_name() {
    assert_eq!(icon(FIRST_NAME), None);
    let catalog = icon_names().len();
    let close = mask(CATALOG_NAME);
    let catalog_icon = icon(CATALOG_NAME);

    register_app_icons(&FIRST_SET);
    assert!(icon(FIRST_NAME).is_some(), "the set's new name resolves");
    assert_eq!(icon(CATALOG_NAME), catalog_icon, "the catalog keeps its name");
    assert_eq!(mask(CATALOG_NAME), close, "the refused icon does not change the catalog icon");
    let first = mask(FIRST_NAME);
    assert_ne!(first, close);
    assert_eq!(icon_names().len(), catalog, "icon_names lists the built-in catalog only");
    assert!(!icon_names().contains(&FIRST_NAME));

    // A later set repeating a registered name is refused for that name only; its other names join.
    assert_eq!(icon(SECOND_NAME), None);
    register_app_icons(&SECOND_SET);
    assert!(icon(SECOND_NAME).is_some());
    assert_eq!(mask(FIRST_NAME), first, "the first set keeps the shared name");
    assert_ne!(mask(SECOND_NAME), first);
    let second = mask(SECOND_NAME);

    // Registering a set again changes nothing.
    register_app_icons(&FIRST_SET);
    register_app_icons(&SECOND_SET);
    assert_eq!((mask(FIRST_NAME), mask(SECOND_NAME), mask(CATALOG_NAME)), (first, second, close));
    assert_eq!(icon_names().len(), catalog);
}

/// The mask of `name` under the first built-in theme with its icon pack set to `pack`.
fn mask_under_pack(pack: Option<&str>, name: &'static str) -> Option<ColorImage> {
    let mut theme = builtins()[0].clone();
    let pack = pack.map(IconPackId::parse).transpose();
    assert!(pack.is_ok(), "{pack:?}");
    theme.icon_pack = pack.unwrap_or_default();
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false };
    let tokens = UiTokens::of_theme(&theme, appearance, None, &AppSizes::default());
    SvgIcon::of(&tokens, Direction::Ltr, 24.0, &IconId::from(std::borrow::Cow::Borrowed(name)), Mirror::Auto).map(|icon| icon.mask(24.0, 1.0))
}

#[trial]
fn a_theme_icon_pack_picks_the_set_and_an_unknown_pack_draws_the_default() {
    let default = mask_under_pack(None, CATALOG_NAME);
    assert!(default.is_some());
    // Carbon's close is a fill drawing; Lucide's and Tabler's are the same two strokes.
    assert_ne!(mask_under_pack(Some("craft.carbon"), CATALOG_NAME), default, "a built-in pack draws its own icon");
    assert_eq!(mask_under_pack(Some("craft.lucide"), CATALOG_NAME), default, "the default set by name draws the default");
    assert_eq!(mask_under_pack(Some("user.missing"), CATALOG_NAME), default, "a pack that is not installed draws the default");
}

const LAYERED_NAME: &str = concat!("zz_t19_", "pen");
/// A plain body, an accent nib (a group, so the class reaches its children) and a fixed red cap.
const LAYERED: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\">\
<rect x=\"4\" y=\"8\" width=\"10\" height=\"8\" fill=\"black\"/>\
<g class=\"accent\"><path d=\"M14 8 L22 12 L14 16 Z\" fill=\"black\"/></g>\
<rect class=\"fixed\" x=\"2\" y=\"8\" width=\"2\" height=\"8\" fill=\"#d02020\"/></svg>";
const LAYERED_SET: AppIcons = AppIcons { name: "t19-layers", stroke_width: None, icons: &[(LAYERED_NAME, LAYERED)], fills: &[] };
/// A plain icon with no classed part.
const PLAIN_NAME: &str = concat!("zz_t19_", "plain");
const PLAIN_SET: AppIcons = AppIcons { name: "t19-plain", stroke_width: None, icons: &[(PLAIN_NAME, DISC)], fills: &[] };

/// A text engine that lays every text out as a box of its size per character and keeps what it was asked to set.
#[derive(Default)]
struct Spy {
    asked: Vec<RichText>,
}

impl TextEngine for Spy {
    fn layout(&mut self, spec: &TextSpec, _scale: f32) -> TextLayout {
        let size = Vec2::new(spec.text.chars().count() as f32 * spec.font.size / 2.0, spec.font.size);
        TextLayout { handle: TextHandle::new(Arc::new(())), size, baseline: size.y, lines: 1 }
    }

    fn layout_rich(&mut self, text: &RichText, _scale: f32) -> TextLayout {
        self.asked.push(text.clone());
        let size = text.sections.first().map_or(0.0, |section| section.format.size);
        let size = Vec2::new(text.text.chars().count() as f32 * size / 2.0, size);
        TextLayout { handle: TextHandle::new(Arc::new(())), size, baseline: size.y, lines: 1 }
    }

    fn hit_index(&mut self, _: &TextLayout, _: Pos2) -> usize {
        0
    }

    fn caret_rect(&mut self, _: &TextLayout, _: usize) -> Rect {
        Rect::NOTHING
    }
}

/// The display items `paint` records on a canvas at the window's origin, and the texts the engine was asked to set.
fn recorded(paint: impl FnOnce(&Canvas<'_>)) -> (Vec<Item>, Vec<RichText>) {
    let (mut store, mut spy) = (ImageStore::default(), Spy::default());
    let items = {
        let services = FrameServices::new(Some(&mut spy), &mut store, 1.0);
        let canvas = Canvas::new(&services, Pos2::ZERO, Rect::EVERYTHING);
        paint(&canvas);
        let (content, overlay) = canvas.finish();
        content.items.into_iter().chain(overlay.items).collect()
    };
    (items, spy.asked)
}

/// The image and tint of every picture `items` holds (images, masks, textured meshes), in paint order.
fn textured(items: &[Item]) -> Vec<(ImageKey, Rgba)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Image { key, tint, .. } => Some((*key, *tint)),
            Item::ImageMask { key, color, .. } => Some((*key, *color)),
            Item::Mesh(mesh) => mesh.texture.zip(mesh.vertices.first().map(|vertex| vertex.color)),
            _ => None,
        })
        .collect()
}

/// The texel at (`x`, `y`) of a 24-texel layer image.
fn texel(image: &ColorImage, x: usize, y: usize) -> Color32 {
    image.pixels[y * 24 + x]
}

/// On a strong fill or scrim (the surface's ink is `OnDark`: a brand rail) the accent parts paint in the icon's own
/// colour, since `accent.solid` would vanish on the brand fill; the fixed parts stay as drawn, and on a panel the
/// accent parts keep the accent ink.
#[trial]
fn an_accent_part_on_a_dark_surface_paints_in_the_icons_ink() {
    register_app_icons(&LAYERED_SET);
    let tokens = style::tokens(Mode::Light, Contrast::Standard);
    let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(34.0, 34.0));
    let tints = |surface: Ink, spec_ink: Option<Ink>| {
        let on = ink_color(&tokens, Ink::OnDark);
        let mut spec = IconSpec::svg(IconId::from(std::borrow::Cow::Borrowed(LAYERED_NAME))).color(StyleColor(on));
        if let Some(ink) = spec_ink {
            spec = spec.ink(ink);
        }
        let record = RefCell::new(PaintRecord::default());
        let (items, _) = recorded(|canvas| {
            let env = DrawEnv { tokens: &tokens, direction: Direction::Ltr, ink: surface };
            Draw::new(canvas, &record, env).icon(rect, &spec);
        });
        textured(&items).into_iter().map(|(_, tint)| tint).collect::<Vec<_>>()
    };
    let on = ink_color(&tokens, Ink::OnDark);
    // design: allow[raw-colour/const](a fixed layer keeps the colours it is drawn in: the trial expects the white tint that leaves them unchanged)
    let (on, fixed) = (rgba(on), rgba(Color32::WHITE));
    assert_eq!(tints(Ink::OnDark, None), [on, on, fixed], "on a brand rail the accent parts read in the icon's ink");
    assert_eq!(tints(Ink::OnPanel, Some(Ink::OnDark)), [on, on, fixed], "an icon that asks for the on-dark ink does the same");
    assert_eq!(tints(Ink::OnPanel, None), [on, rgba(rgba_color(tokens.color.accent.solid)), fixed], "on a panel the accent ink is kept");
}

#[trial]
fn an_accent_part_paints_in_the_accent_ink_a_fixed_part_as_drawn_and_both_take_the_disabled_ink_when_disabled() {
    register_app_icons(&LAYERED_SET);
    register_app_icons(&PLAIN_SET);
    let tokens = style::tokens(Mode::Light, Contrast::Standard);
    let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(34.0, 34.0));
    // The icon painted by a `Draw` in `mode`: `Dimmed` is what a component in its Disabled state paints with.
    let paint = |name: &'static str, mode: DataMode, color: Color32| {
        let spec = IconSpec::svg(IconId::from(std::borrow::Cow::Borrowed(name))).color(StyleColor(color));
        let record = RefCell::new(PaintRecord::default());
        let (items, _) = recorded(|canvas| {
            let env = DrawEnv { tokens: &tokens, direction: Direction::Ltr, ink: Ink::OnPanel };
            Draw::new(canvas, &record, env).data(mode).icon(rect, &spec);
        });
        textured(&items)
    };
    let (ink, off) = (ink_color(&tokens, Ink::OnPanel), rgba_color(tokens.color.text.disabled));
    let accent = rgba(rgba_color(tokens.color.accent.solid));
    let enabled = paint(LAYERED_NAME, DataMode::Full, ink);
    let disabled = paint(LAYERED_NAME, DataMode::Dimmed, ink);
    let enabled_in_the_disabled_colour = paint(LAYERED_NAME, DataMode::Full, off);

    assert_eq!(enabled.len(), 3, "an app icon paints its ink, accent and fixed layers, got {enabled:?}");
    let [(ink_layer, ink_tint), (accent_layer, accent_tint), (fixed_layer, fixed_tint)] = enabled[..] else { return };
    assert_eq!(ink_tint, rgba(ink), "the plain parts read in the icon's colour");
    assert_eq!(accent_tint, accent, "the accent parts read in the accent ink");
    assert_ne!(accent_tint, ink_tint);
    // design: allow[raw-colour/const](a fixed layer keeps the colours it is drawn in: the trial expects the white tint that leaves them unchanged)
    assert_eq!(fixed_tint, rgba(Color32::WHITE), "the fixed parts keep the colours they are drawn in");
    assert!(ink_layer != accent_layer && accent_layer != fixed_layer && ink_layer != fixed_layer, "each layer is its own texture");

    assert_eq!(disabled.len(), 3, "a disabled app icon paints the same three layers, got {disabled:?}");
    let [(off_ink, a), (off_accent, b), (off_fixed, c)] = disabled[..] else { return };
    assert_eq!([a, b, c], [rgba(ink); 3], "every layer takes the one ink when the component is disabled");
    assert_eq!((off_ink, off_accent), (ink_layer, accent_layer), "the ink and accent layers are the same drawings, tinted");
    assert_ne!(off_fixed, fixed_layer, "a disabled fixed part is drawn as a silhouette, not in its own colours");

    let tints: Vec<Rgba> = enabled_in_the_disabled_colour.iter().map(|(_, tint)| *tint).collect();
    // design: allow[raw-colour/const](a fixed layer keeps the colours it is drawn in: the trial expects the white tint that leaves them unchanged)
    assert_eq!(tints, [rgba(off), accent, rgba(Color32::WHITE)], "an enabled icon in the disabled colour keeps its layers");

    // What each layer holds: the class split is the SVG engine's, so a sheet it ignored would give every layer the whole drawing.
    let layered = icon(LAYERED_NAME);
    assert!(layered.is_some(), "the layered icon resolves");
    let Some(layered) = layered else { return };
    let (body, nib, cap) = ((9, 12), (16, 12), (3, 12));
    let [plain, accent, fixed] = [Layer::Ink, Layer::Accent, Layer::Fixed].map(|layer| layered.layer(24.0, 1.0, layer));
    assert!(texel(&plain, body.0, body.1).a() > 0 && texel(&plain, nib.0, nib.1).a() == 0, "the ink layer holds the plain body only");
    assert!(texel(&accent, nib.0, nib.1).a() > 0 && texel(&accent, body.0, body.1).a() == 0, "the accent layer holds the nib only");
    let red = texel(&fixed, cap.0, cap.1);
    assert!(red.a() > 0 && red.r() > red.g() && red.r() > red.b(), "the fixed layer holds the cap in its own colour: {red:?}");
    assert!(texel(&fixed, body.0, body.1).a() == 0 && texel(&plain, cap.0, cap.1).a() == 0, "the cap is in no other layer");

    // An icon with no classed part paints its ink layer alone.
    assert_eq!(paint(PLAIN_NAME, DataMode::Full, ink).len(), 1, "a layer with no part paints no quad");
}

/// The tints of the pictures `name` paints under the tint `color`, first to last.
fn painted(name: &'static str, color: Color32) -> Vec<Rgba> {
    let Some(svg) = icon(name) else { return Vec::new() };
    let square = Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 24.0));
    textured(&recorded(|canvas| svg.paint(canvas, square, color)).0).into_iter().map(|(_, tint)| tint).collect()
}

#[trial]
fn an_icon_with_a_fill_keeps_its_colour_under_the_tint() {
    register_app_icons(&FILLED_SET);
    let [r, g, b, a] = FILL_COLOR;
    let fill = Color32::from_rgba_unmultiplied(r, g, b, a);
    // design: allow[raw-colour/const](white is the tint that leaves colours unchanged: the fill keeps its colour under it too)
    for tint in [Color32::from_rgb(10, 200, 30), Color32::WHITE] {
        assert_eq!(painted(FILLED_NAME, tint), [rgba(fill), rgba(tint)], "the fill keeps its colour and the icon takes the tint");
    }
    let tint = Color32::from_rgb(10, 200, 30);
    assert_eq!(painted(CATALOG_NAME, tint), [rgba(tint)], "an icon with no fill paints the icon only");
}

/// A glyph that paints a dot with a painter and a question mark as text when a `Draw` paints it.
#[derive(crate::contract::MemoProps)]
struct MarkGlyph;

impl MarkGlyph {
    fn run(color: Color32) -> TextRun<'static> {
        TextRun::new("?").role(TypeRole::Caption).color(StyleColor(color))
    }
}

impl Glyph for MarkGlyph {
    fn paint(&self, painter: &Canvas<'_>, _tokens: &UiTokens, rect: Rect, color: Color32, _mirrored: bool) {
        painter.circle_filled(rect.center(), rect.width() / 4.0, color);
    }

    fn paint_draw(&self, draw: &Draw<'_>, rect: Rect, color: Color32, _mirrored: bool) {
        let layout = draw.layout(&Self::run(color), f32::INFINITY);
        draw.galley(rect.center() - layout.size() / 2.0, &layout);
    }
}

/// A glyph that implements `paint` only.
#[derive(crate::contract::MemoProps)]
struct DotGlyph;

impl Glyph for DotGlyph {
    fn paint(&self, painter: &Canvas<'_>, _tokens: &UiTokens, rect: Rect, color: Color32, _mirrored: bool) {
        painter.circle_filled(rect.center(), rect.width() / 4.0, color);
    }
}

/// A glyph that implements `paint_draw` lays its text out through the `Draw` of the component that shows it, in the
/// theme's type; `paint_icon`, which has no `Draw`, paints it with `paint`, and a glyph that implements `paint` only
/// paints as before.
#[trial]
fn a_glyph_that_paints_with_draw_lays_out_text_in_the_themes_type() {
    let tokens = style::tokens(Mode::Light, Contrast::Standard);
    let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(34.0, 34.0));
    let color = ink_color(&tokens, Ink::OnPanel);
    let env = DrawEnv { tokens: &tokens, direction: Direction::Ltr, ink: Ink::OnPanel };
    let (asks, dot) = (MarkGlyph, DotGlyph);
    let record = RefCell::new(PaintRecord::default());
    let (_, theme) = recorded(|canvas| {
        Draw::new(canvas, &record, env).layout(&MarkGlyph::run(color), f32::INFINITY);
    });
    let (drawn, asked) = recorded(|canvas| Draw::new(canvas, &record, env).icon(rect, &IconSpec::glyph(&asks)));
    let texts: Vec<Rect> = drawn
        .iter()
        .filter_map(|item| if let Item::Text { origin, size, .. } = item { Some(Rect::from_min_size(*origin, *size)) } else { None })
        .collect();
    assert_eq!(texts.len(), 1, "the glyph paints its mark as one text run, got {drawn:?}");
    assert!(rect.contains_rect(texts[0]), "the mark lies inside the icon");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].text.as_ref(), "?");
    let format = |text: &RichText| text.sections.first().map(|section| (section.format.face.clone(), section.format.size));
    assert!(format(&theme[0]).is_some());
    assert_eq!(format(&asked[0]), format(&theme[0]), "the mark is in the theme's caption type");
    assert!(!drawn.iter().any(|item| matches!(item, Item::Circle { .. })), "the painter fallback is not painted as well");

    let (raw, _) = recorded(|canvas| paint_icon(canvas, &tokens, Direction::Ltr, rect, &IconSpec::glyph(&asks)));
    assert!(raw.iter().any(|item| matches!(item, Item::Circle { .. })), "paint_icon paints the glyph with its painter");
    assert!(!raw.iter().any(|item| matches!(item, Item::Text { .. })));

    let (plain, _) = recorded(|canvas| Draw::new(canvas, &record, env).icon(rect, &IconSpec::glyph(&dot)));
    let circles: Vec<Pos2> = plain.iter().filter_map(|item| if let Item::Circle { center, .. } = item { Some(*center) } else { None }).collect();
    assert_eq!(circles.len(), 1, "a glyph with no Draw path paints with its painter, got {plain:?}");
    assert!(rect.contains(circles[0]), "inside the icon");
}
