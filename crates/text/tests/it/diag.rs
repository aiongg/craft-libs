//! TEMPORARY diagnostics.

use craft_text::ui::character::{CharacterControls, CharacterValues, Kerning};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

#[trial]
fn diag_kerning() {
    let mut out = String::new();
    for (kerning, label) in
        [(Kerning::Metrics, "Kerning"), (Kerning::Manual(0.0), "Kerning"), (Kerning::Metrics, "Tracking"), (Kerning::Metrics, "Leading")]
    {
        let mut values = CharacterValues::default();
        values.kerning = Some(kerning);
        values.tracking = Some(0.0);
        values.size = Some(12.0);
        values.leading = Some(craft_text::ui::character::Leading::Auto);
        let families = ["Hack".to_owned()];
        let mut fixture = Fixture::new(move |ui| {
            let _ = CharacterControls::new("Character", &values, &families, &families).show(ui);
        });
        fixture.settle();
        let target = fixture.by_label(label).expect("field");
        fixture.click(&target);
        let focus_before = fixture.focused();
        fixture.key(Key::ArrowDown, Modifiers::ALT);
        let tree = fixture.tree();
        let rows: Vec<String> = tree
            .keys()
            .into_iter()
            .filter(|key| {
                matches!(tree.name_of(*key), Some("Optical" | "Auto" | "-100" | "Metrics"))
                    && tree.role_of(*key) != Some(craft_ui::contract::Role::LABEL)
            })
            .map(|key| format!("{:?} {:?}", tree.role_of(key), tree.name_of(key)))
            .collect();
        out += &format!("{kerning:?} {label}: focused {focus_before:?} rows {rows:?}\n");
    }
    panic!("DIAG\n{out}");
}
