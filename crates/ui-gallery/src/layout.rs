//! The gallery's dock layout: its panels, the default arrangement and what the user may do with
//! them.

use craft_ui::docking::{Column, Group, Layout, Node, Permissions, Side, Workspace};
use craft_ui::layout::{SplitAxis, SplitSize};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// A panel of the gallery's dock.
///
/// Saved layouts name a [`PanelId::Component`] by its component's name; the name is mapped back to
/// the registry's own string on load, and a name the registry no longer has loads as a panel that
/// [`PanelId::is_registered`] rejects, so the layout reconciliation drops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelId {
    /// The flat component list with its filter.
    Components,
    /// The selected component's stories.
    Stories,
    /// The selected story's args.
    Properties,
    /// The inspector; inspect mode is on while this tab is shown.
    Inspect,
    /// The centre's first tab: every component of one level as tiles.
    View,
    /// A centre tab with every story of one component as tiles.
    Component(&'static str),
    /// The token browser.
    Tokens,
    /// The Theme workspace's centre: the current [`PreviewView`].
    Preview,
    /// The Theme workspace's theme panel.
    Theme,
    /// The Theme workspace's contrast problems.
    Problems,
    /// The Theme workspace's changes since the last save.
    Changes,
}

/// A set of panels with a dock layout and an options-bar row of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GalleryWorkspace {
    /// Browsing the components and their stories.
    #[default]
    Components,
    /// Previewing, checking and editing a theme.
    Theme,
}

impl GalleryWorkspace {
    pub const ALL: [GalleryWorkspace; 2] = [GalleryWorkspace::Components, GalleryWorkspace::Theme];

    pub fn title(self) -> &'static str {
        match self {
            GalleryWorkspace::Components => "Components",
            GalleryWorkspace::Theme => "Theme",
        }
    }
}

/// What the Theme workspace's preview shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewView {
    /// A mock application.
    #[default]
    App,
    /// The Controls level's overview.
    Components,
}

/// What a Variants view repeats the story over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariantAxis {
    Theme,
    Mode,
    Contrast,
    Size,
}

impl VariantAxis {
    pub const ALL: [VariantAxis; 4] = [VariantAxis::Theme, VariantAxis::Mode, VariantAxis::Contrast, VariantAxis::Size];

    /// The axis in "in every ...".
    pub fn noun(self) -> &'static str {
        match self {
            VariantAxis::Theme => "theme",
            VariantAxis::Mode => "mode",
            VariantAxis::Contrast => "contrast",
            VariantAxis::Size => "size",
        }
    }

    /// The Variants menu's item.
    pub fn menu_label(self) -> &'static str {
        match self {
            VariantAxis::Theme => "Every theme",
            VariantAxis::Mode => "Every mode (light, dark)",
            VariantAxis::Contrast => "Every contrast (standard, high)",
            VariantAxis::Size => "Every size (compact, comfortable)",
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
            PanelId::View => "Overview",
            PanelId::Component(component) => component,
            PanelId::Tokens => "Tokens",
            PanelId::Preview => "Preview",
            PanelId::Theme => "Theme",
            PanelId::Problems => "Problems",
            PanelId::Changes => "Changes",
        }
    }

    /// The workspace whose layout holds the panel.
    pub fn workspace(self) -> GalleryWorkspace {
        match self {
            PanelId::Components
            | PanelId::Stories
            | PanelId::Properties
            | PanelId::Inspect
            | PanelId::View
            | PanelId::Component(_)
            | PanelId::Tokens => GalleryWorkspace::Components,
            PanelId::Preview | PanelId::Theme | PanelId::Problems | PanelId::Changes => GalleryWorkspace::Theme,
        }
    }

    /// Panels move, float and tab freely. Only a component tab closes: any other closed panel
    /// would have no way back besides Reset layout.
    pub fn permissions(&self) -> Permissions {
        Permissions { closable: matches!(self, PanelId::Component(_)), ..Permissions::default() }
    }

    /// Whether the gallery has this panel: every panel but a component tab whose component the
    /// registry no longer has.
    pub fn is_registered(self) -> bool {
        match self {
            PanelId::Component(component) => known_components().contains(&component),
            _ => true,
        }
    }
}

/// The names of the components that have stories.
fn known_components() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names: Vec<&'static str> = Vec::new();
        for story in crate::gallery::stories() {
            if !names.contains(&story.component) {
                names.push(story.component);
            }
        }
        names
    })
}

/// [`PanelId`] as saved: the same variants, with a component tab named by an owned string.
#[derive(Serialize, Deserialize)]
enum PanelRepr {
    Components,
    Stories,
    Properties,
    Inspect,
    View,
    Component(String),
    Tokens,
    Preview,
    Theme,
    Problems,
    Changes,
}

impl Serialize for PanelId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PanelRepr::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PanelId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        PanelRepr::deserialize(deserializer).map(PanelId::from)
    }
}

impl From<PanelId> for PanelRepr {
    fn from(panel: PanelId) -> Self {
        match panel {
            PanelId::Components => PanelRepr::Components,
            PanelId::Stories => PanelRepr::Stories,
            PanelId::Properties => PanelRepr::Properties,
            PanelId::Inspect => PanelRepr::Inspect,
            PanelId::View => PanelRepr::View,
            PanelId::Component(component) => PanelRepr::Component(component.to_owned()),
            PanelId::Tokens => PanelRepr::Tokens,
            PanelId::Preview => PanelRepr::Preview,
            PanelId::Theme => PanelRepr::Theme,
            PanelId::Problems => PanelRepr::Problems,
            PanelId::Changes => PanelRepr::Changes,
        }
    }
}

impl From<PanelRepr> for PanelId {
    fn from(repr: PanelRepr) -> Self {
        match repr {
            PanelRepr::Components => PanelId::Components,
            PanelRepr::Stories => PanelId::Stories,
            PanelRepr::Properties => PanelId::Properties,
            PanelRepr::Inspect => PanelId::Inspect,
            PanelRepr::View => PanelId::View,
            // A name the registry lacks keeps its own (leaked) string so that two stale names stay
            // two panels; `is_registered` rejects it and the reconciliation drops it.
            PanelRepr::Component(name) => {
                PanelId::Component(known_components().iter().copied().find(|c| *c == name).unwrap_or_else(|| Box::leak(name.into_boxed_str())))
            }
            PanelRepr::Tokens => PanelId::Tokens,
            PanelRepr::Preview => PanelId::Preview,
            PanelRepr::Theme => PanelId::Theme,
            PanelRepr::Problems => PanelId::Problems,
            PanelRepr::Changes => PanelId::Changes,
        }
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

/// The preview in the centre; the theme above the problems and the changes at the end edge.
pub fn default_theme_workspace() -> Workspace<PanelId> {
    let side = Column::new(vec![Group::new(vec![PanelId::Theme], 0), Group::new(vec![PanelId::Problems, PanelId::Changes], 0)], SIDE_WIDTH);
    let centre = Node::Tabs { panels: vec![PanelId::Preview], active: 0 };
    let root = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(SIDE_WIDTH),
        first: Box::new(centre),
        second: Box::new(Node::Dock { side: Side::End, columns: vec![side] }),
    };
    Workspace::new(Layout { root: Some(root), floating: Vec::new() })
}

/// The default layout of `workspace`.
pub fn default_for(workspace: GalleryWorkspace) -> Workspace<PanelId> {
    match workspace {
        GalleryWorkspace::Components => default_workspace(),
        GalleryWorkspace::Theme => default_theme_workspace(),
    }
}

/// `saved` made valid for `workspace`: a layout that fails validation is replaced by the default,
/// panels of the other workspace are dropped, and panels that are missing come back where the
/// default has them.
pub fn reconciled(mut saved: Workspace<PanelId>, workspace: GalleryWorkspace) -> Workspace<PanelId> {
    let registered = |panel: &PanelId| panel.workspace() == workspace && panel.is_registered();
    saved.reconcile(registered, |panel| panel.permissions(), &default_for(workspace));
    saved.hidden.retain(|(panel, _)| registered(panel));
    saved
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENTS: [PanelId; 6] = [PanelId::Components, PanelId::Stories, PanelId::Properties, PanelId::Inspect, PanelId::View, PanelId::Tokens];
    const THEME: [PanelId; 4] = [PanelId::Preview, PanelId::Theme, PanelId::Problems, PanelId::Changes];

    #[test]
    fn each_default_layout_is_valid_and_holds_exactly_its_workspaces_panels() {
        for (workspace, panels) in [(GalleryWorkspace::Components, &COMPONENTS[..]), (GalleryWorkspace::Theme, &THEME[..])] {
            let layout = default_for(workspace);
            assert_eq!(layout.validate(), Ok(()));
            for panel in COMPONENTS.iter().chain(&THEME) {
                assert_eq!(layout.layout.contains(panel), panels.contains(panel), "{workspace:?} {panel:?}");
            }
        }
    }

    #[test]
    fn a_panel_of_the_other_workspace_in_a_saved_layout_is_dropped_and_missing_ones_return() {
        let centre = Node::Tabs { panels: vec![PanelId::View, PanelId::Preview], active: 1 };
        let saved = Workspace::new(Layout { root: Some(centre), floating: Vec::new() });
        let reconciled = reconciled(saved, GalleryWorkspace::Components);
        assert_eq!(reconciled.validate(), Ok(()));
        assert!(!reconciled.layout.contains(&PanelId::Preview));
        assert!(reconciled.hidden.iter().all(|(panel, _)| panel.workspace() == GalleryWorkspace::Components));
        for panel in COMPONENTS {
            assert!(reconciled.layout.contains(&panel), "{panel:?}");
        }
    }
}
