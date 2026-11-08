//! The update dialogs on the dialog host and the `Updater` that feeds them: defaults and keys, startup order,
//! the check dialog's states, the install flow and the launcher's version choice.

use craft_host::{AppInfo, Host};
use craft_ui::dialog::{DialogHost, DialogId, DialogResult, DialogStack, StackDialog, StartupStage, dialog_id};
use craft_ui::testing::{Fixture, trial};
use craft_update::pins::{Choice, Pins, VersionTrack};
use craft_update::release::{Release, safe_web_link};
use craft_update::stack_dialogs::{CheckChoice, UpdateCheckDialog, UpdateChoice, UpdatePromptDialog, icon_image};
use craft_update::suite::{LAUNCHER, PHOTOCRAFT, SUITE};
use craft_update::updater::{Action, CheckSource, Status, TestInstall, Updater, download_action};
use craft_update::{App, Arch, Http, Os, Paths, Platform, Stop, self_update};
use egui::{Key, Modifiers, Ui};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

type Dialogs = Rc<RefCell<DialogHost>>;
type Checked = Result<Option<Release>, String>;

fn newer() -> Release {
    Release { tag: "v9.0.0".into(), version: craft_update::parse_version("9.0.0"), ..Release::default() }
}

/// A fixture running `dialogs` every pass for an app named PhotoCraft.
fn rig(dialogs: DialogHost) -> (Fixture, Dialogs) {
    let dialogs: Dialogs = Rc::new(RefCell::new(dialogs));
    let shown = dialogs.clone();
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.host(Arc::new(Host::builder(AppInfo { id: "photocraft", name: "PhotoCraft", version: "1.0" }).build()));
    fixture.settle();
    (fixture, dialogs)
}

fn opened(dialog: impl StackDialog) -> (Fixture, Dialogs) {
    let mut dialogs = DialogHost::new();
    dialogs.open(dialog);
    let (mut fixture, dialogs) = rig(dialogs);
    fixture.idle(3);
    (fixture, dialogs)
}

fn painted(fixture: &Fixture, needle: &str) -> bool {
    fixture.texts().iter().any(|text| text.text.contains(needle))
}

fn press(fixture: &mut Fixture, key: Key) {
    fixture.key(key, Modifiers::NONE);
    fixture.idle(3);
}

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("no `{label}` on screen"));
    fixture.click(&target);
    fixture.idle(3);
}

fn prompt(self_updating: bool) -> UpdatePromptDialog {
    UpdatePromptDialog::new(Release { tag: "v2.0.0".into(), ..Release::default() }, "1.0.0", self_updating)
}

fn available(self_updating: bool) -> UpdateCheckDialog {
    let release = Release { tag: "v2.0.0".into(), html_url: "https://example.com/r".into(), ..Release::default() };
    let mut dialog = UpdateCheckDialog::new("1.0.0").with_self_updating(self_updating);
    dialog.set_status(Status::UpToDate);
    dialog.set_available(Some(release));
    dialog
}

/// A startup modal that draws nothing and finishes when told to.
struct Blocker(bool);

impl StackDialog for Blocker {
    type Result = ();

    fn id(&self) -> DialogId {
        dialog_id("blocker")
    }

    fn show(&mut self, _: &mut Ui, _: &DialogStack) -> Option<DialogResult<()>> {
        self.0.then_some(DialogResult::Ok(()))
    }
}

/// Enter takes Update and Restart when self-updating and Download otherwise; Esc is Later.
#[trial]
fn the_default_button_depends_on_self_updating_and_esc_is_later() {
    let (mut fixture, dialogs) = opened(prompt(true));
    assert!(fixture.by_label("Update and Restart").is_some());
    press(&mut fixture, Key::Enter);
    assert!(dialogs.borrow().is_open::<UpdatePromptDialog>(), "Update and Restart keeps the prompt open for the progress");
    assert!(dialogs.borrow_mut().get_mut::<UpdatePromptDialog>().expect("open").take_update_request());

    let (mut fixture, dialogs) = opened(prompt(false));
    assert!(fixture.by_label("Update and Restart").is_none());
    press(&mut fixture, Key::Enter);
    assert_eq!(dialogs.borrow_mut().take::<UpdatePromptDialog>(), Some(DialogResult::Ok(UpdateChoice::Download)));
    let (mut fixture, dialogs) = opened(prompt(true));
    press(&mut fixture, Key::Escape);
    assert_eq!(dialogs.borrow_mut().take::<UpdatePromptDialog>(), Some(DialogResult::Ok(UpdateChoice::Later)));

    let (mut fixture, dialogs) = opened(prompt(true));
    dialogs.borrow_mut().get_mut::<UpdatePromptDialog>().expect("open").set_installing(true);
    fixture.idle(3);
    press(&mut fixture, Key::Enter);
    assert!(dialogs.borrow().is_open::<UpdatePromptDialog>(), "the buttons do nothing while the install runs");
    press(&mut fixture, Key::Escape);
    assert_eq!(dialogs.borrow_mut().take::<UpdatePromptDialog>(), Some(DialogResult::Ok(UpdateChoice::Later)), "Esc is still Later while installing");
}

/// Enqueued at the Update stage behind a Warning modal, the prompt opens only after the modal closes.
#[trial]
fn the_prompt_waits_for_earlier_startup_modals() {
    let mut dialogs = DialogHost::new();
    dialogs.enqueue(StartupStage::Update, prompt(false));
    dialogs.enqueue(StartupStage::Warning, Blocker(false));
    let (mut fixture, dialogs) = rig(dialogs);
    fixture.idle(3);
    assert!(dialogs.borrow().is_open::<Blocker>() && !dialogs.borrow().is_open::<UpdatePromptDialog>());

    dialogs.borrow_mut().get_mut::<Blocker>().expect("open").0 = true;
    fixture.idle(6);
    assert!(dialogs.borrow().is_open::<UpdatePromptDialog>());
}

/// The available state links to the release notes without closing; Enter updates where the install can, Esc closes,
/// and the install buttons are off while installing.
#[trial]
fn the_check_dialog_can_show_an_available_update() {
    let (mut fixture, dialogs) = opened(available(true));
    assert!(painted(&fixture, "is available"));
    assert!(fixture.by_label("Update and Restart").is_some());
    assert!(fixture.by_label("Check Again").is_some());
    assert!(fixture.by_label("Later").is_none());
    assert!(!painted(&fixture, "Cannot auto-update"));
    click(&mut fixture, "Release Notes");
    assert!(dialogs.borrow().is_open::<UpdateCheckDialog>(), "Release Notes keeps the dialog open");
    press(&mut fixture, Key::Enter);
    assert_eq!(dialogs.borrow_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Update)));

    let (mut fixture, dialogs) = opened(available(false));
    press(&mut fixture, Key::Enter);
    assert_eq!(dialogs.borrow_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Download)));

    let (mut fixture, dialogs) = opened(available(true));
    dialogs.borrow_mut().get_mut::<UpdateCheckDialog>().expect("open").set_installing(true);
    fixture.idle(3);
    press(&mut fixture, Key::Enter);
    assert!(dialogs.borrow().is_open::<UpdateCheckDialog>(), "Enter does nothing while installing");
    press(&mut fixture, Key::Escape);
    assert_eq!(dialogs.borrow_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::Close)));
}

/// An install that can't update itself says so as soon as the dialog opens, with Download as the default.
#[trial]
fn an_install_that_cannot_update_itself_says_so_at_open() {
    let (fixture, _) = opened(available(false));
    assert!(painted(&fixture, "Cannot auto-update this installation type"));
    assert!(fixture.by_label("Download").is_some());
    assert!(fixture.by_label("Update and Restart").is_none());
}

/// Without an override the dialog asks craft-update about the host's app; a cargo build output is not an install
/// that updates itself.
#[trial]
fn detection_decides_when_the_app_does_not_say() {
    let mut dialog = UpdateCheckDialog::new("1.0.0");
    dialog.set_available(Some(Release { tag: "v2.0.0".into(), ..Release::default() }));
    let (fixture, _) = opened(dialog);
    assert!(painted(&fixture, "Cannot auto-update this installation type"));
}

/// The host answers the open dialog with any of its buttons, as `StandardPrompt::set_choice` does.
#[trial]
fn the_host_answers_with_any_button() {
    let (fixture, _) = rig(DialogHost::new());
    let ctx = fixture.ctx().clone();
    let answer = |dialog: UpdateCheckDialog, choice: Option<CheckChoice>| {
        let mut dialogs = DialogHost::new();
        let id = dialog.id();
        dialogs.open(dialog);
        if let Some(choice) = choice {
            dialogs.get_mut_by_id::<UpdateCheckDialog>(id).expect("open").set_choice(choice);
        }
        let confirmed = dialogs.confirm_by_id::<UpdateCheckDialog>(&ctx, id);
        confirmed.then(|| dialogs.take::<UpdateCheckDialog>()).flatten()
    };
    assert_eq!(answer(available(true), None), Some(DialogResult::Ok(CheckChoice::Update)));
    assert_eq!(answer(available(false), None), Some(DialogResult::Ok(CheckChoice::Download)));
    assert_eq!(answer(available(true), Some(CheckChoice::CheckAgain)), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
    assert_eq!(answer(available(true), Some(CheckChoice::Download)), None, "Download is not offered where the install updates itself");
    let mut installing = available(true);
    installing.set_installing(true);
    assert_eq!(answer(installing, None), None);
    let mut failed = UpdateCheckDialog::new("1.0.0");
    failed.set_status(Status::Failed("no network".into()));
    assert_eq!(answer(failed, Some(CheckChoice::CheckAgain)), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
}

/// The check dialog shows the spinner, then the up-to-date or failed text as the status changes; Check Again is
/// offered in every state but checking.
#[trial]
fn the_check_dialog_follows_the_status() {
    let (mut fixture, dialogs) = opened(UpdateCheckDialog::new("1.0.0"));
    assert!(painted(&fixture, "Checking for updates…"));
    assert!(fixture.by_label("Check Again").is_none());

    dialogs.borrow_mut().get_mut::<UpdateCheckDialog>().expect("open").set_status(Status::UpToDate);
    fixture.idle(3);
    assert!(painted(&fixture, "is the newest version"));
    assert!(!painted(&fixture, "Checking for updates…"));
    assert!(fixture.by_label("Check Again").is_some());
    assert!(fixture.by_label("OK").is_some());

    dialogs.borrow_mut().get_mut::<UpdateCheckDialog>().expect("open").set_status(Status::Failed("no network".into()));
    fixture.idle(3);
    assert!(painted(&fixture, "no network"));
    assert!(painted(&fixture, "All releases are listed at"));
    assert!(fixture.by_label("https://github.com/storytold/photocraft/releases").is_some());
    click(&mut fixture, "Check Again");
    assert_eq!(dialogs.borrow_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::CheckAgain)));
}

/// Idle is the "not checked yet" state: it stays open and Check Now (Enter) starts the check.
#[trial]
fn an_idle_check_dialog_offers_check_now() {
    let (mut fixture, dialogs) = opened(UpdateCheckDialog::new("1.0.0"));
    dialogs.borrow_mut().get_mut::<UpdateCheckDialog>().expect("open").set_status(Status::Idle);
    fixture.idle(3);
    assert!(painted(&fixture, "Updates have not been checked yet."));
    assert!(fixture.by_label("Check Again").is_none());
    press(&mut fixture, Key::Enter);
    assert_eq!(dialogs.borrow_mut().take::<UpdateCheckDialog>(), Some(DialogResult::Ok(CheckChoice::CheckNow)));
}

/// A `width` x `height` PNG of one opaque color.
fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba(color)).write_to(&mut out, image::ImageFormat::Png).expect("a PNG encodes");
    out.into_inner()
}

#[trial]
fn icons_decode_and_bad_ones_are_refused() {
    let image = icon_image(&png(3, 2, [10, 20, 30, 255])).expect("a small PNG decodes");
    assert_eq!(image.size, [3, 2]);
    assert_eq!(image.pixels[0], egui::Color32::from_rgb(10, 20, 30));
    assert!(icon_image(b"\x89PNG\r\n\x1a\nnot really").is_none(), "a corrupt PNG must not decode");
    assert!(icon_image(&png(1025, 1, [0, 0, 0, 255])).is_none(), "an oversized icon must not decode");
}

/// The "What's New" link opens a release page only when it is a safe web link; an app that builds its own `Release`
/// (a feed, a cache) can't hand the browser anything else.
#[trial]
fn whats_new_opens_only_a_safe_link() {
    let shown = |url: &str| {
        let release = Release { tag: "v2.0.0".into(), html_url: url.into(), ..Release::default() };
        let (fixture, _) = opened(UpdatePromptDialog::new(release, "1.0.0", false));
        fixture.by_label("What's New").is_some()
    };
    let page = "https://github.com/storytold/photocraft/releases/tag/v0.2.0";
    assert!(safe_web_link(page) && shown(page));
    for bad in ["", "file:///etc/passwd", "javascript:alert(1)", "https://evil.example/a&calc", "https://user@evil.example/"] {
        assert!(!safe_web_link(bad) && !shown(bad), "{bad:?}");
    }
}

#[trial]
fn the_prompt_names_the_developer() {
    let (fixture, _) = opened(prompt(false));
    assert!(painted(&fixture, "by ArtCraft"), "every suite app is ArtCraft's by default");
    assert!(painted(&fixture, "PhotoCraft"));
}

fn ctx() -> egui::Context {
    Fixture::new(|_| {}).ctx().clone()
}

/// An updater that never reads the real version-choice file.
fn updater(answer: Checked) -> Updater {
    Updater::new(&PHOTOCRAFT, "0.1.0").with_source(Arc::new(move || answer.clone())).with_pins_path(no_pins())
}

fn no_pins() -> std::path::PathBuf {
    std::env::temp_dir().join("craft-update-tests-no-such-dir").join("versions.json")
}

fn install_ends(updater: &mut Updater, outcome: TestInstall) {
    updater.finish_install_for_tests(newer(), outcome);
    updater.tick(&ctx());
}

/// Download opens only a safe web link, whoever built the `Release`.
#[trial]
fn download_opens_only_a_safe_release_page() {
    let with = |url: &str| Release { html_url: url.into(), ..newer() };
    let page = "https://github.com/storytold/photocraft/releases/tag/v9.0.0";
    assert_eq!(download_action(&PHOTOCRAFT, &with(page)), Action::OpenUrl(page.into()));
    let releases = Action::OpenUrl(PHOTOCRAFT.releases_url());
    for bad in [
        "",
        "http://github.com/x",
        "file:///C:/Windows/system32/calc.exe",
        "javascript:alert(1)",
        "https://evil.example/a|calc",
        "https://user@evil.example/",
        "https://x.example/a b",
        "https://github.com/storytold/photocraft/releases/tag/v9&calc",
    ] {
        assert_eq!(download_action(&PHOTOCRAFT, &with(bad)), releases, "{bad:?}");
    }
}

/// The fallback passes the check for every app, so Download always opens a page.
#[trial]
fn every_releases_page_is_a_safe_web_link() {
    for app in SUITE.iter().chain([&LAUNCHER]) {
        assert!(safe_web_link(&app.releases_url()), "{}", app.id);
    }
    let broken = App::new("x", "X", "x", "a|b/c");
    assert_eq!(download_action(&broken, &Release { html_url: "javascript:x".into(), ..newer() }), Action::None);
}

/// The release in GitHub's release list `json` to offer a copy running `running`, if any.
fn offered_release(json: &str, repo: &str, running: &str) -> Checked {
    let latest = craft_update::parse_github_release_list(json, repo, false)?;
    let running = craft_update::parse_version(running);
    Ok(latest.filter(|release| {
        release.version.as_ref().zip(running.as_ref()).is_some_and(|(offered, running)| craft_update::version::offers_update(offered, running))
    }))
}

// Child processes isolate environment overrides from parallel tests without unsafe set_var.
#[trial]
fn pretend_versions_trigger_startup_prompts_for_every_app() {
    const CHILD: &str = "CRAFT_UPDATE_PRETEND_TEST_APP";
    if let Ok(id) = std::env::var(CHILD) {
        let app = craft_update::suite::by_id(&id).expect("a suite app");
        let mut u = Updater::new(app, "99.0.0").with_pins_path(no_pins());
        assert_eq!(u.version(), "0.0.1", "{} ignored its pretend version", app.id);
        let version = u.version().to_owned();
        let repo = app.repo;
        u = u.with_source(Arc::new(move || {
            offered_release(r#"[{"tag_name":"v1.0.0","name":"Test release","draft":false,"prerelease":false}]"#, repo, &version)
        }));
        let ctx = ctx();
        u.check_at_start();
        u.settle_for_tests(&ctx);
        assert!(u.prompt_open(), "{} did not offer the newer release", app.id);
        // The explicit test override also applies to the pre-existing installation guard.
        // No assets means the installer stops before any network request or file replacement.
        let dir = tempfile::tempdir().expect("a temp dir");
        let error = self_update::update(
            app,
            &self_update::Install::AppImage(dir.path().join("test.AppImage")),
            &newer(),
            Platform { os: Os::Linux, arch: Arch::X64 },
            &Http::new("ArtCraft-Launcher-Test/0.0").expect("an agent"),
            &Paths::under(dir.path()),
            &Stop::default(),
            &mut |_| {},
            &|| Ok(()),
            Some("99.0.0"),
        )
        .expect_err("an installer without assets fails");
        assert!(error.contains("has no"), "{}: {error}", app.id);
        return;
    }
    for app in &SUITE {
        let status = std::process::Command::new(std::env::current_exe().expect("this binary"))
            .args(["--exact", "update::pretend_versions_trigger_startup_prompts_for_every_app", "--nocapture"])
            .env(CHILD, app.id)
            .env(format!("{}_PRETEND_VERSION", app.id.to_uppercase()), "0.0.1")
            .env_remove(self_update::PRETEND_VAR)
            .status()
            .expect("the child runs");
        assert!(status.success(), "{} pretend-version regression failed", app.id);
    }
}

#[trial]
fn a_requested_check_reports_an_up_to_date_app() {
    let (ctx, mut u) = (ctx(), updater(Ok(None)));
    u.check_now();
    assert_eq!(u.status(), &Status::Checking);
    u.settle_for_tests(&ctx);
    assert_eq!(u.status(), &Status::UpToDate);
    u.dismiss_status();
    assert_eq!(u.status(), &Status::Idle);
}

#[trial]
fn dismissing_a_running_check_keeps_it_quiet() {
    let (ctx, mut u) = (ctx(), updater(Ok(None)));
    u.check_now();
    u.dismiss_status();
    u.settle_for_tests(&ctx);
    assert_eq!(u.status(), &Status::Idle);
}

#[trial]
fn a_requested_check_reports_a_failure_but_the_startup_check_stays_quiet() {
    let ctx = ctx();
    let mut u = updater(Err("offline".into()));
    u.check_now();
    u.settle_for_tests(&ctx);
    assert_eq!(u.status(), &Status::Failed("offline".into()));

    let mut u = updater(Err("offline".into()));
    u.check_at_start();
    u.settle_for_tests(&ctx);
    assert_eq!(u.status(), &Status::Idle);
}

#[trial]
fn a_newer_release_opens_the_prompt_and_later_closes_it() {
    let (mut fixture, dialogs) = rig(DialogHost::new());
    let mut u = updater(Ok(Some(newer())));
    u.check_at_start();
    u.settle_for_tests(fixture.ctx());
    assert!(u.prompt_open());
    assert_eq!(u.status(), &Status::Idle);
    assert_eq!(u.run(&mut dialogs.borrow_mut(), true), Action::None);
    fixture.idle(3);
    assert!(dialogs.borrow().is_open::<UpdatePromptDialog>(), "the prompt is on the host");
    click(&mut fixture, "Later");
    assert_eq!(u.run(&mut dialogs.borrow_mut(), true), Action::None);
    assert!(!u.prompt_open());
    fixture.idle(3);
    assert!(!dialogs.borrow().is_open::<UpdatePromptDialog>());
}

#[trial]
fn the_startup_check_runs_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let asked = Arc::new(AtomicUsize::new(0));
    let counted = asked.clone();
    let ctx = ctx();
    let mut u = Updater::new(&PHOTOCRAFT, "0.1.0")
        .with_source(Arc::new(move || {
            counted.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        }))
        .with_pins_path(no_pins());
    u.check_at_start();
    u.settle_for_tests(&ctx);
    u.check_at_start();
    u.settle_for_tests(&ctx);
    assert_eq!(asked.load(Ordering::Relaxed), 1);
}

#[trial]
fn a_request_during_the_startup_check_is_not_lost() {
    let (ctx, mut u) = (ctx(), updater(Ok(None)));
    u.check_at_start();
    u.check_now();
    u.settle_for_tests(&ctx);
    assert_eq!(u.status(), &Status::UpToDate);
}

#[trial]
fn a_panicking_check_is_an_error_not_a_hang() {
    let ctx = ctx();
    let mut u = Updater::new(&PHOTOCRAFT, "0.1.0").with_source(Arc::new(|| std::panic::panic_any("boom"))).with_pins_path(no_pins());
    u.check_now();
    u.settle_for_tests(&ctx);
    assert!(matches!(u.status(), Status::Failed(_)));
}

#[trial]
fn a_failed_install_returns_to_the_prompt_even_after_it_was_dismissed() {
    let mut u = updater(Ok(None));
    install_ends(&mut u, TestInstall::Failed("disk full".into()));
    assert!(u.prompt_open());
    assert!(!u.is_installing());
    assert_eq!(u.status_message().as_deref(), Some("disk full"));
}

#[trial]
fn a_dropped_install_worker_is_an_error() {
    let mut u = updater(Ok(None));
    install_ends(&mut u, TestInstall::Dropped);
    assert!(u.status_message().is_some());
}

fn shout(text: &str, args: &[(&str, &str)]) -> String {
    craft_update::translate::fill(&text.to_uppercase(), args)
}

/// A translator given after the prompt opened reaches it too.
#[trial]
fn a_translator_given_later_reaches_the_open_prompt() {
    let mut u = updater(Ok(None));
    u.offer(newer());
    u.request_install(false);
    let mut u = u.with_translator(shout);
    let message = |u: &Updater| u.status_message().unwrap_or_default();
    assert!(message(&u).starts_with("SAVE OR CLOSE"), "{}", message(&u));
    // And the prompts it opens later.
    u.offer(newer());
    u.request_install(false);
    assert!(message(&u).starts_with("SAVE OR CLOSE"), "{}", message(&u));
}

/// The updater's own failures (a dropped or panicking worker) are in the interface language; the installer's are
/// shown as they came.
#[trial]
fn the_updaters_own_failures_are_translated_and_the_installers_are_not() {
    for (outcome, want) in [
        (TestInstall::Panicked, "THE UPDATE FAILED UNEXPECTEDLY."),
        (TestInstall::Dropped, "THE UPDATE STOPPED UNEXPECTEDLY."),
        (TestInstall::Failed("disk full".into()), "disk full"),
    ] {
        let mut u = updater(Ok(None)).with_translator(shout);
        install_ends(&mut u, outcome);
        assert_eq!(u.status_message().unwrap_or_default(), want);
    }
}

#[trial]
fn nothing_starts_once_an_update_is_installed() {
    let mut u = updater(Ok(Some(newer())));
    install_ends(&mut u, TestInstall::Done);
    assert_eq!(u.run(&mut DialogHost::new(), true), Action::Close);
    u.check_now();
    assert_eq!(u.status(), &Status::Idle, "no check after an install");
    u.offer(newer());
    assert!(!u.prompt_open(), "no second offer after an install");
}

#[trial]
fn an_install_waits_while_the_app_holds_unsaved_work() {
    let (mut fixture, dialogs) = rig(DialogHost::new());
    let mut u = updater(Ok(None));
    u.offer(newer());
    u.request_install(false);
    assert!(!u.is_installing(), "nothing is scheduled while there is unsaved work");
    assert!(u.status_message().is_some());
    u.run(&mut dialogs.borrow_mut(), false);
    fixture.idle(3);
    click(&mut fixture, "Later");
    u.run(&mut dialogs.borrow_mut(), false);
    assert!(!u.prompt_open());
    assert!(u.request_install(false), "a release the updater knows of needs no open prompt");
    assert!(!updater(Ok(None)).request_install(true), "no known release reports false");
    assert!(u.status_message().is_some_and(|m| m.contains("Save or close")), "the public status line carries the note");
}

/// The updater's own messages follow the interface language while the prompt shows them: they are translated when
/// the prompt is fed, not when they are set.
#[trial]
fn the_prompts_own_messages_follow_a_language_change() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static FRENCH: AtomicBool = AtomicBool::new(false);
    fn translate(text: &str, args: &[(&str, &str)]) -> String {
        let text = if FRENCH.load(Ordering::Relaxed) { format!("FR {text}") } else { text.to_owned() };
        craft_update::translate::fill(&text, args)
    }
    let mut u = updater(Ok(None)).with_translator(translate);
    u.offer(newer());
    u.request_install(false);
    let message = |u: &Updater| u.status_message().unwrap_or_default();
    assert!(message(&u).starts_with("Save or close"), "{}", message(&u));
    FRENCH.store(true, Ordering::Relaxed);
    assert!(message(&u).starts_with("FR Save or close"), "{}", message(&u));
    // An installed update names the app, in the language of the moment.
    install_ends(&mut u, TestInstall::Done);
    assert!(message(&u).starts_with("FR The update is installed. Close PhotoCraft"), "{}", message(&u));
    FRENCH.store(false, Ordering::Relaxed);
    assert!(message(&u).starts_with("The update is installed."), "{}", message(&u));
}

/// An error written by the installer shows even when a note was set before.
#[trial]
fn a_literal_error_wins_over_a_stale_note() {
    let mut u = updater(Ok(None));
    u.offer(newer());
    u.request_install(false);
    assert!(u.status_message().is_some_and(|m| m.starts_with("Save or close")));
    install_ends(&mut u, TestInstall::Failed("disk full".into()));
    assert_eq!(u.status_message().as_deref(), Some("disk full"));
}

#[trial]
fn a_finished_install_keeps_asking_to_close_until_the_app_can() {
    let mut u = updater(Ok(None));
    u.offer(newer());
    install_ends(&mut u, TestInstall::Done);
    let mut dialogs = DialogHost::new();
    assert_eq!(u.run(&mut dialogs, false), Action::None, "unsaved work holds the close back");
    assert_eq!(u.run(&mut dialogs, true), Action::Close);
    assert_eq!(u.run(&mut dialogs, true), Action::None);
}

#[trial]
fn take_close_gives_the_restart_close_without_showing() {
    let mut u = updater(Ok(None));
    install_ends(&mut u, TestInstall::Done);
    assert!(!u.take_close(false), "unsaved work holds the close back");
    assert!(u.take_close(true));
    assert!(!u.take_close(true), "once per install");
}

#[trial]
fn a_finished_install_asks_the_app_to_close_once() {
    let mut u = updater(Ok(None));
    install_ends(&mut u, TestInstall::Done);
    let mut dialogs = DialogHost::new();
    assert_eq!(u.run(&mut dialogs, true), Action::Close);
    assert_eq!(u.run(&mut dialogs, true), Action::None);
}

/// A version-choice file holding `track` for PhotoCraft.
fn pins_with(track: &VersionTrack) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut pins = Pins::default();
    pins.set(PHOTOCRAFT.id, track);
    pins.save(&dir.path().join("versions.json")).expect("the pins save");
    dir
}

fn pin(path: &Path, track: &VersionTrack) {
    let mut pins = Pins::default();
    pins.set(PHOTOCRAFT.id, track);
    pins.save(path).expect("the pins save");
}

#[trial]
fn a_pinned_app_is_not_prompted_and_the_network_is_not_asked() {
    let ctx = ctx();
    for track in [VersionTrack::Pinned("v0.13.0".into()), VersionTrack::Nightly] {
        let dir = pins_with(&track);
        let mut u = Updater::new(&PHOTOCRAFT, "0.13.0")
            .with_source(Arc::new(|| std::panic::panic_any("the network must not be asked")))
            .with_pins_path(dir.path().join("versions.json"));
        u.check_at_start();
        u.settle_for_tests(&ctx);
        assert!(!u.prompt_open(), "{track:?}");
        assert_eq!(u.status(), &Status::Idle);
        u.check_now();
        u.settle_for_tests(&ctx);
        assert_eq!(u.status(), &Status::UpToDate, "{track:?}");
        assert_eq!(u.managed(), Some(&Choice::Managed(track.clone())));
        u.offer(newer());
        assert!(!u.prompt_open(), "an offer is held too: {track:?}");
    }
}

#[trial]
fn unpinning_in_the_launcher_brings_updates_back_on_the_next_check() {
    let ctx = ctx();
    let dir = pins_with(&VersionTrack::Pinned("v0.13.0".into()));
    let path = dir.path().join("versions.json");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(Some(newer())))).with_pins_path(&path);
    u.check_now();
    u.settle_for_tests(&ctx);
    assert!(!u.prompt_open());
    Pins::default().save(&path).expect("the pins save");
    u.check_now();
    u.settle_for_tests(&ctx);
    assert!(u.prompt_open());
    assert_eq!(u.managed(), None);
}

#[trial]
fn an_unreadable_choice_from_a_newer_launcher_also_holds_updates() {
    let ctx = ctx();
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    std::fs::write(&path, r#"{"apps":{"photocraft":{"Beta":1}}}"#).expect("the file is written");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(Some(newer())))).with_pins_path(&path);
    u.check_at_start();
    u.settle_for_tests(&ctx);
    assert!(!u.prompt_open());
    assert_eq!(u.managed(), Some(&Choice::Unknown));
}

#[trial]
fn another_apps_pin_does_not_hold_this_one() {
    let ctx = ctx();
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    let mut pins = Pins::default();
    pins.set("pdfcraft", &VersionTrack::Pinned("v0.2.0".into()));
    pins.save(&path).expect("the pins save");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(Some(newer())))).with_pins_path(&path);
    u.check_at_start();
    u.settle_for_tests(&ctx);
    assert!(u.prompt_open());
}

#[trial]
fn a_prompt_opened_before_the_pin_cannot_install_the_stable_release() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(Some(newer())))).with_pins_path(&path);
    u.offer(newer());
    assert!(u.prompt_open());
    pin(&path, &VersionTrack::Pinned("v0.13.0".into()));
    u.request_install(true);
    assert!(!u.is_installing(), "nothing installs once the launcher pinned the app");
    assert!(!u.prompt_open());
}

#[trial]
fn a_check_after_the_pin_closes_a_prompt_opened_before_it() {
    let ctx = ctx();
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(Some(newer())))).with_pins_path(&path);
    u.offer(newer());
    pin(&path, &VersionTrack::Nightly);
    u.check_now();
    u.settle_for_tests(&ctx);
    assert!(!u.prompt_open());
    assert_eq!(u.status(), &Status::UpToDate);
}

#[trial]
fn a_running_install_keeps_its_prompt_when_the_app_is_pinned() {
    let ctx = ctx();
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(Arc::new(|| Ok(None))).with_pins_path(&path);
    u.offer(newer());
    u.check_now();
    u.finish_install_for_tests(newer(), TestInstall::Running);
    pin(&path, &VersionTrack::Pinned("v0.13.0".into()));
    u.settle_for_tests(&ctx);
    assert!(u.prompt_open(), "tick reports the running install through the prompt");
}

#[trial]
fn a_pin_made_while_the_check_runs_still_holds_its_answer() {
    let ctx = ctx();
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("versions.json");
    let during = path.clone();
    // The check itself pins the app before answering, as the launcher could mid-check.
    let source: CheckSource = Arc::new(move || {
        pin(&during, &VersionTrack::Pinned("v0.13.0".into()));
        Ok(Some(newer()))
    });
    let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(source).with_pins_path(&path);
    u.check_now();
    u.settle_for_tests(&ctx);
    assert!(!u.prompt_open());
    assert_eq!(u.status(), &Status::UpToDate);
}

#[trial]
fn every_answer_after_a_pin_made_mid_check_reports_the_launchers_choice() {
    let ctx = ctx();
    let answers: [Checked; 2] = [Ok(None), Err("offline".into())];
    for answer in answers {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("versions.json");
        let during = path.clone();
        let source: CheckSource = Arc::new(move || {
            pin(&during, &VersionTrack::Nightly);
            answer.clone()
        });
        let mut u = Updater::new(&PHOTOCRAFT, "0.13.0").with_source(source).with_pins_path(&path);
        u.check_now();
        u.settle_for_tests(&ctx);
        assert_eq!(u.status(), &Status::UpToDate);
        assert_eq!(u.managed(), Some(&Choice::Managed(VersionTrack::Nightly)));
    }
}
