//! The components index (`docs/components-index.md`) is what the registrations give, and every registered type says
//! what it is in the first sentence of its doc comment.

use craft_ui::testing::index::{components_index, types_without_purpose};
use craft_ui::testing::trial;
use std::path::Path;

#[trial]
fn every_registered_type_has_a_purpose() {
    let missing = types_without_purpose();
    assert!(missing.is_empty(), "registered types without a doc comment: {missing:?}");
}

#[trial]
fn the_committed_components_index_is_what_the_registrations_give() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let committed = std::fs::read_to_string(root.join("docs/components-index.md")).unwrap_or_default();
    assert!(committed == components_index(&root), "docs/components-index.md is stale; run `cargo run -p craft-ui-gallery --bin components-index`");
}
