//! The test kit: the [`Harness`], the suite runner and the story helpers.
//!
//! [`Harness`] runs a view or a story on craft-ui-rt and the headless platform and reads what the runtime keeps:
//! display lists ([`paint`]), the access tree ([`access`]), hit shapes and focus stops. `suite` runs the shared
//! cases over every registered type.
//!
//! The registration half of the kit: `#[trial]` functions list themselves in `TRIALS`, and [`run`],
//! every test executable's runner, collects them.

pub mod access;
pub mod contract;
pub mod dump;
pub mod face_runs;
#[cfg(feature = "suite")]
pub(crate) mod icon_trials;
#[cfg(feature = "suite")]
pub mod index;
pub mod layout;
pub mod memory;
pub mod paint;
pub mod pictures;
#[cfg(feature = "suite")]
mod registry;
pub mod relations;
#[cfg(all(feature = "snapshot", not(target_arch = "wasm32")))]
pub mod snapshot;
mod stage;
#[cfg(feature = "suite")]
pub(crate) mod stories;
pub mod story;
pub mod story_run;
pub mod style;
#[cfg(feature = "suite")]
pub mod suite;
pub mod windows;

pub use crate::contract::rt::commands::modal_checks;
pub use crate::contract::rt::harness::{FRAME, Harness, HarnessBuilder, LayerInfo, Look, ScrollRegion, Target};
pub use crate::contract::{Exemption, PatternKind, PatternProbe, Probe, Registration};
#[cfg(feature = "testing")]
pub use crate::fonts::document::set_budget as document_face_budget;
#[cfg(feature = "testing")]
pub use crate::runtime::env::env_walks;
pub use craft_ui_macros::{story, trial};
pub use paint::{Painted, PaintedText};
#[cfg(feature = "suite")]
pub use registry::{Registered, Registry};
pub use stage::{least_stage_width, room_height, stage, stage_fitting, stage_width};
#[doc(hidden)]
pub use story::{
    CollectionKind, Drive, EditKind, Grip, Required, Story, StoryCommands, StoryData, StoryOutput, StoryRegistration, StorySnapshot, ValueAt,
    show_as_only_item, show_as_only_item_across,
};
pub use story_run::{Impure, Room, StoryHarness, StoryRun};

/// One registered trial.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Trial {
    /// The module path of the function, crate name first.
    pub module: &'static str,
    /// The function's name.
    pub name: &'static str,
    /// The trial; it fails by panicking.
    pub run: fn(),
    /// Whether it runs only when asked for (`--ignored`): a fixture that another trial starts as a subprocess.
    pub ignored: bool,
}

impl Trial {
    /// A registration, as `#[trial]` writes it.
    pub const fn new(module: &'static str, name: &'static str, run: fn()) -> Trial {
        Trial { module, name, run, ignored: false }
    }

    /// The registration of an `#[ignore]`d trial, as `#[trial]` writes it.
    pub const fn ignored(module: &'static str, name: &'static str, run: fn()) -> Trial {
        Trial { module, name, run, ignored: true }
    }

    /// The name filters match: the module path below the crate, then the function.
    pub fn full_name(&self) -> String {
        match self.module.split_once("::") {
            Some((_, below)) => format!("{below}::{}", self.name),
            None => self.name.to_owned(),
        }
    }
}

/// Runs a test executable and exits: the `#[trial]`s declared in the crate whose root module is `crate_root` or in
/// `package`'s library crate, and the suite's trials over `registered`. Every test executable's `main` calls it, with
/// its own `module_path!()`.
#[cfg(feature = "suite")]
pub fn run(crate_root: &str, package: &'static str, registry: &Registry) -> ! {
    if !registry.install_story_sets() {
        eprintln!("the selected story commands are invalid");
        std::process::exit(1);
    }
    let library = package.replace('-', "_");
    let own = |module: &str| {
        [crate_root, library.as_str()].into_iter().any(|root| module.strip_prefix(root).is_some_and(|rest| rest.is_empty() || rest.starts_with("::")))
    };
    let mut tests: Vec<libtest_mimic::Trial> = registry
        .trials()
        .iter()
        .filter(|trial| own(trial.module))
        .map(|trial| {
            let run = trial.run;
            let story_sets = registry.story_selection();
            libtest_mimic::Trial::test(trial.full_name(), move || {
                crate::runtime::commands::with_story_sets(&story_sets, || {
                    run();
                    Ok(())
                })
            })
            .with_ignored_flag(trial.ignored)
        })
        .collect();
    tests.extend(suite::trials(package, registry));
    // A trial that re-runs its executable as a stand-in for another program passes that program's arguments after
    // `--`; they are the stand-in's, not the runner's.
    let args = libtest_mimic::Arguments::from_iter(std::env::args().take_while(|arg| arg != "--"));
    libtest_mimic::run(&args, tests).exit()
}

#[cfg(feature = "suite")]
craft_ui_macros::registry!();
