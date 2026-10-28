//! What the user picked (theme per mode, mode, contrast, size, text size), how the suite's
//! choice, an app's own choice and the app's defaults combine, and resolution of the result
//! against what the OS reports.

use super::builtins;
use super::ramp::Mode;
use super::resolve::{Resolved, resolve};
use super::theme::{Appearance, Contrast, Size, TextSize, ThemeDef, Variants};
use crate::{AppId, ThemeId};

/// Light, dark, or whatever the OS reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum ModeChoice {
    Light,
    Dark,
    /// The OS's mode, else the app's fallback mode.
    System,
}

/// Standard, high, or whatever the OS reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum ContrastChoice {
    Standard,
    High,
    /// The OS's contrast, else standard.
    #[default]
    System,
}

/// A theme for each mode. Either slot may hold [`crate::APP_DEFAULT`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThemePair {
    pub light: ThemeId,
    pub dark: ThemeId,
}

impl ThemePair {
    /// `id` in both slots.
    pub fn same(id: ThemeId) -> Self {
        ThemePair { light: id.clone(), dark: id }
    }

    /// The slot for `mode`.
    pub fn slot(&self, mode: Mode) -> &ThemeId {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }

    /// Puts `theme` in both slots if it has both variants, else in its own mode's slot.
    pub fn pick(&mut self, theme: &ThemeDef) {
        match theme.variants {
            Variants::Both => *self = ThemePair::same(theme.id.clone()),
            Variants::LightOnly => self.light = theme.id.clone(),
            Variants::DarkOnly => self.dark = theme.id.clone(),
        }
    }
}

/// What the user picked, every field concrete (after defaults are filled in).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Selection {
    pub theme: ThemePair,
    pub mode: ModeChoice,
    pub contrast: ContrastChoice,
    pub size: Size,
    pub text_size: TextSize,
}

/// The suite's choice as `selection.json` holds it. Each field is set once a user picks it in
/// some app; until then every app shows its own default for that field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct SuiteChoice {
    pub theme_light: Option<ThemeId>,
    pub theme_dark: Option<ThemeId>,
    pub mode: Option<ModeChoice>,
    pub contrast: Option<ContrastChoice>,
    pub size: Option<Size>,
    pub text_size: Option<TextSize>,
}

impl SuiteChoice {
    /// Nothing chosen yet.
    pub fn is_empty(&self) -> bool {
        *self == SuiteChoice::default()
    }

    /// What an app that follows the suite shows: the chosen fields, else the app's defaults,
    /// contrast [`ContrastChoice::System`] and text size [`TextSize::Default`].
    pub fn with_defaults(&self, defaults: &AppDefaults) -> Selection {
        let slot = |chosen: &Option<ThemeId>, default: &ThemeId| chosen.clone().unwrap_or_else(|| default.clone());
        Selection {
            theme: ThemePair { light: slot(&self.theme_light, &defaults.theme.light), dark: slot(&self.theme_dark, &defaults.theme.dark) },
            mode: self.mode.unwrap_or(defaults.mode),
            contrast: self.contrast.unwrap_or_default(),
            size: self.size.unwrap_or(defaults.size),
            text_size: self.text_size.unwrap_or_default(),
        }
    }

    /// Chooses `theme` for both modes if it has both variants, else for its own mode only.
    pub fn pick_theme(&mut self, theme: &ThemeDef) {
        let id = Some(theme.id.clone());
        match theme.variants {
            Variants::Both => (self.theme_light, self.theme_dark) = (id.clone(), id),
            Variants::LightOnly => self.theme_light = id,
            Variants::DarkOnly => self.theme_dark = id,
        }
    }
}

impl From<Selection> for SuiteChoice {
    fn from(s: Selection) -> Self {
        SuiteChoice {
            theme_light: Some(s.theme.light),
            theme_dark: Some(s.theme.dark),
            mode: Some(s.mode),
            contrast: Some(s.contrast),
            size: Some(s.size),
            text_size: Some(s.text_size),
        }
    }
}

/// What an app shows for every field the suite hasn't chosen.
///
/// Each app states its own, normally [`crate::SUITE_DEFAULT`] in both slots with the mode and
/// size that suit it (the creative apps Dark and Compact; office apps may ship Light).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppDefaults {
    pub theme: ThemePair,
    pub mode: ModeChoice,
    pub size: Size,
}

/// An app's own choice, kept in the app's preferences: "Use a different theme / size in this
/// app". Fields left `None` follow the suite.
///
/// No `Default`: an app states what a fresh install and a legacy preference file each become, so
/// a `#[serde(default)]` can't silently move existing users to the suite.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AppChoice {
    pub theme: Option<ThemePair>,
    pub size: Option<Size>,
}

impl AppChoice {
    /// Follows the suite in every field.
    pub const FOLLOW_SUITE: AppChoice = AppChoice { theme: None, size: None };

    /// `suite` with this app's theme and size where set.
    pub fn apply(&self, suite: &Selection) -> Selection {
        Selection { theme: self.theme.clone().unwrap_or_else(|| suite.theme.clone()), size: self.size.unwrap_or(suite.size), ..suite.clone() }
    }
}

/// What the OS reports; `None` where it gives no answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SystemAppearance {
    pub mode: Option<Mode>,
    pub contrast: Option<Contrast>,
    pub reduced_motion: Option<bool>,
}

/// What an app paints for a selection.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// The theme resolved: the selection's, the app's default or [`crate::SUITE_DEFAULT`].
    pub theme: ThemeId,
    /// The appearance it was resolved for, "System" replaced by the OS's answer or the
    /// fallbacks. `mode` is the mode asked for; [`Resolved::variant_used`] is the one shown.
    pub appearance: Appearance,
    pub resolved: Resolved,
    /// Why the result differs from what the selection asked for, in the order the steps found
    /// them: mode, theme, variant.
    pub notices: Vec<Fallback>,
}

/// One way the result differs from the selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fallback {
    /// No theme has this id (deleted, or from another machine). Reported for each id tried.
    UnknownTheme(ThemeId),
    /// The theme has only this variant and the other mode was asked for.
    OnlyVariant(Mode),
    /// The mode follows the system and the OS gave no answer: the app's fallback mode.
    SystemSilent,
}

/// Resolves `sel` for this app. Pure.
///
/// `System` mode takes `system.mode`, else `fallback_mode`; `System` contrast takes
/// `system.contrast`, else standard; reduced motion is off unless the OS says otherwise. The
/// theme is the pair's slot for the mode; [`crate::APP_DEFAULT`] means the slot of
/// `defaults.theme`. An id `lookup` doesn't know falls back to that slot, then to the built-in
/// [`crate::SUITE_DEFAULT`]. `app` selects the theme's per-app overrides.
pub fn resolve_selection(
    sel: &Selection,
    system: SystemAppearance,
    fallback_mode: Mode,
    defaults: &AppDefaults,
    app: Option<&AppId>,
    lookup: &dyn Fn(&ThemeId) -> Option<ThemeDef>,
) -> Shown {
    let mut notices = Vec::new();
    let mode = match sel.mode {
        ModeChoice::Light => Mode::Light,
        ModeChoice::Dark => Mode::Dark,
        ModeChoice::System => system.mode.unwrap_or_else(|| {
            notices.push(Fallback::SystemSilent);
            fallback_mode
        }),
    };
    let contrast = match sel.contrast {
        ContrastChoice::Standard => Contrast::Standard,
        ContrastChoice::High => Contrast::High,
        ContrastChoice::System => system.contrast.unwrap_or_default(),
    };
    let appearance = Appearance { mode, contrast, size: sel.size, text_size: sel.text_size, reduced_motion: system.reduced_motion.unwrap_or(false) };

    let wanted = sel.theme.slot(mode);
    let default = defaults.theme.slot(mode);
    let mut candidates = vec![wanted];
    if default != wanted {
        candidates.push(default);
    }
    let mut found = None;
    for id in candidates.into_iter().filter(|id| !id.is_app_default()) {
        match lookup(id) {
            Some(def) => {
                found = Some(def);
                break;
            }
            None => notices.push(Fallback::UnknownTheme(id.clone())),
        }
    }
    let theme = found.unwrap_or_else(|| builtins::suite_default().clone());

    let resolved = resolve(&theme, appearance, app);
    if resolved.variant_used != mode {
        notices.push(Fallback::OnlyVariant(resolved.variant_used));
    }
    Shown { theme: theme.id, appearance, resolved, notices }
}
