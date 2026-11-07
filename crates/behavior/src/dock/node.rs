//! The docked tree, its leaves and the floating windows.

use super::{
    edit::{stack_tree, tree_groups, tree_panels},
    groups::{Column, Group, Side},
    split::{SplitAxis, SplitSize},
};

/// A drop zone on a group: the middle, or a side half.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Zone {
    /// The middle of the group.
    Center,
    /// The left half.
    Left,
    /// The right half.
    Right,
    /// The top half.
    Top,
    /// The bottom half.
    Bottom,
}

/// A node of a layout tree.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Node<P> {
    /// Two children side by side or stacked.
    Split {
        /// Direction the children lie in.
        axis: SplitAxis,
        /// How the two children share the room.
        size: SplitSize,
        /// The first child: left or top.
        first: Box<Node<P>>,
        /// The second child: right or bottom.
        second: Box<Node<P>>,
    },
    /// A group of tabs.
    Tabs {
        /// The tabs, first to last.
        panels: Vec<P>,
        /// Index into `panels` of the selected tab.
        active: usize,
    },
    /// An accordion keeps its open state and preferred heights when other panels move.
    Stack {
        /// The accordion sections, first to last.
        entries: Vec<StackEntry<P>>,
    },
    /// One window edge's dock, columns from the window edge inwards. A dock is the root or,
    /// following only same-side children of horizontal splits, at its window edge. At most one
    /// per side. An empty dock (no columns) is valid and accepts `Target::NewDock`.
    Dock {
        /// The window edge the dock sits on.
        side: Side,
        /// The columns, in order from the window edge inward.
        columns: Vec<Column<P>>,
    },
}

/// One panel of a [`Node::Stack`] accordion.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct StackEntry<P> {
    /// The section's panel.
    pub panel: P,
    /// The section shows its body.
    pub open: bool,
    /// Preferred height in points; `None` shares the free height.
    pub height: Option<f32>,
}

/// A floating window: a tree of groups like the docked layout's, without docks.
///
/// Serde reads the v1 shape `{panels, active, rect}` as one group, and a stack of groups
/// `{groups, rect}` as a vertical split of them, sized by their heights. A window with one
/// group (not minimized, no native rect) writes the v1 shape, so older builds can read files
/// that use no newer feature. Every window writes its `id`; a window read without one gets a
/// fresh one.
///
/// Equality ignores `id`: two windows with the same contents and place are the same layout.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Floating<P> {
    /// Names the window across panel moves, and its OS window.
    /// Unique within a layout: [`Self::new`] draws a fresh id, [`Self::with_id`] sets one,
    /// loading keeps the saved id (or draws one when it is missing or taken), and merging
    /// windows keeps the receiving window's.
    pub id: u64,
    /// `Split`, `Tabs` and `Stack` nodes; a `Dock` is invalid here.
    pub root: Node<P>,
    /// `[x, y, width, height]` in logical points, in the application's viewport.
    pub rect: [f32; 4],
    /// Desktop rect while the window is in its own OS window.
    pub native: Option<[f32; 4]>,
    /// The monitor holding the center of `native`, from the application's monitor list.
    pub monitor: Option<Monitor>,
    /// Without a monitor list: the size of the monitor reported for the native window.
    pub monitor_size: Option<[f32; 2]>,
    /// Only the title bar shows.
    pub minimized: bool,
}

impl<P: PartialEq> PartialEq for Floating<P> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
            && self.rect == other.rect
            && self.native == other.native
            && self.monitor == other.monitor
            && self.monitor_size == other.monitor_size
            && self.minimized == other.minimized
    }
}

/// A display, in desktop logical points. Applications on an egui with monitor reports fill
/// the application's monitor list with these; native floating windows store the one they are on.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Monitor {
    /// Display name from the OS, when it reports one.
    pub name: Option<String>,
    /// `[x, y, width, height]` on the desktop.
    pub rect: [f32; 4],
    /// Physical pixels per logical point.
    pub scale: f32,
}

impl<P> Node<P> {
    /// A group of tabs with tab `active` selected.
    pub fn tabs(panels: Vec<P>, active: usize) -> Self {
        Self::Tabs { panels, active }
    }

    /// Two children along `axis`, sharing the room by `size`.
    pub fn split(axis: SplitAxis, size: SplitSize, first: Node<P>, second: Node<P>) -> Self {
        Self::Split { axis, size, first: Box::new(first), second: Box::new(second) }
    }
}

impl<P> StackEntry<P> {
    /// An open accordion section for `panel` with no preferred height.
    pub fn new(panel: P) -> Self {
        Self { panel, open: true, height: None }
    }
}

impl Monitor {
    /// A display at `rect` with `scale` physical pixels per logical point.
    pub fn new(name: Option<String>, rect: [f32; 4], scale: f32) -> Self {
        Self { name, rect, scale }
    }
}

/// A window id no other window of this process has drawn, from a random start so ids saved
/// by earlier runs are unlikely to repeat.
pub(super) fn fresh_window_id() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    };
    static NEXT: OnceLock<AtomicU64> = OnceLock::new();
    let next = NEXT.get_or_init(|| AtomicU64::new(std::collections::hash_map::RandomState::new().build_hasher().finish() >> 1));
    next.fetch_add(1, Ordering::Relaxed)
}

impl<P> Floating<P> {
    /// A window holding `root`. Each call makes a new window, with a fresh [`Self::id`]: build
    /// a layout once and keep it, or give its windows ids with [`Self::with_id`]. A layout
    /// built anew each frame without them opens new OS windows each frame.
    pub fn new(root: Node<P>, rect: [f32; 4]) -> Self {
        Self { id: fresh_window_id(), root, rect, native: None, monitor: None, monitor_size: None, minimized: false }
    }

    /// A window with one group. Each call makes a new window, as [`Self::new`] does.
    pub fn single(panels: Vec<P>, active: usize, rect: [f32; 4]) -> Self {
        Self::new(Node::Tabs { panels, active }, rect)
    }

    /// The window with `id`, for a layout the app builds in code, such as a default layout:
    /// built with the same id, it is the same window, and the same OS window, on every frame.
    /// Ids must be unique within a layout.
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    /// Every panel in the window, in tree order.
    pub fn panels(&self) -> impl Iterator<Item = &P> {
        tree_panels(&self.root).into_iter()
    }

    /// Groups stacked top to bottom, as a v1 layout stored them: each group over a vertical
    /// split of the ones below it, sized by their heights. `None` without groups. Each call
    /// makes a new window, as [`Self::new`] does.
    pub fn stack(groups: Vec<Group<P>>, rect: [f32; 4]) -> Option<Self> {
        stack_tree(groups).map(|root| Self::new(root, rect))
    }

    /// The window's groups, first to last.
    pub fn groups(&self) -> Vec<Group<P>>
    where
        P: Clone,
    {
        tree_groups(&self.root)
    }

    /// True when a group of this window holds `panel`. Takes any borrowed form of `P`, such
    /// as `&str` for `String` panels.
    pub fn contains<Q: ?Sized + PartialEq>(&self, panel: &Q) -> bool
    where
        P: std::borrow::Borrow<Q>,
    {
        self.panels().any(|p| p.borrow() == panel)
    }
}

mod floating_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{Floating, Group, Monitor, Node, fresh_window_id};
    use crate::dock::edit::stack_tree;

    #[derive(Serialize)]
    struct V1<'a, P> {
        id: u64,
        panels: &'a [P],
        active: usize,
        rect: [f32; 4],
    }

    #[derive(Serialize)]
    struct Tree<'a, P> {
        id: u64,
        root: &'a Node<P>,
        rect: [f32; 4],
        #[serde(skip_serializing_if = "Option::is_none")]
        native: Option<[f32; 4]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        monitor: Option<&'a Monitor>,
        #[serde(skip_serializing_if = "Option::is_none")]
        monitor_size: Option<[f32; 2]>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        minimized: bool,
    }

    impl<P: Serialize> Serialize for Floating<P> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match &self.root {
                Node::Tabs { panels, active } if !self.minimized && self.native.is_none() => {
                    V1 { id: self.id, panels, active: *active, rect: self.rect }.serialize(serializer)
                }
                root => Tree {
                    id: self.id,
                    root,
                    rect: self.rect,
                    native: self.native,
                    monitor: self.monitor.as_ref(),
                    monitor_size: self.monitor_size,
                    minimized: self.minimized,
                }
                .serialize(serializer),
            }
        }
    }

    /// The fields of a saved window, whichever of the three shapes it was written in. A key is
    /// read as [`Serialize`] writes it: `id`, `root`, `groups`, `panels` and `rect` plain, and the
    /// optional `native`, `monitor` and `monitor_size` as options, which RON marks with `Some`
    /// and JSON writes bare.
    struct Repr<P> {
        id: Option<u64>,
        root: Option<Node<P>>,
        groups: Option<Vec<Group<P>>>,
        panels: Option<Vec<P>>,
        active: usize,
        rect: Option<[f32; 4]>,
        native: Option<[f32; 4]>,
        monitor: Option<Monitor>,
        monitor_size: Option<[f32; 2]>,
        minimized: bool,
    }

    impl<'de, P: Deserialize<'de>> Deserialize<'de> for Repr<P> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            use serde::de::{Error, MapAccess, Visitor};
            use std::marker::PhantomData;

            const FIELDS: &[&str] = &["id", "root", "groups", "panels", "active", "rect", "native", "monitor", "monitor_size", "minimized"];
            struct V<P>(PhantomData<P>);

            impl<'de, P: Deserialize<'de>> Visitor<'de> for V<P> {
                type Value = Repr<P>;

                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("a floating window")
                }

                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Repr<P>, A::Error> {
                    let mut repr = Repr {
                        id: None,
                        root: None,
                        groups: None,
                        panels: None,
                        active: 0,
                        rect: None,
                        native: None,
                        monitor: None,
                        monitor_size: None,
                        minimized: false,
                    };
                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "id" => repr.id = Some(map.next_value()?),
                            "root" => repr.root = Some(map.next_value()?),
                            "groups" => repr.groups = Some(map.next_value()?),
                            "panels" => repr.panels = Some(map.next_value()?),
                            "active" => repr.active = map.next_value()?,
                            "rect" => repr.rect = Some(map.next_value()?),
                            "native" => repr.native = map.next_value()?,
                            "monitor" => repr.monitor = map.next_value()?,
                            "monitor_size" => repr.monitor_size = map.next_value()?,
                            "minimized" => repr.minimized = map.next_value()?,
                            other => return Err(Error::unknown_field(other, FIELDS)),
                        }
                    }
                    Ok(repr)
                }
            }

            deserializer.deserialize_struct("Floating", FIELDS, V(PhantomData))
        }
    }

    /// Floating windows with a fresh id for each one whose id an earlier window took.
    pub(super) fn unique_ids<'de, D: Deserializer<'de>, P: Deserialize<'de>>(deserializer: D) -> Result<Vec<Floating<P>>, D::Error> {
        let mut windows = Vec::<Floating<P>>::deserialize(deserializer)?;
        let mut seen = std::collections::HashSet::new();
        for window in &mut windows {
            while !seen.insert(window.id) {
                window.id = fresh_window_id();
            }
        }
        Ok(windows)
    }

    impl<'de, P: Deserialize<'de>> Deserialize<'de> for Floating<P> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = Repr::<P>::deserialize(deserializer)?;
            let mut minimized = repr.minimized;
            let root = match (repr.root, repr.groups, repr.panels) {
                (Some(root), None, None) => root,
                (None, Some(groups), None) => {
                    minimized |= !groups.is_empty() && groups.iter().all(|g| g.minimized);
                    stack_tree(groups).ok_or_else(|| serde::de::Error::invalid_length(0, &"at least one group"))?
                }
                (None, None, Some(panels)) => Node::Tabs { panels, active: repr.active },
                (None, None, None) => return Err(serde::de::Error::missing_field("root")),
                _ => return Err(serde::de::Error::duplicate_field("root")),
            };
            Ok(Floating {
                id: repr.id.unwrap_or_else(fresh_window_id),
                root,
                rect: repr.rect.ok_or_else(|| serde::de::Error::missing_field("rect"))?,
                native: repr.native,
                monitor: repr.monitor,
                monitor_size: repr.monitor_size,
                minimized,
            })
        }
    }
}
