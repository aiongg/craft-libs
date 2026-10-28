//! The OS's own interface face, for [`InterfaceFont::System`](craft_theme::InterfaceFont).
//!
//! It is read from the installed fonts at run time: fontique's `system-ui` family (San Francisco
//! on macOS, Segoe UI on Windows, fontconfig's `system-ui` match on Linux, which falls back to
//! `sans-serif`), preferring Segoe UI Variable where Windows 11 has it.

use egui::FontData;

/// The optical size (`opsz`) the system face draws at, in points: the body type size. egui sets a
/// face's variation once, not per text size, so titles and headings draw at this optical size
/// too. A face without an `opsz` axis ignores it; one whose axis starts above it takes the axis
/// minimum.
pub const SYSTEM_UI_OPTICAL_SIZE: f32 = 13.0;

/// The OS's interface face at regular and semibold weight.
#[derive(Clone, Debug)]
pub struct SystemUiFace {
    /// The family's name, e.g. `DejaVu Sans` or `System Font`.
    pub family: String,
    /// The regular (400) face.
    pub regular: FontData,
    /// The semibold face: a variable face's `wght` 600 instance, else the family's lightest static
    /// face from 600 up (Bold when it has no SemiBold), else the regular face. Never emboldened.
    pub semibold: FontData,
    /// The weight [`Self::semibold`] draws at: 400 when it is the regular face.
    pub semibold_weight: u16,
}

impl SystemUiFace {
    /// The egui name of [`Self::regular`].
    pub fn regular_name(&self) -> String {
        format!("system-ui:{}", self.family)
    }

    /// The egui name of [`Self::semibold`].
    pub fn semibold_name(&self) -> String {
        format!("system-ui:{} @{}", self.family, self.semibold_weight)
    }
}

/// The OS's interface face, or `None` on the web, without the `system_fonts` feature, or when no
/// installed family resolves. Resolved once per process; the first call waits for the index of
/// installed fonts the system font provider shares. Use it to offer the interface-font setting
/// only where it can take effect.
#[cfg(all(feature = "system_fonts", not(target_arch = "wasm32")))]
pub fn system_ui_face() -> Option<&'static SystemUiFace> {
    static FACE: std::sync::OnceLock<Option<SystemUiFace>> = std::sync::OnceLock::new();
    FACE.get_or_init(super::system_index::system_ui_face).as_ref()
}

/// The OS's interface face, or `None` on the web, without the `system_fonts` feature, or when no
/// installed family resolves.
#[cfg(not(all(feature = "system_fonts", not(target_arch = "wasm32"))))]
pub fn system_ui_face() -> Option<&'static SystemUiFace> {
    None
}

/// Set `data`'s variation coordinates to `coords`, each clamped to its axis's range, plus the
/// optical size if the face has that axis. Axes the face lacks are skipped.
#[cfg_attr(not(all(feature = "system_fonts", not(target_arch = "wasm32"))), allow(dead_code, reason = "used when resolving the system face"))]
pub(super) fn instance(mut data: FontData, coords: impl IntoIterator<Item = ([u8; 4], f32)>) -> FontData {
    let axes = data.variation_axes();
    let opsz = (*b"opsz", SYSTEM_UI_OPTICAL_SIZE);
    for (tag, value) in coords.into_iter().chain([opsz]) {
        let tag_value = egui::epaint::text::Tag::new(&tag);
        if data.tweak.coords.as_ref().iter().any(|(t, _)| *t == tag_value) {
            continue;
        }
        if let Some(axis) = axes.iter().find(|axis| axis.tag == tag_value) {
            data.tweak.coords.push(tag, value.clamp(axis.range.min, axis.range.max));
        }
    }
    data
}
