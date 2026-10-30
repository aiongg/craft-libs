//! What the context has been set to: the theme base, the direction and zoom of the appearance, the
//! faces, and the OS's answer they were resolved with.
use std::sync::Arc;

use craft_i18n::{Embedded, I18n, Lang};
use craft_platform::appearance::SystemReader;
use craft_theme::{Mode, SystemAppearance};
use craft_ui::icons;
use craft_ui::interface_scale::InterfaceScale;
use craft_ui::localized::Words;
use craft_ui::theme::{Base, install};

use crate::appearance::Appearance;
use crate::faces;

/// The gallery ships no translations: English and the pseudo locale need none.
static CATALOGS: &Embedded = &[];

pub(super) struct Applied {
    /// The appearance `base` and the context's direction and zoom were set for.
    applied: Option<Appearance>,
    /// The scale the shown story asks for, once applied; `None` while no story asks.
    story_ppp: Option<f32>,
    /// The display scale the interface scale was last applied for. A fixed percent is absolute,
    /// so it is applied again when the window moves to a display with another scale; the zoom
    /// is otherwise left alone, so egui's Ctrl +/- zoom keeps working.
    native_applied: Option<f32>,
    /// Whether the OS's interface face had resolved when the faces were last installed.
    system_face_applied: bool,
    os: SystemReader,
    /// The OS's answer `base` was resolved with.
    pub(super) os_applied: SystemAppearance,
    pub(super) base: Base,
    /// The catalogs the installed [`Words`] read; the pseudo locale records what it looked up here.
    pub(super) i18n: Arc<I18n>,
}

impl Applied {
    pub(super) fn new(appearance: &Appearance) -> Self {
        Self {
            applied: None,
            story_ppp: None,
            native_applied: None,
            system_face_applied: false,
            os: SystemReader::for_egui(),
            os_applied: SystemAppearance::default(),
            base: appearance.base(SystemAppearance::default()),
            i18n: Arc::new(I18n::new(CATALOGS, &[]).0),
        }
    }

    /// Configures the context for `appearance` when it, the faces or the OS's answer changed, and
    /// installs the base.
    pub(super) fn apply(&mut self, ctx: &egui::Context, appearance: &Appearance) {
        let system_face = faces::system_face(ctx).is_some();
        let system = SystemAppearance::from(self.os.read_in(ctx));
        if self.applied.as_ref() != Some(appearance) || system_face != self.system_face_applied || system != self.os_applied {
            let previous = self.applied.replace(appearance.clone());
            self.base = appearance.configure(ctx, previous.as_ref(), system_face != self.system_face_applied, system);
            self.os_applied = system;
            self.system_face_applied = system_face;
            if previous.as_ref().is_none_or(|p| p.icon_set != appearance.icon_set) {
                icons::set_active(ctx, appearance.icon_set());
            }
            if previous.as_ref().is_none_or(|p| p.language != appearance.language) {
                let lang = self.i18n.from_code(&appearance.language).unwrap_or(Lang::EN);
                Words::from_i18n(Arc::clone(&self.i18n), lang).install(ctx);
            }
            // A story's scale stays until it stops asking.
            if self.story_ppp.is_none() {
                appearance.scale.apply(ctx);
            }
        }
        let native = ctx.native_pixels_per_point();
        if native != self.native_applied {
            self.native_applied = native;
            self.scale_of(appearance).apply(ctx);
        }
        install(ctx, &self.base);
    }

    /// The scale the story asks for, else the appearance's.
    fn scale_of(&self, appearance: &Appearance) -> InterfaceScale {
        match self.story_ppp {
            Some(ppp) => InterfaceScale::percent((ppp * 100.0).round() as u16),
            None => appearance.scale,
        }
    }

    /// Sets the whole gallery's scale when a story starts or stops asking for one
    /// ([`story::request_pixels_per_point`](craft_ui::story::request_pixels_per_point)): the
    /// story's request, or back to the options bar's. In between the zoom is left alone, so
    /// egui's Ctrl +/− zoom keeps working.
    pub(super) fn apply_story_scale(&mut self, ctx: &egui::Context, appearance: &Appearance) {
        let requested = craft_ui::story::take_pixels_per_point_request(ctx);
        if requested == self.story_ppp {
            return;
        }
        self.story_ppp = requested;
        self.scale_of(appearance).apply(ctx);
    }
}

/// The mode `ctx` shows, which is the OS's answer while the mode choice is System.
pub(super) fn shown_mode(ctx: &egui::Context) -> Mode {
    match ctx.theme() {
        egui::Theme::Dark => Mode::Dark,
        egui::Theme::Light => Mode::Light,
    }
}

#[cfg(test)]
mod tests {
    use egui::{RawInput, ViewportId, ViewportInfo};

    use super::*;

    fn frame(ctx: &egui::Context, applied: &mut Applied, appearance: &Appearance, native: f32) {
        let mut input = RawInput::default();
        input.viewports.insert(ViewportId::ROOT, ViewportInfo { native_pixels_per_point: Some(native), ..Default::default() });
        ctx.run_ui(input, |ui| applied.apply(ui.ctx(), appearance)).drop_without_applying_deltas();
    }

    #[test]
    fn a_fixed_percent_follows_the_window_to_a_display_with_another_scale() {
        let ctx = egui::Context::default();
        let appearance = Appearance { scale: InterfaceScale::percent(100), ..Appearance::default() };
        let mut applied = Applied::new(&appearance);
        frame(&ctx, &mut applied, &appearance, 1.0);
        frame(&ctx, &mut applied, &appearance, 1.0);
        assert!((ctx.pixels_per_point() - 1.0).abs() < 1e-3);
        frame(&ctx, &mut applied, &appearance, 2.0);
        frame(&ctx, &mut applied, &appearance, 2.0);
        assert!((ctx.pixels_per_point() - 1.0).abs() < 1e-3, "100 % stays 100 % on a 2x display, got {}", ctx.pixels_per_point());
    }
}
