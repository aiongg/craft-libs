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
    themed(|ui, look| {
        assert_eq!(&DockLook::of(ui), look);
        let sizes = [
            ("tab_height", look.tab_height, 28.0),
            ("gap", look.gap, 4.0),
            ("body_gap", look.body_gap, 0.0),
            ("pad", look.pad, 6.0),
            ("tab_padding", look.tab_padding, 24.0),
            ("border", look.border.width, 1.0),
            ("floating_stroke", look.floating_stroke.width, 1.0),
            ("title_height", look.title_height, 14.0),
            ("column_bar_height", look.column_bar_height, 14.0),
            ("icon_size", look.icon_size, 20.0),
            ("iconic_width", look.iconic_width, 36.0),
            ("icon_pad", look.icon_pad, 10.0),
            ("icon_radius", look.icon_radius, 3.0),
            ("hint_radius", look.hint_radius, 4.0),
            ("drop_width", look.drop_width, 2.0),
            ("focus_width", look.focus_width, 1.0),
            ("marker_radius", look.marker_radius, 1.0),
            ("drop_fill", look.drop_fill, 0.15),
            ("ghost_fill", look.ghost_fill, 0.3),
            ("ghost_stroke", look.ghost_stroke, 0.6),
            ("tab_fill", look.tab_fill, 0.08),
            ("grip_alpha", look.grip_alpha, 0.5),
            ("min_pane", look.min_pane, 40.0),
            ("drag_bar_width", look.drag_bar_width, 40.0),
            ("overflow_width", look.overflow_width, 22.0),
            ("menu_max_height", look.menu_max_height, 360.0),
            ("iconic_max_width", look.iconic_max_width, 240.0),
            ("iconic_label_width", look.iconic_label_width, 96.0),
            ("flyout_height", look.flyout_height, 320.0),
            ("float_width", look.float_width, 260.0),
            ("float_height", look.float_height, 320.0),
            ("min_fitted_height", look.min_fitted_height, 120.0),
            ("cascade_offset", look.cascade_offset, 24.0),
            ("hint_offset", look.hint_offset, 28.0),
            ("float_min_width", look.float_min_width, 40.0),
            ("float_min_body_height", look.float_min_body_height, 20.0),
        ];
        for (name, got, want) in sizes {
            assert_eq!(got, want, "{name}");
        }
        assert_eq!(look.floating_radius, egui::CornerRadius::ZERO);
        assert!(look.valid());
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
