//! One call for the OS's colour scheme, contrast and reduced-motion preferences on every target:
//! the settings portal's [`Watcher`] where one exists, the browser's media queries on the web.

use super::{Contrast, Theme, Watcher};

/// What the OS reports now. `None` is no answer; the host decides the fallback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    pub theme: Option<Theme>,
    pub contrast: Option<Contrast>,
    pub reduced_motion: Option<bool>,
}

#[cfg(feature = "egui")]
impl Reading {
    /// The colour scheme as an egui theme.
    pub fn egui_theme(&self) -> Option<egui::Theme> {
        self.theme.map(|t| match t {
            Theme::Dark => egui::Theme::Dark,
            Theme::Light => egui::Theme::Light,
        })
    }

    /// `theme`, else `fallback`, which a host takes from egui's own system theme.
    fn with_theme_from(mut self, fallback: Option<egui::Theme>) -> Self {
        self.theme = self.theme.or(fallback.map(|t| match t {
            egui::Theme::Dark => Theme::Dark,
            egui::Theme::Light => Theme::Light,
        }));
        self
    }
}

#[cfg(feature = "theme")]
impl From<Reading> for craft_theme::SystemAppearance {
    fn from(reading: Reading) -> Self {
        craft_theme::SystemAppearance {
            mode: reading.theme.map(|t| match t {
                Theme::Dark => craft_theme::Mode::Dark,
                Theme::Light => craft_theme::Mode::Light,
            }),
            contrast: reading.contrast.map(|c| match c {
                Contrast::Standard => craft_theme::Contrast::Standard,
                Contrast::High => craft_theme::Contrast::High,
            }),
            reduced_motion: reading.reduced_motion,
        }
    }
}

/// Reads the OS's appearance preferences. Owns the desktop [`Watcher`] where the target has
/// one; on the web it asks the browser on each read, since media queries have no change
/// notification, so read it when building a frame. A target with neither reports no answers.
pub struct SystemReader {
    watcher: Option<Watcher>,
    /// Set by the first read with a context, for the watcher's wake-up.
    #[cfg(feature = "egui")]
    ctx: std::sync::Arc<std::sync::OnceLock<egui::Context>>,
}

impl SystemReader {
    /// Starts watching. `wake` runs on the watcher's worker when a preference changes and must
    /// not block.
    pub fn start(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            watcher: Watcher::start(wake),
            #[cfg(feature = "egui")]
            ctx: std::sync::Arc::default(),
        }
    }

    /// Starts watching; a change repaints the context [`read_in`](Self::read_in) saw first.
    #[cfg(feature = "egui")]
    pub fn for_egui() -> Self {
        let ctx = std::sync::Arc::new(std::sync::OnceLock::<egui::Context>::new());
        let woken = std::sync::Arc::clone(&ctx);
        let watcher = Watcher::start(move || {
            if let Some(ctx) = woken.get() {
                ctx.request_repaint();
            }
        });
        Self { watcher, ctx }
    }

    /// Whether a desktop watcher is running.
    pub fn watching(&self) -> bool {
        self.watcher.is_some()
    }

    /// GNOME's `button-layout` as the watcher last heard it; see [`Watcher::button_layout`].
    /// `None` on the web and where no watcher runs.
    pub fn button_layout(&self) -> Option<String> {
        self.watcher.as_ref().and_then(Watcher::button_layout)
    }

    /// GNOME's `icon-theme` name as the watcher last heard it; see [`Watcher::icon_theme`].
    /// `None` on the web and where no watcher runs.
    pub fn icon_theme(&self) -> Option<String> {
        self.watcher.as_ref().and_then(Watcher::icon_theme)
    }

    /// The preferences now, without blocking.
    pub fn read(&self) -> Reading {
        #[cfg(target_arch = "wasm32")]
        {
            Reading { theme: None, contrast: super::browser_contrast(), reduced_motion: super::browser_reduced_motion() }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let watcher = self.watcher.as_ref();
            Reading {
                theme: watcher.and_then(Watcher::current),
                contrast: watcher.and_then(Watcher::contrast),
                reduced_motion: watcher.and_then(Watcher::reduced_motion),
            }
        }
    }

    /// [`read`](Self::read) for an egui host: registers `ctx` for wake-ups, hands the native
    /// window the observed scheme (see [`sync_native_window_theme`](super::sync_native_window_theme))
    /// and answers the scheme from egui's own system theme where the OS reader has none (the web,
    /// macOS, Windows).
    #[cfg(feature = "egui")]
    pub fn read_in(&self, ctx: &egui::Context) -> Reading {
        self.observe(ctx).with_theme_from(ctx.system_theme())
    }

    /// [`read_in`](Self::read_in) without the egui fallback for the scheme.
    #[cfg(feature = "egui")]
    fn observe(&self, ctx: &egui::Context) -> Reading {
        let _ = self.ctx.set(ctx.clone());
        let reading = self.read();
        super::sync_native_window_theme(ctx, reading.theme);
        reading
    }

    /// The scheme the watcher observed, for [`egui_service`](super::egui_service).
    #[cfg(feature = "egui")]
    pub(super) fn observed_theme(&self, ctx: &egui::Context) -> Option<egui::Theme> {
        self.observe(ctx).egui_theme()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_without_answers_is_empty() {
        assert_eq!(Reading::default(), Reading { theme: None, contrast: None, reduced_motion: None });
    }

    #[cfg(feature = "theme")]
    #[test]
    fn a_reading_converts_to_the_themes_system_appearance() {
        use craft_theme::{Contrast as C, Mode, SystemAppearance};
        let reading = Reading { theme: Some(Theme::Dark), contrast: Some(Contrast::High), reduced_motion: Some(true) };
        assert_eq!(SystemAppearance::from(reading), SystemAppearance { mode: Some(Mode::Dark), contrast: Some(C::High), reduced_motion: Some(true) });
        assert_eq!(SystemAppearance::from(Reading::default()), SystemAppearance::default());
    }

    #[cfg(feature = "egui")]
    #[test]
    fn the_scheme_falls_back_to_eguis_system_theme_only_without_an_answer() {
        let answered = Reading { theme: Some(Theme::Light), ..Reading::default() };
        assert_eq!(answered.with_theme_from(Some(egui::Theme::Dark)).theme, Some(Theme::Light));
        assert_eq!(Reading::default().with_theme_from(Some(egui::Theme::Dark)).theme, Some(Theme::Dark));
        assert_eq!(Reading::default().with_theme_from(None).theme, None);
    }
}
