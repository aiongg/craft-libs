//! The signature pad's stored and drawn forms.

use craft_review::signature::{SavedSignature, SignatureDraft, SignatureTab};
use craft_ui::testing::trial;

/// Apply returns what the current tab holds: typed text, the strokes or the picked image.
#[trial]
fn apply_returns_the_current_tab() {
    let mut draft = SignatureDraft::new(false, "Ana Reyes");
    draft.tab = SignatureTab::Type;
    assert!(matches!(draft.saved(), Some(SavedSignature::Typed(text)) if text == "Ana Reyes"));
    draft.tab = SignatureTab::Draw;
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(matches!(draft.saved(), Some(SavedSignature::Drawn(strokes)) if strokes.len() == 1));
}

/// Apply waits for content: an empty draft is not ready.
#[trial]
fn apply_waits_for_content() {
    let mut draft = SignatureDraft::new(false, "");
    assert!(!draft.ready());
    draft.tab = SignatureTab::Draw;
    assert!(!draft.ready());
    draft.strokes = vec![vec![[0.1, 0.2]]];
    assert!(!draft.ready(), "a bare click is not a stroke");
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(draft.ready());
}
