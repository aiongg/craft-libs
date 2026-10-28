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

#[test]
fn token_browser_renders_in_both_modes_with_a_row_selected() {
    use craft_theme::{ColorToken, ComponentField, TokenRef};
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_tokens(true);
    let field = ComponentField::from_path("button.primary.text.rest").unwrap();
    for (dark, selected) in [(false, TokenRef::Color(ColorToken::TextFaint)), (true, TokenRef::Component(field))] {
        let a = gallery.appearance_mut();
        a.dark = dark;
        a.high_contrast = dark;
        gallery.select_token(selected);
        frame(&ctx, &mut gallery);
        frame(&ctx, &mut gallery);
    }
}

#[test]
fn the_inspector_numbers_the_focus_story_in_tab_order_in_both_views() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.select("primitives/focus-ring/focus");
    gallery.set_inspect(true);
    let items: Vec<String> = (1..=4).map(|i| format!("Item {i}")).collect();
    for compare in [false, true] {
        gallery.set_compare(compare);
        // The first pass turns AccessKit on, the second builds the tree, the third draws it.
        for _ in 0..3 {
            frame(&ctx, &mut gallery);
        }
        let badges = gallery.focus_badges();
        assert_eq!(badges.len(), if compare { 4 } else { 1 }, "one canvas, or one per compare column");
        assert!(badges.iter().all(|b| *b == items), "{badges:?}");
    }
    assert!(craft_ui::inspect::enabled(&ctx));
    gallery.set_tokens(true);
    frame(&ctx, &mut gallery);
    assert!(!craft_ui::inspect::enabled(&ctx), "the token browser has no inspector");
}

#[test]
fn every_control_in_the_bar_has_an_accessible_name() {
    use craft_ui::inspect;
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_inspect(true);
    for _ in 0..3 {
        frame(&ctx, &mut gallery);
    }
    let tree = inspect::tree(&ctx).expect("the AccessKit tree while inspecting");
    let nodes: Vec<_> = tree.walk(tree.root()).into_iter().map(|(id, _)| id).collect();
    let tokens = nodes.iter().copied().find(|id| tree.node(*id).and_then(|n| n.label()) == Some("Tokens")).expect("the Tokens toggle");
    let bar = nodes.iter().copied().find(|id| tree.node(*id).is_some_and(|n| n.children().contains(&tokens))).expect("the bar's Ui");
    let controls = tree.interactive(bar);
    assert!(controls.len() > 10, "the bar's controls: {}", controls.len());
    let unnamed: Vec<String> = controls
        .into_iter()
        .filter(|id| tree.name(*id).is_none())
        .filter_map(|id| tree.node(id))
        .map(|n| format!("{:?} at {:?}", n.role(), n.bounds()))
        .collect();
    assert!(unnamed.is_empty(), "unnamed controls in the bar: {unnamed:#?}");
}

#[test]
fn the_icon_set_picker_sets_the_active_set() {
    use craft_ui::icons::{self, IconSet};
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.select("primitives/icon/sizes");
    frame(&ctx, &mut gallery);
    assert_eq!(icons::active(&ctx), IconSet::Lucide);
    for set in IconSet::ALL {
        gallery.appearance_mut().icon_set = set.id().to_owned();
        frame(&ctx, &mut gallery);
        assert_eq!(icons::active(&ctx), set);
    }
    gallery.appearance_mut().icon_set = "no-such-set".to_owned();
    frame(&ctx, &mut gallery);
    assert_eq!(icons::active(&ctx), IconSet::Lucide, "an unknown saved id falls back to Lucide");
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
