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
use crate::self_update;
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
        check_now: "update.checkNow" = "Check Now";
        not_checked: "update.notChecked" = "Updates have not been checked yet.";
        cannot_auto_update: "update.cannotAutoUpdate" = "Cannot auto-update this installation type. Please download manually.";
        /// The check dialog's button that opens the release notes in its available state.
        release_notes: "update.releaseNotes" = "Release Notes";
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

    /// The release the dialog offers.
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
    /// Close the dialog.
    Close,
    /// Not-checked state only: run the first check.
    CheckNow,
    /// Every state but checking: run another check.
    CheckAgain,
    /// Available state only: install the release and restart (the default where the install can
    /// update itself); the app calls `Updater::request_install`, which works for any release the updater knows of.
    Update,
    /// Available state only: open the release page in the browser (the default where the install
    /// can't update itself).
    Download,
    /// Available state only: open the release notes. The dialog opens the page itself and stays
    /// open, so this is never a result; it exists for [`UpdateCheckDialog::set_choice`].
    ReleaseNotes,
}

/// The result of a manual "Check for Updates": "not checked yet" with Check Now before the first
/// check, a spinner while checking, "newest version" when up to date and the error when it
/// failed, each with OK or Close and Check Again. The app opens it when the user starts a
/// check and feeds [`set_status`](UpdateCheckDialog::set_status) each pass; an available release
/// closes it and opens an [`UpdatePromptDialog`] instead, unless the app calls
/// [`set_available`](UpdateCheckDialog::set_available): the dialog then shows the "update
/// available" state itself, with Update and Restart (the default), Release Notes and Check Again
/// (while [`set_installing`](UpdateCheckDialog::set_installing) the install button is disabled
/// and the progress line shows). Where the install can't update itself the dialog says so from
/// the start and Download replaces Update and Restart. Enter takes the default; Esc and the
/// close box close it. A host answers with [`set_choice`](UpdateCheckDialog::set_choice) and
/// `DialogHost::confirm_by_id`. Result: [`CheckChoice`].
pub struct UpdateCheckDialog {
    id: DialogId,
    running_version: String,
    status: Status,
    available: Option<Release>,
    /// Whether this install can update itself: what the app set, else craft-update's detection
    /// for the Host's app, made on the first pass.
    self_updating: Option<bool>,
    installing: bool,
    message: Option<String>,
    /// What a programmatic answer gives; the state's default button's until set.
    chosen: Option<CheckChoice>,
    state: DialogState<()>,
}

impl UpdateCheckDialog {
    /// A dialog in the checking state for `running_version`.
    pub fn new(running_version: impl Into<String>) -> Self {
        Self {
            id: dialog_id(CHECK_KEY),
            running_version: running_version.into(),
            status: Status::Checking,
            available: None,
            self_updating: None,
            installing: false,
            message: None,
            chosen: None,
            state: DialogState::new(),
        }
    }

    /// Sets whether the install can update itself, for an app that knows better than
    /// [`self_update::is_self_updating`] for the Host's app (the dialog asks that when this isn't
    /// called). Without it Download replaces Update and Restart.
    pub fn with_self_updating(mut self, self_updating: bool) -> Self {
        self.self_updating = Some(self_updating);
        self
    }

    /// The updater's status this pass.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// Shows the "update available" state for `release`; `None` returns to the state of the
    /// [`status`](UpdateCheckDialog::set_status).
    pub fn set_available(&mut self, release: Option<Release>) {
        self.available = release;
    }

    /// Disables the install buttons while the installer runs (`Updater::is_installing`).
    pub fn set_installing(&mut self, installing: bool) {
        self.installing = installing;
    }

    /// The install's progress or error line in the available state (`Updater::status_message`).
    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    /// The answer a host-side `DialogHost::confirm_by_id` gives, as pressing that button would;
    /// the state's default button until set. A choice the state doesn't offer, an install
    /// choice while installing, and [`ReleaseNotes`](CheckChoice::ReleaseNotes) (which opens the
    /// page and leaves the dialog open) give no answer.
    pub fn set_choice(&mut self, choice: CheckChoice) {
        self.chosen = Some(choice);
    }

    /// The release the available state offers, when it can be opened.
    fn notes_url(&self) -> Option<&str> {
        self.available.as_ref().map(|release| release.html_url.as_str()).filter(|url| crate::release::safe_web_link(url))
    }

    /// The choices the current state offers, the default first.
    fn offered(&self, self_updating: bool) -> Vec<CheckChoice> {
        if self.available.is_some() {
            let mut offered = vec![if self_updating { CheckChoice::Update } else { CheckChoice::Download }];
            if self.notes_url().is_some() {
                offered.push(CheckChoice::ReleaseNotes);
            }
            offered.push(CheckChoice::CheckAgain);
            return offered;
        }
        match self.status {
            Status::Idle => vec![CheckChoice::CheckNow, CheckChoice::Close],
            Status::Checking => vec![CheckChoice::Close],
            Status::UpToDate | Status::Failed(_) => vec![CheckChoice::Close, CheckChoice::CheckAgain],
        }
    }

    /// Whether the install can update itself; asked once, on the first pass.
    fn resolve_self_updating(&mut self, subject: &Subject) -> bool {
        *self.self_updating.get_or_insert_with(|| subject.app.is_some_and(self_update::is_self_updating))
    }
}

impl StackDialog for UpdateCheckDialog {
    type Result = CheckChoice;

    fn id(&self) -> DialogId {
        self.id
    }

    fn confirm(&mut self, ctx: &Context) -> Option<CheckChoice> {
        let self_updating = self.resolve_self_updating(&Subject::of(ctx));
        let offered = self.offered(self_updating);
        let choice = self.chosen.unwrap_or(offered[0]);
        if !offered.contains(&choice) {
            return None;
        }
        match choice {
            CheckChoice::ReleaseNotes => {
                if let Some(url) = self.notes_url() {
                    ctx.open_url(egui::OpenUrl::new_tab(url));
                }
                None
            }
            CheckChoice::Update | CheckChoice::Download if self.installing => None,
            choice => Some(choice),
        }
    }

    // pc: crates/update/src/dialog.rs:323 `CheckResultDialog::show`, on `Dialog` with a
    // `Spinner` while checking and `Footer::standard().without_cancel()`; Check Again as the
    // Alternate in every state but checking. The states and buttons follow
    // crates/ui-egui/src/updates_ui.rs:131-190.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<CheckChoice>> {
        let labels = UpdateLabels::of_ctx(ctx);
        let subject = Subject::of(ctx);
        let self_updating = self.resolve_self_updating(&subject);
        if self.available.is_some() {
            return self.show_available(ctx, stack, &labels, &subject, self_updating);
        }
        let mut footer = Footer::standard().without_cancel();
        footer = match self.status {
            Status::Idle => footer.ok(labels.check_now.as_str()),
            Status::Checking => footer,
            Status::UpToDate | Status::Failed(_) => footer.alternate(labels.check_again.as_str()),
        };
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
                    Status::Idle => {
                        ui.add(Text::new(labels.not_checked.as_str()));
                    }
                }
            },
        );
        let idle = self.status == Status::Idle;
        match shown.outcome? {
            Outcome::Ok if idle => Some(DialogResult::Ok(CheckChoice::CheckNow)),
            Outcome::Alternate => Some(DialogResult::Ok(CheckChoice::CheckAgain)),
            Outcome::Ok | Outcome::Cancel | Outcome::Close => Some(DialogResult::Ok(CheckChoice::Close)),
            _ => None,
        }
    }
}

impl UpdateCheckDialog {
    /// The available state: the prompt's content under the check dialog's title, with Release
    /// Notes (which keeps the dialog open) and Check Again next to the install button.
    fn show_available(
        &mut self,
        ctx: &Context,
        stack: &DialogStack,
        labels: &UpdateLabels,
        subject: &Subject,
        self_updating: bool,
    ) -> Option<DialogResult<CheckChoice>> {
        let version = self.available.as_ref()?.version_label();
        let message =
            labels.available_message.replace("{app}", subject.name).replace("{version}", &version).replace("{running}", &self.running_version);
        let (default, default_label) =
            if self_updating { (CheckChoice::Update, &labels.update_and_restart) } else { (CheckChoice::Download, &labels.download) };
        let notes = self.notes_url().map(str::to_owned);
        let mut buttons = vec![FooterButton::new(ButtonRole::Default, default_label.as_str()).enabled(!self.installing)];
        if notes.is_some() {
            buttons.push(FooterButton::new(ButtonRole::Apply, labels.release_notes.as_str()));
        }
        buttons.push(FooterButton::new(ButtonRole::Alternate, labels.check_again.as_str()).enabled(!self.installing));
        let footer = Footer::custom(move |ui, fx| {
            fx.buttons(ui, buttons);
        });
        let (installing, status, running) = (self.installing, &self.message, &self.running_version);
        let shown = Dialog::new(CHECK_KEY, labels.check_title.as_str()).stack(stack).width(Width::Standard).footer(footer).show(
            ctx,
            &mut self.state,
            &mut (),
            |ui, ()| {
                subject.head(ui, &labels.version_line.replace("{running}", running).replace("{version}", &version));
                ui.add_space(current(ui.ctx()).component.dialog.pad);
                if !self_updating {
                    Notice::new(labels.cannot_auto_update.as_str()).tone(Tone::Warn).id_salt("cannot-update").show(ui);
                }
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
            },
        );
        match shown.outcome? {
            Outcome::Ok | Outcome::Alternate if self.installing => None,
            Outcome::Ok => Some(DialogResult::Ok(default)),
            Outcome::Alternate => Some(DialogResult::Ok(CheckChoice::CheckAgain)),
            Outcome::Apply => {
                if let Some(url) = notes {
                    ctx.open_url(egui::OpenUrl::new_tab(url));
                }
                None
            }
            Outcome::Cancel | Outcome::Close => Some(DialogResult::Ok(CheckChoice::Close)),
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

    fn available(self_updating: bool) -> UpdateCheckDialog {
        let release = Release { tag: "v2.0.0".into(), html_url: "https://example.com/r".into(), ..Release::default() };
        let mut dialog = UpdateCheckDialog::new("1.0.0").with_self_updating(self_updating);
        dialog.set_status(Status::UpToDate);
        dialog.set_available(Some(release));
        dialog
    }

    // The available state shows Release Notes as a button that keeps the dialog open; Enter
    // updates where the install can, Esc closes, and the install buttons are off while installing.
    #[test]
    fn the_check_dialog_can_show_an_available_update() {
        let mut harness = opened(available(true));
        assert!(harness.query_by_label_contains("is available").is_some());
        assert!(harness.query_by_label("Update and Restart").is_some());
        assert!(harness.query_by_label("Check Again").is_some());
        assert!(harness.query_by_label("Later").is_none());
        assert!(harness.query_by_label("Cannot auto-update this installation type. Please download manually.").is_none());
        harness.get_by_label("Release Notes").click();
        harness.run_steps(3);
        assert!(harness.state().is_open::<UpdateCheckDialog>(), "Release Notes keeps the dialog open");
        harness.key_press(Key::Enter);
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Update)));

        let mut harness = opened(available(false));
        harness.key_press(Key::Enter);
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Download)));

        let mut harness = opened(available(true));
        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_installing(true);
        harness.run_steps(3);
        harness.key_press(Key::Enter);
        harness.run_steps(3);
        assert!(harness.state().is_open::<UpdateCheckDialog>(), "Enter does nothing while installing");
        harness.key_press(Key::Escape);
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Close)));
    }

    // An install that can't update itself says so as soon as the dialog opens, with Download as
    // the default and no Update and Restart.
    #[test]
    fn an_install_that_cannot_update_itself_says_so_at_open() {
        let harness = opened(available(false));
        assert!(harness.query_all_by_label_contains("Cannot auto-update this installation type").next().is_some());
        assert!(harness.query_by_label("Download").is_some());
        assert!(harness.query_by_label("Update and Restart").is_none());
    }

    // Without an override the dialog asks craft-update about the Host's app; a cargo build
    // output is not an install that updates itself.
    #[test]
    fn detection_decides_when_the_app_does_not_say() {
        let mut dialog = UpdateCheckDialog::new("1.0.0");
        dialog.set_available(Some(Release { tag: "v2.0.0".into(), ..Release::default() }));
        let harness = opened(dialog);
        assert!(harness.query_all_by_label_contains("Cannot auto-update this installation type").next().is_some());
    }

    // The host answers the open dialog with any of its buttons, as StandardPrompt's set_choice does.
    #[test]
    fn the_host_answers_with_any_button() {
        let ctx = Context::default();
        let answer = |dialog: UpdateCheckDialog, choice: Option<CheckChoice>| {
            let mut host = DialogHost::new();
            let id = dialog.id();
            host.open(dialog);
            if let Some(choice) = choice {
                host.get_mut_by_id::<UpdateCheckDialog>(id).unwrap().set_choice(choice);
            }
            let confirmed = host.confirm_by_id::<UpdateCheckDialog>(&ctx, id);
            confirmed.then(|| host.take::<UpdateCheckDialog>()).flatten()
        };
        assert_eq!(answer(available(true), None), Some(DialogResult::Ok(CheckChoice::Update)));
        assert_eq!(answer(available(false), None), Some(DialogResult::Ok(CheckChoice::Download)));
        assert_eq!(answer(available(true), Some(CheckChoice::CheckAgain)), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
        assert_eq!(answer(available(true), Some(CheckChoice::Download)), None, "Download is not offered where the install updates itself");
        assert_eq!(answer(available(true), Some(CheckChoice::ReleaseNotes)), None, "Release Notes opens the page and stays open");
        let mut installing = available(true);
        installing.set_installing(true);
        assert_eq!(answer(installing, None), None);
        let mut failed = UpdateCheckDialog::new("1.0.0");
        failed.set_status(Status::Failed("no network".into()));
        assert_eq!(answer(failed, Some(CheckChoice::CheckAgain)), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
    }

    // The check dialog shows the spinner, then the up-to-date or failed text as the status
    // changes; Check Again is offered in every state but checking.
    #[test]
    fn the_check_dialog_follows_the_status() {
        let mut harness = opened(UpdateCheckDialog::new("1.0.0"));
        assert!(harness.query_by_label("Checking for updates…").is_some());
        assert!(harness.query_by_label("Check Again").is_none());

        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::UpToDate);
        harness.run_steps(3);
        assert!(harness.query_by_label_contains("is the newest version").is_some());
        assert!(harness.query_by_label("Checking for updates…").is_none());
        assert!(harness.query_by_label("Check Again").is_some());
        assert!(harness.query_by_label("OK").is_some());

        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::Failed("no network".into()));
        harness.run_steps(3);
        assert!(harness.query_all_by_label_contains("no network").next().is_some());
        assert!(harness.query_by_label("All releases are listed at").is_some());
        assert!(harness.query_by_label("https://github.com/storytold/photocraft/releases").is_some());
        harness.get_by_label("Check Again").click();
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
    }

    // Idle is the "not checked yet" state: it stays open and Check Now (Enter) starts the check.
    #[test]
    fn an_idle_check_dialog_offers_check_now() {
        let mut harness = opened(UpdateCheckDialog::new("1.0.0"));
        harness.state_mut().get_mut::<UpdateCheckDialog>().unwrap().set_status(Status::Idle);
        harness.run_steps(3);
        assert!(harness.query_by_label("Updates have not been checked yet.").is_some());
        assert!(harness.query_by_label("Check Again").is_none());
        harness.key_press(Key::Enter);
        harness.run_steps(3);
        assert_eq!(harness.state_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::CheckNow)));
    }
}
