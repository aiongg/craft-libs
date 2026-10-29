//! The gallery's dock layout: its panels, the default arrangement and what the user may do with
//! them.

use craft_ui::docking::{Column, Group, Layout, Node, Permissions, Side, Workspace};
use craft_ui::layout::{SplitAxis, SplitSize};
use serde::{Deserialize, Serialize};

/// A panel of the gallery's dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelId {
    /// The flat component list with its filter.
    Components,
    /// The selected component's stories.
    Stories,
    /// The selected story's args, and the compare options.
    Properties,
    /// The inspector; inspect mode is on while this tab is shown.
    Inspect,
    /// The centre: the current [`CentreView`].
    View,
    /// The token browser.
    Tokens,
}

/// What the centre shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CentreView {
    /// One component's stories, or the compare grid.
    #[default]
    Component,
}

impl CentreView {
    pub fn title(self) -> &'static str {
        match self {
            CentreView::Component => "Component",
        }
    }
}

impl PanelId {
    pub fn title(self) -> &'static str {
        match self {
            PanelId::Components => "Components",
            PanelId::Stories => "Stories",
            PanelId::Properties => "Properties",
            PanelId::Inspect => "Inspect",
            PanelId::View => "View",
            PanelId::Tokens => "Tokens",
        }
    }

    /// Panels move, float and tab freely but never close: a closed panel would have no way back
    /// besides Reset layout.
    pub fn permissions(&self) -> Permissions {
        Permissions { closable: false, ..Permissions::default() }
    }
}

const COMPONENTS_WIDTH: f32 = 220.0;
const SIDE_WIDTH: f32 = 340.0;

/// Components at the start edge; the view in the centre; the stories above the properties, the
/// inspector and the token browser at the end edge.
pub fn default_workspace() -> Workspace<PanelId> {
    let components = Column::new(vec![Group::new(vec![PanelId::Components], 0)], COMPONENTS_WIDTH);
    let side = Column::new(
        vec![Group::new(vec![PanelId::Stories], 0), Group::new(vec![PanelId::Properties, PanelId::Inspect, PanelId::Tokens], 0)],
        SIDE_WIDTH,
    );
    let centre = Node::Tabs { panels: vec![PanelId::View], active: 0 };
    let end = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(SIDE_WIDTH),
        first: Box::new(centre),
        second: Box::new(Node::Dock { side: Side::End, columns: vec![side] }),
    };
    let root = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedFirst(COMPONENTS_WIDTH),
        first: Box::new(Node::Dock { side: Side::Start, columns: vec![components] }),
        second: Box::new(end),
    };
    Workspace::new(Layout { root: Some(root), floating: Vec::new() })
}

/// `saved` made valid: a layout that fails validation is replaced by the default, and panels that
/// are missing come back where the default has them.
pub fn reconciled(mut saved: Workspace<PanelId>) -> Workspace<PanelId> {
    saved.reconcile(|_| true, |panel| panel.permissions(), &default_workspace());
    saved
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PanelId; 6] = [PanelId::Components, PanelId::Stories, PanelId::Properties, PanelId::Inspect, PanelId::View, PanelId::Tokens];

    #[test]
    fn the_default_layout_is_valid_and_holds_every_panel() {
        let workspace = default_workspace();
        assert_eq!(workspace.validate(), Ok(()));
        for panel in ALL {
            assert!(workspace.layout.contains(&panel), "{panel:?}");
        }
    }
}
