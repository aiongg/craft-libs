//! `Area` with its left-top pivot, `Order`, sublayers, transform layers and `set_layer_order`.

#[cfg(feature = "testing")]
use crate::runtime::egui::memory::unique;
#[cfg(feature = "testing")]
use egui::{Context, Ui};

/// Shows an `egui::Area` named `name` and runs `body` on its `Ui`.
#[cfg(feature = "testing")]
pub(crate) fn show_area<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Area::new(unique(name)).show(ctx, body).inner
}

/// An egui layer, as the runtime's shape records name it.
pub type LayerKey = egui::LayerId;
