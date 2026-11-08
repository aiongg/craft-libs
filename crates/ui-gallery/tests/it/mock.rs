//! The mock editor's own logic: what deleting a layer and showing or hiding one do to the document.

use craft_ui::testing::trial;
use craft_ui_gallery::mock::Document;

#[trial]
fn deleting_the_current_layer_keeps_a_current_layer_and_marks_the_document_changed() {
    let mut document = Document::default();
    let before = document.layers.len();
    let removed = document.delete_current();
    assert_eq!(removed.map(|layer| layer.name), Some("Mountains".to_owned()));
    assert_eq!(document.layers.len(), before - 1);
    assert!(document.current_layer().is_some());
    assert!(document.dirty);
}

#[trial]
fn the_last_layer_cannot_be_deleted() {
    let mut document = Document::default();
    while document.delete_current().is_some() {}
    assert_eq!(document.layers.len(), 1);
    assert!(document.delete_current().is_none());
}

#[trial]
fn hiding_a_layer_marks_the_document_changed_and_setting_the_same_state_does_not() {
    let mut document = Document::default();
    document.set_visible(0, true);
    assert!(!document.dirty);
    document.set_visible(0, false);
    assert!(document.dirty);
    assert!(!document.layers[0].visible);
}
