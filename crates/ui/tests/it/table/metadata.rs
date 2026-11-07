//! The search over a metadata table's rows.

use craft_ui::table::MetadataRow;
use craft_ui::testing::trial;

#[trial]
fn a_row_matches_its_name_or_value_ignoring_case_and_surrounding_space() {
    let row = MetadataRow::new("Exposure Time", "1/250 s");
    assert!(row.matches(""));
    assert!(row.matches("  "));
    assert!(row.matches(" exposure "));
    assert!(row.matches("250"));
    assert!(!row.matches("aperture"));
}
