//! Display components: the value maths of `Badge::count` and `Progress`.

use craft_ui::contract::{CommandSet, Props as _, Show as _};
use craft_ui::display::{Badge, Kbd, Progress};
use craft_ui::testing::{Fixture, Frames, trial};

/// Commands of the Kbd trial: one with a default chord and one without.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "kbd_trial")]
#[non_exhaustive]
pub enum KbdCommand {
    #[command(label = "Chorded", default = "Cmd+Alt+Shift+K")]
    Chorded,
    #[command(label = "Bare")]
    Bare,
}

/// A Kbd bound to a command measures the chord step 1 resolved for it, so the room it takes matches what it paints.
#[trial]
fn a_kbd_bound_to_a_command_measures_the_resolved_chord() {
    let width = |command: KbdCommand| {
        let mut frames = Frames::new();
        let mut width = 0.0;
        frames.idle(3, |ui| width = Kbd::command(command).id_salt("kbd").show(ui).rect.width());
        width
    };
    assert!(width(KbdCommand::Chorded) > width(KbdCommand::Bare), "a command with a chord takes the chord's room");
}

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
