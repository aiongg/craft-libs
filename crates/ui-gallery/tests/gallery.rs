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
