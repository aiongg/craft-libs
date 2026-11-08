//! The viewer's panel layout as plain data: the shares of its columns and of the right column's two panels.
//!
//! The dock layout is rebuilt from the shares on load, so a saved layout the build cannot read costs only the layout.

use craft_behavior::Axis;
use craft_behavior::dock::{Layout, Node, SplitSize};
use serde::{Deserialize, Serialize};

/// A panel of the viewer's dock layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GalleryPanel {
    /// The component tree.
    Components,
    /// The shown stories.
    Stories,
    /// The component's facts.
    Facts,
    /// The decision record.
    Record,
    /// The theme's contrast problems.
    Problems,
    /// The theme's token browser.
    Tokens,
    /// The picked token's detail.
    Detail,
    /// The app sizes editor.
    Overrides,
}

/// How wide each column is and how the right column divides, as shares of the whole.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PanelShares {
    /// The component tree, the stories and the right column, side by side.
    pub columns: [f32; 3],
    /// The facts over the decision record in the right column.
    pub right: [f32; 2],
}

/// Shares are equal when they divide the room alike: a layout keeps ratios, so only the proportions survive it.
impl PartialEq for PanelShares {
    fn eq(&self, other: &Self) -> bool {
        fn alike(a: &[f32], b: &[f32]) -> bool {
            let (sum_a, sum_b) = (a.iter().sum::<f32>(), b.iter().sum::<f32>());
            a.iter().zip(b).all(|(a, b)| (a / sum_a - b / sum_b).abs() < 1e-4)
        }
        alike(&self.columns, &other.columns) && alike(&self.right, &other.right)
    }
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

    /// The dock layout of the shares: each split gives its first child the fraction of what its two children share.
    pub fn tree(&self) -> Layout<GalleryPanel> {
        let [first, second, third] = self.columns;
        let [above, below] = self.right;
        let group = |id: GalleryPanel| Node::tabs(vec![id], 0);
        let right = Node::split(Axis::Y, SplitSize::Ratio(above / (above + below)), group(GalleryPanel::Facts), group(GalleryPanel::Record));
        let rest = Node::split(Axis::X, SplitSize::Ratio(second / (second + third)), group(GalleryPanel::Stories), right);
        Layout::new(Some(Node::split(Axis::X, SplitSize::Ratio(first / (first + second + third)), group(GalleryPanel::Components), rest)), Vec::new())
    }

    /// The shares of `layout`, when it has the shape `tree` builds.
    pub fn of_tree(layout: &Layout<GalleryPanel>) -> Option<PanelShares> {
        let Some(Node::Split { axis: Axis::X, size: SplitSize::Ratio(outer), second: rest, .. }) = &layout.root else { return None };
        let Node::Split { axis: Axis::X, size: SplitSize::Ratio(middle), second: right, .. } = rest.as_ref() else { return None };
        let Node::Split { axis: Axis::Y, size: SplitSize::Ratio(inner), .. } = right.as_ref() else { return None };
        let shares = PanelShares { columns: [*outer, (1.0 - outer) * middle, (1.0 - outer) * (1.0 - middle)], right: [*inner, 1.0 - inner] };
        shares.is_valid().then_some(shares)
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

/// The shares of the theme panels' columns: the problems, the tokens and the right column.
const THEME_COLUMNS: [f32; 3] = [2.0, 4.0, 3.0];
/// The shares of the right column: the detail over the app sizes.
const THEME_RIGHT: [f32; 2] = [1.0, 1.0];

/// The layout of the theme panels. Problems come first in the tree, so they are shown before the tokens in a pass:
/// the token browser reveals the row a problem picks in the same pass.
pub fn theme_layout() -> Layout<GalleryPanel> {
    let [first, second, third] = THEME_COLUMNS;
    let [above, below] = THEME_RIGHT;
    let group = |id: GalleryPanel| Node::tabs(vec![id], 0);
    let right = Node::split(Axis::Y, SplitSize::Ratio(above / (above + below)), group(GalleryPanel::Detail), group(GalleryPanel::Overrides));
    let rest = Node::split(Axis::X, SplitSize::Ratio(second / (second + third)), group(GalleryPanel::Tokens), right);
    Layout::new(Some(Node::split(Axis::X, SplitSize::Ratio(first / (first + second + third)), group(GalleryPanel::Problems), rest)), Vec::new())
}
