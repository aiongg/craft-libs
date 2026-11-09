//! Paper sizes, margins and orientation.

use super::*;

#[test]
fn orientation_turns_size_and_margins() {
    let mut setup = PageSetup { margins: Margins { top: 10.0, bottom: 20.0, left: 30.0, right: 40.0 }, ..PageSetup::default() };
    setup.set_orientation(Orientation::Landscape);
    assert_eq!((setup.width, setup.height), (792.0, 612.0));
    assert_eq!(setup.margins, Margins { top: 30.0, bottom: 40.0, left: 20.0, right: 10.0 });
    setup.set_orientation(Orientation::Portrait);
    assert_eq!((setup.width, setup.height), (612.0, 792.0));
    assert_eq!(setup.margins, Margins { top: 10.0, bottom: 20.0, left: 30.0, right: 40.0 });

    setup.set_size(800.0, 600.0);
    assert_eq!(setup.orientation, Orientation::Landscape);
    setup.set_size(500.0, 500.0);
    assert_eq!(setup.orientation, Orientation::Landscape, "a square keeps the orientation");
    setup.set_size(500.0, 700.0);
    assert_eq!(setup.orientation, Orientation::Portrait);
}
