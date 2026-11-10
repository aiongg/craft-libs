//! The fixture, the suite runner and the story helpers.
//!
//! `Fixture` shows a closure or a story over [`Frames`]; `suite` runs the shared cases over every registered type.
//!
//! The registration half of the kit: `#[trial]` functions list themselves in [`TRIALS`], and the integration
//! binary's runner collects them. The fixture half drives frames ([`Frames`]) and reaches the runtime's
//! crate-private services the trials check.

pub mod access;
pub mod bypass;
pub mod canvas_story;
pub mod contract;
pub mod dump;
pub mod fixture;
pub mod focus;
pub mod frames;
pub mod index;
pub mod input;
pub mod internals;
pub mod layout;
pub mod memory;
pub mod pointer;
pub mod props;
pub mod relations;
pub mod sensing;
mod stage;
mod stories;
pub mod story;
pub mod style;
pub mod suite;
pub mod surface;
pub mod text;
pub mod tool_fixture;
mod tool_story;
pub mod wheel;
pub mod windows;

pub use crate::contract::{Exemption, PatternKind, PatternProbe, Probe, Registration};
#[cfg(feature = "testing")]
pub use crate::runtime::measure::{active as is_measuring, body_runs as measuring_runs, run as measuring_run, take_effects, widest_runs};
pub use craft_ui_macros::{story, trial};
pub use fixture::{Fixture, Impure, Room, Target};
pub use frames::Frames;
pub use props::story_props;
pub use stage::stage;
#[doc(hidden)]
pub use story::{
    CollectionKind, Drive, EditKind, Grip, Required, Story, StoryData, StoryOutput, StoryRegistration, StorySnapshot, show_as_only_item, stories,
};
pub use tool_fixture::ToolFixture;
pub use tool_story::tool_story;
pub use windows::install_spec;

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
}

impl Trial {
    /// A registration, as `#[trial]` writes it.
    pub const fn new(module: &'static str, name: &'static str, run: fn()) -> Trial {
        Trial { module, name, run }
    }

    /// The name filters match: the module path below the crate, then the function.
    pub fn full_name(&self) -> String {
        match self.module.split_once("::") {
            Some((_, below)) => format!("{below}::{}", self.name),
            None => self.name.to_owned(),
        }
    }
}

inventory::collect!(Trial);

/// Every `#[trial]` linked into the running binary, by full name.
pub fn trials() -> Vec<&'static Trial> {
    let mut all: Vec<_> = inventory::iter::<Trial>().collect();
    all.sort_by_key(|trial| trial.full_name());
    all
}

/// Every registration declared in the library crate of the package `crate_name` and linked into the running
/// binary, by module and type name. Types declared in the package's integration binaries are fixtures, not the package's.
pub fn registered_in(crate_name: &str) -> Vec<&'static Registration> {
    let library = crate_name.replace('-', "_");
    crate::contract::registrations()
        .into_iter()
        .filter(|registration| registration.crate_name() == crate_name && registration.crate_target() == library)
        .collect()
}

/// Writes `pub fn registered()` for the invoking package: its library crate's registrations in the running binary.
/// A crate invokes it once, in its `testing` module, and its integration binary calls the function by name. The
/// call references the library, so the linker includes it; the registrations are constructors in the library's
/// object files, which run as long as those files are linked.
#[macro_export]
macro_rules! testing_registered {
    () => {
        /// Every registration of this package linked into the running binary.
        pub fn registered() -> ::std::vec::Vec<&'static $crate::testing::Registration> {
            $crate::testing::registered_in(env!("CARGO_PKG_NAME"))
        }
    };
}

testing_registered!();

#[doc(hidden)]
pub mod __private {
    pub use inventory;
}
