//! Section label tests: the text, its colour and role.

use crate::theme::ToEgui as _;
use egui::accesskit::Role;
use egui::{Context, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::theme::{Base, Tokens, TokensOf as _, install};

/// A label is a static text node in `text.faint`, in the `label` role's size.
#[test]
fn a_label_is_static_text_in_the_faint_label_style() {
    let harness = Harness::builder().with_size([320.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui(
        |ui: &mut Ui| {
            let _ = SectionLabel::new("Blend mode").show(ui);
        },
    );
    let node = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Blend mode"));
    assert!(node.rect().height() > 0.0);
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let tokens = Tokens::of(ui);
        let style = SectionLabel::style(&tokens);
        assert_eq!(style.role, craft_theme::TypeRole::Label);
        assert_eq!(style.color, crate::primitives::Content::Faint.color(&tokens).to_egui());
    })
    .drop_without_applying_deltas();
}
