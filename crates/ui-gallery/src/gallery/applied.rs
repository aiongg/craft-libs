//! What the context has been set to: the theme base, the direction and zoom of the appearance, the
//! faces, and the OS's answer they were resolved with.
use craft_platform::appearance::SystemReader;
use craft_theme::{Mode, SystemAppearance};
use craft_ui::icons;
use craft_ui::theme::{Base, install};

use crate::appearance::Appearance;
use crate::faces;

pub(super) struct Applied {
    /// The appearance `base` and the context's direction and zoom were set for.
    applied: Option<Appearance>,
    /// The scale the shown story asks for, once applied; `None` while no story asks.
    story_ppp: Option<f32>,
    /// Whether the OS's interface face had resolved when the faces were last installed.
    system_face_applied: bool,
    os: SystemReader,
    /// The OS's answer `base` was resolved with.
    pub(super) os_applied: SystemAppearance,
    pub(super) base: Base,
}

impl Applied {
    pub(super) fn new(appearance: &Appearance) -> Self {
        Self {
            applied: None,
            story_ppp: None,
            system_face_applied: false,
            os: SystemReader::for_egui(),
            os_applied: SystemAppearance::default(),
            base: appearance.base(SystemAppearance::default()),
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
            // A story's scale stays until it stops asking.
            if self.story_ppp.is_none() {
                set_zoom(ctx, appearance.zoom());
            }
        }
        install(ctx, &self.base);
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
        let zoom = match (requested, ctx.native_pixels_per_point()) {
            (Some(ppp), Some(native)) if native > 0.0 => ppp / native,
            _ => appearance.zoom(),
        };
        set_zoom(ctx, zoom);
    }
}

/// The mode `ctx` shows, which is the OS's answer while the mode choice is System.
pub(super) fn shown_mode(ctx: &egui::Context) -> Mode {
    match ctx.theme() {
        egui::Theme::Dark => Mode::Dark,
        egui::Theme::Light => Mode::Light,
    }
}

/// Sets the context's zoom factor. The gallery is the host of the craft-ui showcase: its interface
/// scale (the appearance bar's, or a story's request) is a window setting that craft-ui has no
/// token for.
fn set_zoom(ctx: &egui::Context, zoom: f32) {
    // design: allow[style-mutation](the gallery's interface scale is host configuration; craft-ui has no zoom setting)
    ctx.set_zoom_factor(zoom);
}
