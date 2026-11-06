//! The shared tool icons.

use super::{IconSet, ToolIcon};

#[test]
fn every_tool_has_an_icon_and_an_id_that_round_trips() {
    let mut ids = std::collections::HashSet::new();
    for tool in ToolIcon::ALL {
        assert!(!tool.icon().svg(IconSet::Phosphor).is_empty(), "{tool:?} has no base-set drawing");
        assert!(ids.insert(tool.id()), "{tool:?} repeats the id {}", tool.id());
        assert_eq!(ToolIcon::from_id(tool.id()), Some(tool));
    }
    assert_eq!(ToolIcon::from_id("rectangularMarquee"), Some(ToolIcon::RectangularMarquee));
    assert_eq!(ToolIcon::from_id("nonsense"), None);
}
