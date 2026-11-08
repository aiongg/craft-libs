//! The craft-ui story viewer: the registered stories of the linked crates, shown under a chosen appearance.

pub mod chrome;
pub mod viewer;

// The theme workspace.
pub mod theme;

// The mock editor window.
pub mod mock;

/// The test registry of this package's stories.
pub mod testing {
    craft_ui::testing_registered!();
}
