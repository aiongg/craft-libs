//! Story of the password dialog in its three purposes: a password that opens a file, a
//! permissions password and a new password typed twice. The answer shows how many characters
//! were typed, never the password.

use egui::Ui;

use super::framework_stories::{claim, drive};
use super::{PasswordDialog, PasswordLabels, PasswordPurpose};
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PurposeArg {
    Open,
    Permissions,
    Set,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PasswordArgs {
    open: bool,
    purpose: PurposeArg,
    #[arg(name = "wrong password")]
    wrong: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/dialog/password",
        Level::Patterns,
        "Dialog",
        "Password",
        PasswordArgs { open: true, purpose: PurposeArg::Open, wrong: false },
        show_password
    )]
}

/// The password dialog; `wrong password` reopens Open and Permissions with the app's error text.
fn show_password(ui: &mut Ui, args: &mut PasswordArgs) {
    if !claim(ui.ctx(), "password", ui.scope_id()) {
        return;
    }
    let (purpose, wrong, words, labels) = (args.purpose, args.wrong, StoryLabels::of(ui), PasswordLabels::of(ui));
    drive(
        ui,
        "password",
        &mut args.open,
        (purpose, wrong),
        || {
            let dialog = match purpose {
                PurposeArg::Open => PasswordDialog::open("report.pdf"),
                PurposeArg::Permissions => PasswordDialog::permissions("report.pdf"),
                PurposeArg::Set => PasswordDialog::new(PasswordPurpose::Set),
            };
            if wrong && purpose != PurposeArg::Set { dialog.error(labels.wrong.as_str()) } else { dialog }
        },
        |_| {},
        |answer| craft_i18n::fmt(&words.result_ok, &[("value", &format!("{} ••••", answer.secret.expose().chars().count()))]),
    );
}
