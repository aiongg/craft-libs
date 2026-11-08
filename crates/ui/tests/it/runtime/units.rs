//! `UnitKind::LENGTHS`: the kinds that measure a length, each written with its own unit.

use craft_ui::runtime::env::{Env, UnitKind};
use craft_ui::testing::{Frames, trial};

#[trial]
fn every_length_unit_writes_a_length_with_its_own_suffix_and_converts_to_pixels() {
    let mut frames = Frames::new();
    let mut seen = Vec::new();
    frames.frame(vec![], |ui| {
        let units = Env::of(ui).units();
        for kind in UnitKind::LENGTHS {
            seen.push((units.show(kind, 3.0), units.symbol(kind), kind.to_pixels(1.0, 100.0, 96.0, UnitKind::POINTS_PER_INCH).is_some()));
        }
    });
    assert_eq!(seen.len(), UnitKind::LENGTHS.len());
    for (text, symbol, converts) in &seen {
        assert!(!symbol.is_empty() && text.ends_with(symbol.as_str()), "{text} ends with {symbol}");
        assert!(converts, "{text} measures a length");
    }
    let mut symbols: Vec<&String> = seen.iter().map(|(_, symbol, _)| symbol).collect();
    symbols.sort();
    symbols.dedup();
    assert_eq!(symbols.len(), seen.len(), "every length unit has its own suffix");
}
