//! The update flow of an egui app in one place: a check at start or on request, the startup prompt,
//! and the self-update worker. An app creates one [`Updater`], calls [`Updater::tick`] and
//! [`Updater::run`] each frame before it runs its `DialogHost`, and acts on the [`Action`] it returns,
//! so every app gets the same handling of failures, dropped workers, repaints and quitting.
//!
//! An app the launcher keeps on another version (see [`crate::pins`]) offers nothing itself: the
//! check reads the shared choice first, and a requested check reports [`Status::UpToDate`] while
//! [`Updater::managed`] says why.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use craft_ui::dialog::{DialogHost, StartupStage, dialog_id};
use craft_ui::runtime::frame::request_repaint_after;

use crate::pins::{Choice, Pins};
use crate::release::Release;
use crate::stack_dialogs::{UpdateChoice, UpdatePromptDialog};
use crate::translate::{Note, Translate, english};
use crate::{App, Http, Paths, Platform, Stop, check_for_update, self_update};

/// Asks for the newest release (blocking; it runs on its own thread). Tests stand in for the network
/// with one; without it the check asks GitHub.
pub type CheckSource = Arc<dyn Fn() -> Result<Option<Release>, String> + Send + Sync>;

/// What a requested check came to, for a window that answers Help ▸ Check for Updates.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// Nothing to report.
    Idle,
    /// A requested check is running.
    Checking,
    /// A requested check found nothing newer.
    UpToDate,
    /// A requested check failed.
    Failed(String),
}

/// What the app should do after [`Updater::run`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Action {
    /// Nothing for the app to do this frame.
    None,
    /// Open this page in the browser.
    OpenUrl(String),
    /// Close the app through its normal close path (which asks about unsaved work): the update is
    /// installed and a helper restarts the app once it has exited.
    Close,
}

type Checked = Result<Option<Release>, String>;

/// The seconds between passes while a check or an installation runs; the poll is what picks up a finished worker.
const POLL_SECONDS: f64 = 0.1;

/// The line under the prompt's text: the installer's own words, or one of the updater's, translated each pass.
#[derive(Debug)]
enum Message {
    Installer(String),
    Note(Note),
}

/// The update prompt the updater holds for the app's dialog host.
#[derive(Debug)]
struct Offer {
    release: Release,
    /// The prompt is on the host's startup queue or open.
    shown: bool,
    message: Option<Message>,
}

impl Offer {
    fn new(release: Release) -> Self {
        Self { release, shown: false, message: None }
    }

    fn text(&self, translate: Translate) -> Option<String> {
        self.message.as_ref().map(|message| match message {
            Message::Installer(text) => text.clone(),
            Message::Note(note) => note.translated(translate),
        })
    }
}

/// Why an installation failed: the installer's own words, shown as they are, or the updater's,
/// shown in the interface language.
#[derive(Debug)]
enum Failure {
    Installer(String),
    Updater(Note),
}

impl Failure {
    /// The worker panicked.
    fn panicked() -> Self {
        Self::Updater(Note::new("The update failed unexpectedly.", &[]))
    }

    /// The worker went away without an answer.
    fn dropped() -> Self {
        Self::Updater(Note::new("The update stopped unexpectedly.", &[]))
    }
}

impl From<String> for Failure {
    fn from(e: String) -> Self {
        Self::Installer(e)
    }
}

impl From<&str> for Failure {
    fn from(e: &str) -> Self {
        Self::Installer(e.to_owned())
    }
}

/// One app's update flow: the check, the startup prompt and the self-update worker. See the
/// module docs.
pub struct Updater {
    app: &'static App,
    version: String,
    source: Option<CheckSource>,
    status: Status,
    /// The check running (or queued behind it) was asked for by the user.
    requested: bool,
    started: bool,
    check: Option<Receiver<Checked>>,
    install: Option<(Receiver<Result<(), Failure>>, Release)>,
    prompt: Option<Offer>,
    /// A prompt the updater dropped is still on the host; it is withdrawn when it shows.
    stale: bool,
    /// The newest release a check found or the app offered; [`Updater::request_install`] works
    /// from it after the prompt closed.
    known: Option<Release>,
    restart: bool,
    /// An update is installed and waits for the app to exit; nothing starts a second one.
    installed: bool,
    /// The version-choice file to read in place of [`Pins::default_path`] (tests).
    pins_path: Option<PathBuf>,
    /// The launcher's choice for this app when it is not stable, as of the last check or offer.
    managed: Option<Choice>,
    /// Puts the prompt and its messages in the interface language ([`crate::translate`]).
    translate: Translate,
}

impl Updater {
    /// An updater for `app`, running `version`.
    pub fn new(app: &'static App, version: impl Into<String>) -> Self {
        let version = version.into();
        let version = self_update::current_version(app, &version).map_or(version, |v| v.to_string());
        Self {
            app,
            version,
            source: None,
            status: Status::Idle,
            requested: false,
            started: false,
            check: None,
            install: None,
            prompt: None,
            stale: false,
            known: None,
            restart: false,
            installed: false,
            pins_path: None,
            managed: None,
            translate: english,
        }
    }

    /// Words the messages under the startup prompt with `translate`, in the app's interface language
    /// ([`crate::translate`]). Errors from the network stay as they came. The prompt's own words come from the
    /// app's `UpdateWords`.
    #[must_use]
    pub fn with_translator(mut self, translate: Translate) -> Self {
        self.translate = translate;
        self
    }

    /// "The update is installed…", translated when the prompt shows it.
    fn installed_note(&self) -> Note {
        Note::new("The update is installed. Close {app} to finish; it restarts by itself.", &[("app", self.app.name)])
    }

    /// Drops the prompt; one already on the host is withdrawn on a later [`Updater::run`].
    fn drop_prompt(&mut self) {
        if self.prompt.take().is_some_and(|offer| offer.shown) {
            self.stale = true;
        }
    }

    /// Replaces the network with `source` (tests).
    pub fn with_source(mut self, source: CheckSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Reads the version choices from `path` instead of the launcher's file (tests).
    pub fn with_pins_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.pins_path = Some(path.into());
        self
    }

    /// The launcher's choice for this app when it keeps it off stable releases (a pinned version,
    /// nightlies, or a choice this version cannot read), as of the last check. A window answering
    /// Help ▸ Check for Updates can say so instead of "newest version".
    pub fn managed(&self) -> Option<&Choice> {
        self.managed.as_ref()
    }

    /// Reads the launcher's choice for this app; remembers it when it is not stable.
    fn read_choice(&mut self) -> bool {
        let pins = match &self.pins_path {
            Some(path) => Pins::load(path),
            None => Pins::load_default(),
        };
        self.managed = Some(pins.choice(self.app.id)).filter(|c| *c != Choice::Stable);
        if let Some(choice) = &self.managed {
            log::info!("{} follows the launcher's choice ({choice:?}); no update offered here", self.app.name);
            // A prompt opened before the pin would still install the stable release. One that is
            // installing or finished stays: `tick` reports through it.
            if self.install.is_none() && !self.installed {
                self.drop_prompt();
            }
        }
        self.managed.is_some()
    }

    /// The check at start, once. Later calls do nothing.
    pub fn check_at_start(&mut self) {
        if !std::mem::replace(&mut self.started, true) {
            self.begin(false);
        }
    }

    /// Help ▸ Check for Updates: a check whose outcome [`Self::status`] reports. A check already
    /// running becomes a requested one; nothing starts while an update installs.
    pub fn check_now(&mut self) {
        self.started = true;
        self.begin(true);
    }

    /// Opens the startup prompt for a release the app found itself.
    pub fn offer(&mut self, release: Release) {
        self.known = Some(release.clone());
        if self.install.is_none() && !self.installed && !self.read_choice() {
            self.drop_prompt();
            self.prompt = Some(Offer::new(release));
        }
    }

    /// What the last requested check came to.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Dismisses the outcome of a requested check. One still running carries on, and reports
    /// like the check at start (a prompt if there is a release, otherwise nothing).
    pub fn dismiss_status(&mut self) {
        self.requested = false;
        self.status = Status::Idle;
    }

    /// Whether an installation is running.
    pub fn is_installing(&self) -> bool {
        self.install.is_some()
    }

    /// The prompt's status line in the interface language: the installer's error, or one of the
    /// updater's own notes ("Save or close your open work first"). `None` when there is none or
    /// no prompt is open.
    pub fn status_message(&self) -> Option<String> {
        self.prompt.as_ref().and_then(|offer| offer.text(self.translate))
    }

    /// Whether the startup prompt is showing.
    pub fn prompt_open(&self) -> bool {
        self.prompt.is_some()
    }

    fn begin(&mut self, requested: bool) {
        if self.install.is_some() || self.installed {
            return;
        }
        if requested {
            self.requested = true;
            self.status = Status::Checking;
        }
        if self.check.is_some() {
            return;
        }
        // The file is a few bytes, read once per check; the launcher may have changed it since.
        if self.read_choice() {
            self.finish_check(Ok(None));
            return;
        }
        let (tx, rx) = channel();
        let (source, app, version) = (self.source.clone(), self.app, self.version.clone());
        let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let ask = || match source {
                Some(ask) => ask(),
                None => Http::new(&format!("{}/Update", app.name)).and_then(|http| check_for_update(app, &http, &version)),
            };
            let answer =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(ask)).unwrap_or_else(|_| Err("the update check failed unexpectedly".into()));
            let _ = tx.send(answer);
        });
        match spawned {
            Ok(_) => self.check = Some(rx),
            Err(e) => self.finish_check(Err(format!("couldn't start the update check ({e})"))),
        }
    }

    fn finish_check(&mut self, answer: Checked) {
        let requested = std::mem::take(&mut self.requested);
        // The launcher may have pinned or unpinned the app while the check ran: whatever the
        // answer, an app it manages offers nothing and reports nothing to do.
        if self.read_choice() {
            if let Err(e) = &answer {
                log::info!("{} update check failed ({e}), but the launcher manages its version", self.app.name);
            }
            self.status = if requested { Status::UpToDate } else { Status::Idle };
            return;
        }
        match answer {
            Ok(Some(release)) => {
                self.status = Status::Idle;
                self.known = Some(release.clone());
                if self.prompt.is_none() && self.install.is_none() && !self.installed {
                    self.prompt = Some(Offer::new(release));
                }
            }
            Ok(None) => self.status = if requested { Status::UpToDate } else { Status::Idle },
            Err(e) => {
                log::warn!("{} update check failed: {e}", self.app.name);
                self.status = if requested { Status::Failed(e) } else { Status::Idle };
            }
        }
    }

    /// Picks up a finished check or installation. Call once per frame, before [`Self::run`]. It
    /// asks for the next pass every [`POLL_SECONDS`] while either is running.
    pub fn tick(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.check {
            match rx.try_recv() {
                Ok(answer) => {
                    self.check = None;
                    self.finish_check(answer);
                }
                Err(TryRecvError::Empty) => request_repaint_after(ctx, POLL_SECONDS),
                Err(TryRecvError::Disconnected) => {
                    self.check = None;
                    self.finish_check(Err("the update check stopped unexpectedly".into()));
                }
            }
        }
        if let Some((rx, _)) = &self.install {
            let outcome = match rx.try_recv() {
                Ok(outcome) => Some(outcome),
                Err(TryRecvError::Empty) => {
                    request_repaint_after(ctx, POLL_SECONDS);
                    None
                }
                Err(TryRecvError::Disconnected) => Some(Err(Failure::dropped())),
            };
            if let Some(outcome) = outcome {
                let release = self.install.take().map(|(_, release)| release);
                match outcome {
                    Ok(()) => {
                        self.installed = true;
                        self.restart = true;
                        let note = self.installed_note();
                        if let Some(offer) = &mut self.prompt {
                            offer.message = Some(Message::Note(note));
                        }
                    }
                    Err(failure) => {
                        match &failure {
                            Failure::Installer(e) => log::warn!("{} self-update failed: {e}", self.app.name),
                            Failure::Updater(note) => log::warn!("{} self-update failed: {}", self.app.name, note.translated(english)),
                        }
                        // The prompt may have been dismissed meanwhile: bring it back with the reason.
                        if self.prompt.is_none()
                            && let Some(release) = release
                        {
                            self.prompt = Some(Offer::new(release));
                        }
                        if let Some(offer) = &mut self.prompt {
                            offer.message = Some(match failure {
                                Failure::Installer(e) => Message::Installer(e),
                                Failure::Updater(note) => Message::Note(note),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Whether the app should close now so the installed update can restart it, without showing
    /// the startup prompt (a host that shows the progress in its own dialog calls this instead of
    /// [`run`](Self::run)). True once per finished install, and only with `can_start`
    /// true: while the app holds unsaved work the close waits and is asked again.
    pub fn take_close(&mut self, can_start: bool) -> bool {
        let close = self.restart && can_start;
        if close {
            self.restart = false;
        }
        close
    }

    /// Puts the startup prompt on `dialogs` at [`StartupStage::Update`] when one is due, feeds the open one the
    /// install's progress, and starts what the user chose. An installation only starts when `can_start` is true:
    /// an app passes `false` while it holds unsaved work, because the restart helper is scheduled before the app
    /// closes, and a close that the user then cancels would leave it waiting for an exit that never comes. The app
    /// calls this before `DialogHost::run`.
    pub fn run(&mut self, dialogs: &mut DialogHost, can_start: bool) -> Action {
        // The close waits for the app to hold no unsaved work: the user may have edited while the
        // update installed, and a close they cancel is asked again once the work is saved.
        if self.take_close(can_start) {
            return Action::Close;
        }
        // Update and Restart leaves the prompt open to show the progress; the pick comes through the dialog.
        if self.prompt.is_some() && dialogs.get_mut_by_id::<UpdatePromptDialog>(prompt_id()).is_some_and(UpdatePromptDialog::take_update_request) {
            self.request_install(can_start);
        }
        let mut result = dialogs.take::<UpdatePromptDialog>();
        if self.stale {
            if let Some(dialog) = dialogs.get_mut_by_id::<UpdatePromptDialog>(prompt_id()) {
                dialog.withdraw();
            }
            self.stale = result.is_none() && self.stale;
            result = None;
        }
        let mut action = Action::None;
        if let (Some(result), Some(offer)) = (result, &mut self.prompt) {
            offer.shown = false;
            match result.ok().unwrap_or(UpdateChoice::Later) {
                UpdateChoice::UpdateAndRestart => {
                    self.request_install(can_start);
                }
                UpdateChoice::Download => {
                    action = download_action(self.app, &offer.release);
                    self.prompt = None;
                }
                UpdateChoice::Later => {
                    // An installation carries on and reports through `tick`.
                    self.prompt = None;
                }
            }
        }
        self.sync(dialogs);
        action
    }

    /// Enqueues the prompt when it is not on the host, else shows it the install's progress.
    fn sync(&mut self, dialogs: &mut DialogHost) {
        let Some(offer) = &self.prompt else { return };
        let (installing, message) = (self.install.is_some(), offer.text(self.translate));
        if offer.shown {
            if let Some(dialog) = dialogs.get_mut_by_id::<UpdatePromptDialog>(prompt_id()) {
                dialog.set_installing(installing);
                dialog.set_status(message);
            }
        } else if !self.stale {
            let mut dialog = UpdatePromptDialog::new(offer.release.clone(), self.version.clone(), self_update::is_self_updating(self.app));
            dialog.set_installing(installing);
            dialog.set_status(message);
            dialogs.enqueue(StartupStage::Update, dialog);
            if let Some(offer) = &mut self.prompt {
                offer.shown = true;
            }
        }
    }

    /// Starts installing the newest release the updater knows of, as the prompt's Update and
    /// Restart does: the one a check found or [`offer`](Self::offer) was given, whether or not
    /// the prompt is still open (a closed one is made again to hold the progress). Returns false
    /// when it knows of no release (or the launcher manages the version), and then nothing
    /// happens. With `can_start` false (the app holds unsaved work) nothing starts and
    /// [`status_message`](Self::status_message) asks the user to save first. Progress and failures
    /// show through [`is_installing`](Self::is_installing) and `status_message`, and a finished
    /// install ends in [`Action::Close`] from [`run`](Self::run).
    pub fn request_install(&mut self, can_start: bool) -> bool {
        if self.prompt.is_none()
            && let Some(release) = self.known.clone()
            && !self.read_choice()
        {
            self.prompt = Some(Offer::new(release));
        }
        if self.prompt.is_none() {
            return false;
        }
        if can_start {
            self.start_install();
        } else if let Some(offer) = &mut self.prompt {
            offer.message = Some(Message::Note(Note::new("Save or close your open work first; the update restarts the app.", &[])));
        }
        true
    }

    fn start_install(&mut self) {
        // The launcher may have pinned the app while the prompt was open; that closes it.
        if self.install.is_none() && !self.installed && self.read_choice() {
            return;
        }
        let installed_note = self.installed_note();
        let Some(offer) = &mut self.prompt else { return };
        if self.installed {
            offer.message = Some(Message::Note(installed_note));
            return;
        }
        let Some(installed) = self_update::current(self.app) else {
            offer.message = Some(Message::Note(Note::new("Cannot auto-update this installation type. Please download manually.", &[])));
            return;
        };
        offer.message = None;
        let (tx, rx) = channel();
        let (app, version, release) = (self.app, self.version.clone(), offer.release.clone());
        let worker_release = release.clone();
        let spawned = std::thread::Builder::new().name("self-update".into()).spawn(move || {
            let run = || install(app, &installed, worker_release, &version).map_err(Failure::Installer);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| Err(Failure::panicked()));
            let _ = tx.send(outcome);
        });
        match spawned {
            Ok(_) => self.install = Some((rx, release)),
            Err(e) => {
                offer.message = Some(Message::Note(Note::new("Couldn't start the update ({error})", &[("error", &e.to_string())])));
            }
        }
    }
}

/// The id of the update prompt on the host.
fn prompt_id() -> craft_ui::dialog::DialogId {
    dialog_id("updateDialog.prompt")
}

#[cfg(feature = "screen-testing")]
impl Updater {
    /// Ticks until the running check has reported (tests).
    pub fn settle_for_tests(&mut self, ctx: &egui::Context) {
        use std::time::Duration;
        for _ in 0..2000 {
            if self.check.is_none() {
                break;
            }
            self.tick(ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn install(app: &'static App, installed: &self_update::Install, mut release: Release, version: &str) -> Result<(), String> {
    let http = Http::new(&format!("{}/Update", app.name)).map_err(|e| format!("could not initialize HTTP: {e}"))?;
    if release.assets.is_empty()
        && let Some(full) = check_for_update(app, &http, version).map_err(|e| format!("could not fetch release assets: {e}"))?
    {
        release = full;
    }
    let paths = Paths::for_app(app).map_err(|e| format!("could not find paths: {e}"))?;
    let platform = Platform::current().ok_or("unsupported platform for self update")?;
    self_update::update(app, installed, &release, platform, &http, &paths, &Stop::default(), &mut |_| {}, &|| Ok(()), Some(version))
}

/// Download in the prompt: the release page, when it is a safe web link (an app may build its
/// `Release` from its own feed or cache, so the link isn't always one GitHub's parser checked).
/// A link that fails the check falls back to the app's releases page: the prompt has closed, so
/// Download must still open something.
pub fn download_action(app: &App, release: &Release) -> Action {
    let safe = crate::release::safe_web_link;
    if safe(&release.html_url) {
        return Action::OpenUrl(release.html_url.clone());
    }
    let releases = app.releases_url();
    if safe(&releases) { Action::OpenUrl(releases) } else { Action::None }
}
