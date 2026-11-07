//! `compact_size`: the unit a byte count is shown in.

use craft_ui::bars::{SizeUnit, compact_size};
use craft_ui::testing::trial;

const KIB: u64 = 1024;

/// A count is shown in the largest unit that keeps it at 1 or more, kilo for anything below a mega.
#[trial]
fn compact_sizes_step_by_1024() {
    assert_eq!((compact_size(0).unit, compact_size(0).value), (SizeUnit::Kilo, 0.0));
    assert_eq!(compact_size(KIB * KIB - 1).unit, SizeUnit::Kilo);
    assert_eq!((compact_size(KIB * KIB).unit, compact_size(KIB * KIB).value), (SizeUnit::Mega, 1.0));
    assert_eq!(compact_size(KIB * KIB * KIB - 1).unit, SizeUnit::Mega);
    assert_eq!((compact_size(3 * KIB * KIB * KIB / 2).unit, compact_size(3 * KIB * KIB * KIB / 2).value), (SizeUnit::Giga, 1.5));
}
