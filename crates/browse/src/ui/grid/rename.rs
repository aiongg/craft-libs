//! The rename field of a tile's caption.

use craft_ui::controls::{ControlSize, TextField};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::list::RenameEnd;
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Align, Id, Layout, Rect, Ui, UiBuilder, vec2};

/// The open rename: the tile, the text typed so far and whether the field is new (it takes the
/// focus and selects its text).
#[derive(Clone, Debug)]
pub(crate) struct Rename<K> {
    pub key: K,
    pub text: String,
    pub fresh: bool,
}

/// What the field showed this pass: the text, and how the rename ended, if it did.
pub(crate) struct Outcome {
    pub text: String,
    pub end: Option<RenameEnd>,
}

/// Shows the field for the tile `tile` in `band` (the caption's rect) with `rename`'s text.
pub(crate) fn show<K>(ui: &mut Ui, tile: Id, band: Rect, label: &str, rename: &Rename<K>) -> Outcome {
    let height = TextField::style(&Tokens::of(ui), ControlSize::Sm).height.min(band.height());
    let band = Rect::from_center_size(band.center(), vec2(band.width(), height));
    // A child, not a scope: a scope would move the parent's cursor into the tile it was allocated.
    let mut child = ui.new_child(UiBuilder::new().id_salt(tile.with("rename-scope")).max_rect(band).layout(Layout::top_down_justified(Align::Min)));
    let ui = &mut child;
    let field = Interactive::id_in(ui, "rename");
    if rename.fresh {
        ui.memory_mut(|m| m.request_focus(field));
        let mut state = TextEditState::default();
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(rename.text.chars().count()))));
        state.store(ui.ctx(), field);
    }
    let edit = TextField::new("rename", label, &rename.text).size(ControlSize::Sm).show(ui);
    let mut text = rename.text.clone();
    let mut cancelled = false;
    match &edit.event {
        Some(
            EditEvent::Begin { value: Some(value), .. }
            | EditEvent::Update { value, .. }
            | EditEvent::Set { value, .. }
            | EditEvent::Commit { value, .. },
        ) => text.clone_from(value),
        Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }) => cancelled = true,
        _ => {}
    }
    let end = if cancelled || edit.escaped {
        Some(RenameEnd::Escape)
    } else if edit.submitted {
        Some(RenameEnd::Enter)
    } else if edit.response.lost_focus() && !rename.fresh {
        Some(RenameEnd::Blur)
    } else {
        None
    };
    Outcome { text, end }
}
