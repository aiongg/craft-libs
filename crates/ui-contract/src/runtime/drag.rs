//! The drag session (8.4): one per context. It holds the payload, the preview, the held modifiers, the operation and the
//! mark of the drop target that would take the payload, as data. The drop-target registrations of a pass resolve in the
//! next pass, like the press arena's shapes. The ghost layer paints from this data (`contract::ghost`).

use crate::runtime::env::{Env, Label};

use crate::style::UiTokens;
use craft_behavior::{CopyKeys, DropOperation};
use craft_ui_rt::NodeId;
use egui::{Pos2, Rect, Vec2};
use std::any::Any;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

/// What the ghost layer paints beside the pointer during a drag.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum DragPreview {
    /// Nothing.
    #[default]
    None,
    /// A badge with a short text ("3 files").
    Badge(Label),
    /// Nothing beside the pointer: the payload's name shows inside the area of a region mark, as a panel or group
    /// being moved names the place it would take. With no mark, nothing shows.
    Named(Label),
    /// An outline of `size` centred on the pointer, with an optional label inside its top corner: the ghost of a part
    /// dragged out of a row (a thumbnail).
    #[non_exhaustive]
    Frame {
        /// The size of the frame, in points.
        size: Vec2,
        /// A name shown inside the frame.
        label: Option<Label>,
    },
}

impl DragPreview {
    /// A frame of `size` with an optional `label`.
    pub fn frame(size: Vec2, label: Option<Label>) -> Self {
        DragPreview::Frame { size, label }
    }
}

/// Where a drop would land, as geometry in the target's egui layer space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum DropMark {
    /// An insertion line.
    #[non_exhaustive]
    Line {
        /// One end.
        from: Pos2,
        /// The other end.
        to: Pos2,
    },
    /// The payload goes into this rect.
    Into(Rect),
    /// The payload will occupy this area once dropped, such as the rect a docked panel takes beside a group.
    #[non_exhaustive]
    Region {
        /// The rect the payload takes.
        area: Rect,
        /// Where in it the payload goes in, such as the tab position of a panel joining a group.
        line: Option<[Pos2; 2]>,
    },
}

impl crate::runtime::Shift for DropMark {
    fn shifted(&self, by: &egui::RecordShift) -> Self {
        self.mapped(|point| by.pos(point))
    }

    fn snapped(&self, to: &Self, within: f32) -> Self {
        let pos = |value: Pos2, to: Pos2| crate::runtime::Shift::snapped(&value, &to, within);
        let rect = |value: Rect, to: Rect| Rect::from_min_max(pos(value.min, to.min), pos(value.max, to.max));
        match (self, to) {
            (DropMark::Line { from, to: end }, DropMark::Line { from: to_from, to: to_end }) => {
                DropMark::Line { from: pos(*from, *to_from), to: pos(*end, *to_end) }
            }
            (DropMark::Into(target), DropMark::Into(to_target)) => DropMark::Into(rect(*target, *to_target)),
            (DropMark::Region { area, line }, DropMark::Region { area: to_area, line: to_line }) => DropMark::Region {
                area: rect(*area, *to_area),
                line: match (line, to_line) {
                    (Some([a, b]), Some([to_a, to_b])) => Some([pos(*a, *to_a), pos(*b, *to_b)]),
                    _ => *line,
                },
            },
            _ => *self,
        }
    }
}

impl DropMark {
    /// An insertion line from `from` to `to`.
    pub const fn line(from: Pos2, to: Pos2) -> Self {
        DropMark::Line { from, to }
    }

    /// The payload will occupy `area`.
    pub const fn region(area: Rect) -> Self {
        DropMark::Region { area, line: None }
    }

    /// The mark with every point passed through `map`.
    pub(crate) fn mapped(self, map: impl Fn(Pos2) -> Pos2) -> Self {
        let rect = |rect: Rect| Rect::from_two_pos(map(rect.min), map(rect.max));
        match self {
            DropMark::Line { from, to } => DropMark::Line { from: map(from), to: map(to) },
            DropMark::Into(target) => DropMark::Into(rect(target)),
            DropMark::Region { area, line } => DropMark::Region { area: rect(area), line: line.map(|line| line.map(&map)) },
        }
    }
}

/// A file the OS drags over the window or drops on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct OsFile {
    /// The file's name; empty while the OS tells only a hovered file's type.
    pub name: String,
    /// Where the file is, on platforms that give a path.
    pub path: Option<PathBuf>,
    /// The file's media type, as far as the OS knows it.
    pub mime: String,
    /// The file's content, on platforms that give no path; read only for a dropped file.
    pub bytes: Option<Arc<[u8]>>,
}

/// The payload of a drag that comes from the OS: files held over the window, then the files dropped. Targets take it
/// like any payload, with `Pointer::drop_target` or `Pointer::window_drop_target`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct OsFiles {
    /// The files, in the order the OS lists them.
    pub files: Vec<OsFile>,
}

/// What a component registers with `Pointer::drop_target`.
pub trait DropTarget<P: ?Sized> {
    /// What a drop on this target carries to the component's caller.
    type Drop;

    /// `None` rejects. `at` is the pointer in the component's egui layer space, so a target picks its sub-area (a port,
    /// a split side) from it; `op` is the operation the held modifiers pick, which a target may refuse.
    fn accept(&self, payload: &P, at: Pos2, op: DropOperation) -> Option<Self::Drop>;

    /// Where `drop` would land, a value the ghost layer paints.
    fn mark(&self, drop: &Self::Drop) -> DropMark;

    /// Whether the target takes OS file drops (`OsFiles`) while a modal layer above it holds input, as an app whose
    /// alerts let files through (VectorCraft placed files dropped under its alert). By default a modal refuses them.
    fn under_modal(&self) -> bool {
        false
    }

    /// The held key that makes a drop on this target a copy: the `op` that [`accept`](Self::accept) gets, and the
    /// pointer over the target, follow it.
    fn copy_keys(&self) -> CopyKeys {
        CopyKeys::Alt
    }
}

/// What a component registers with `Pointer::window_drop_target`: a target over the whole window, optionally waiting
/// for a pointer position before it accepts.
pub trait WindowDropTarget<P> {
    /// What a drop on this target carries to the component's caller.
    type Drop;

    /// Whether this target waits for a pointer position before taking a drop.
    fn requires_pointer(&self) -> bool {
        false
    }

    /// `None` rejects. `op` is the operation the held modifiers pick, which a target may refuse.
    fn accept(&self, payload: &P, op: DropOperation) -> Option<Self::Drop>;

    /// Where `drop` would land, a value the ghost layer paints.
    fn mark(&self, drop: &Self::Drop) -> DropMark;

    /// Whether the target takes OS file drops (`OsFiles`) while a modal layer holds input, as
    /// [`DropTarget::under_modal`] says.
    fn under_modal(&self) -> bool {
        false
    }
}

/// Where a drop target lies: a rect on an egui layer, in that layer's space. A component's own target lies over its
/// visible rect on its layer; a component that takes drops over an area it does not draw (a preset browser's canvas)
/// names the area with [`DropZone::new`] and registers with [`zone_target`].
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
pub struct DropZone {
    owner: Option<NodeId>,
    rect: Rect,
}

impl DropZone {
    /// The zone `rect` of the node `owner` (the node of the `Ui` that draws the area; `None` for the window's own
    /// layer), in that node's space.
    pub const fn new(owner: Option<NodeId>, rect: Rect) -> Self {
        DropZone { owner, rect }
    }

    /// The zone's rect, in its owner's space.
    pub const fn rect(&self) -> Rect {
        self.rect
    }
}

/// Where a drop target takes drops beyond its own mark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TargetReach {
    /// The target covers the whole window: any target over a component beats it.
    pub(crate) window: bool,
    /// The target takes OS files under a modal layer (`DropTarget::under_modal`).
    pub(crate) under_modal: bool,
}

/// A target of payload type `P` seen through the payload's erased type.
pub(crate) struct Typed<'t, P, T>(&'t T, PhantomData<fn(&P)>);

impl<'t, P, T> Typed<'t, P, T> {
    pub(crate) fn new(target: &'t T) -> Self {
        Typed(target, PhantomData)
    }
}

impl<P: 'static, T: DropTarget<P>> DropTarget<dyn Any + Send + Sync> for Typed<'_, P, T> {
    type Drop = T::Drop;

    fn accept(&self, payload: &(dyn Any + Send + Sync), at: Pos2, op: DropOperation) -> Option<T::Drop> {
        self.0.accept(payload.downcast_ref::<P>()?, at, op)
    }

    fn mark(&self, drop: &T::Drop) -> DropMark {
        self.0.mark(drop)
    }

    fn under_modal(&self) -> bool {
        self.0.under_modal()
    }

    fn copy_keys(&self) -> CopyKeys {
        self.0.copy_keys()
    }
}

/// What the ghost layer paints: the session's preview and the mark of the target that would take a drop.
pub(crate) struct Ghost {
    pub preview: DragPreview,
    /// The env of the region the drag started in.
    pub env: Option<Env>,
    /// The mark of the target that would take a drop, and the tokens of the target's region.
    pub mark: Option<(DropMark, Arc<UiTokens>)>,
}
