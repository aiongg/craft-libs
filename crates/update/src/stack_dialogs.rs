//! The update dialogs on craft-ui's dialog host: [`UpdatePromptDialog`] (a release is available)
//! and [`UpdateCheckDialog`] (the result of a manual check), on [`Dialog`](craft_ui::dialog::Dialog)
//! with the app's name and version from `Host::app` (craft-host) and the words from
//! [`UpdateLabels`]. [`dialog`](crate::dialog) keeps the hand-drawn cards until the apps have moved.

use craft_host::Host;
use craft_ui::composites::{Notice, Tone};
use craft_ui::controls::{Link, Spinner};
use craft_ui::dialog::{
    ButtonRole, Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, FooterButton, Outcome, StackDialog, Width, dialog_id,
};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::current;
use egui::{Context, Ui};

use crate::app::App;
use crate::dialog::icon_texture;
use crate::release::Release;
use crate::updater::Status;

const PROMPT_KEY: &str = "update-prompt";
const CHECK_KEY: &str = "update-check";

craft_ui::words! {
    /// The update dialogs' words, English by default.
    #[non_exhaustive]
    pub struct UpdateLabels {
        available_title: "update.available.title" = "Update available";
        available_message: "update.available.message" = "{app} {version} is available. You have {running}.";
        update_and_restart: "update.updateAndRestart" = "Update and Restart";
        download: "update.download" = "Download";
        later: "update.later" = "Later";
        whats_new: "update.whatsNew" = "What's New";
        releases_listed: "update.releasesListed" = "All releases are listed at";
        version_line: "update.versionLine" = "v{running} - {version}";
        installing: "update.installing" = "Installing…";
        check_title: "update.check.title" = "Check for Updates";
        checking: "update.checking" = "Checking for updates…";
        up_to_date: "update.upToDate" = "{app} {running} is the newest version.";
        failed: "update.failed" = "The update check failed.";
        check_again: "update.checkAgain" = "Check Again";
    }
}

/// What the user chose in the update prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UpdateChoice {
    /// Update and Restart: the default where the app can update itself.
    UpdateAndRestart,
    /// Open the release page in the browser (the default elsewhere).
    Download,
    /// Later: close the prompt for this run.
    Later,
}

/// "Update available", on the startup queue at [`StartupStage::Update`](craft_ui::dialog::StartupStage::Update)
/// so it never overlaps the GPU warning or recovery (gap 75), or opened by a manual check that
/// found a release. The user sees the app's icon and name, the new and running versions, the
/// release notes link, and Update and Restart / Download / Later in the platform order; while an
/// install runs the install buttons are disabled and a status line shows. Enter takes the default, Esc is
/// Later. Later and Esc stay available while the install runs. Result: [`UpdateChoice`].
pub struct UpdatePromptDialog {
    id: DialogId,
    release: Release,
    running_version: String,
    self_updating: bool,
    status: Option<String>,
    installing: bool,
    state: DialogState<()>,
}

/// The app the dialogs are for: the installed Host's name and the suite entry that carries its
/// icon and developer.
struct Subject {
    name: &'static str,
    app: Option<&'static App>,
}

impl Subject {
    fn of(ctx: &Context) -> Self {
        let info = Host::try_of_ctx(ctx).map(|host| host.app().clone());
        Self { name: info.as_ref().map_or("", |info| info.name), app: info.and_then(|info| App::by_id(info.id)) }
    }

    /// The icon, the name and the developer, as the head of both dialogs.
    fn head(&self, ui: &mut Ui, second_line: &str) {
        let tokens = current(ui.ctx());
        let side = tokens.size.control.md;
        ui.horizontal(|ui| {
            if let Some(icon) = self.app.and_then(|app| icon_texture(ui.ctx(), app)) {
                ui.add(egui::Image::new((icon.id(), egui::vec2(side, side))));
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add(Text::new(self.name));
                ui.add(Text::new(second_line).content(Content::Muted));
            });
        });
    }
}

impl UpdatePromptDialog {
    /// A prompt for `release`; `self_updating` offers Update and Restart as the default.
    pub fn new(release: Release, running_version: impl Into<String>, self_updating: bool) -> Self {
        Self {
            id: dialog_id(PROMPT_KEY),
            release,
            running_version: running_version.into(),
            self_updating,
            status: None,
            installing: false,
            state: DialogState::new(),
        }
    }

    /// The install's progress line while [`installing`](UpdatePromptDialog::set_installing); the
    /// app sets it through `DialogHost::get_mut`.
    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }

    /// Disables the buttons while the installer runs.
    pub fn set_installing(&mut self, installing: bool) {
        self.installing = installing;
    }

    pub fn release(&self) -> &Release {
        &self.release
    }
}

impl StackDialog for UpdatePromptDialog {
    type Result = UpdateChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/update/src/dialog.rs:42 `StartupPrompt::show` (the card's content and buttons),
    // on `Dialog::new(..).stack(stack).width(Width::Standard).footer(Footer::standard()..)`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<UpdateChoice>> {
        let labels = UpdateLabels::of_ctx(ctx);
        let subject = Subject::of(ctx);
        let version = self.release.version_label();
        let message =
            labels.available_message.replace("{app}", subject.name).replace("{version}", &version).replace("{running}", &self.running_version);
        let (default, other) = if self.self_updating {
            ((UpdateChoice::UpdateAndRestart, &labels.update_and_restart), Some((UpdateChoice::Download, &labels.download)))
        } else {
            ((UpdateChoice::Download, &labels.download), None)
        };
        let enabled = !self.installing;
        let mut buttons = vec![FooterButton::new(ButtonRole::Default, default.1.as_str()).enabled(enabled)];
        if let Some((_, label)) = other {
            buttons.push(FooterButton::new(ButtonRole::Alternate, label.as_str()).enabled(enabled));
        }
        buttons.push(FooterButton::new(ButtonRole::Cancel, labels.later.as_str()));
        let footer = Footer::custom(move |ui, fx| {
            fx.buttons(ui, buttons);
        });
        let (status, installing, release) = (&self.status, self.installing, &self.release);
        let shown = Dialog::new(PROMPT_KEY, labels.available_title.as_str()).stack(stack).width(Width::Standard).footer(footer).show(
            ctx,
            &mut self.state,
            &mut (),
            |ui, ()| {
                subject.head(ui, &labels.version_line.replace("{running}", &self.running_version).replace("{version}", &version));
                ui.add_space(current(ui.ctx()).component.dialog.pad);
                if let Some(status) = status {
                    Notice::new(status.as_str()).tone(Tone::Warn).id_salt("status").show(ui);
                }
                if installing {
                    ui.horizontal(|ui| {
                        ui.add(Spinner::new());
                        ui.add(Text::new(labels.installing.as_str()));
                    });
                } else {
                    ui.add(Text::new(message.as_str()));
                }
                if !release.html_url.is_empty() {
                    ui.add(Link::new(labels.whats_new.as_str()).url(release.html_url.as_str()));
                }
            },
        );
        match shown.outcome? {
            Outcome::Ok | Outcome::Alternate if self.installing => None,
            Outcome::Ok => Some(DialogResult::Ok(default.0)),
            Outcome::Alternate => Some(DialogResult::Ok(other.map_or(UpdateChoice::Later, |(choice, _)| choice))),
            Outcome::Cancel | Outcome::Close => Some(DialogResult::Ok(UpdateChoice::Later)),
            _ => None,
        }
    }
}

/// What the user chose in the check-result dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckChoice {
    Close,
    /// Failed state only: run another check.
    CheckAgain,
}

/// The result of a manual "Check for Updates": a spinner while checking, "newest version" when
/// up to date, the error and Check Again when it failed. The app opens it when the user starts a
/// check and feeds [`set_status`](UpdateCheckDialog::set_status) each pass; an available release
/// closes it and opens an [`UpdatePromptDialog`] instead. OK, Enter, Esc and the close box close
/// it. Result: [`CheckChoice`].
pub struct UpdateCheckDialog {
    id: DialogId,
    running_version: String,
    status: Status,
    state: DialogState<()>,
}

impl UpdateCheckDialog {
    pub fn new(running_version: impl Into<String>) -> Self {
        Self { id: dialog_id(CHECK_KEY), running_version: running_version.into(), status: Status::Checking, state: DialogState::new() }
    }

    /// The updater's status this pass.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

impl StackDialog for UpdateCheckDialog {
    type Result = CheckChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/update/src/dialog.rs:323 `CheckResultDialog::show`, on `Dialog` with a
    // `Spinner` while checking and `Footer::standard().without_cancel()`; Check Again as the
    // Alternate in the failed state.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<CheckChoice>> {
        // Idle means the check ended with nothing to show (a release opened the prompt, or the
        // check was dismissed): the dialog closes instead of staying on the stack unseen.
        if self.status == Status::Idle {
            return Some(DialogResult::Ok(CheckChoice::Close));
        }
        let labels = UpdateLabels::of_ctx(ctx);
        let subject = Subject::of(ctx);
        let failed = matches!(self.status, Status::Failed(_));
        let mut footer = Footer::standard().without_cancel();
        if failed {
            footer = footer.alternate(labels.check_again.as_str());
        }
        let (status, running) = (&self.status, &self.running_version);
        let shown = Dialog::new(CHECK_KEY, labels.check_title.as_str()).stack(stack).width(Width::Standard).footer(footer).show(
            ctx,
            &mut self.state,
            &mut (),
            |ui, ()| {
                subject.head(ui, &format!("v{running}"));
                ui.add_space(current(ui.ctx()).component.dialog.pad);
                match status {
                    Status::Checking => {
                        ui.horizontal(|ui| {
                            ui.add(Spinner::new());
                            ui.add(Text::new(labels.checking.as_str()));
                        });
                    }
                    Status::UpToDate => {
                        ui.add(Text::new(labels.up_to_date.replace("{app}", subject.name).replace("{running}", running)));
                    }
                    Status::Failed(error) => {
                        Notice::new(error.as_str()).title(labels.failed.as_str()).tone(Tone::Bad).id_salt("failed").show(ui);
                        if let Some(app) = subject.app {
                            let url = app.releases_url();
                            ui.horizontal_wrapped(|ui| {
                                ui.add(Text::new(labels.releases_listed.as_str()).content(Content::Muted));
                                ui.add(Link::new(url.as_str()).url(url.as_str()));
                            });
                        }
                    }
                    Status::Idle => {}
                }
            },
        );
        match shown.outcome? {
            Outcome::Alternate => Some(DialogResult::Ok(CheckChoice::CheckAgain)),
            Outcome::Ok | Outcome::Cancel | Outcome::Close => Some(DialogResult::Ok(CheckChoice::Close)),
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "screen-testing"))]
mod tests {
    use craft_ui::dialog::{DialogHost, StartupStage};
    use craft_ui::theme::{Base, install};
    use egui::{Key, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::*;

    fn harness(host: DialogHost) -> Harness<'static, DialogHost> {
        Harness::<DialogHost>::builder()
            .with_size(vec2(640.0, 480.0))
            .with_context_setup(|ctx| {
                install(ctx, &Base::suite_default());
                craft_host::Host::builder(craft_host::AppInfo { id: "photocraft", name: "PhotoCraft", version: "1.0" }).build().install(ctx);
            })
            .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host)
    }

    fn prompt(self_updating: bool) -> UpdatePromptDialog {
        UpdatePromptDialog::new(Release { tag: "v2.0.0".into(), ..Release::default() }, "1.0.0", self_updating)
    }

    fn opened(dialog: impl StackDialog) -> Harness<'static, DialogHost> {
        let mut host = DialogHost::new();
        host.open(dialog);
        let mut harness = harness(host);
        harness.run_steps(3);
        harness
    }

    /// A startup modal that draws nothing and finishes when told to.
    struct Blocker(bool);

    impl StackDialog for Blocker {
        type Result = ();

        fn id(&self) -> DialogId {
            dialog_id("blocker")
        }

        fn show(&mut self, _ctx: &Context, _stack: &DialogStack) -> Option<DialogResult<()>> {
            self.0.then_some(DialogResult::Ok(()))
        }
    }

    // Enter takes Update and Restart when self-updating and Download otherwise; Esc is Later.
    #[test]
    fn the_default_button_depends_on_self_updating_and_esc_is_later() {
        for (self_updating, expected) in [(true, UpdateChoice::UpdateAndRestart), (false, UpdateChoice::Download)] {
            let mut harness = opened(prompt(self_updating));
            assert_eq!(harness.query_by_label("Update and Restart").is_some(), self_updating);
            harness.key_press(Key::Enter);
            harness.run_steps(3);
            assert_eq!(harness.state_mut().take::<UpdatePromptDialog>(), Some(DialogResult::Ok(expected)));
        }
        let mut harness = opened(prompt(true));
        harness.key_press(Key::Escape);
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdatePromptDialog>(), Some(DialogResult::Ok(UpdateChoice::Later)));

        let mut harness = opened(prompt(true));
        harness.state_mut().get_mut::<UpdatePromptDialog>().unwrap().set_installing(true);
        harness.run_steps(3);
        harness.key_press(Key::Enter);
        harness.run_steps(3);
        assert!(harness.state().is_open::<UpdatePromptDialog>(), "the buttons do nothing while the install runs");
        harness.key_press(Key::Escape);
        harness.run_steps(3);
        assert_eq!(
            harness.state_mut().take::<UpdatePromptDialog>(),
            Some(DialogResult::Ok(UpdateChoice::Later)),
            "Esc is still Later while installing"
        );
    }

    // Enqueued at the Update stage behind a Warning alert, the prompt opens only after the alert
    // closes (gap 75).
    #[test]
    fn the_prompt_waits_for_earlier_startup_modals() {
        let mut host = DialogHost::new();
        host.enqueue(StartupStage::Update, prompt(false));
        host.enqueue(StartupStage::Warning, Blocker(false));
        let mut harness = harness(host);
        harness.run_steps(3);
        assert!(harness.state().is_open::<Blocker>() && !harness.state().is_open::<UpdatePromptDialog>());

        harness.state_mut().get_mut::<Blocker>().unwrap().0 = true;
        harness.run_steps(3);
        harness.run_steps(3);
        assert!(harness.state().is_open::<UpdatePromptDialog>());
    }

    // The check dialog shows the spinner, then the up-to-date or failed text as the status
    // changes, and Check Again only when failed.
    #[test]
    fn the_check_dialog_follows_the_status() {
        let mut harness = opened(UpdateCheckDialog::new("1.0.0"));
        assert!(harness.query_by_label("Checking for updates…").is_some());
        assert!(harness.query_by_label("Check Again").is_none());

        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::UpToDate);
        harness.run_steps(3);
        assert!(harness.query_by_label_contains("is the newest version").is_some());
        assert!(harness.query_by_label("Checking for updates…").is_none());

        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::Failed("no network".into()));
        harness.run_steps(3);
        assert!(harness.query_all_by_label_contains("no network").next().is_some());
        assert!(harness.query_by_label("All releases are listed at").is_some());
        assert!(harness.query_by_label("https://github.com/storytold/photocraft/releases").is_some());
        harness.get_by_label("Check Again").click();
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
    }

    // Idle means the check ended: the dialog closes instead of staying open and unseen.
    #[test]
    fn an_idle_check_dialog_closes() {
        let mut harness = opened(UpdateCheckDialog::new("1.0.0"));
        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::Idle);
        harness.run_steps(3);
        assert!(!harness.state().is_open::<UpdateCheckDialog>());
    }
}
