//! The craft-ui story viewer: the registered stories of the linked crates, shown under a chosen appearance.

pub mod chrome;
pub mod viewer;

// The theme workspace.
pub mod theme;

// The mock editor window.
pub mod mock;

/// The registrations of every crate whose stories the viewer shows, by crate.
///
/// Each entry is a function of its crate that is passed by pointer, so the linker keeps that crate's object files and
/// the story registrations in them, which a release build for the web would otherwise drop.
type Registered = fn() -> Vec<&'static craft_ui::testing::Registration>;

fn crates() -> Vec<(&'static str, Registered)> {
    let crates: Vec<(&'static str, Registered)> = vec![
        ("craft-automation", craft_automation::testing::registered),
        ("craft-browse", craft_browse::ui::testing::registered),
        ("craft-color", craft_color::ui::testing::registered),
        ("craft-draw", craft_draw::testing::registered),
        ("craft-imaging", craft_imaging::testing::registered),
        ("craft-layers", craft_layers::testing::registered),
        ("craft-menu", craft_menu::testing::registered),
        ("craft-pages", craft_pages::testing::registered),
        ("craft-print", craft_print::testing::registered),
        ("craft-review", craft_review::testing::registered),
        ("craft-table", craft_table::ui::testing::registered),
        ("craft-text", craft_text::testing::registered),
        ("craft-theme-store", craft_theme_store::ui::registered),
        ("craft-timeline", craft_timeline::testing::registered),
    ];
    // These do not build for the web, so their stories are in the native gallery only.
    #[cfg(not(target_arch = "wasm32"))]
    let crates = [
        crates,
        vec![
            ("craft-dialogs", craft_dialogs::testing::registered as Registered),
            ("craft-font-manager", craft_font_manager::testing::registered),
            ("craft-update", craft_update::testing::registered),
        ],
    ]
    .concat();
    std::hint::black_box(crates)
}

/// How many registrations each linked crate contributes, with the viewer's own, and how many stories there are in all.
pub fn linked_counts() -> (Vec<(&'static str, usize)>, usize) {
    let mut counts: Vec<(&'static str, usize)> = crates().into_iter().map(|(name, registered)| (name, registered().len())).collect();
    counts.push((env!("CARGO_PKG_NAME"), testing::registered().len()));
    (counts, craft_ui::testing::stories().len())
}

/// The test registry of this package's stories.
pub mod testing {
    craft_ui::testing_registered!();
}

#[cfg(test)]
mod tests {
    /// A crate with no registrations here was linked out, which in a release web build loses its stories.
    #[test]
    fn every_linked_crate_registers() {
        let (counts, stories) = super::linked_counts();
        assert!(stories > 0);
        let empty: Vec<_> = counts.iter().filter(|(_, count)| *count == 0).collect();
        assert!(empty.is_empty(), "no registrations from {empty:?}");
    }
}
