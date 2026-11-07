//! The text nodes of AccessKit: a text component's value, actions and selection, and the `TextRun` nodes below it.

use egui::{Context, Id, Rect};

/// A selection end as AccessKit names it: a run node's key and a character index in the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeEnd {
    pub node: u64,
    pub character: usize,
}

/// One `TextRun` node.
pub(crate) struct RunNode<'a> {
    pub id: Id,
    pub parent: Id,
    pub rect: Rect,
    pub text: &'a str,
    pub rtl: bool,
    /// Each character's start offset from the run's left edge, and its advance.
    pub positions: &'a [f32],
    pub widths: &'a [f32],
}

/// What a text component's own node says beyond the common facts.
pub(crate) struct TextNode<'a> {
    pub value: &'a str,
    pub editable: bool,
    /// The anchor and focus of the selection, `None` while the text has no runs.
    pub selection: Option<(Id, usize, Id, usize)>,
}

fn lengths(text: &str) -> Vec<u8> {
    text.chars().map(|c| u8::try_from(c.len_utf8()).unwrap_or(u8::MAX)).collect()
}

/// The character index each word starts at.
fn word_starts(text: &str) -> Vec<u8> {
    let mut starts = Vec::new();
    let mut previous_space = true;
    for (at, c) in text.chars().enumerate() {
        if previous_space && !c.is_whitespace() {
            starts.push(u8::try_from(at).unwrap_or(u8::MAX));
        }
        previous_space = c.is_whitespace();
    }
    starts
}

/// Creates and fills the `TextRun` node of `run` below its parent.
pub(crate) fn write_run(ctx: &Context, run: &RunNode<'_>) {
    super::register_parent(ctx, run.id, run.parent);
    crate::runtime::egui::widgets::note_node(ctx, run.id);
    ctx.accesskit_node_builder(run.id, |node| {
        node.set_role(accesskit::Role::TextRun);
        node.set_bounds(accesskit::Rect {
            x0: f64::from(run.rect.min.x),
            y0: f64::from(run.rect.min.y),
            x1: f64::from(run.rect.max.x),
            y1: f64::from(run.rect.max.y),
        });
        node.set_value(run.text.to_owned());
        node.set_character_lengths(lengths(run.text));
        node.set_character_positions(run.positions.to_vec());
        node.set_character_widths(run.widths.to_vec());
        node.set_word_starts(word_starts(run.text));
        node.set_text_direction(if run.rtl { accesskit::TextDirection::RightToLeft } else { accesskit::TextDirection::LeftToRight });
    });
}

/// Adds the text value, the text actions and the selection to the node of `id`.
pub(crate) fn write_text(ctx: &Context, id: Id, text: &TextNode<'_>) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_value(text.value.to_owned());
        node.add_action(accesskit::Action::SetTextSelection);
        if text.editable {
            node.add_action(accesskit::Action::SetValue);
            node.add_action(accesskit::Action::ReplaceSelectedText);
        }
        if let Some((anchor, anchor_at, focus, focus_at)) = text.selection {
            node.set_text_selection(accesskit::TextSelection {
                anchor: accesskit::TextPosition { node: anchor.accesskit_id(), character_index: anchor_at },
                focus: accesskit::TextPosition { node: focus.accesskit_id(), character_index: focus_at },
            });
        }
    });
}

/// The selection a `SetTextSelection` request carries.
pub(crate) fn requested_selection(request: &craft_behavior::AccessRequest) -> Option<(NodeEnd, NodeEnd)> {
    if request.action != accesskit::Action::SetTextSelection {
        return None;
    }
    let Some(accesskit::ActionData::SetTextSelection(selection)) = &request.data else { return None };
    let end = |position: accesskit::TextPosition| NodeEnd { node: position.node.0, character: position.character_index };
    Some((end(selection.anchor), end(selection.focus)))
}

/// The text a `SetValue` request carries.
pub(crate) fn requested_value(request: &craft_behavior::AccessRequest) -> Option<String> {
    if request.action != accesskit::Action::SetValue {
        return None;
    }
    match &request.data {
        Some(accesskit::ActionData::Value(text)) => Some(text.to_string()),
        _ => None,
    }
}

/// Whether `request` is one the text adapter serves.
pub(crate) fn is_text_request(request: &craft_behavior::AccessRequest) -> bool {
    requested_selection(request).is_some() || requested_value(request).is_some()
}
