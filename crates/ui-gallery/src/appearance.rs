//! The appearance bar's settings and what they resolve to.

use craft_theme::{
    AppDefaults, Contrast, ContrastChoice, Mode, ModeChoice, MotionChoice, Selection, Size, SystemAppearance, TextSize, ThemeDef, ThemeId, ThemePair,
    builtins,
};
use craft_ui::direction::Direction;
use craft_ui::icons::IconSet;
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
    pub mode: ModeChoice,
    pub contrast: ContrastChoice,
    pub comfortable: bool,
    /// Index into [`TextSize::ALL`].
    pub text_size: usize,
    pub motion: MotionChoice,
    pub rtl: bool,
    /// Interface scale in percent.
    pub scale: u32,
    /// The interface face in the UI slots: a candidate or [`crate::faces::SYSTEM`] (see
    /// [`crate::faces`]).
    pub ui_face: String,
    /// The candidate monospace face in the mono slot.
    pub mono_face: String,
    /// The candidate icon set every story draws from: an [`IconSet::id`].
    pub icon_set: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeId::suite_default().as_str().to_owned(),
            mode: ModeChoice::Light,
            contrast: ContrastChoice::Standard,
            comfortable: false,
            text_size: 1,
            motion: MotionChoice::Full,
            rtl: false,
            scale: 100,
            ui_face: crate::faces::UI_FACES[0].to_owned(),
            mono_face: crate::faces::MONO_FACES[0].to_owned(),
            icon_set: IconSet::Lucide.id().to_owned(),
        }
    }
}

/// The built-in theme named `id`.
pub fn builtin(id: &str) -> Option<&'static ThemeDef> {
    builtins().iter().find(|t| t.id.as_str() == id)
}

impl Appearance {
    /// The mode shown: the chosen one, or `shown` (the mode the context displays) when the
    /// choice is System.
    pub fn mode(&self, shown: Mode) -> Mode {
        match self.mode {
            ModeChoice::Light => Mode::Light,
            ModeChoice::Dark => Mode::Dark,
            ModeChoice::System => shown,
        }
    }

    /// The contrast shown: the chosen one, or what the OS reports (standard when it doesn't)
    /// when the choice is System.
    pub fn contrast(&self, system: SystemAppearance) -> Contrast {
        match self.contrast {
            ContrastChoice::Standard => Contrast::Standard,
            ContrastChoice::High => Contrast::High,
            ContrastChoice::System => system.contrast.unwrap_or(Contrast::Standard),
        }
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

    /// The icon set, Lucide when the saved id is unknown.
    pub fn icon_set(&self) -> IconSet {
        IconSet::ALL.into_iter().find(|s| s.id() == self.icon_set).unwrap_or(IconSet::Lucide)
    }

    pub fn zoom(&self) -> f32 {
        self.scale.clamp(*SCALE.start(), *SCALE.end()) as f32 / 100.0
    }

    /// The theme and settings resolved for the gallery's own chrome, with `system` as the OS's
    /// answer for the System choices.
    pub fn base(&self, system: SystemAppearance) -> Base {
        let id = builtin(&self.theme).map_or_else(ThemeId::suite_default, |t| t.id.clone());
        let selection = Selection {
            theme: ThemePair::same(id),
            mode: self.mode,
            contrast: self.contrast,
            motion: self.motion,
            size: self.size(),
            text_size: self.text_size(),
        };
        let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: self.mode, size: self.size() };
        Base::resolve(&selection, system, &defaults, None, &mut |id| builtins().iter().find(|t| t.id == *id).cloned())
    }

    /// Sets `ctx` up for these settings, given the ones it was last set up for (`None` the first
    /// time): the interface direction and the font stack, each only when it changed.
    /// `system_face_changed` is true when the OS's interface face resolved since; `system` is
    /// what the OS reports for the System mode and contrast. Returns the base for
    /// [`install`](craft_ui::theme::install), which the caller runs every frame.
    pub fn configure(&self, ctx: &egui::Context, previous: Option<&Appearance>, system_face_changed: bool, system: SystemAppearance) -> Base {
        if previous.is_none_or(|p| p.rtl != self.rtl) {
            craft_ui::direction::apply(ctx, self.direction());
        }
        let faces_changed = previous.is_none_or(|p| p.ui_face != self.ui_face || p.mono_face != self.mono_face);
        if crate::faces::available() && (faces_changed || system_face_changed) {
            crate::faces::install(ctx, &self.ui_face, &self.mono_face);
        }
        self.base(system)
    }

    /// The overrides that show the bar's theme and size in a scope; the mode and contrast are the
    /// scope's own to set, see [`mode`](Self::mode) and [`contrast`](Self::contrast).
    pub fn overrides(&self, o: &mut Overrides) {
        o.size = Some(self.size());
        o.theme = builtin(&self.theme).cloned();
    }
}
