//! Signature drafts.

use crate::signature::*;

fn stroke(points: usize) -> Vec<[f32; 2]> {
    (0..points).map(|i| [i as f32 / 10.0, 0.5]).collect()
}

#[test]
fn initials_draft_takes_first_letters() {
    assert_eq!(SignatureDraft::new(true, "Ana María Ruiz").text, "AMR");
    assert_eq!(SignatureDraft::new(false, "  Ana Ruiz ").text, "Ana Ruiz");
}

#[test]
fn ready_and_saved_follow_the_tab() {
    let mut draft = SignatureDraft::new(false, "  ");
    assert!(!draft.ready() && draft.saved().is_none());
    draft.text = " Ana ".into();
    assert_eq!(draft.saved(), Some(SavedSignature::Typed("Ana".into())));

    draft.tab = SignatureTab::Draw;
    draft.strokes = vec![stroke(1)];
    assert!(!draft.ready(), "a bare dot is not a stroke");
    draft.strokes.push(stroke(4));
    assert_eq!(draft.saved(), Some(SavedSignature::Drawn(draft.strokes.clone())));

    draft.tab = SignatureTab::Image;
    assert!(draft.saved().is_none());
    let image = SignatureImage::new(1, 1, vec![0, 0, 0, 255]);
    draft.image = Some(image.clone());
    assert_eq!(draft.saved(), Some(SavedSignature::Image(image)));

    // Typed text is capped.
    let mut draft = SignatureDraft::new(false, &"a".repeat(MAX_SIGNATURE_CHARS));
    assert!(draft.ready());
    draft.text.push('a');
    assert!(!draft.ready() && draft.saved().is_none());
}

// A bare click leaves a one-point stroke; Undo takes it with the stroke before it.
#[test]
fn undo_skips_bare_clicks() {
    let mut draft = SignatureDraft { strokes: vec![stroke(4), stroke(3), stroke(1)], ..SignatureDraft::default() };
    assert!(draft.undo_stroke());
    assert_eq!(draft.strokes, vec![stroke(4)]);
    assert!(draft.undo_stroke() && draft.strokes.is_empty());
    assert!(!draft.undo_stroke());
}

// The cache key is made at construction from the content.
#[test]
fn image_key_follows_content() {
    let image = SignatureImage::new(1, 1, vec![0, 0, 0, 255]);
    assert_eq!(image.key(), SignatureImage::new(1, 1, vec![0, 0, 0, 255]).key());
    assert_ne!(image.key(), SignatureImage::new(1, 1, vec![0, 0, 1, 255]).key());
}
