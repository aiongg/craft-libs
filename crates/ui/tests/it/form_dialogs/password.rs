//! The password dialog's strength readout: the word under the first field and the meter's fill.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{PasswordDialog, PasswordPurpose};
use craft_ui::testing::{Fixture, trial};
use egui::Rect;

/// Types `password` into the first field of a Set Password dialog and returns the fixture.
fn typed(password: &str) -> Fixture {
    let mut fixture = Fixture::new(|ui| {
        let mut open = true;
        PasswordDialog::new(PasswordPurpose::Set).opened(Bind::bound(&mut open)).show(ui);
    });
    fixture.settle();
    let field = fixture.control("Type Password").expect("the first password field");
    fixture.click(&field);
    fixture.type_text(password);
    fixture.settle();
    fixture
}

/// The width of the meter's fill: the narrowest shape inside the meter's bounds, the track being the widest.
fn fill_width(fixture: &Fixture) -> f32 {
    let meter = fixture.by_label("Password strength").expect("the strength meter shows").rect.expand(1.0);
    fixture.shape_bounds().iter().filter(|shape| meter.contains_rect(**shape) && shape.width() > 1.0).map(Rect::width).fold(f32::MAX, f32::min)
}

/// A short or single-class password reads Weak, eight characters in two classes Medium and twelve in three classes
/// Strong, in the word under the field; the meter fills further with each.
#[trial]
fn the_strength_meter_has_three_states() {
    let cases = [("abc123", "Weak"), ("abcdefg1", "Medium"), ("abcdefgh1234", "Medium"), ("Abcdefgh1234", "Strong")];
    let mut widths = Vec::new();
    for (password, word) in cases {
        let fixture = typed(password);
        let shown = format!("Strength: {word}");
        assert!(fixture.texts().iter().any(|run| run.text == shown), "{password:?} reads {word}");
        widths.push(fill_width(&fixture));
    }
    assert!(widths[0] < widths[1] && widths[1] == widths[2] && widths[2] < widths[3], "the fill grows with the strength: {widths:?}");
    let empty = typed("");
    assert!(empty.by_label("Password strength").is_none(), "no meter before anything is typed");
}
