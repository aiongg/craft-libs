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
    /// The story canvas, or the compare grid.
    Canvas,
    /// The token browser.
    Tokens,
}

impl PanelId {
    pub fn title(self) -> &'static str {
        match self {
            PanelId::Components => "Components",
            PanelId::Stories => "Stories",
            PanelId::Properties => "Properties",
            PanelId::Inspect => "Inspect",
            PanelId::Canvas => "Canvas",
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
const SIDE_WIDTH: f32 = 280.0;

/// Components at the start edge; the canvas and the token browser as tabs in the centre; the
/// stories above the properties and the inspector at the end edge.
pub fn default_workspace() -> Workspace<PanelId> {
    let components = Column::new(vec![Group::new(vec![PanelId::Components], 0)], COMPONENTS_WIDTH);
    let side = Column::new(vec![Group::new(vec![PanelId::Stories], 0), Group::new(vec![PanelId::Properties, PanelId::Inspect], 0)], SIDE_WIDTH);
    let centre = Node::Tabs { panels: vec![PanelId::Canvas, PanelId::Tokens], active: 0 };
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

    const ALL: [PanelId; 6] = [PanelId::Components, PanelId::Stories, PanelId::Properties, PanelId::Inspect, PanelId::Canvas, PanelId::Tokens];

    #[test]
    fn the_default_layout_is_valid_and_holds_every_panel() {
        let workspace = default_workspace();
        assert_eq!(workspace.validate(), Ok(()));
        for panel in ALL {
            assert!(workspace.layout.contains(&panel), "{panel:?}");
        }
    }
}
