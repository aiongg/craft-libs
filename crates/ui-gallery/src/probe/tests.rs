use craft_theme::{ColorToken, Mode, SystemAppearance, TokenRef};
use craft_ui::inspect::AccessTree;
use craft_ui::story::registry;
use craft_ui::theme::{ToEgui, Tokens, TokensOf};
use egui::accesskit::Role;
use egui::{Color32, Context, Pos2, RawInput, Rect, Ui, vec2};
use std::sync::Arc;

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
    let usage = Probe::run(
        theme(&appearance),
        &appearance,
        Mode::Light,
        Pass::Semantic,
        size,
        |_| {},
        |ui| {
            let tokens = Tokens::of(ui);
            let (a, b) = (tokens.color.accent.solid.to_egui(), tokens.color.status.good.fg.to_egui());
            let mix = a.lerp_to_gamma(b, 0.5);
            blended = Some(mix);
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(50.0, 20.0)), 0.0, mix);
        },
    );
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
        Probe::run(
            theme(&appearance),
            &appearance,
            Mode::Light,
            pass,
            size,
            |_| {},
            |ui| {
                let color = Tokens::of(ui).color.text.muted.to_egui();
                let galley = ui.painter().layout_no_wrap("muted".to_owned(), egui::FontId::proportional(14.0), color);
                ui.painter().galley(Pos2::new(10.0, 10.0), galley.clone(), Color32::PLACEHOLDER);
                let mut over = egui::epaint::TextShape::new(Pos2::new(10.0, 40.0), galley, Color32::PLACEHOLDER);
                over.override_text_color = Some(Tokens::of(ui).color.text.normal.to_egui());
                ui.painter().add(over);
            },
        )
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
    assert!(first.uses.iter().all(|u| bounds.contains_rect(u.visible())), "every rect lies in the drawn size");
}

#[test]
fn the_cache_probes_once_per_key() {
    let appearance = Appearance::default();
    let theme = theme(&appearance);
    let key = Key::new(theme, "x", "", &appearance, Mode::Light, Pass::Semantic, vec2(10.0, 10.0));
    let mut cache = Cache::default();
    let mut runs = 0;
    for _ in 0..3 {
        cache.get_or_run(key.clone(), || {
            runs += 1;
            Usage::default()
        });
    }
    assert_eq!(runs, 1);
    let other = Key { size: [20, 10], ..key.clone() };
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
    Probe::run(theme(&appearance), &appearance, Mode::Light, pass, APP_SIZE, |_| {}, |ui| preview.show(ui))
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
    assert_eq!(usage.at(row.center()), Some(path("state.selected")), "the selected Layers row at {:?}", row.center());
    let doc = bounds_of("Document", Role::Canvas);
    let doc_token = usage.at(doc.center());
    assert_eq!(doc_token, None, "the document is not painted with a token");
}

const SMALL: egui::Vec2 = vec2(300.0, 200.0);

fn token_color(ui: &Ui, field: impl Fn(&craft_ui::theme::UiTokens) -> craft_theme::Rgba) -> Color32 {
    field(&Tokens::of(ui)).to_egui()
}

fn small(setup: impl FnOnce(&Context), draw: impl FnMut(&mut Ui)) -> Usage {
    let appearance = Appearance::default();
    Probe::run(theme(&appearance), &appearance, Mode::Light, Pass::Semantic, SMALL, setup, draw)
}

#[test]
fn a_story_is_probed_without_its_surface_frame() {
    let appearance = Appearance::default();
    let size = vec2(480.0, 320.0);
    let stories = registry();
    let story = stories
        .iter()
        .find(|s| matches!(s.surface, craft_ui::story::Surface::Canvas | craft_ui::story::Surface::Chrome))
        .expect("a story on a bar surface");
    let usage = Probe::story(theme(&appearance), &appearance, Mode::Light, Pass::Semantic, size, story, &mut (story.args)());
    let whole = Rect::from_min_size(Pos2::ZERO, size);
    assert!(
        !usage.uses.iter().any(|u| u.rect.contains_rect(whole) || (u.token.is_some() && u.rect.width() >= size.x && u.rect.height() >= size.y)),
        "no shape of the frame is recorded"
    );
    let button = button_primary(Pass::Semantic, size);
    let fill = button.rects(path("accent.solid"));
    assert!(fill.iter().all(|r| r.min.x > 0.0 && r.min.y > 0.0), "the surface margin offsets the story as in a tile: {fill:?}");
}

#[test]
fn a_popup_over_content_is_found_before_what_is_below_it() {
    let popup = egui::Id::NULL.with("probe-test-popup");
    let draw = |ui: &mut Ui| {
        let below = token_color(ui, |t| t.color.surface.panel);
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(100.0, 100.0)), 0.0, below);
        if ui.ctx().data(|d| d.get_temp::<bool>(popup)).unwrap_or(false) {
            let above = token_color(ui, |t| t.color.accent.solid);
            egui::Area::new(popup).order(egui::Order::Foreground).fade_in(false).fixed_pos(Pos2::new(60.0, 60.0)).show(ui.ctx(), |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(100.0, 100.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, above);
            });
        }
    };
    let closed = small(|_| {}, draw);
    assert_eq!(closed.at(Pos2::new(80.0, 80.0)), Some(TokenRef::Color(ColorToken::SurfacePanel)));
    let open = small(
        |ctx| {
            ctx.data_mut(|d| {
                d.insert_temp(popup, true);
            })
        },
        draw,
    );
    assert_eq!(open.at(Pos2::new(80.0, 80.0)), Some(TokenRef::Color(ColorToken::AccentSolid)), "the setup hook opened the popup");
    assert_eq!(open.at(Pos2::new(20.0, 20.0)), Some(TokenRef::Color(ColorToken::SurfacePanel)));
}

#[test]
fn a_scroll_area_clips_what_it_paints() {
    let usage = small(
        |_| {},
        |ui| {
            let backdrop = token_color(ui, |t| t.color.surface.panel);
            let content = token_color(ui, |t| t.color.accent.solid);
            ui.painter().rect_filled(Rect::from_min_size(Pos2::ZERO, SMALL), 0.0, backdrop);
            egui::ScrollArea::vertical().max_height(50.0).auto_shrink([false, false]).show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(100.0, 300.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, content);
            });
        },
    );
    assert_eq!(usage.at(Pos2::new(20.0, 20.0)), Some(TokenRef::Color(ColorToken::AccentSolid)));
    assert_eq!(usage.at(Pos2::new(20.0, 150.0)), Some(TokenRef::Color(ColorToken::SurfacePanel)), "below the clip, inside the unclipped rect");
    let used = usage.uses.iter().find(|u| u.token == Some(TokenRef::Color(ColorToken::AccentSolid))).expect("the content");
    assert!(used.rect.height() >= 300.0, "the rect is not cut: {:?}", used.rect);
    assert!(used.visible().height() <= 100.0, "the visible part is: {:?}", used.visible());
}

#[test]
fn a_computed_paint_over_a_token_is_found_as_computed() {
    let usage = small(
        |_| {},
        |ui| {
            let base = token_color(ui, |t| t.color.surface.panel);
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(100.0, 100.0)), 0.0, base);
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(60.0, 60.0), vec2(100.0, 100.0)), 0.0, base.gamma_multiply(0.5));
        },
    );
    assert_eq!(usage.at(Pos2::new(20.0, 20.0)), Some(TokenRef::Color(ColorToken::SurfacePanel)));
    assert_eq!(usage.at(Pos2::new(80.0, 80.0)), None);
    assert!(usage.use_at(Pos2::new(80.0, 80.0)).is_some_and(|u| u.token.is_none()), "a computed paint, not nothing");
    assert!(usage.use_at(Pos2::new(250.0, 190.0)).is_none(), "nothing is painted there");
}

#[test]
fn a_ring_does_not_claim_what_it_encloses() {
    let usage = small(
        |_| {},
        |ui| {
            let fill = token_color(ui, |t| t.color.surface.panel);
            let ring = token_color(ui, |t| t.color.accent.solid);
            let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(100.0, 100.0));
            ui.painter().rect_filled(rect, 0.0, fill);
            ui.painter().circle_stroke(rect.center(), 40.0, egui::Stroke::new(3.0, ring));
        },
    );
    let accent = Some(TokenRef::Color(ColorToken::AccentSolid));
    assert_eq!(usage.at(Pos2::new(60.0, 60.0)), Some(TokenRef::Color(ColorToken::SurfacePanel)), "inside the ring");
    assert_eq!(usage.at(Pos2::new(60.0, 21.0)), accent, "on the ring");
}

#[test]
fn a_paint_callback_is_computed() {
    let usage = small(
        |_| {},
        |ui| {
            let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(50.0, 50.0));
            ui.painter().add(egui::PaintCallback { rect, callback: Arc::new(()) });
        },
    );
    assert!(usage.use_at(Pos2::new(20.0, 20.0)).is_some_and(|u| u.token.is_none()));
}

#[test]
fn a_probe_copy_of_the_app_has_the_flyout_open_that_the_live_one_has() {
    let appearance = Appearance::default();
    let ctx = Context::default();
    let base = appearance.configure(&ctx, None, false, SystemAppearance::default());
    let mut live = AppPreview::default();
    live.open_history_flyout();
    for _ in 0..4 {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, APP_SIZE)), ..RawInput::default() };
        let output = ctx.run_ui(raw, |ui| {
            craft_ui::theme::install(ui.ctx(), &base);
            live.show(ui);
        });
        output.drop_without_applying_deltas();
    }
    let probe =
        |mut preview: AppPreview| Probe::run(theme(&appearance), &appearance, Mode::Light, Pass::Semantic, APP_SIZE, |_| {}, |ui| preview.show(ui));
    let closed = probe(live.clone());
    let open = probe(live.for_probe());
    assert!(open.uses.len() > closed.uses.len(), "the flyout adds paint: {} / {}", open.uses.len(), closed.uses.len());
}

#[test]
#[ignore = "prints timings; run with --ignored --nocapture"]
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
    Probe::run(theme, &appearance, Mode::Light, Pass::Component, vec2(10.0, 10.0), |_| {}, |_| {});
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
