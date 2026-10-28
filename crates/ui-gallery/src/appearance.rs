//! The appearance bar's settings and what they resolve to.

use craft_theme::{
    AppDefaults, Contrast, ContrastChoice, Mode, ModeChoice, Selection, Size, SystemAppearance, TextSize, ThemeDef, ThemeId, ThemePair, builtins,
};
use craft_ui::direction::Direction;
use craft_ui::theme::{Base, Overrides};
use serde::{Deserialize, Serialize};

/// Interface scale limits, in percent.
pub const SCALE: std::ops::RangeInclusive<u32> = 75..=200;

/// Everything the appearance bar sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// A built-in theme id.
    pub theme: String,
    pub dark: bool,
    pub high_contrast: bool,
    pub comfortable: bool,
    /// Index into [`TextSize::ALL`].
    pub text_size: usize,
    pub reduced_motion: bool,
    pub rtl: bool,
    /// Interface scale in percent.
    pub scale: u32,
    /// The candidate interface face in the UI slots (see [`crate::faces`]).
    pub ui_face: String,
    /// The candidate monospace face in the mono slot.
    pub mono_face: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeId::suite_default().as_str().to_owned(),
            dark: false,
            high_contrast: false,
            comfortable: false,
            text_size: 1,
            reduced_motion: false,
            rtl: false,
            scale: 100,
            ui_face: crate::faces::UI_FACES[0].to_owned(),
            mono_face: crate::faces::MONO_FACES[0].to_owned(),
        }
    }
}

/// The built-in theme named `id`.
pub fn builtin(id: &str) -> Option<&'static ThemeDef> {
    builtins().iter().find(|t| t.id.as_str() == id)
}

impl Appearance {
    pub fn mode(&self) -> Mode {
        if self.dark { Mode::Dark } else { Mode::Light }
    }

    pub fn contrast(&self) -> Contrast {
        if self.high_contrast { Contrast::High } else { Contrast::Standard }
    }

    pub fn size(&self) -> Size {
        if self.comfortable { Size::Comfortable } else { Size::Compact }
    }

    pub fn text_size(&self) -> TextSize {
        TextSize::ALL.get(self.text_size).copied().unwrap_or_default()
    }

    pub fn direction(&self) -> Direction {
        if self.rtl { Direction::Rtl } else { Direction::Ltr }
    }

    pub fn zoom(&self) -> f32 {
        self.scale.clamp(*SCALE.start(), *SCALE.end()) as f32 / 100.0
    }

    /// The theme and settings resolved for the gallery's own chrome.
    pub fn base(&self) -> Base {
        let id = builtin(&self.theme).map_or_else(ThemeId::suite_default, |t| t.id.clone());
        let mode = if self.dark { ModeChoice::Dark } else { ModeChoice::Light };
        let contrast = if self.high_contrast { ContrastChoice::High } else { ContrastChoice::Standard };
        let selection = Selection { theme: ThemePair::same(id), mode, contrast, size: self.size(), text_size: self.text_size() };
        let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode, size: self.size() };
        let system = SystemAppearance { reduced_motion: Some(self.reduced_motion), ..SystemAppearance::default() };
        Base::resolve(&selection, system, &defaults, None, &mut |id| builtins().iter().find(|t| t.id == *id).cloned())
    }

    /// The overrides that show the bar's settings in a scope.
    pub fn overrides(&self, o: &mut Overrides) {
        o.mode = Some(self.mode());
        o.contrast = Some(self.contrast());
        o.size = Some(self.size());
        o.theme = builtin(&self.theme).cloned();
    }
}
