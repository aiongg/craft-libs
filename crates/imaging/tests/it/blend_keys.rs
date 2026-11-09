//! The blend mode keys: the steps wrap, a letter picks its listed mode, a letter of a missing mode picks nothing.

use craft_imaging::ui::{BlendKeyEntry, BlendKeyPick, BlendKeys, BlendShortcut, BlendStep};
use craft_ui::runtime::commands::{CommandRef, CommandSet as _, FirePhase};
use craft_ui::testing::trial;

fn modes() -> Vec<BlendKeyEntry<&'static str>> {
    vec![
        BlendKeyEntry::new("normal", BlendShortcut::Normal),
        BlendKeyEntry::new("multiply", BlendShortcut::Multiply),
        BlendKeyEntry::new("screen", BlendShortcut::Screen),
        BlendKeyEntry::unlettered("subtract"),
    ]
}

fn step(command: BlendStep, current: Option<&'static str>, entries: &[BlendKeyEntry<&'static str>]) -> Option<BlendKeyPick<&'static str>> {
    let keys = BlendKeys::new(entries);
    let keys = match &current {
        Some(current) => keys.current(current),
        None => keys,
    };
    keys.command(&CommandRef::from(command), FirePhase::Press)
}

fn letter(shortcut: BlendShortcut, entries: &[BlendKeyEntry<&'static str>]) -> Option<BlendKeyPick<&'static str>> {
    BlendKeys::new(entries).command(&CommandRef::from(shortcut), FirePhase::Press)
}

#[trial]
fn shift_plus_and_minus_step_and_wrap() {
    let entries = modes();
    assert_eq!(step(BlendStep::Next, Some("subtract"), &entries), Some(BlendKeyPick::Stepped("normal")));
    assert_eq!(step(BlendStep::Previous, Some("normal"), &entries), Some(BlendKeyPick::Stepped("subtract")));
    assert_eq!(step(BlendStep::Next, Some("unlisted"), &entries), Some(BlendKeyPick::Stepped("normal")));
    assert_eq!(step(BlendStep::Previous, None, &entries), Some(BlendKeyPick::Stepped("subtract")));
    assert_eq!(step(BlendStep::Next, Some("normal"), &entries), Some(BlendKeyPick::Stepped("multiply")));
    let released = BlendKeys::new(&entries).command(&CommandRef::from(BlendStep::Next), FirePhase::Release);
    assert_eq!(released, None, "a key release steps nothing");
}

#[trial]
fn shift_alt_letter_picks_the_listed_mode() {
    let pick = letter(BlendShortcut::Multiply, &modes());
    assert_eq!(pick, Some(BlendKeyPick::Lettered("multiply")));
    assert_eq!(pick.map(BlendKeyPick::into_key), Some("multiply"));
}

#[trial]
fn a_letter_for_a_missing_mode_is_consumed_and_does_nothing() {
    let entries = modes();
    assert_eq!(letter(BlendShortcut::PassThrough, &entries), None);
    assert_eq!(letter(BlendShortcut::Overlay, &entries), None);
}

#[trial]
fn every_shortcut_has_its_own_letter() {
    let defs = BlendShortcut::defs().expect("the shortcuts are valid commands");
    assert_eq!(defs.len(), BlendShortcut::ALL.len());
    let chords: std::collections::HashSet<String> = defs
        .iter()
        .map(|def| {
            assert_eq!(def.defaults.len(), 1, "{} has one default chord", def.command.id);
            format!("{:?}", def.defaults[0].trigger)
        })
        .collect();
    assert_eq!(chords.len(), defs.len(), "no two shortcuts share a chord");
}

#[trial]
fn behind_is_consumed_and_reported() {
    assert_eq!(letter(BlendShortcut::Behind, &modes()), None, "layers have no Behind");
    let mut entries = modes();
    entries.push(BlendKeyEntry::behind("behind"));
    assert_eq!(letter(BlendShortcut::Behind, &entries), Some(BlendKeyPick::Lettered("behind")));
}
