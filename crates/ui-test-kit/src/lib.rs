//! The test kits the craft-ui crates' integration binaries share: components written with the derives. Child
//! windows run on the `Harness`'s headless platform. Their registrations are this package's, so no library's suite runs them.
#![warn(missing_docs)]

// The kits name the library as `craft_ui`, as the test files do.
extern crate craft_ui_contract as craft_ui;

pub mod body;
pub mod contract;
pub mod rt;

/// Registrations of the shared test components.
#[cfg(feature = "suite")]
pub mod testing {
    craft_ui::registry!();
}
