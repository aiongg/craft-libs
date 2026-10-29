//! The look of a dock area (egui-typed colors, strokes, sizes and font) and [`DockSpec`], what
//! [`super::DockArea`] draws from.

use std::borrow::Cow;

use egui::{Color32, FontId, Stroke, Ui, Visuals};

use super::{DockConfig, DockStyle};

/// Default horizontal room around a tab's label.
pub(super) const TAB_PADDING: f32 = 24.0;

/// Colors, strokes, sizes and font of a [`super::DockArea`]'s chrome. Behaviour and labels are
/// in [`DockConfig`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DockLook {
    pub tab_height: f32,
    /// Space between the groups of a split.
    pub gap: f32,
    pub background: Color32,
    /// Tab strip, column bar and floating title bar.
    pub tab_strip: Color32,
    pub tab_active: Color32,
    pub tab_hover: Color32,
    /// Fill of the current icon cell in an iconic column.
    pub icon_selected: Color32,
    pub text: Color32,
    pub inactive_text: Color32,
    pub border: Stroke,
    /// A resize handle under the pointer or being dragged.
    pub resize: Color32,
    /// Drop previews, insertion marker and drop outlines.
    pub drop: Color32,
    /// Keyboard focus strokes.
    pub focus: Color32,
    pub font: FontId,
    /// Horizontal room around a tab's label, both when measuring its natural width and when
    /// cutting it.
    pub tab_padding: f32,
    /// Painted under every floating window, so it stays opaque when the panel colors are not.
    pub floating_background: Color32,
    pub floating_stroke: Stroke,
    /// Painted over a native floating window while [`super::DockArea::input_blocked`] is on.
    pub blocked_dim: Color32,
    /// Height of a floating window's title bar.
    pub title_height: f32,
    /// [D] Height of the bar above each dock column. It drags the column and holds its »/« button.
    pub column_bar_height: f32,
    /// [D] Side of a panel icon in an iconic column; each icon row is 10 pt taller.
    pub icon_size: f32,
    /// [D] Width of an iconic column without a saved width, and the narrowest it gets.
    pub iconic_width: f32,
}

impl DockLook {
    /// The look of the `ui`'s theme.
    pub fn of(ui: &Ui) -> Self {
        Self::from_visuals(ui.visuals())
    }

    /// Colors from egui `visuals`, sizes at their defaults.
    pub fn from_visuals(visuals: &Visuals) -> Self {
        Self {
            tab_height: 26.0,
            gap: 4.0,
            background: visuals.panel_fill,
            tab_strip: visuals.extreme_bg_color,
            tab_active: visuals.widgets.active.bg_fill,
            tab_hover: visuals.widgets.active.bg_fill,
            icon_selected: visuals.widgets.active.bg_fill,
            text: visuals.text_color(),
            inactive_text: visuals.weak_text_color(),
            border: visuals.widgets.noninteractive.bg_stroke,
            resize: visuals.selection.stroke.color,
            drop: visuals.selection.stroke.color,
            focus: visuals.selection.stroke.color,
            font: FontId::proportional(12.0),
            tab_padding: TAB_PADDING,
            floating_background: visuals.window_fill,
            floating_stroke: visuals.window_stroke,
            blocked_dim: visuals.panel_fill.gamma_multiply(0.6),
            title_height: super::chrome::TITLE_HEIGHT,
            column_bar_height: 14.0,
            icon_size: 20.0,
            iconic_width: 36.0,
        }
    }

    /// Sizes finite and within 0..=4096, and the tab height and font size positive.
    pub(super) fn valid(&self) -> bool {
        [
            self.tab_height,
            self.gap,
            self.border.width,
            self.font.size,
            self.tab_padding,
            self.floating_stroke.width,
            self.title_height,
            self.column_bar_height,
            self.icon_size,
            self.iconic_width,
        ]
        .iter()
        .all(|v| v.is_finite() && (0.0..=4096.0).contains(v))
            && self.tab_height > 0.0
            && self.font.size > 0.0
    }
}

/// What a [`super::DockArea`] draws from: its behaviour and labels, and its look. Implemented by
/// [`DockConfig`] (the look of the `ui`'s theme), [`DockWith`] (a config and a look the
/// application built) and the flat [`DockStyle`].
pub trait DockSpec {
    /// The config and the look for this frame.
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>);
}

impl<T: DockSpec + ?Sized> DockSpec for &T {
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (**self).parts(ui)
    }
}

impl DockSpec for DockConfig {
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (Cow::Borrowed(self), Cow::Owned(DockLook::of(ui)))
    }
}

impl DockSpec for DockStyle {
    fn parts(&self, _ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        let (config, look) = self.split();
        (Cow::Owned(config), Cow::Owned(look))
    }
}

/// A config and a look the application built.
#[derive(Clone, Debug)]
pub struct DockWith {
    pub config: DockConfig,
    pub look: DockLook,
}

impl DockSpec for DockWith {
    fn parts(&self, _ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (Cow::Borrowed(&self.config), Cow::Borrowed(&self.look))
    }
}

/// Whether `config` and `look` together describe a drawable dock area.
pub(super) fn valid(config: &DockConfig, look: &DockLook) -> bool {
    config.valid(look) && look.valid() && look.iconic_width <= config.iconic_max_width
}
