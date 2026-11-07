//! The viewer's panel layout as plain data: the shares of its columns and of the right column's two panels.
//!
//! The dock tree is rebuilt from the shares on load, so a saved layout the build cannot read costs only the layout.

use craft_behavior::{Axis, DockTree, GroupId};
use serde::{Deserialize, Serialize};

/// The group of the component tree.
pub const COMPONENTS: GroupId = GroupId::new(0);
/// The group of the shown stories.
pub const STORIES: GroupId = GroupId::new(1);
/// The group of the component's facts.
pub const FACTS: GroupId = GroupId::new(2);
/// The group of the decision record.
pub const RECORD: GroupId = GroupId::new(3);

/// How wide each column is and how the right column divides, as shares of the whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PanelShares {
    /// The component tree, the stories and the right column, side by side.
    pub columns: [f32; 3],
    /// The facts over the decision record in the right column.
    pub right: [f32; 2],
}

impl Default for PanelShares {
    fn default() -> Self {
        PanelShares { columns: [2.0, 6.0, 3.0], right: [1.0, 1.0] }
    }
}

impl PanelShares {
    /// Whether every share is a positive finite number.
    pub fn is_valid(&self) -> bool {
        self.columns.iter().chain(&self.right).all(|share| share.is_finite() && *share > 0.0)
    }

    /// The dock tree of the shares.
    pub fn tree(&self) -> DockTree {
        let [first, second, third] = self.columns;
        let [above, below] = self.right;
        let right = DockTree::split(Axis::Y, vec![(above, DockTree::group(FACTS)), (below, DockTree::group(RECORD))]);
        DockTree::split(Axis::X, vec![(first, DockTree::group(COMPONENTS)), (second, DockTree::group(STORIES)), (third, right)])
    }

    /// The shares of `tree`, when it has the shape `tree` builds.
    pub fn of_tree(tree: &DockTree) -> Option<PanelShares> {
        let DockTree::Split(outer) = tree else { return None };
        let [(first, _), (second, _), (third, right)] = outer.children.as_slice() else { return None };
        let DockTree::Split(inner) = right else { return None };
        let [(above, _), (below, _)] = inner.children.as_slice() else { return None };
        let shares = PanelShares { columns: [*first, *second, *third], right: [*above, *below] };
        (outer.axis == Axis::X && inner.axis == Axis::Y && shares.is_valid()).then_some(shares)
    }

    /// The saved text of the shares.
    pub fn save(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The shares saved as `text`; the default when the text is empty, unreadable or holds a share that is not
    /// positive.
    pub fn load(text: &str) -> PanelShares {
        serde_json::from_str::<PanelShares>(text).ok().filter(PanelShares::is_valid).unwrap_or_default()
    }
}
