//! Which path overlay a pen or anchor tool runs, and with what: [`PathMode::resolve`] (gap row 39).

use egui::Modifiers;

use super::BezierTool;

/// What the pen draft does with presses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PenMode {
    /// The Pen: clicks add corner anchors, drags pull out handles (the draft as built).
    #[default]
    Pen,
    /// The Curvature Pen: clicks add smooth anchors the curve flows through; handles follow
    /// [`curvature_handles`](crate::geometry::curvature::curvature_handles).
    Curvature,
}

/// The host's path tool, the input of [`PathMode::resolve`]. The Path Selection and Freeform Pen
/// tools are the app's (step7-pen lead call 27).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathTool {
    Pen,
    Curvature,
    AddAnchor,
    DeleteAnchor,
    ConvertPoint,
    DirectSelection,
}

/// What a pass of the pen and path tools shows: the [`BezierOverlay`](super::BezierOverlay)'s
/// tool, and the pen draft's mode when it shows (`None`: no draft overlay this pass).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PathMode {
    /// The path overlay's tool.
    pub bezier: BezierTool,
    /// The pen draft's mode, or `None` when the draft is not shown.
    pub draft: Option<PenMode>,
    /// Whether the draft takes presses ([`PenDraft::sensing`](super::PenDraft::sensing)): false
    /// while ⌘ makes the pointer Direct Selection and under the anchor tools, so the draft is
    /// drawn and keeps its keys while the overlay edits it ([`resolve`](Self::resolve)).
    pub draft_senses: bool,
}

impl PathMode {
    /// The table of which overlay a tool and the held keys select, PhotoCraft's `direct`,
    /// `converting`, `curvature_idle`, `pen_auto` and the anchor tools' `effective`, for `tool`
    /// with `modifiers` held, `drawing` true while a pen draft is open, and the Pen's
    /// `auto_add_delete` option:
    ///
    /// | Tool | Keys | `bezier` | `draft` | `draft_senses` |
    /// |---|---|---|---|---|
    /// | any pen or anchor tool, Convert Point | ⌘ | `Direct` | the tool's draft mode if `drawing`, else `None` | false |
    /// | Pen | none, drawing | `Show` | `Pen` | true |
    /// | Pen | none, not drawing | `AutoAddDelete` if `auto_add_delete`, else `Show` | `Pen` | true |
    /// | Pen | ⌥, not drawing | `Convert` | `Pen` | true |
    /// | Pen | ⇧ or ⌥, drawing; ⇧, not drawing | `Show` | `Pen` | true |
    /// | Curvature | none or ⇧ or ⌥, not drawing | `Curvature` | `Curvature` | true |
    /// | Curvature | drawing | `Show` | `Curvature` | true |
    /// | Add Anchor, Delete Anchor | no ⌘ | `AddAnchor`, `DeleteAnchor` (the overlay swaps them with ⌥) | `Pen` if `drawing` (shown, not sensing), else `None` | false |
    /// | Convert Point | no ⌘ | `Convert` | as Add Anchor | false |
    /// | Direct Selection | any | `Direct` | as Add Anchor | false |
    ///
    /// The draft's own Auto Add/Delete (while drawing) is [`PenDraft::auto_add_delete`](super::PenDraft::auto_add_delete),
    /// which the host passes the same option to; it is off while ⇧, ⌥ or ⌘ is held.
    ///
    /// Where the draft shows without sensing (⌘, Add Anchor, Delete Anchor, Convert Point, Direct
    /// Selection), the anchor tools edit it as PhotoCraft's do (`anchor_tools.rs:116-128`,
    /// `PathRef::Pen`; Photoshop edits an open path with them too): the host passes the open draft
    /// to the [`BezierOverlay`](super::BezierOverlay) as one more [`EditPath`](super::EditPath),
    /// last in the slice under a key of its own, and applies the overlay's `insert`, `delete` and
    /// `edit` for that key to its draft ([`EditSubpath::insert_at`](super::EditSubpath::insert_at),
    /// [`remove_knot`](super::EditSubpath::remove_knot), [`PathEdit::apply`](super::PathEdit::apply)).
    /// The draft is not in the document, so these are draft changes, not document commands, and
    /// drawing goes on from the draft's last anchor.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-B5)")]
    pub fn resolve(tool: PathTool, modifiers: Modifiers, drawing: bool, auto_add_delete: bool) -> PathMode {
        // pc: crates/ui-egui/src/tools/direct_select.rs:135-153 `direct`, `converting`,
        // `curvature_idle`; crates/ui-egui/src/tools/anchor_tools.rs:27-35 `effective`, :50-60
        // `pen_auto`.
        todo!()
    }
}
