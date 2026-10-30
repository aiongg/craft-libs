use craft_theme::{ContrastChoice, ModeChoice};
use craft_ui_gallery::Gallery;
use craft_ui_gallery::layout::{CentreView, PanelId, VariantAxis};
use egui::{Context, RawInput};

/// The window the dock lays out in; an empty screen would draw no panel.
fn input() -> RawInput {
    RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..RawInput::default() }
}

fn frame(ctx: &Context, gallery: &mut Gallery) {
    ctx.run_ui(input(), |ui| gallery.show(ui)).drop_without_applying_deltas();
}

#[test]
fn every_level_renders_in_the_overview() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    assert_eq!(gallery.view(), CentreView::Overview);
    let ids: Vec<&str> = gallery.stories().iter().map(|s| s.id).collect();
    for id in ids {
        gallery.select(id);
        frame(&ctx, &mut gallery);
    }
}

#[test]
fn every_story_renders_in_canvas_and_variants_views() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_view(CentreView::Component);
    let ids: Vec<&str> = gallery.stories().iter().map(|s| s.id).collect();
    assert!(!ids.is_empty());
    for axis in [None, Some(VariantAxis::Theme), Some(VariantAxis::Mode), Some(VariantAxis::Contrast), Some(VariantAxis::Size)] {
        gallery.set_variants(axis);
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
        a.mode = ModeChoice::Dark;
        a.contrast = ContrastChoice::High;
        a.comfortable = true;
        a.rtl = true;
        a.text_size = 3;
        a.scale = 150;
    }
    frame(&ctx, &mut gallery);
    frame(&ctx, &mut gallery);
}

#[test]
fn the_system_choices_render_in_canvas_and_variants_views() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    let a = gallery.appearance_mut();
    a.mode = ModeChoice::System;
    a.contrast = ContrastChoice::System;
    frame(&ctx, &mut gallery);
    for axis in VariantAxis::ALL {
        gallery.set_variants(Some(axis));
        frame(&ctx, &mut gallery);
    }
    gallery.activate(PanelId::Tokens);
    frame(&ctx, &mut gallery);
}

#[test]
fn token_browser_renders_in_both_modes_with_a_row_selected() {
    use craft_theme::{ColorToken, ComponentField, TokenRef};
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.activate(PanelId::Tokens);
    let field = ComponentField::from_path("button.primary.text.rest").unwrap();
    for (dark, selected) in [(false, TokenRef::Color(ColorToken::TextFaint)), (true, TokenRef::Component(field))] {
        let a = gallery.appearance_mut();
        a.mode = if dark { ModeChoice::Dark } else { ModeChoice::Light };
        a.contrast = if dark { ContrastChoice::High } else { ContrastChoice::Standard };
        gallery.select_token(selected);
        frame(&ctx, &mut gallery);
        frame(&ctx, &mut gallery);
    }
}

#[test]
fn the_inspector_numbers_the_focus_story_in_tab_order() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_view(CentreView::Component);
    gallery.select("primitives/focus-ring/focus");
    gallery.set_inspect(true);
    let items: Vec<String> = (1..=4).map(|i| format!("Item {i}")).collect();
    // The first pass turns AccessKit on, the second builds the tree, the third draws it.
    for _ in 0..3 {
        frame(&ctx, &mut gallery);
    }
    let badges = gallery.focus_badges();
    assert_eq!(badges.len(), 1, "one canvas");
    assert!(badges.iter().all(|b| *b == items), "{badges:?}");
    // The inspector works on the Component view only.
    gallery.set_variants(Some(VariantAxis::Mode));
    frame(&ctx, &mut gallery);
    assert!(!craft_ui::inspect::enabled(&ctx), "Variants has no inspector");
    gallery.set_variants(None);
    for _ in 0..3 {
        frame(&ctx, &mut gallery);
    }
    assert!(craft_ui::inspect::enabled(&ctx));
    gallery.activate(PanelId::Tokens);
    frame(&ctx, &mut gallery);
    assert!(!craft_ui::inspect::enabled(&ctx), "the token browser has no inspector");
}

/// The AccessKit tree of one gallery frame.
fn tree_of(ctx: &Context, gallery: &mut Gallery) -> craft_ui::inspect::AccessTree {
    let output = ctx.run_ui(input(), |ui| gallery.show(ui));
    let tree = output.platform_output.accesskit_update.as_ref().and_then(craft_ui::inspect::AccessTree::from_update);
    output.drop_without_applying_deltas();
    tree.expect("an AccessKit tree")
}

/// The interactive nodes without a name in the `Ui` that holds the node named `anchor`.
fn unnamed_beside(tree: &craft_ui::inspect::AccessTree, anchor: &str) -> Vec<String> {
    let nodes: Vec<_> = tree.walk(tree.root()).into_iter().map(|(id, _)| id).collect();
    let anchor_id = nodes.iter().copied().find(|id| tree.name(*id).as_deref() == Some(anchor)).unwrap_or_else(|| panic!("no node named {anchor:?}"));
    let parent = nodes.iter().copied().find(|id| tree.node(*id).is_some_and(|n| n.children().contains(&anchor_id))).expect("the anchor's Ui");
    let controls = tree.interactive(parent);
    assert!(controls.len() > 1, "{anchor:?}'s Ui holds more than the anchor");
    controls
        .into_iter()
        .filter(|id| tree.name(*id).is_none())
        .filter_map(|id| tree.node(id))
        // egui's own scroll bars have no label to set; the List names its own.
        .filter(|n| n.role() != egui::accesskit::Role::ScrollBar)
        .map(|n| format!("{:?} at {:?} beside {anchor:?}", n.role(), n.bounds()))
        .collect()
}

/// The bar, the story tree and the args panel of every story: each control has a name.
#[test]
fn every_control_in_the_gallery_chrome_has_an_accessible_name() {
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let ids: Vec<&str> = gallery.stories().iter().map(|s| s.id).collect();
    let mut unnamed = Vec::new();
    for id in ids {
        gallery.select(id);
        frame(&ctx, &mut gallery);
        let tree = tree_of(&ctx, &mut gallery);
        for anchor in ["Theme", "Filter components"] {
            for missing in unnamed_beside(&tree, anchor) {
                unnamed.push(format!("{id}: {missing}"));
            }
        }
    }
    unnamed.dedup();
    assert!(unnamed.is_empty(), "unnamed controls in the gallery chrome: {unnamed:#?}");
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

#[test]
fn a_system_pick_installs_the_system_face_where_it_resolves_and_is_kept_where_not() {
    use craft_ui::fonts::{SYSTEM_UI_REGULAR, system_ui_face};
    use craft_ui_gallery::faces::{self, SYSTEM, UI_FACES};
    use egui::FontFamily;
    if !faces::available() {
        return; // built without CRAFT_FONTS_DIR
    }
    let face = system_ui_face();
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.select("specimens/type/faces");
    for stored in [SYSTEM, "Retired Sans"] {
        gallery.appearance_mut().ui_face = stored.to_owned();
        frame(&ctx, &mut gallery);
        frame(&ctx, &mut gallery);
        assert_eq!(gallery.appearance_mut().ui_face, stored, "the stored pick is kept, so it is saved as it was");
        let definitions = ctx.fonts(|f| f.definitions().clone());
        let installed = |family: &FontFamily, name: &str| {
            definitions.families.get(family).is_some_and(|chain| chain.iter().any(|n| n == name)) && definitions.font_data.contains_key(name)
        };
        let offered = faces::ui_faces(&ctx).contains(&SYSTEM);
        assert_eq!(offered, installed(&faces::regular_family(SYSTEM), SYSTEM_UI_REGULAR), "System is offered where its face is installed");
        assert_eq!(offered, face.is_some());
        let system_draws = definitions.families[&FontFamily::Proportional].first().map(String::as_str) == Some(SYSTEM_UI_REGULAR);
        assert_eq!(system_draws, stored == SYSTEM && offered, "{stored}: the system face heads the UI slot exactly where it is picked and offered");
        let first_candidate = definitions.families[&FontFamily::Proportional].iter().any(|n| n.starts_with(UI_FACES[0]));
        assert!(first_candidate, "{stored}: the first candidate is installed in the UI slot, behind the system face or in its place");
        let shown = if stored == SYSTEM && offered { SYSTEM } else { UI_FACES[0] };
        assert_eq!(faces::shown_ui_face(&ctx, stored), shown);
    }
}

#[test]
fn a_story_scale_applies_once_and_keyboard_zoom_stays() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_view(CentreView::Component);
    gallery.select("primitives/canvas/document");
    let run = |ctx: &Context, gallery: &mut Gallery, ppp: Option<f32>| {
        let mut input = input();
        input.viewports.entry(input.viewport_id).or_default().native_pixels_per_point = Some(1.0);
        ctx.run_ui(input, |ui| {
            gallery.show(ui);
            // A story's request is read at the end of the pass it is made in, so it is made after
            // `show` here and read by the next frame's `show`.
            if let Some(ppp) = ppp {
                craft_ui::story::request_pixels_per_point(ui.ctx(), ppp);
            }
        })
        .drop_without_applying_deltas();
    };
    // A zoom set in one pass takes effect at the start of the next, and a request is read one
    // frame after it is made.
    let settle = |ctx: &Context, gallery: &mut Gallery, ppp: Option<f32>| (0..3).for_each(|_| run(ctx, gallery, ppp));
    settle(&ctx, &mut gallery, None);
    let own = ctx.zoom_factor();
    // egui's Ctrl +/− sets the zoom factor; the gallery leaves it.
    ctx.set_zoom_factor(own * 1.1);
    settle(&ctx, &mut gallery, None);
    assert!((ctx.zoom_factor() - own * 1.1).abs() < 1e-4, "keyboard zoom kept: {}", ctx.zoom_factor());
    // A story asks for 2 pixels per point: applied once, then left alone too.
    settle(&ctx, &mut gallery, Some(2.0));
    assert!((ctx.zoom_factor() - 2.0).abs() < 1e-4, "the story's scale: {}", ctx.zoom_factor());
    ctx.set_zoom_factor(2.2);
    settle(&ctx, &mut gallery, Some(2.0));
    assert!((ctx.zoom_factor() - 2.2).abs() < 1e-4, "keyboard zoom over the story's scale kept: {}", ctx.zoom_factor());
    // The request ends: back to the appearance bar's scale.
    settle(&ctx, &mut gallery, None);
    assert!((ctx.zoom_factor() - own).abs() < 1e-4, "back to the gallery's scale: {}", ctx.zoom_factor());
}

/// Frames until the tree has settled, then the last one's AccessKit tree.
fn settled_tree(ctx: &Context, gallery: &mut Gallery) -> craft_ui::inspect::AccessTree {
    for _ in 0..3 {
        frame(ctx, gallery);
    }
    tree_of(ctx, gallery)
}

/// The nodes named `name` with the given role.
fn named(tree: &craft_ui::inspect::AccessTree, name: &str, role: egui::accesskit::Role) -> Vec<egui::accesskit::NodeId> {
    tree.walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))
        .collect()
}

/// Clicks the one control named `name` as a screen reader would, and runs the frame.
fn click(ctx: &Context, gallery: &mut Gallery, name: &str, role: egui::accesskit::Role) {
    use egui::accesskit::{Action, ActionRequest, TreeId};
    let tree = settled_tree(ctx, gallery);
    let nodes = named(&tree, name, role);
    assert_eq!(nodes.len(), 1, "one control named {name:?}");
    let mut raw = input();
    raw.events.push(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: nodes[0],
        data: None,
    }));
    ctx.run_ui(raw, |ui| gallery.show(ui)).drop_without_applying_deltas();
    frame(ctx, gallery);
}

#[test]
fn variants_show_one_card_per_value_and_mark_the_current_one() {
    use craft_theme::builtins;
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.select("controls/button/variants");
    let themes: Vec<String> = builtins().iter().map(|t| t.name.clone()).collect();
    let cases: [(VariantAxis, Vec<String>); 4] = [
        (VariantAxis::Theme, themes),
        (VariantAxis::Mode, vec!["Light".into(), "Dark".into()]),
        (VariantAxis::Contrast, vec!["Standard".into(), "High".into()]),
        (VariantAxis::Size, vec!["Compact".into(), "Comfortable".into()]),
    ];
    for (axis, names) in cases {
        gallery.set_variants(Some(axis));
        assert_eq!(gallery.view(), CentreView::Component, "Variants opens the Component view");
        let tree = settled_tree(&ctx, &mut gallery);
        for name in &names {
            assert_eq!(named(&tree, name, Role::Button).len(), 1, "{axis:?}: a card for {name:?}");
        }
        assert_eq!(named(&tree, "shown now", Role::Label).len() + named(&tree, "shown now", Role::GenericContainer).len(), 1, "{axis:?}");
    }
}

#[test]
fn choosing_a_card_applies_its_value_and_closes_variants() {
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.select("controls/button/variants");
    gallery.set_variants(Some(VariantAxis::Mode));
    click(&ctx, &mut gallery, "Dark", egui::accesskit::Role::Button);
    assert_eq!(gallery.appearance_mut().mode, ModeChoice::Dark);
    assert_eq!(gallery.variants(), None);
    assert_eq!(gallery.view(), CentreView::Component);
}

#[test]
fn escape_closes_variants_and_the_overview_has_none() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_variants(Some(VariantAxis::Size));
    frame(&ctx, &mut gallery);
    assert_eq!(gallery.variants(), Some(VariantAxis::Size));
    let mut raw = input();
    raw.events.push(egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
    ctx.run_ui(raw, |ui| gallery.show(ui)).drop_without_applying_deltas();
    assert_eq!(gallery.variants(), None);
    gallery.set_variants(Some(VariantAxis::Size));
    gallery.set_view(CentreView::Overview);
    frame(&ctx, &mut gallery);
    assert_eq!(gallery.variants(), None, "the Overview closes Variants");
}

#[test]
fn the_more_popover_holds_the_moved_settings() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let tree = settled_tree(&ctx, &mut gallery);
    for label in ["Direction", "Reduced motion", "Scale"] {
        assert!(tree.walk(tree.root()).iter().all(|(id, _)| tree.name(*id).as_deref() != Some(label)), "{label} is not in the bar");
    }
    assert_eq!(named(&tree, "More appearance settings", Role::Button).len(), 1);
    click(&ctx, &mut gallery, "More appearance settings", Role::Button);
    let tree = settled_tree(&ctx, &mut gallery);
    for label in ["Direction", "Reduced motion", "Text", "Icons", "Scale"] {
        assert!(tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).as_deref() == Some(label)), "{label} is in the popover");
    }
    assert!(!gallery.appearance_mut().rtl);
    click(&ctx, &mut gallery, "RTL", Role::RadioButton);
    assert!(gallery.appearance_mut().rtl, "Direction in the popover sets the gallery's");
}
