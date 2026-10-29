use egui::{Context, Shadow, Ui};

use super::DockLook;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

fn themed(mut test: impl FnMut(&Ui, &DockLook)) {
    let ctx = Context::default();
    let mut ran = false;
    let mut output = ctx.run_ui(Default::default(), |ui| {
        test(ui, &DockLook::from_tokens(&Tokens::of(ui)));
        ran = true;
    });
    output.textures_delta.clear();
    assert!(ran);
}

/// Pro Compact tokens keep every size and factor the renderer used to hold as a literal.
#[test]
fn compact_tokens_reproduce_the_literal_sizes() {
    themed(|ui, tokens| {
        assert_eq!(&DockLook::of(ui), tokens);
        let visuals = DockLook::from_visuals(&egui::Visuals::default());
        assert_eq!(tokens.tab_height, 28.0);
        assert_eq!(tokens.gap, visuals.gap);
        assert_eq!(tokens.pad, visuals.pad);
        assert_eq!(tokens.tab_padding, visuals.tab_padding);
        assert_eq!(tokens.border.width, visuals.border.width);
        assert_eq!(tokens.floating_stroke.width, visuals.floating_stroke.width);
        assert_eq!(tokens.title_height, visuals.title_height);
        assert_eq!(tokens.column_bar_height, visuals.column_bar_height);
        assert_eq!(tokens.icon_size, visuals.icon_size);
        assert_eq!(tokens.iconic_width, visuals.iconic_width);
        assert_eq!(tokens.icon_pad, visuals.icon_pad);
        assert_eq!(tokens.icon_radius, visuals.icon_radius);
        assert_eq!(tokens.hint_radius, visuals.hint_radius);
        assert_eq!(tokens.drop_width, visuals.drop_width);
        assert_eq!(tokens.focus_width, visuals.focus_width);
        assert_eq!(tokens.marker_radius, visuals.marker_radius);
        assert_eq!(tokens.drop_fill, visuals.drop_fill);
        assert_eq!(tokens.ghost_fill, visuals.ghost_fill);
        assert_eq!(tokens.ghost_stroke, visuals.ghost_stroke);
        assert_eq!(tokens.tab_fill, visuals.tab_fill);
        assert_eq!(tokens.grip_alpha, visuals.grip_alpha);
        assert_eq!(tokens.floating_radius, visuals.floating_radius);
        assert!(tokens.valid());
    });
}

/// The strip, hover and drop colours come from the `tab` and `dock` families, the shadow from
/// `float`.
#[test]
fn colors_and_shadow_come_from_the_component_tokens() {
    themed(|ui, look| {
        let t = Tokens::of(ui);
        assert_eq!(look.tab_strip, t.component.tab.strip.to_egui());
        assert_eq!(look.drop, t.component.dock.drop.to_egui());
        assert_ne!(look.floating_shadow, Shadow::NONE);
    });
}

/// A panel body outside a dock area puts its first control at the margin inside the dock's edge,
/// and stacks controls `body_gap` apart.
#[test]
fn a_panel_body_puts_its_content_at_the_margin_and_spaces_it_by_body_gap() {
    let ctx = Context::default();
    let mut found = None;
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let look = DockLook::of(ui);
        let margin = egui::Margin::same(10);
        let body = look.panel_body(ui, margin, |ui| {
            let first = ui.label("one").rect;
            let second = ui.label("two").rect;
            (first, second)
        });
        found = Some((look, body.response.rect, body.inner));
    });
    output.textures_delta.clear();
    let (look, body, (first, second)) = found.unwrap();
    assert_eq!(first.min, body.min + egui::vec2(look.border.width + 10.0, look.border.width + 10.0));
    assert_eq!(second.top() - first.bottom(), look.body_gap);
}
