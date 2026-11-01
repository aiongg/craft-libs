//! Page grids.

use super::grid::*;

// Start -30, increment 12 in a 50 pt page gives 6, 18, 30, 42; an increment of 0.2 counts as 1; never more than MAX_BASELINES lines.
#[test]
fn baseline_lines_start_inside_the_page() {
    assert_eq!(baseline_lines(-30.0, 12.0, 50.0), [6.0, 18.0, 30.0, 42.0]);
    assert_eq!(baseline_lines(0.0, 0.2, 3.5), [0.0, 1.0, 2.0, 3.0]);
    assert_eq!(baseline_lines(0.0, 1.0, 1.0e9).len(), MAX_BASELINES);
    assert!(baseline_lines(f64::NAN, 12.0, 50.0).is_empty());
}
