use craft_theme::{ColorToken, Mode, SystemAppearance, TokenRef};
use craft_ui::inspect::AccessTree;
use craft_ui::story::registry;
use craft_ui::theme::{ToEgui, Tokens, TokensOf};
use egui::accesskit::Role;
use egui::{Color32, Context, Pos2, RawInput, Rect, vec2};

use super::{Cache, Key, Pass, Probe, Usage};
use crate::appearance::{Appearance, builtin};
use crate::preview::AppPreview;

fn theme(appearance: &Appearance) -> &'static craft_theme::ThemeDef {
    builtin(&appearance.theme).expect("a built-in theme")
}

fn button_primary(pass: Pass, size: egui::Vec2) -> Usage {
    let appearance = Appearance::default();
    let story = registry().into_iter().find(|s| s.id == "controls/button/variants").expect("the Button variants story");
    let mut args = (story.args)();
    Probe::story(theme(&appearance), &appearance, Mode::Light, pass, size, &story, &mut args)
}

fn path(p: &str) -> TokenRef {
    TokenRef::from_path(p).unwrap_or_else(|| panic!("{p} is a token"))
}

#[test]
fn the_primary_button_fill_is_a_semantic_token_in_pass_a_and_a_field_in_pass_b() {
    let size = vec2(480.0, 320.0);
    let a = button_primary(Pass::Semantic, size);
    let b = button_primary(Pass::Component, size);
    let fill = a.rects(path("accent.solid"));
    assert!(!fill.is_empty(), "accent.solid paints the primary fill: {:?}", a.tokens());
    let field = b.rects(path("button.primary.fill.rest"));
    assert!(!field.is_empty(), "button.primary.fill.rest paints it in pass B");
    assert!(field.iter().any(|r| fill.contains(r)), "the same rect: {fill:?} / {field:?}");
    assert!(b.rects(path("accent.solid")).iter().all(|r| !field.contains(r)));
}

#[test]
fn a_token_nothing_paints_has_no_uses_and_a_blend_is_computed() {
    let size = vec2(200.0, 100.0);
    let appearance = Appearance::default();
    let mut blended = None;
    let usage = Probe::run(theme(&appearance), &appearance, Mode::Light, Pass::Semantic, size, |ui| {
        let tokens = Tokens::of(ui);
        let (a, b) = (tokens.color.accent.solid.to_egui(), tokens.color.status.good.fg.to_egui());
        let mix = a.lerp_to_gamma(b, 0.5);
        blended = Some(mix);
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(50.0, 20.0)), 0.0, mix);
    });
    assert!(blended.is_some_and(|c| c != Color32::TRANSPARENT));
    assert_eq!(usage.computed(), 1);
    assert_eq!(usage.at(Pos2::new(20.0, 20.0)), None, "the blend is not a token");
    assert!(usage.tokens().is_empty());
    assert!(usage.rects(ColorToken::AccentSolid).is_empty());
}

#[test]
fn text_is_found_by_its_section_colour_and_by_an_override_colour() {
    let appearance = Appearance::default();
    let size = vec2(300.0, 100.0);
    let run = |pass| {
        Probe::run(theme(&appearance), &appearance, Mode::Light, pass, size, |ui| {
            let color = Tokens::of(ui).color.text.muted.to_egui();
            let galley = ui.painter().layout_no_wrap("muted".to_owned(), egui::FontId::proportional(14.0), color);
            ui.painter().galley(Pos2::new(10.0, 10.0), galley.clone(), Color32::PLACEHOLDER);
            let mut over = egui::epaint::TextShape::new(Pos2::new(10.0, 40.0), galley, Color32::PLACEHOLDER);
            over.override_text_color = Some(Tokens::of(ui).color.text.normal.to_egui());
            ui.painter().add(over);
        })
    };
    let usage = run(Pass::Semantic);
    assert_eq!(usage.at(Pos2::new(14.0, 18.0)), Some(TokenRef::Color(ColorToken::TextMuted)));
    assert_eq!(usage.at(Pos2::new(14.0, 48.0)), Some(TokenRef::Color(ColorToken::TextNormal)));
}

#[test]
fn two_runs_give_the_same_usage_inside_the_drawn_size() {
    let size = vec2(480.0, 320.0);
    let first = button_primary(Pass::Component, size);
    let second = button_primary(Pass::Component, size);
    assert_eq!(first, second);
    let bounds = Rect::from_min_size(Pos2::ZERO, size);
    assert!(first.uses.iter().all(|u| bounds.contains_rect(u.rect)), "every rect lies in the drawn size");
}

#[test]
fn the_cache_probes_once_per_key() {
    let appearance = Appearance::default();
    let key = Key { what: "x".to_owned(), args: String::new(), appearance, mode: Mode::Light, pass: Pass::Semantic, size: [10, 10] };
    let mut cache = Cache::default();
    let mut runs = 0;
    for _ in 0..3 {
        cache.get_or_run(key.clone(), || {
            runs += 1;
            Usage::default()
        });
    }
    assert_eq!(runs, 1);
    let other = Key { size: [20, 10], ..key };
    cache.get_or_run(other, || {
        runs += 1;
        Usage::default()
    });
    assert_eq!(runs, 2);
}

const APP_SIZE: egui::Vec2 = vec2(1280.0, 800.0);

fn app(pass: Pass) -> Usage {
    let appearance = Appearance::default();
    let mut preview = AppPreview::default();
    Probe::run(theme(&appearance), &appearance, Mode::Light, pass, APP_SIZE, |ui| preview.show(ui))
}

/// The bounds of the node named `name` with `role`, laid out as the probe lays out the app.
fn bounds_of(name: &str, role: Role) -> Rect {
    let appearance = Appearance::default();
    let ctx = Context::default();
    ctx.enable_accesskit();
    let base = appearance.configure(&ctx, None, false, SystemAppearance::default());
    let mut preview = AppPreview::default();
    let mut tree = None;
    for _ in 0..4 {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, APP_SIZE)), ..RawInput::default() };
        let output = ctx.run_ui(raw, |ui| {
            craft_ui::theme::install(ui.ctx(), &base);
            preview.show(ui);
        });
        tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update).or(tree);
        output.drop_without_applying_deltas();
    }
    let tree = tree.expect("an AccessKit tree");
    let id = tree
        .walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))
        .unwrap_or_else(|| panic!("a {role:?} named {name}"));
    tree.bounds(id).expect("bounds")
}

#[test]
fn the_mock_app_paints_chrome_tokens_over_a_layer_row_but_not_over_the_document() {
    let usage = app(Pass::Semantic);
    let row = bounds_of("Shapes", Role::ListBoxOption);
    let row_token = usage.at(row.center());
    assert!(row_token.is_some(), "a token paints the Layers row at {:?}: {row:?}", row.center());
    let doc = bounds_of("Document", Role::Canvas);
    let doc_token = usage.at(doc.center());
    assert_eq!(doc_token, None, "the document is not painted with a token");
}

#[test]
fn probe_times() {
    let appearance = Appearance::default();
    let theme = theme(&appearance);
    let stories = crate::gallery::stories();
    let mut components: Vec<&craft_ui::story::Story> = Vec::new();
    for s in &stories {
        if !components.iter().any(|c| c.component == s.component) {
            components.push(s);
        }
    }
    let start = std::time::Instant::now();
    Probe::run(theme, &appearance, Mode::Light, Pass::Component, vec2(10.0, 10.0), |_| {});
    println!("empty drawing (theme, context, fonts, two frames): {:?}", start.elapsed());
    for pass in [Pass::Semantic, Pass::Component] {
        let start = std::time::Instant::now();
        let mut slowest = (std::time::Duration::ZERO, "");
        for story in &components {
            let mut args = (story.args)();
            let one = std::time::Instant::now();
            Probe::story(theme, &appearance, Mode::Light, pass, vec2(480.0, 640.0), story, &mut args);
            slowest = slowest.max((one.elapsed(), story.id));
        }
        println!("{pass:?}: {} default stories in {:?}, slowest {:?} ({})", components.len(), start.elapsed(), slowest.0, slowest.1);
        let start = std::time::Instant::now();
        app(pass);
        println!("{pass:?}: mock app in {:?}", start.elapsed());
    }
}
