//! The craft-ui story viewer: the registered stories of the linked crates, shown under a chosen appearance.

pub mod chrome;
pub mod viewer;

// The theme workspace.
pub mod theme;

// The mock editor window.
pub mod mock;

// Linked so that each domain crate's registered stories are in the viewer; the gallery uses nothing else of them.
use craft_automation as _;
use craft_browse as _;
use craft_color as _;
use craft_dialogs as _;
use craft_draw as _;
use craft_font_manager as _;
use craft_imaging as _;
use craft_layers as _;
use craft_menu as _;
use craft_pages as _;
use craft_print as _;
use craft_review as _;
use craft_table as _;
use craft_text as _;
use craft_theme_store as _;
use craft_timeline as _;
use craft_update as _;

/// The test registry of this package's stories.
pub mod testing {
    craft_ui::testing_registered!();
}
