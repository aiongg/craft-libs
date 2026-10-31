//! The profile prompts in a kittest harness, and the status words.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{ProfileAction, ProfileLabels, ProfileMismatchDialog, ProfileMode, ProfileQuestion, ProfileStatus};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::theme::{Base, install};

fn mismatch() -> ProfileQuestion {
    ProfileQuestion::Mismatch { embedded: "Display P3".into(), working: "sRGB IEC61966-2.1".into() }
}

fn host(dialog: ProfileMismatchDialog) -> Harness<'static, DialogHost> {
    let mut host = DialogHost::new();
    host.open(dialog);
    let mut harness = Harness::<DialogHost>::builder()
        .with_size([700.0, 600.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host);
    harness.run();
    harness
}

fn radios(harness: &Harness<'_, DialogHost>) -> Vec<String> {
    harness.query_all_by_role(Role::RadioButton).filter_map(|n| n.accesskit_node().label()).collect()
}

fn boxes(harness: &Harness<'_, DialogHost>) -> usize {
    harness.query_all_by_role(Role::CheckBox).count()
}

/// A mismatch offers the three actions naming both profiles; a missing profile the two.
#[test]
fn the_choices_follow_the_question() {
    let harness = host(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()));
    assert_eq!(
        radios(&harness),
        [
            "Use the embedded profile (Display P3)",
            "Convert to the working space (sRGB IEC61966-2.1)",
            "Discard the embedded profile (don't color manage)",
        ]
    );
    let missing = ProfileQuestion::Missing { working: "sRGB IEC61966-2.1".into() };
    let harness = host(ProfileMismatchDialog::new("photo.psd", ProfileMode::Cmyk, missing));
    assert_eq!(radios(&harness), ["Leave as is (don't color manage)", "Assign working space"]);
}

/// Remember this choice shows only for a rememberable mismatch and comes back in the answer.
#[test]
fn remember_shows_for_a_mismatch_only() {
    let harness = host(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()));
    assert_eq!(boxes(&harness), 0, "not rememberable");
    let missing = ProfileQuestion::Missing { working: "sRGB".into() };
    let harness = host(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, missing).rememberable(true));
    assert_eq!(boxes(&harness), 0, "a missing profile has no policy");

    let mut harness = host(ProfileMismatchDialog::new("photo.psd", ProfileMode::Rgb, mismatch()).rememberable(true));
    assert_eq!(boxes(&harness), 1);
    harness.get_by_label("Remember this choice for future images").click();
    harness.get_by_label("Convert to the working space (sRGB IEC61966-2.1)").click();
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    harness.run();
    match harness.state_mut().take::<ProfileMismatchDialog>() {
        Some(DialogResult::Ok(answer)) => {
            assert_eq!(answer.action, ProfileAction::ConvertToWorking);
            assert!(answer.remember);
        }
        other => panic!("{other:?}"),
    }
}

/// A profile status reads Untagged, Invalid, Unnamed or the profile's name.
#[test]
fn the_profile_status_reads_photoshops_words() {
    let labels = ProfileLabels::default();
    assert_eq!(ProfileStatus::of(None), ProfileStatus::Untagged);
    assert_eq!(ProfileStatus::of(Some(b"not a profile")), ProfileStatus::Invalid);
    let srgb = craft_cms::builtin::srgb().to_bytes();
    let named = ProfileStatus::of(Some(&srgb));
    assert!(matches!(&named, ProfileStatus::Named(name) if !name.is_empty()), "{named:?}");
    assert_eq!(ProfileStatus::Untagged.label(ProfileMode::Rgb, &labels), "Untagged RGB");
    assert_eq!(ProfileStatus::Invalid.label(ProfileMode::Cmyk, &labels), "Invalid CMYK profile");
    assert_eq!(ProfileStatus::Unnamed.label(ProfileMode::Gray, &labels), "Unnamed Gray profile");
    assert_eq!(ProfileStatus::Named("Display P3".into()).label(ProfileMode::Rgb, &labels), "Display P3");
}

/// A monitor profile's problem is worded by the labels, with the profile's name and reason.
#[test]
fn a_monitor_problem_is_worded_by_the_labels() {
    use crate::display::{DisplayCaches, MonitorProblem, MonitorStatus};
    let labels = ProfileLabels::default();
    let caches = DisplayCaches::new();
    let cmyk = craft_cms::Builtin::CoatedFogra39.profile();
    caches.set_monitor(Some(1), Some(&cmyk.to_bytes()));
    let MonitorStatus::Unusable(problem) = caches.monitor_status(Some(1)) else { panic!("a CMYK profile is not a monitor profile") };
    assert_eq!(problem.label(&labels), format!("“{}” is a CMYK profile, not an RGB profile.", cmyk.description));
    caches.set_monitor(Some(2), Some(b"garbage"));
    let MonitorStatus::Unusable(problem @ MonitorProblem::Unreadable { .. }) = caches.monitor_status(Some(2)) else {
        panic!("garbage does not parse")
    };
    assert!(problem.label(&labels).starts_with("The display profile can't be read: "));
}
