//! Display components: the value maths of `Badge::count` and `Progress`.

use craft_ui::contract::Show as _;
use craft_ui::display::{Badge, Progress};
use craft_ui::testing::{Fixture, trial};

/// A count shows its digits up to the cap and `99+` above it.
#[trial]
fn a_count_badge_caps_at_ninety_nine() {
    for (count, shown) in [(7, "7"), (99, "99"), (100, "99+"), (1234, "99+")] {
        let mut fixture = Fixture::new(move |ui| {
            Badge::count(count).show(ui);
        });
        fixture.settle();
        assert!(fixture.by_label(shown).is_some(), "a count of {count} reads {shown}");
    }
}

/// A fraction outside 0 to 1 draws and reports as the nearest end, and one that is not a number as empty.
#[trial]
fn a_progress_fraction_is_clamped() {
    assert_eq!(Progress::clamped(1.7), 1.0);
    assert_eq!(Progress::clamped(-3.0), 0.0);
    assert_eq!(Progress::clamped(f32::NAN), 0.0);
    assert_eq!(Progress::clamped(0.25), 0.25);
}
