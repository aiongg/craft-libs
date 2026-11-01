//! The bin search field and `name_matches`.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// Whether `name` matches the search `filter`: every whitespace-separated word of `filter` is in
/// `name`, in any order, both compared in Unicode lower case (`str::to_lowercase`; FilmCraft's
/// ASCII lower case missed other scripts). An empty or blank filter matches everything.
pub fn name_matches(name: &str, filter: &str) -> bool {
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:85 (`name_matches`).
    todo!()
}
