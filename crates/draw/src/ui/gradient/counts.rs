//! The count words of the gradient bar's summary, written with [`Words::counted`](craft_ui::localized::Words::counted).

use craft_ui::localized::words::Counted;

pub(crate) const BAR_STOPS: Counted = Counted::new("gradient.barStops", "{n} stop", "{n} stops");
pub(crate) const COLOR_STOPS: Counted = Counted::new("gradient.colorStops", "{n} color stop", "{n} color stops");
pub(crate) const OPACITY_STOPS: Counted = Counted::new("gradient.opacityStops", "{n} opacity stop", "{n} opacity stops");

#[cfg(test)]
pub(crate) const ALL: [Counted; 3] = [BAR_STOPS, COLOR_STOPS, OPACITY_STOPS];
