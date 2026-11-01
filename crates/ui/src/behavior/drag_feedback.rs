#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! What the user sees while dragging: the [`DragBadge`] at the pointer saying what is dragged
//! or what a release would do, the per-frame [drop hint](set_drop_hint) a target gives it, and
//! the [`DropVeil`] over the window while files from outside hover over it.
//!
//! A [`List`](crate::list::List)'s own row drag draws its ghost box
//! ([`paint_ghost`](crate::list::paint_ghost)); the badge is for drags that leave their list
//! (photos from a grid onto an album, a layer onto another document) and for drags no list
//! owns.

use std::borrow::Cow;
use std::path::PathBuf;

use egui::{AsIdSalt, Context, IdSalt};

crate::words! {
    /// The words of drag feedback, English by default.
    #[non_exhaustive]
    pub struct DragFeedbackLabels {
        /// The badge's default text: `{n}` the number of items (LightCraft's "{n} photos"). A
        /// [`Counted`](crate::localized::words::Counted) word ("{n} item", "{n} items"), written
        /// with [`Words::counted`](crate::localized::Words::counted); S5C-F9 adds it to the
        /// kernel's `counts` and [`COUNTS`](crate::localized::words::COUNTS).
        count: "drag.count" = "{n} items";
    }
}

/// The badge that follows the pointer during a drag, after LightCraft's photo and album drags.
///
/// ```ignore
/// // While photos are dragged (every frame, after the drop targets drew):
/// DragBadge::count("photos", ids.len()).show(ctx);
/// // In an album row under the pointer:
/// set_drop_hint(ctx, format!("Add to “{}”", album.name));
/// ```
///
/// - **Look.** A pill on the tooltip layer, 14 points right of and 10 below the pointer
///   (LightCraft's `pos + vec2(14, 10)`), filled with `accent.solid` (`shape.pill` radius), 9 by 3
///   points of padding, its text in the `BodyStrong` role and `text.on_accent`; not interactable. The
///   window shows the grabbing cursor.
/// - **Text.** This pass's [drop hint](drop_hint) when a target under the pointer gave one,
///   else the badge's own text ([`count`](Self::count): [`DragFeedbackLabels::count`] in the
///   locale's plural form).
/// - **Order.** The drop targets draw first and set their hint, then the host shows the badge,
///   in the same pass; a hint set after the badge drew is dropped at the end of the pass.
/// - **End.** The host stops showing it when the drag ends (the button up, or Esc); drop targets
///   act on the release pass before it.
/// - It asks for a repaint each frame it shows, so it follows the pointer.
pub struct DragBadge<'a> {
    id_salt: IdSalt,
    text: Option<Cow<'a, str>>,
    count: Option<usize>,
    labels: Option<DragFeedbackLabels>,
}

impl<'a> DragBadge<'a> {
    /// A badge saying `text` ("Paris.jpg", an album's name); `id_salt` names its layer.
    pub fn new(id_salt: impl AsIdSalt, text: impl Into<Cow<'a, str>>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), text: Some(text.into()), count: None, labels: None }
    }

    /// A badge counting `n` items in the installed words.
    pub fn count(id_salt: impl AsIdSalt, n: usize) -> Self {
        Self { id_salt: IdSalt::new(id_salt), text: None, count: Some(n), labels: None }
    }

    /// Words other than the installed ones (LightCraft's "{n} photos").
    pub fn labels(mut self, labels: DragFeedbackLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // lightcraft: crates/ui-egui/src/panels/grid.rs:1120 drag_feedback; panels/left/albums.rs:366 (album drag)
    pub fn show(self, ctx: &Context) {
        todo!()
    }
}

/// What a release on the drop target under the pointer would do ("Add to “Trips”"), for this
/// pass's [`DragBadge`]. A target calls it each pass the pointer is over it while a drag it
/// takes is under way, before the badge is shown; the hint lasts this pass only.
// lightcraft: crates/ui-egui/src/panels/grid.rs:1115 drag_hint
pub fn set_drop_hint(ctx: &Context, text: impl Into<String>) {
    todo!()
}

/// This pass's drop hint, if a target gave one.
// lightcraft: crates/ui-egui/src/panels/grid.rs:1133 (hint for this frame)
pub fn drop_hint(ctx: &Context) -> Option<String> {
    todo!()
}

/// The files hovering over the window this pass, from egui's `hovered_files`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HoveredFiles {
    /// The paths the platform gave, non-empty ones only.
    pub paths: Vec<PathBuf>,
    /// How many came without a path (browsers, some platforms); the app counts them in its own
    /// words (LightCraft's "{n} files").
    pub unnamed: usize,
}

impl HoveredFiles {
    /// This pass's hovering files; empty when none.
    // lightcraft: crates/ui-egui/src/drop_overlay.rs:81 (count, known)
    pub fn of(ctx: &Context) -> Self {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.unnamed == 0
    }
}

/// The veil over the window while files from outside hover over it, after LightCraft's drop
/// overlay: what dropping them will do, in the app's words, before the user lets go.
///
/// ```ignore
/// let files = HoveredFiles::of(ctx);
/// if !files.is_empty() && !app.blocks_input() {
///     DropVeil::new("drop", &describe_drop(&files)).show(ctx);
/// }
/// ```
///
/// - **Look.** On the tooltip layer over the whole window: `surface.scrim` (LightCraft's black at
///   150 of 255), a 2-point `accent.solid` frame 10 points in with 12-point corners, and a card in
///   the middle (`surface.raised`, 10-point corners, 1-point `accent.solid` edge) with the
///   lines centred in the `Title` role, 8 points apart, 24 points of padding at the top and 32 at
///   the sides, wrapping at the window's width less 120 (at least 200).
/// - **When.** The host shows it each frame [`HoveredFiles::of`] is not empty and its drop would
///   land (LightCraft skips it while a modal prompt holds the input, so the veil never promises
///   a drop the barrier would refuse). It takes no input and asks for a repaint.
/// - **AccessKit.** The card is a live `Alert` region with the lines as its text.
pub struct DropVeil<'a> {
    id_salt: IdSalt,
    lines: &'a [String],
}

impl<'a> DropVeil<'a> {
    /// A veil saying `lines` ("Import 3 photos", "1 preset"; the app sorts the files the way
    /// its drop does, LightCraft's `classify`); `id_salt` names its layer.
    pub fn new(id_salt: impl AsIdSalt, lines: &'a [String]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), lines }
    }

    // lightcraft: crates/ui-egui/src/drop_overlay.rs:73 show
    pub fn show(self, ctx: &Context) {
        todo!()
    }
}
