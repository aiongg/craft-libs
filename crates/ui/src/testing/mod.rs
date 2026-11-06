//! The fixture, the suite runner and the story helpers.
//!
//! The registration half of the kit: `#[trial]` functions list themselves in [`TRIALS`], and the integration
//! binary's runner collects them. The fixture half drives frames ([`Frames`]) and reaches the runtime's
//! crate-private services the trials check.

pub mod frames;
pub mod input;
pub mod memory;
pub mod style;

pub use craft_ui_macros::trial;
pub use frames::Frames;

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

/// Every `#[trial]` linked into the running binary.
#[linkme::distributed_slice]
pub static TRIALS: [Trial];

#[doc(hidden)]
pub mod __private {
    pub use linkme;
}
