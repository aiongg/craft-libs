use craft_ui_gallery::Gallery;
use egui::{Context, RawInput};

fn frame(ctx: &Context, gallery: &mut Gallery) {
    ctx.run_ui(RawInput::default(), |ui| gallery.show(ui)).drop_without_applying_deltas();
}

#[test]
fn every_story_renders_in_canvas_and_compare_views() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    let ids: Vec<&str> = gallery.stories().iter().map(|s| s.id).collect();
    assert!(!ids.is_empty());
    for compare in [false, true] {
        gallery.set_compare(compare);
        for id in &ids {
            gallery.select(id);
            frame(&ctx, &mut gallery);
        }
    }
}

#[test]
fn appearance_changes_render() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    frame(&ctx, &mut gallery);
    {
        let a = gallery.appearance_mut();
        a.dark = true;
        a.high_contrast = true;
        a.comfortable = true;
        a.rtl = true;
        a.text_size = 3;
        a.scale = 150;
    }
    frame(&ctx, &mut gallery);
    frame(&ctx, &mut gallery);
}

/// Width of `text` in the candidate face's regular family, with and without `tnum`.
fn digits_width(ctx: &Context, face: &str, text: &str, tnum: bool) -> f32 {
    use craft_ui::theme::tabular;
    use egui::text::{LayoutJob, TextFormat};
    let mut format = TextFormat { font_id: egui::FontId::new(20.0, craft_ui_gallery::faces::regular_family(face)), ..TextFormat::default() };
    if tnum {
        format = tabular(format);
    }
    let mut width = 0.0;
    ctx.run_ui(RawInput::default(), |ui| {
        width = ui.ctx().fonts_mut(|f| f.layout_job(LayoutJob::single_section(text.to_owned(), format.clone())).rect.width());
    })
    .drop_without_applying_deltas();
    width
}

#[test]
fn tnum_equalises_inter_digits() {
    use craft_ui_gallery::faces;
    if !faces::available() {
        return; // built without CRAFT_FONTS_DIR
    }
    let ctx = Context::default();
    faces::install(&ctx, "Inter", "JetBrains Mono");
    frame(&ctx, &mut Gallery::new(None)); // applies the fonts
    let (plain_1, plain_0) = (digits_width(&ctx, "Inter", "1111", false), digits_width(&ctx, "Inter", "0000", false));
    assert!(plain_1 > 0.0 && plain_1 != plain_0, "proportional digits differ: {plain_1} {plain_0}");
    assert_eq!(digits_width(&ctx, "Inter", "1111", true), digits_width(&ctx, "Inter", "0000", true));
}

#[test]
fn face_pickers_reinstall_the_fonts_and_the_specimen_renders() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.select("specimens/type/faces");
    for (ui_face, mono_face) in [("IBM Plex Sans", "IBM Plex Mono"), ("Source Sans 3", "Source Code Pro"), ("Inter", "JetBrains Mono")] {
        let a = gallery.appearance_mut();
        a.ui_face = ui_face.to_owned();
        a.mono_face = mono_face.to_owned();
        frame(&ctx, &mut gallery);
        frame(&ctx, &mut gallery);
    }
}
