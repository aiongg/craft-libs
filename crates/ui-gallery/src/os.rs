//! What the OS reports for the System mode and contrast choices, read the way an app reads it:
//! the desktop's settings portal through `craft_platform::appearance::Watcher`, the browser's
//! media queries on the web.

use craft_platform::appearance as platform;
use craft_theme::{Contrast, Mode, SystemAppearance};

/// The OS's appearance preferences.
pub struct Os {
    #[cfg(not(target_arch = "wasm32"))]
    watcher: Option<platform::Watcher>,
    /// Set by the first [`read`](Os::read), for the watcher's wake-up.
    #[cfg(not(target_arch = "wasm32"))]
    ctx: std::sync::Arc<std::sync::OnceLock<egui::Context>>,
}

impl Os {
    /// Starts watching the desktop's preferences. A change repaints the context that read last.
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = std::sync::Arc::new(std::sync::OnceLock::<egui::Context>::new());
            let woken = std::sync::Arc::clone(&ctx);
            let watcher = platform::Watcher::start(move || {
                if let Some(ctx) = woken.get() {
                    ctx.request_repaint();
                }
            });
            Self { watcher, ctx }
        }
        #[cfg(target_arch = "wasm32")]
        Self {}
    }

    /// The OS's mode and contrast now; `None` where it gives no answer. Reduced motion is the
    /// gallery's own checkbox, not read here.
    pub fn read(&self, ctx: &egui::Context) -> SystemAppearance {
        #[cfg(not(target_arch = "wasm32"))]
        let (observed, contrast) = {
            let _ = self.ctx.set(ctx.clone());
            let observed = self.watcher.as_ref().and_then(platform::Watcher::current);
            platform::sync_native_window_theme(ctx, observed);
            (observed, self.watcher.as_ref().and_then(platform::Watcher::contrast))
        };
        #[cfg(target_arch = "wasm32")]
        let (observed, contrast) = (None, platform::browser_contrast());
        let mode = observed
            .map(|t| match t {
                platform::Theme::Dark => Mode::Dark,
                platform::Theme::Light => Mode::Light,
            })
            .or_else(|| {
                ctx.system_theme().map(|t| match t {
                    egui::Theme::Dark => Mode::Dark,
                    egui::Theme::Light => Mode::Light,
                })
            });
        let contrast = contrast.map(|c| match c {
            platform::Contrast::High => Contrast::High,
            platform::Contrast::Standard => Contrast::Standard,
        });
        SystemAppearance { mode, contrast, reduced_motion: None }
    }
}

/// The mode `ctx` shows, which is the OS's answer while the mode choice is System.
pub fn shown(ctx: &egui::Context) -> Mode {
    match ctx.theme() {
        egui::Theme::Dark => Mode::Dark,
        egui::Theme::Light => Mode::Light,
    }
}
