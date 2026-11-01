//! `FullWindowTitle` text.

use super::FullWindowTitle;

// The name, subject and zoom in each of the four forms, the name's trailing "…" dropped, the
// percent rounded to a whole number.
#[test]
fn title_text_has_name_subject_and_zoom_in_each_form() {
    let ctx = egui::Context::default();
    let title = |name: &str| FullWindowTitle::new(name);
    assert_eq!(title("Liquify…").subject("Layer 1").zoom(66.4).text(&ctx), "Liquify (Layer 1, 66%)");
    assert_eq!(title("Liquify…").zoom(66.6).text(&ctx), "Liquify (67%)");
    assert_eq!(title("Adaptive Wide Angle").subject("Layer 1").text(&ctx), "Adaptive Wide Angle (Layer 1)");
    assert_eq!(title("Filter Gallery…").text(&ctx), "Filter Gallery");
    assert_eq!(title("Liquify").subject("").text(&ctx), "Liquify", "an empty subject is no subject");
}
