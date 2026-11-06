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

/// Every size and factor of the look is a semantic token, an expression of one, or a value of the
/// dock style.
#[test]
fn sizes_come_from_the_semantic_tokens_and_the_dock_style() {
    themed(|ui, look| {
        assert_eq!(&DockLook::of(ui), look);
        let t = Tokens::of(ui);
        let style = crate::style::dock::DockStyle::new(&t);
        let float = crate::style::float::FloatStyle::new(&t);
        let (space, size, stroke, shape) = (&t.space, &t.size, &t.stroke, &t.shape);
        let sizes = [
            ("tab_height", look.tab_height, size.panel_header),
            ("gap", look.gap, space.gap.inline),
            ("body_gap", look.body_gap, space.gap.stack),
            ("pad", look.pad, space.pad.control),
            ("tab_padding", look.tab_padding, 4.0 * space.pad.control),
            ("border", look.border.width, stroke.thin),
            ("floating_stroke", look.floating_stroke.width, stroke.thin),
            ("title_height", look.title_height, size.panel_header),
            ("column_bar_height", look.column_bar_height, size.panel_header),
            ("icon_size", look.icon_size, size.icon.lg),
            ("iconic_width", look.iconic_width, style.iconic_width),
            ("icon_pad", look.icon_pad, space.gap.stack),
            ("icon_radius", look.icon_radius, shape.radius.control),
            ("hint_radius", look.hint_radius, shape.radius.popup),
            ("drop_width", look.drop_width, stroke.thick),
            ("focus_width", look.focus_width, stroke.thin),
            ("marker_radius", look.marker_radius, shape.radius.sm),
            ("drop_fill", look.drop_fill, style.drop_fill),
            ("ghost_fill", look.ghost_fill, style.ghost_fill),
            ("ghost_stroke", look.ghost_stroke, style.ghost_stroke),
            ("tab_fill", look.tab_fill, style.tab_fill),
            ("grip_alpha", look.grip_alpha, style.grip_alpha),
            ("min_pane", look.min_pane, style.min_pane),
            ("drag_bar_width", look.drag_bar_width, style.drag_bar_width),
            ("overflow_width", look.overflow_width, style.overflow_width),
            ("menu_max_height", look.menu_max_height, style.menu_max_height),
            ("iconic_max_width", look.iconic_max_width, style.iconic_max_width),
            ("iconic_label_width", look.iconic_label_width, style.iconic_label_width),
            ("flyout_height", look.flyout_height, style.flyout_height),
            ("float_width", look.float_width, float.width),
            ("float_height", look.float_height, float.height),
            ("min_fitted_height", look.min_fitted_height, float.min_fitted_height),
            ("cascade_offset", look.cascade_offset, size.control.sm),
            ("hint_offset", look.hint_offset, size.control.md),
            ("float_min_width", look.float_min_width, float.min_width),
            ("float_min_body_height", look.float_min_body_height, float.min_body_height),
        ];
        for (name, got, want) in sizes {
            assert_eq!(got, want, "{name}");
        }
        assert!(look.valid());
    });
}

/// The strip comes from `surface.panel`, the drop colour from `accent.solid`, the shadow from
/// `elevation.floating`.
#[test]
fn colors_and_shadow_come_from_the_semantic_tokens() {
    themed(|ui, look| {
        let t = Tokens::of(ui);
        assert_eq!(look.tab_strip, t.color.surface.panel.to_egui());
        assert_eq!(look.drop, t.color.accent.solid.to_egui());
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
