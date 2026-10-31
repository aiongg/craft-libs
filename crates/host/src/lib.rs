#![doc = include_str!("../README.md")]

mod services;

use std::sync::Arc;

use craft_i18n::{I18n, Lang};

#[cfg(not(target_arch = "wasm32"))]
pub use services::ThreadJobs;
pub use services::{AppInfo, Commands, InlineJobs, Jobs, MemorySections, NoCommands, Section, SectionStore};

/// The app's shared services, built once at startup and installed on the egui context.
/// Every library feature reads them with `Host::of(ui)`; none takes them as parameters.
#[derive(Clone)]
pub struct Host {
    app: AppInfo,
    i18n: Option<Arc<I18n>>,
    lang: Lang,
    settings: Arc<dyn SectionStore>,
    commands: Arc<dyn Commands>,
    jobs: Arc<dyn Jobs>,
}

/// Collects a [`Host`]'s services; each one defaults when not set.
pub struct HostBuilder {
    app: AppInfo,
    i18n: Option<Arc<I18n>>,
    lang: Option<Lang>,
    settings: Option<Arc<dyn SectionStore>>,
    commands: Option<Arc<dyn Commands>>,
    jobs: Option<Arc<dyn Jobs>>,
}

impl Host {
    pub fn builder(app: AppInfo) -> HostBuilder {
        HostBuilder { app, i18n: None, lang: None, settings: None, commands: None, jobs: None }
    }

    pub fn app(&self) -> &AppInfo {
        &self.app
    }

    /// The app's catalogs; `None` until the app uses craft-i18n (library words are English then).
    pub fn i18n(&self) -> Option<&I18n> {
        self.i18n.as_deref()
    }

    /// [`Host::i18n`] as the shared handle, for code that keeps the catalogs (craft-ui's `Words`).
    pub fn i18n_shared(&self) -> Option<&Arc<I18n>> {
        self.i18n.as_ref()
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn settings(&self) -> &dyn SectionStore {
        &*self.settings
    }

    pub fn commands(&self) -> &dyn Commands {
        &*self.commands
    }

    pub fn jobs(&self) -> &dyn Jobs {
        &*self.jobs
    }

    /// A copy in another language; install it to switch the interface.
    pub fn with_lang(&self, lang: Lang) -> Host {
        Host { lang, ..self.clone() }
    }

    /// The stored value of section `S`, or `S::default()` when it is missing or does not fit `S`.
    pub fn setting<S: Section>(&self) -> S {
        self.settings.read(S::KEY).and_then(|table| table.try_into().ok()).unwrap_or_default()
    }

    /// Stores `value` as section `S`.
    pub fn set_setting<S: Section>(&self, value: &S) {
        if let Ok(table) = toml::Table::try_from(value) {
            self.settings.write(S::KEY, table);
        }
    }
}

impl HostBuilder {
    /// Default: none (English).
    pub fn i18n(mut self, i18n: Arc<I18n>) -> Self {
        self.i18n = Some(i18n);
        self
    }

    /// Default: the catalogs' system language, else English.
    pub fn lang(mut self, lang: Lang) -> Self {
        self.lang = Some(lang);
        self
    }

    /// Default: [`MemorySections`].
    pub fn settings(mut self, settings: Arc<dyn SectionStore>) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Default: [`NoCommands`] (English labels).
    pub fn commands(mut self, commands: Arc<dyn Commands>) -> Self {
        self.commands = Some(commands);
        self
    }

    /// Default: `ThreadJobs`; [`InlineJobs`] on wasm32.
    pub fn jobs(mut self, jobs: Arc<dyn Jobs>) -> Self {
        self.jobs = Some(jobs);
        self
    }

    pub fn build(self) -> Host {
        let lang = self.lang.or_else(|| self.i18n.as_ref().map(|i18n| i18n.system_lang())).unwrap_or(Lang::EN);
        Host {
            app: self.app,
            i18n: self.i18n,
            lang,
            settings: self.settings.unwrap_or_else(|| Arc::new(MemorySections::default())),
            commands: self.commands.unwrap_or_else(|| Arc::new(NoCommands)),
            jobs: self.jobs.unwrap_or_else(default_jobs),
        }
    }

    /// `build().install(ctx)`.
    #[cfg(feature = "ui")]
    pub fn install(self, ctx: &egui::Context) {
        self.build().install(ctx);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_jobs() -> Arc<dyn Jobs> {
    Arc::new(ThreadJobs)
}

#[cfg(target_arch = "wasm32")]
fn default_jobs() -> Arc<dyn Jobs> {
    Arc::new(InlineJobs)
}

#[cfg(feature = "ui")]
mod ui {
    use std::sync::Arc;

    use egui::{Context, Plugin, Ui};

    use super::Host;

    /// The installed host, kept as a plugin so that a reset of the context's memory does not drop it.
    #[derive(Default)]
    struct Installed(Option<Arc<Host>>);

    impl Plugin for Installed {
        fn debug_name(&self) -> &'static str {
            "craft_host::Host"
        }
    }

    impl Host {
        /// Stores the host in a context plugin, so a memory reset leaves it installed.
        pub fn install(self, ctx: &Context) {
            ctx.plugin_or_default::<Installed>().lock().0 = Some(Arc::new(self));
        }

        /// The installed host, or `None`. Library words and numbers use this and fall back to
        /// English, so tests, stories and apps without a Host keep working.
        pub fn try_of(ui: &Ui) -> Option<Arc<Host>> {
            Host::try_of_ctx(ui.ctx())
        }

        pub fn try_of_ctx(ctx: &Context) -> Option<Arc<Host>> {
            ctx.plugin_opt::<Installed>().and_then(|installed| installed.lock().0.clone())
        }

        /// The installed host. Panics with "Host::install was not called"; only for features that
        /// cannot work without a service.
        #[allow(clippy::panic, reason = "a missing Host is a programming error of the app")]
        pub fn of(ui: &Ui) -> Arc<Host> {
            Host::try_of(ui).expect("Host::install was not called")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    const APP: AppInfo = AppInfo { id: "test", name: "Test", version: "1.0" };

    #[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
    #[serde(default)]
    struct Panel {
        open: bool,
        width: u32,
    }

    impl Section for Panel {
        const KEY: &'static str = "panel";
    }

    #[test]
    fn defaults_are_english_with_memory_sections() {
        let host = Host::builder(APP).build();
        assert_eq!(host.lang(), Lang::EN);
        assert!(host.i18n().is_none());
        assert_eq!(host.commands().label("file.save", "Save"), "Save");
    }

    #[test]
    fn with_lang_changes_only_the_language() {
        let host = Host::builder(APP).build();
        let other = host.with_lang(Lang::PSEUDO);
        assert_eq!(other.lang(), Lang::PSEUDO);
        assert_eq!(host.lang(), Lang::EN);
        assert_eq!(other.app().name, "Test");
    }

    #[test]
    fn settings_round_trip_through_memory_sections() {
        let host = Host::builder(APP).build();
        assert_eq!(host.setting::<Panel>(), Panel::default());
        host.set_setting(&Panel { open: true, width: 240 });
        assert_eq!(host.setting::<Panel>(), Panel { open: true, width: 240 });
    }

    #[test]
    fn inline_jobs_run_at_once() {
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&done);
        InlineJobs.spawn("t", Box::new(move || flag.store(true, std::sync::atomic::Ordering::SeqCst)));
        assert!(done.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[cfg(feature = "ui")]
    #[test]
    fn installed_host_survives_a_memory_reset() {
        let ctx = egui::Context::default();
        assert!(Host::try_of_ctx(&ctx).is_none());
        Host::builder(APP).lang(Lang::PSEUDO).install(&ctx);
        ctx.memory_mut(|m| *m = Default::default());
        assert_eq!(Host::try_of_ctx(&ctx).map(|h| h.lang()), Some(Lang::PSEUDO));
    }
}
