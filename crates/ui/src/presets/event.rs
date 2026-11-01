//! What a `PresetBrowser` reports.

use egui::{Modifiers, Pos2, Response};

use super::footer::FooterEvent;
use crate::behavior::{ListDrag, Move};
use crate::edit::EditEvent;
use crate::list::OpenChange;

/// What a preset browser reports, in the order it happened. The host performs every command; the
/// browser changes only its `PresetState`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PresetEvent<K> {
    /// A click or Space on a preset. Not an edit: the host sets its selection (and applies the
    /// preset, in Swatches; `modifiers` let Alt, Ctrl or Cmd choose the background colour).
    Select {
        /// The preset.
        key: K,
        /// The modifiers held.
        modifiers: Modifiers,
    },
    /// A double click or Enter on a preset.
    Activate {
        /// The preset.
        key: K,
        /// The modifiers held.
        modifiers: Modifiers,
    },
    /// A group was opened or closed. The browser has already changed its state; the event lets a
    /// host with its own open state follow.
    Open(OpenChange<K>),
    /// A rename was committed with a changed, non-empty name.
    Rename {
        /// The preset or group.
        key: K,
        /// The new name.
        text: String,
    },
    /// Delete was chosen (context menu, Delete Group) for a preset or group.
    Delete(K),
    /// Move to was chosen: the preset goes to the end of `group`.
    MoveTo {
        /// The preset.
        key: K,
        /// The group (or folder) it goes to.
        group: K,
    },
    /// A drag to reorder or nest, or a Cmd/Ctrl+] or [ move, as one edit gesture. `Cancel` with
    /// `Dismissed` also reports a drag released over a host widget: the host ignores it and takes
    /// the payload from that widget.
    Reorder(EditEvent<Move<K>>),
    /// A preset was dragged from the browser and released inside its
    /// [`drop_zone`](super::PresetBrowser::drop_zone) (a canvas, a timeline, a layer list): the
    /// host applies `key` at `pos`. The drag's gesture has ended as `Reorder(Cancel(Dismissed))`
    /// in the same pass, before this event, and the drag payload is cleared. Items only: a group
    /// dropped on the zone reports nothing.
    DropAt {
        /// The preset.
        key: K,
        /// The pointer at the release, in points.
        pos: Pos2,
        /// The modifiers held at the release.
        modifiers: Modifiers,
    },
    /// An entry the host added to the context menu was chosen for `key`.
    Extra {
        /// The preset or group the menu was opened on.
        key: K,
        /// The id the host gave the entry.
        id: &'static str,
    },
}

/// What `PresetBrowser::show` returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the browser's events"]
pub struct PresetOutput<K> {
    /// The browser: its focus and its rect.
    pub response: Response,
    /// What happened this frame, in order.
    pub events: Vec<PresetEvent<K>>,
    /// While a preset or group is dragged, its payload, so the host can draw its own drop
    /// highlight; `None` otherwise.
    pub drag: Option<ListDrag<K>>,
    /// The preset under the pointer this pass, for a hover preview (LightCraft's develop presets
    /// show the photo with it); `None` off the list, over a header, or while a drag is under way.
    /// A field, not an event: a pass without a change sends nothing.
    pub hovered: Option<K>,
    /// The buttons of the [`footer`](super::PresetBrowser::footer) pressed this pass, in order;
    /// empty without one.
    pub footer: Vec<FooterEvent>,
}
