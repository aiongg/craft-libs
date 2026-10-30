use craft_theme::{ContrastChoice, ModeChoice};
use craft_ui_gallery::Gallery;
use craft_ui_gallery::layout::{GalleryWorkspace, PanelId, PreviewView, VariantAxis};
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
    assert_eq!(gallery.front_tab(), PanelId::View);
    let ids: Vec<&str> = gallery.stories().iter().map(|s| s.id).collect();
    for id in ids {
        gallery.select(id);
        frame(&ctx, &mut gallery);
    }
}

#[test]
fn every_story_renders_in_its_component_tab_and_in_variants_views() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    let mut components: Vec<&str> = Vec::new();
    for story in gallery.stories() {
        if !components.contains(&story.component) {
            components.push(story.component);
        }
    }
    assert!(!components.is_empty());
    for component in components {
        let ids: Vec<&str> = gallery.stories().iter().filter(|s| s.component == component).map(|s| s.id).collect();
        gallery.open_component(component);
        for axis in [None, Some(VariantAxis::Theme), Some(VariantAxis::Mode), Some(VariantAxis::Contrast), Some(VariantAxis::Size)] {
            gallery.set_variants(axis);
            for id in &ids {
                gallery.select(id);
                frame(&ctx, &mut gallery);
            }
        }
        gallery.set_variants(None);
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
    gallery.select("controls/button/variants");
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
    // Variants has no inspector.
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
    gallery.open_component("Canvas");
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
        assert_eq!(gallery.variants(), Some(axis));
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
}

#[test]
fn escape_closes_variants_and_so_does_fronting_another_centre_tab() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.set_variants(Some(VariantAxis::Size));
    assert_eq!(gallery.variants(), None, "nothing is selected to repeat");
    gallery.select("controls/button/variants");
    gallery.set_variants(Some(VariantAxis::Size));
    frame(&ctx, &mut gallery);
    assert_eq!(gallery.variants(), Some(VariantAxis::Size));
    let mut raw = input();
    raw.events.push(egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
    ctx.run_ui(raw, |ui| gallery.show(ui)).drop_without_applying_deltas();
    assert_eq!(gallery.variants(), None);
    gallery.open_component("Button");
    gallery.set_variants(Some(VariantAxis::Size));
    frame(&ctx, &mut gallery);
    assert_eq!(gallery.variants(), Some(VariantAxis::Size), "Variants shows in the Button tab");
    gallery.activate(PanelId::View);
    frame(&ctx, &mut gallery);
    assert_eq!(gallery.variants(), None, "fronting the Overview closes Variants");
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

#[test]
fn the_theme_workspace_renders_both_previews_at_the_default_size() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.set_workspace(GalleryWorkspace::Theme);
    for preview in [PreviewView::App, PreviewView::Components] {
        gallery.set_preview(preview);
        let tree = settled_tree(&ctx, &mut gallery);
        for tab in ["Preview", "Theme", "Problems", "Changes"] {
            assert_eq!(named(&tree, tab, Role::Tab).len(), 1, "{tab} tab");
        }
    }
}

/// What `gallery` saves under `key`.
fn saved_under(gallery: &mut Gallery, key: &str) -> Option<String> {
    #[derive(Default)]
    struct Memory(std::collections::HashMap<String, String>);
    impl eframe::Storage for Memory {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }
    let mut storage = Memory::default();
    eframe::App::save(gallery, &mut storage);
    eframe::Storage::get_string(&storage, key)
}

#[test]
fn the_workspace_switch_changes_workspace_and_reset_layout_resets_the_shown_one_only() {
    const LAYOUT_KEY: &str = "craft-ui-gallery.layout.v2";
    const THEME_LAYOUT_KEY: &str = "craft-ui-gallery.theme-layout.v1";
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let default_theme = saved_under(&mut Gallery::new(None), THEME_LAYOUT_KEY);
    gallery.activate(PanelId::Tokens);
    click(&ctx, &mut gallery, "Theme", Role::RadioButton);
    assert_eq!(gallery.workspace(), GalleryWorkspace::Theme);
    gallery.activate(PanelId::Changes);
    let (components, theme) = (saved_under(&mut gallery, LAYOUT_KEY), saved_under(&mut gallery, THEME_LAYOUT_KEY));
    assert_ne!(theme, default_theme);
    click(&ctx, &mut gallery, "Reset layout", Role::Button);
    assert_eq!(saved_under(&mut gallery, THEME_LAYOUT_KEY), default_theme);
    assert_eq!(saved_under(&mut gallery, LAYOUT_KEY), components, "the Components layout is untouched");
}

#[test]
fn opening_a_component_from_the_preview_closes_variants_and_fronts_its_tab() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.select("controls/button/variants");
    gallery.set_variants(Some(VariantAxis::Mode));
    gallery.activate(PanelId::Tokens);
    gallery.set_workspace(GalleryWorkspace::Theme);
    gallery.set_preview(PreviewView::Components);
    let tree = settled_tree(&ctx, &mut gallery);
    let component = gallery
        .stories()
        .iter()
        .filter(|s| s.level == craft_ui::story::Level::Controls)
        .map(|s| s.component)
        .find(|name| named(&tree, &format!("Open {name}"), Role::Button).len() == 1)
        .expect("a Controls tile Open button");
    click(&ctx, &mut gallery, &format!("Open {component}"), Role::Button);
    assert_eq!(gallery.workspace(), GalleryWorkspace::Components);
    assert_eq!(gallery.variants(), None);
    assert_eq!(gallery.open_tabs(), [component]);
    assert!(gallery.is_front(PanelId::Component(component)));
}

#[test]
fn the_inspector_is_off_in_the_theme_workspace() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    gallery.select("controls/button/variants");
    gallery.set_inspect(true);
    for _ in 0..3 {
        frame(&ctx, &mut gallery);
    }
    assert!(craft_ui::inspect::enabled(&ctx));
    gallery.set_workspace(GalleryWorkspace::Theme);
    frame(&ctx, &mut gallery);
    assert!(!craft_ui::inspect::enabled(&ctx), "the Theme workspace has no inspector");
    gallery.set_workspace(GalleryWorkspace::Components);
    for _ in 0..3 {
        frame(&ctx, &mut gallery);
    }
    assert!(craft_ui::inspect::enabled(&ctx));
}

#[test]
fn a_preview_tile_link_opens_the_component_in_the_components_workspace() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.set_workspace(GalleryWorkspace::Theme);
    gallery.set_preview(PreviewView::Components);
    let tree = settled_tree(&ctx, &mut gallery);
    let component = gallery
        .stories()
        .iter()
        .filter(|s| s.level == craft_ui::story::Level::Controls)
        .map(|s| s.component)
        .find(|name| named(&tree, &format!("Open {name}"), Role::Button).len() == 1)
        .expect("a Controls tile Open button");
    click(&ctx, &mut gallery, &format!("Open {component}"), Role::Button);
    assert_eq!(gallery.workspace(), GalleryWorkspace::Components);
    assert_eq!(gallery.front_tab(), PanelId::Component(component));
    assert_eq!(gallery.selected().and_then(|id| gallery.stories().iter().find(|s| s.id == id)).map(|s| s.component), Some(component));
}

/// A built-in theme with a pair below its minimum in Standard contrast, compact, light, and
/// that pair's row label and foreground.
fn theme_with_a_problem() -> (String, String, craft_theme::TokenRef) {
    use craft_theme::report::contrast_report;
    use craft_theme::{Contrast, Mode, Size};
    craft_theme::builtins()
        .iter()
        .find_map(|t| {
            let report = contrast_report(t);
            let w = report.for_appearance(None, Mode::Light, Contrast::Standard, Size::Compact)?.warnings.first().copied()?;
            Some((t.id.as_str().to_owned(), format!("{} on {}", w.fg.path(), w.bg.path()), w.fg))
        })
        .expect("a built-in theme with a contrast warning")
}

fn problems_gallery(theme: &str) -> Gallery {
    let mut gallery = Gallery::new(None);
    gallery.set_workspace(GalleryWorkspace::Theme);
    let a = gallery.appearance_mut();
    a.theme = theme.to_owned();
    a.mode = ModeChoice::Light;
    a.contrast = ContrastChoice::Standard;
    a.comfortable = false;
    gallery
}

#[test]
fn problems_lists_the_warnings_of_the_shown_appearance_and_a_click_selects_the_token() {
    use egui::accesskit::Role;
    let (theme, label, fg) = theme_with_a_problem();
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = problems_gallery(&theme);
    let tree = settled_tree(&ctx, &mut gallery);
    assert_eq!(named(&tree, &label, Role::ListBoxOption).len(), 1, "the pair is listed");
    assert!(
        tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).is_some_and(|n| n.starts_with("Light: ") && n.ends_with("below minimum"))),
        "Standard contrast reports pairs below minimum"
    );
    assert_ne!(gallery.selected_token(), Some(fg));
    click(&ctx, &mut gallery, &label, Role::ListBoxOption);
    assert_eq!(gallery.selected_token(), Some(fg));
    assert!(gallery.is_front(PanelId::Theme));
    gallery.activate(PanelId::Problems);
    let tree = settled_tree(&ctx, &mut gallery);
    let row = named(&tree, &label, Role::ListBoxOption);
    assert_eq!(row.len(), 1);
    assert!(tree.node(row[0]).is_some_and(|n| n.is_selected() == Some(true)), "the clicked row stays selected");
    gallery.appearance_mut().contrast = ContrastChoice::High;
    let tree = settled_tree(&ctx, &mut gallery);
    assert!(
        tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).is_some_and(|n| n.starts_with("Light: ") && !n.contains("below minimum"))),
        "High contrast does not say below minimum"
    );
}

#[test]
fn the_tokens_tab_and_the_theme_panel_share_the_selection() {
    use craft_theme::{ColorToken, TokenRef};
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    gallery.activate(PanelId::Tokens);
    let token = TokenRef::Color(ColorToken::TextFaint);
    gallery.select_token(token);
    let shows =
        |tree: &craft_ui::inspect::AccessTree| tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).as_deref() == Some(token.path().as_str()));
    for workspace in [GalleryWorkspace::Components, GalleryWorkspace::Theme, GalleryWorkspace::Components] {
        gallery.set_workspace(workspace);
        assert!(shows(&settled_tree(&ctx, &mut gallery)), "the detail of {} in {workspace:?}", token.path());
        assert_eq!(gallery.selected_token(), Some(token));
    }
}

/// The nodes named `name` with `role`, leftmost first.
fn named_left_to_right(tree: &craft_ui::inspect::AccessTree, name: &str, role: egui::accesskit::Role) -> Vec<egui::accesskit::NodeId> {
    let mut nodes = named(tree, name, role);
    nodes.sort_by(|a, b| {
        let x = |id: &egui::accesskit::NodeId| tree.node(*id).and_then(|n| n.bounds()).map_or(0.0, |b| b.x0);
        x(a).total_cmp(&x(b))
    });
    nodes
}

/// Clicks the `nth` node (leftmost first) named `name` with `role`, as a screen reader would.
fn click_nth(ctx: &Context, gallery: &mut Gallery, name: &str, role: egui::accesskit::Role, nth: usize) {
    use egui::accesskit::{Action, ActionRequest, TreeId};
    let tree = settled_tree(ctx, gallery);
    let nodes = named_left_to_right(&tree, name, role);
    assert!(nodes.len() > nth, "{} controls named {name:?}", nodes.len());
    let mut raw = input();
    raw.events.push(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: nodes[nth],
        data: None,
    }));
    ctx.run_ui(raw, |ui| gallery.show(ui)).drop_without_applying_deltas();
    frame(ctx, gallery);
}

/// A component with several stories: its name and the ids of its stories. It is the first by name
/// in the first level that has one, so its row is near the top of the list.
fn component_with_stories(gallery: &Gallery) -> (&'static str, Vec<&'static str>) {
    let stories = gallery.stories();
    let several = |c: &str| stories.iter().filter(|s| s.component == c).count() > 1;
    let level = craft_ui::story::Level::ALL.into_iter().find(|l| stories.iter().any(|s| s.level == *l && several(s.component))).expect("a level");
    let component =
        stories.iter().filter(|s| s.level == level && several(s.component)).map(|s| s.component).min_by_key(|c| c.to_lowercase()).unwrap();
    (component, stories.iter().filter(|s| s.component == component).map(|s| s.id).collect())
}

/// The first component by name in `component`'s level other than `component`.
fn neighbour_of(gallery: &Gallery, component: &str) -> &'static str {
    let stories = gallery.stories();
    let level = stories.iter().find(|s| s.component == component).map(|s| s.level).unwrap();
    stories.iter().filter(|s| s.level == level && s.component != component).map(|s| s.component).min_by_key(|c| c.to_lowercase()).unwrap()
}

fn story_name(gallery: &Gallery, id: &str) -> &'static str {
    gallery.stories().iter().find(|s| s.id == id).map(|s| s.name).expect("a story")
}

#[test]
fn a_list_row_click_selects_the_component_in_the_overview_without_opening_a_tab() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let (component, _) = component_with_stories(&gallery);
    let other = gallery.stories().iter().find(|s| s.component != component).unwrap().component;
    gallery.open_component(other);
    assert_eq!(gallery.front_tab(), PanelId::Component(other));
    click(&ctx, &mut gallery, component, Role::ListBoxOption);
    assert_eq!(gallery.overview_selected(), Some(component));
    assert_eq!(gallery.front_tab(), PanelId::View);
    assert_eq!(gallery.open_tabs(), [other], "no tab was added or replaced");
}

#[test]
fn open_on_a_row_and_open_on_a_tile_open_one_tab_between_them() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let (component, _) = component_with_stories(&gallery);
    click(&ctx, &mut gallery, component, Role::ListBoxOption);
    let open = format!("Open {component}");
    let tree = settled_tree(&ctx, &mut gallery);
    assert_eq!(named(&tree, &open, Role::Button).len(), 2, "the selected row and its tile each have Open");
    click_nth(&ctx, &mut gallery, &open, Role::Button, 0);
    assert_eq!(gallery.open_tabs(), [component]);
    assert_eq!(gallery.front_tab(), PanelId::Component(component));
    gallery.activate(PanelId::View);
    click_nth(&ctx, &mut gallery, &open, Role::Button, 1);
    assert_eq!(gallery.open_tabs(), [component], "a second Open fronts the one tab");
    assert_eq!(gallery.front_tab(), PanelId::Component(component));
}

/// Opens the context menu of the tab named `name`, as a secondary click does.
fn open_tab_menu(ctx: &Context, gallery: &mut Gallery, name: &str) {
    use egui::accesskit::Role;
    let tree = settled_tree(ctx, gallery);
    let tab = named(&tree, name, Role::Tab);
    assert_eq!(tab.len(), 1, "one tab named {name:?}");
    let b = tree.node(tab[0]).and_then(|n| n.bounds()).expect("the tab's bounds");
    let at = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    for pressed in [None, Some(true), Some(false)] {
        let mut raw = input();
        raw.events.push(egui::Event::PointerMoved(at));
        if let Some(pressed) = pressed {
            raw.events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        ctx.run_ui(raw, |ui| gallery.show(ui)).drop_without_applying_deltas();
    }
}

#[test]
fn closing_a_tab_fronts_the_overview_and_drops_the_story() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let (component, ids) = component_with_stories(&gallery);
    let other = gallery.stories().iter().find(|s| s.component != component).unwrap().component;
    gallery.open_component(other);
    gallery.open_component(component);
    gallery.select(ids[1]);
    assert_eq!(gallery.selected(), Some(ids[1]));
    open_tab_menu(&ctx, &mut gallery, component);
    click(&ctx, &mut gallery, "Close Panel", Role::Button);
    assert_eq!(gallery.open_tabs(), [other]);
    assert_eq!(gallery.front_tab(), PanelId::View);
    gallery.open_component(component);
    assert_eq!(gallery.selected(), gallery.stories().iter().find(|s| s.component == component).map(|s| s.id), "the story was forgotten");
}

#[test]
fn a_tile_click_moves_the_side_panels_and_a_story_pick_changes_the_tile() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let (component, ids) = component_with_stories(&gallery);
    let neighbour = neighbour_of(&gallery, component);
    let tree = settled_tree(&ctx, &mut gallery);
    assert_eq!(gallery.selected(), None);
    assert!(
        named(&tree, "Select a component", Role::Label).len() + named(&tree, "Select a component", Role::GenericContainer).len() >= 2,
        "Stories and Properties ask for a selection"
    );
    gallery.select(ids[0]);
    click(&ctx, &mut gallery, &format!("Select {neighbour}"), Role::Button);
    assert_eq!(gallery.overview_selected(), Some(neighbour));
    click(&ctx, &mut gallery, &format!("Select {component}"), Role::Button);
    assert_eq!(gallery.overview_selected(), Some(component));
    assert_eq!(gallery.selected(), Some(ids[0]));
    let tree = settled_tree(&ctx, &mut gallery);
    for id in &ids {
        assert_eq!(named(&tree, story_name(&gallery, id), Role::ListBoxOption).len(), 1, "Stories lists {id}");
    }
    let second = story_name(&gallery, ids[1]);
    click(&ctx, &mut gallery, second, Role::ListBoxOption);
    assert_eq!(gallery.selected(), Some(ids[1]));
    let tree = settled_tree(&ctx, &mut gallery);
    let note = format!("· {second}");
    assert_eq!(named(&tree, &note, Role::Label).len() + named(&tree, &note, Role::GenericContainer).len(), 1, "the tile names the story it shows");
}

#[test]
fn a_component_tab_has_a_tile_per_story_and_a_stories_row_selects_its_tile() {
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let (component, ids) = component_with_stories(&gallery);
    gallery.open_component(component);
    let tree = settled_tree(&ctx, &mut gallery);
    for id in &ids {
        let name = format!("Select {}", story_name(&gallery, id));
        assert_eq!(named(&tree, &name, Role::Button).len(), 1, "a tile for {id}");
    }
    let (second, first) = (story_name(&gallery, ids[1]), format!("Select {}", story_name(&gallery, ids[0])));
    click(&ctx, &mut gallery, second, Role::ListBoxOption);
    assert_eq!(gallery.selected(), Some(ids[1]));
    click(&ctx, &mut gallery, &first, Role::Button);
    assert_eq!(gallery.selected(), Some(ids[0]), "a tile click selects its story");
    let tree = settled_tree(&ctx, &mut gallery);
    let ring = |id: &str| {
        let name = format!("Select {}", story_name(&gallery, id));
        named(&tree, &name, Role::Button).first().and_then(|n| tree.node(*n)).and_then(|n| n.toggled())
    };
    assert_eq!(ring(ids[0]), Some(egui::accesskit::Toggled::True));
    assert_eq!(ring(ids[1]), Some(egui::accesskit::Toggled::False));
}

#[test]
fn reset_shows_on_the_tile_and_in_properties_only_while_args_differ() {
    use craft_ui::story::ArgValue;
    use egui::accesskit::Role;
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut gallery = Gallery::new(None);
    let story = *gallery
        .stories()
        .iter()
        .find(|s| s.level == craft_ui::story::Level::Controls && s.defaults().iter().any(|a| matches!(a.value, ArgValue::Bool(_))))
        .expect("a controls story with a bool arg");
    gallery.select(story.id);
    let reset_tile = format!("Reset {}", story.component);
    let tree = settled_tree(&ctx, &mut gallery);
    assert!(named(&tree, &reset_tile, Role::Button).is_empty() && named(&tree, "Reset args", Role::Button).is_empty(), "defaults: no Reset");
    {
        let args = gallery.args_of_story(story.id).unwrap();
        let arg = args.iter_mut().find(|a| matches!(a.value, ArgValue::Bool(_))).unwrap();
        if let ArgValue::Bool(b) = &mut arg.value {
            *b = !*b;
        }
    }
    let tree = settled_tree(&ctx, &mut gallery);
    assert_eq!(named(&tree, &reset_tile, Role::Button).len(), 1);
    assert_eq!(named(&tree, "Reset args", Role::Button).len(), 1);
    click(&ctx, &mut gallery, &reset_tile, Role::Button);
    assert_eq!(*gallery.args_of_story(story.id).unwrap(), story.defaults());
    let tree = settled_tree(&ctx, &mut gallery);
    assert!(named(&tree, &reset_tile, Role::Button).is_empty() && named(&tree, "Reset args", Role::Button).is_empty());
}

#[test]
fn the_inspector_works_on_the_selected_tile_of_a_component_tab() {
    let ctx = Context::default();
    let mut gallery = Gallery::new(None);
    let (component, ids) = component_with_stories(&gallery);
    gallery.open_component(component);
    gallery.select(ids[1]);
    gallery.set_inspect(true);
    for _ in 0..4 {
        frame(&ctx, &mut gallery);
    }
    assert!(craft_ui::inspect::enabled(&ctx));
    assert_eq!(gallery.focus_badges().len(), 1, "one tile is inspected");
}
