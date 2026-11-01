//! The scope panel: Photoshop's Histogram panel and Camera Raw's scope with tone zones,
//! clipping warnings and the vectorscope, plus the clipping overlay for a preview.
//!
//! The app counts (colour management, the histogram, cache levels and jobs stay its own); the
//! library draws, samples the pointer and reports:
//!
//! ```ignore
//! let frame = ScopeFrame::histogram(HistogramSet::new(&channels).original(adjusted_over.as_deref()).stale(stale))
//!     .vectorscope(VectorscopeFrame { counts: &scope.counts, hues: HUES, saturations: SATURATIONS, hover: pointer_sample })
//!     .clipping(ClippingFrame { shadows, highlights });
//! let out = ScopePanel::new("histogram", &mut state.scope).frame(&frame).sources(&sources).markers(&marks).show(ui);
//! if out.refresh { uncached_refresh(app); }
//! if out.settings_changed { persist(&state.scope.settings); }
//! if let Some(edit) = out.zone { app.edit_tone_zone(edit); }
//! ClippingOverlay::new("clip").show(ui.ctx(), ui.painter(), preview_rect, &mask, revision, state.scope.settings.clipping_mode(diagnostic));
//! ```
//!
//! | Part | Names |
//! |---|---|
//! | The panel | [`ScopePanel`], [`ScopePanelOutput`], [`ScopePanelState`], [`ScopeSettings`], [`ScopeView`], [`ScopePanelLabels`], [`ScopePanelStyle`] |
//! | The data | [`ScopeFrame`], [`HistogramSet`], [`ScopeChannel`], [`ChannelOverlay`], [`ScopeSource`], [`ScopeReadout`], [`VectorscopeFrame`], [`ClippingFrame`] |
//! | Tone zones | [`ToneZone`], [`ScopeZoneEdit`], [`ZoneChange`] |
//! | Menus | [`ScopePanel::panel_menu`], [`ScopePanel::scope_menu`], [`ScopePanel::run_command`], [`ScopeMenuAction`] |
//! | Clipping overlay | [`ClippingOverlay`], [`ClippingMode`], [`ClippingWarnings`], [`ClippingDiagnostic`] |
//! | Vectorscope alone | [`VectorscopePlot`], [`scope_point`] |

mod clipping;
#[cfg(test)]
mod clipping_tests;
mod labels;
mod menu;
mod panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
mod plot;
#[cfg(test)]
mod plot_tests;
#[cfg(test)]
mod settings_tests;
pub(crate) mod stories;
mod style;
mod vectorscope;
#[cfg(test)]
mod vectorscope_tests;
mod zones;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod zones_tests;

pub use clipping::{ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingOverlay, ClippingWarnings};
pub use labels::ScopePanelLabels;
pub use menu::ScopeMenuAction;
pub use panel::{
    COLORS_CHANNEL, ChannelOverlay, HistogramSet, ScopeChannel, ScopeFrame, ScopePanel, ScopePanelOutput, ScopePanelState, ScopeReadout,
    ScopeSettings, ScopeSource, ScopeView,
};
pub use style::ScopePanelStyle;
pub use vectorscope::{VectorscopeFrame, VectorscopePlot, scope_point};
pub use zones::{ScopeZoneEdit, ToneZone, ZoneChange};
