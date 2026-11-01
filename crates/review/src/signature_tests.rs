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
    let image = SignatureImage { width: 1, height: 1, rgba: vec![0, 0, 0, 255] };
    draft.image = Some(image.clone());
    assert_eq!(draft.saved(), Some(SavedSignature::Image(image)));
}
