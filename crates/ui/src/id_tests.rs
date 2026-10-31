//! The id a control or composite shows under is the one `Interactive::id_in` (or its own
//! `id_in`) gives its salt, so a host that reads focus or state before showing it finds the
//! same widget.

use egui::{Context, Id, Pos2, RawInput, Rect, Ui, vec2};

use crate::composites::{Curve, CurveEditor, SectionHeader};
use crate::controls::angle_dial::DialOp;
use crate::controls::{AngleDial, NumberField};
use crate::edit::parts;
use crate::localized::LocaleNumbers;
use crate::primitives::Interactive;
use crate::theme::{Base, install};

fn run(show: impl FnMut(&mut Ui)) {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut show = show;
    for _ in 0..2 {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
        ctx.run_ui(input, &mut show).textures_delta.clear();
    }
}

#[test]
fn id_in_is_the_shown_id() {
    let numbers = LocaleNumbers::from_tag("en");
    let curve = Curve::identity(255.0);
    run(|ui| {
        let at = |salt: &str| -> Id { Interactive::id_in(ui, salt) };
        let (dial, number, header) = (at("dial"), at("number"), at("header"));
        // The dial's response is its needle element; the control's own state is at its id.
        let _ = AngleDial::new("dial", "Angle", 10.0, &numbers).show(ui);
        assert!(ui.data(|d| d.get_temp::<parts::State<DialOp>>(dial)).is_some(), "AngleDial");
        assert_eq!(NumberField::new("number", "Width", 1.0, &numbers).show(ui).response.id, number, "NumberField");
        assert_eq!(SectionHeader::new("header", "Header").show(ui, |_| ()).response.id, header, "SectionHeader");
        let curve_id = CurveEditor::id_in(ui, "curve");
        assert_eq!(CurveEditor::new("curve", "Curve", &curve, &numbers).show(ui).response.id, curve_id, "CurveEditor");
    });
}
