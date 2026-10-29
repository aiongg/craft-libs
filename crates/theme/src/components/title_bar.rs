//! Title bars: the shell's caption row, with the window's caption buttons at its ends and the
//! document title between the app's menus and controls. Sizes are the Size preset's bar height and
//! panel padding; the colours are the surface, text, state and line tokens.
//!
//! The title is text on the bar (4.5:1, 7:1 with high contrast). The caption glyphs are the only
//! mark of their buttons, so each is measured as a non-text mark (3:1, 4.5:1) on the bar, the
//! hovered and the pressed fill, and an inactive window's glyph on the bar. The Close buttons'
//! reds are the desktops' own colours: fixed fields that no theme overrides and no pair can name,
//! checked by their own tests (glyph on red, 4.5:1).

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::derive::over;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// Title bar tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TitleBarTokens {
    /// Height of the bar: `size.bar.title`.
    pub height: f32,
    /// Least height of the bar with Adwaita caption buttons.
    pub height_adwaita: f32,
    /// End margin of a side without caption buttons: `space.pad.panel`.
    pub pad: f32,
    /// Least room between the title and what is beside it.
    pub title_gap: f32,
    /// Least gap that still shows an elided title.
    pub title_min_gap: f32,
    /// Type role of the title.
    pub title_role: TypeRole,
    /// Width of the resize zones along the window edges.
    pub edge: f32,
    /// Side of the resize zones at the window corners.
    pub corner: f32,
    /// The bar's background: `surface.panel`.
    pub bar: Rgba,
    /// The title: `text.muted`.
    pub title: Rgba,
    /// The caption glyphs in the active window: `text.normal`.
    pub glyph: Rgba,
    /// The caption glyphs in an inactive window: `text.muted`.
    pub glyph_inactive: Rgba,
    /// The fill under the pointer: `state.hover` over the bar.
    pub hover: Rgba,
    /// The fill while pressed: `state.pressed` over the bar.
    pub pressed: Rgba,
    /// The hairline along the bar's bottom edge: `line.divider`.
    pub separator: Rgba,
    /// Windows 11 Close button under the pointer; pressed is 90 % of it. Fixed.
    pub windows_close: Rgba,
    /// The glyph on Windows' Close button. Fixed.
    pub windows_close_glyph: Rgba,
    /// KDE Breeze Close button under the pointer. Fixed.
    pub breeze_close_hover: Rgba,
    /// The glyph on Breeze's hovered Close button. Fixed.
    pub breeze_close_hover_glyph: Rgba,
    /// KDE Breeze Close button held down. Fixed.
    pub breeze_close_pressed: Rgba,
    /// The glyph on Breeze's pressed Close button. Fixed.
    pub breeze_close_pressed_glyph: Rgba,
}

impl TitleBarTokens {
    fn color_mut(&mut self, f: TitleBarField) -> &mut Rgba {
        match f {
            TitleBarField::Bar => &mut self.bar,
            TitleBarField::Title => &mut self.title,
            TitleBarField::Glyph => &mut self.glyph,
            TitleBarField::GlyphInactive => &mut self.glyph_inactive,
            TitleBarField::Hover => &mut self.hover,
            TitleBarField::Pressed => &mut self.pressed,
            TitleBarField::Separator => &mut self.separator,
        }
    }
}

/// An overridable title bar token. Paths: `bar`, `title`, `glyph`, `glyph.inactive`, `hover`,
/// `pressed`, `separator`, e.g. `title_bar.glyph.inactive`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TitleBarField {
    /// `title_bar.bar`
    Bar,
    /// `title_bar.title`
    Title,
    /// `title_bar.glyph`
    Glyph,
    /// `title_bar.glyph.inactive`
    GlyphInactive,
    /// `title_bar.hover`
    Hover,
    /// `title_bar.pressed`
    Pressed,
    /// `title_bar.separator`
    Separator,
}

impl TitleBarField {
    const fn path(self) -> &'static str {
        match self {
            TitleBarField::Bar => "bar",
            TitleBarField::Title => "title",
            TitleBarField::Glyph => "glyph",
            TitleBarField::GlyphInactive => "glyph.inactive",
            TitleBarField::Hover => "hover",
            TitleBarField::Pressed => "pressed",
            TitleBarField::Separator => "separator",
        }
    }
}

impl Family for TitleBarTokens {
    type Field = TitleBarField;
    const FIELDS: &'static [TitleBarField] = &[
        TitleBarField::Bar,
        TitleBarField::Title,
        TitleBarField::Glyph,
        TitleBarField::GlyphInactive,
        TitleBarField::Hover,
        TitleBarField::Pressed,
        TitleBarField::Separator,
    ];

    fn defaults(s: &Semantic<'_>) -> TitleBarTokens {
        let c = s.color;
        let bar = c.surface.panel;
        TitleBarTokens {
            height: s.size.bar.title,
            height_adwaita: values::TITLE_BAR_HEIGHT_ADWAITA,
            pad: s.space.pad.panel,
            title_gap: values::TITLE_BAR_TITLE_GAP,
            title_min_gap: values::TITLE_BAR_TITLE_MIN_GAP,
            title_role: TypeRole::Title,
            edge: values::TITLE_BAR_EDGE,
            corner: values::TITLE_BAR_CORNER,
            bar,
            title: c.text.muted,
            glyph: c.text.normal,
            glyph_inactive: c.text.muted,
            hover: over(c.state.hover, bar),
            pressed: over(c.state.pressed, bar),
            separator: c.line.divider,
            windows_close: values::TITLE_BAR_WINDOWS_CLOSE,
            windows_close_glyph: values::TITLE_BAR_WINDOWS_CLOSE_GLYPH,
            breeze_close_hover: values::TITLE_BAR_BREEZE_CLOSE_HOVER,
            breeze_close_hover_glyph: values::TITLE_BAR_BREEZE_CLOSE_HOVER_GLYPH,
            breeze_close_pressed: values::TITLE_BAR_BREEZE_CLOSE_PRESSED,
            breeze_close_pressed_glyph: values::TITLE_BAR_BREEZE_CLOSE_PRESSED_GLYPH,
        }
    }

    fn path(f: TitleBarField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<TitleBarField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(_: TitleBarField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: TitleBarField) -> OverrideValue {
        let mut copy = *self;
        OverrideValue::Color(*copy.color_mut(f))
    }

    fn set(&mut self, f: TitleBarField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let component = |f| TokenRef::Component(ComponentField::TitleBar(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: component(fg), bg: component(bg), minimum, alternate: None };
        let (text, mark) = (values::TEXT_MIN, values::NON_TEXT_MIN);
        vec![
            pair(TitleBarField::Title, TitleBarField::Bar, text),
            pair(TitleBarField::Glyph, TitleBarField::Bar, mark),
            pair(TitleBarField::Glyph, TitleBarField::Hover, mark),
            pair(TitleBarField::Glyph, TitleBarField::Pressed, mark),
            pair(TitleBarField::GlyphInactive, TitleBarField::Bar, mark),
        ]
    }
}
