//! Text accessibility (12.3, 18.4): the value, the text actions, the selection and one `TextRun` sub-node per run of
//! each visible line, for a text part and for an editor that draws its own caret. The facts come from the
//! component's layout and reach the run through `CxCore::text_access`; the rt access writer adds them to the node it
//! builds at step 16 ([`write`]), and a `SetTextSelection` request on the node maps back to positions through the run
//! nodes that write kept ([`selection_input`]).

use crate::contract::NodeInput;
use crate::contract::rt::access::{AccessAction, Built, BuiltSub};
use crate::primitives::Id;
use accesskit::{Action, ActionData, Node, Role, TextDirection, TextPosition, TextSelection};
use craft_behavior::{AccessRequest, EditorInput, PartKey, PartRef, TextPos};
use craft_ui_rt::{NodeId, Rect};
use std::ops::Range;

/// The part key of a run sub-node, indexed in document order.
const RUN: PartKey = PartKey::new("text_run");

/// One bidi run of a line, owned.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Run {
    pub bytes: Range<usize>,
    pub rtl: bool,
    /// Each character's start along the run, from the line's left edge.
    pub positions: Vec<f32>,
    pub widths: Vec<f32>,
}

/// One visible line of the text.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    pub text: String,
    pub rect: Rect,
    /// The line's runs in visual order; at least one, empty for an empty line.
    pub runs: Vec<Run>,
}

/// What a text part or an editor reports to AccessKit for one run.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextFacts {
    /// The widget that holds the text's key focus.
    pub widget: Id,
    /// The whole text, lines joined with `\n`.
    pub text: String,
    /// A password: no value and no run nodes.
    pub masked: bool,
    pub editable: bool,
    /// The text is shown in a disabled component.
    pub disabled: bool,
    pub lines: Vec<Line>,
    /// `(anchor, focus)` as line and byte positions.
    pub selection: Option<(TextPos, TextPos)>,
    /// The unit the component draws after the text, read after a shown text.
    pub unit: Option<String>,
    /// The value read while the text is empty (a mixed value).
    pub value_override: Option<String>,
}

/// A run sub-node as the last report wrote it, for mapping a request back to a position.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    node: accesskit::NodeId,
    line: usize,
    bytes: Range<usize>,
    text: String,
}

/// The run and character index `pos` falls in among `entries`: the run of its line that holds its byte, or the line's
/// last run.
fn end_of(entries: &[Entry], pos: TextPos) -> Option<TextPosition> {
    let mut on_line = entries.iter().filter(|entry| entry.line == pos.line).peekable();
    let mut found = None;
    while let Some(entry) = on_line.next() {
        found = Some(entry);
        if pos.byte < entry.bytes.end || on_line.peek().is_none() {
            break;
        }
    }
    let entry = found?;
    let within = pos.byte.saturating_sub(entry.bytes.start).min(entry.text.len());
    let character = entry.text.get(..within).map_or(entry.text.chars().count(), |head| head.chars().count());
    Some(TextPosition { node: entry.node, character_index: character })
}

/// Adds the text facts to `built`, the node of the ui-rt node `node` placed at `origin` (window coordinates), and its
/// run sub-nodes, whose bounds are in the node's space; returns the entries a selection request maps back through. The owner's node is the one node of the text, so a component whose root edits
/// its value through the text (a text field, a spin button, an editable combo box) is one node that takes focus and
/// the text actions. Call it after the node's common facts are written.
pub(crate) fn write(built: &mut Built, facts: &TextFacts, node: NodeId, origin: craft_ui_rt::Pos2) -> Vec<Entry> {
    let mut entries = Vec::new();
    if !facts.masked {
        for (line_at, line) in facts.lines.iter().enumerate() {
            for run in &line.runs {
                let text = line.text.get(run.bytes.clone()).unwrap_or_default();
                let part = PartRef { key: RUN, index: u64::try_from(entries.len()).ok() };
                let key = crate::contract::rt::node::part_id(part);
                let mut sub = Node::new(Role::TextRun);
                sub.set_value(text);
                let rect = line.rect.translate(craft_ui_rt::Pos2::ZERO - origin);
                sub.set_bounds(accesskit::Rect::new(f64::from(rect.min.x), f64::from(rect.min.y), f64::from(rect.max.x), f64::from(rect.max.y)));
                sub.set_character_lengths(text.chars().map(|c| u8::try_from(c.len_utf8()).unwrap_or(4)).collect::<Vec<_>>());
                sub.set_character_positions(run.positions.clone());
                sub.set_character_widths(run.widths.clone());
                sub.set_text_direction(if run.rtl { TextDirection::RightToLeft } else { TextDirection::LeftToRight });
                built.subs.push(BuiltSub { key, part, parent: None, node: sub });
                entries.push(Entry { node: craft_ui_rt::access_sub_id(node, key), line: line_at, bytes: run.bytes.clone(), text: text.to_owned() });
            }
        }
    }
    let owner = &mut built.node;
    if let Some((anchor, focus)) = facts.selection
        && let (Some(anchor), Some(focus)) = (end_of(&entries, anchor), end_of(&entries, focus))
    {
        owner.set_text_selection(TextSelection { anchor, focus });
    }
    // An empty editor can name a mixed value without turning its label into editable text.
    if !facts.masked {
        let value = if facts.text.is_empty() {
            facts.value_override.clone().unwrap_or_default()
        } else if let Some(unit) = &facts.unit {
            format!("{} {unit}", facts.text)
        } else {
            facts.text.clone()
        };
        owner.set_value(value);
    }
    owner.add_action(Action::SetTextSelection);
    if facts.editable && !facts.disabled {
        owner.add_action(Action::SetValue);
    } else {
        owner.set_read_only();
    }
    if facts.disabled {
        owner.set_disabled();
    }
    entries
}

/// Whether `request` is a text request a text part serves: `SetValue` with text or `SetTextSelection`.
pub(crate) fn is_text_request(request: &AccessRequest) -> bool {
    matches!(
        (request.action, &request.data),
        (Action::SetValue, Some(ActionData::Value(_))) | (Action::SetTextSelection, Some(ActionData::SetTextSelection(_)))
    )
}

/// The text a `SetValue` request carries.
pub(crate) fn requested_value(request: &AccessRequest) -> Option<String> {
    match (request.action, &request.data) {
        (Action::SetValue, Some(ActionData::Value(text))) => Some(text.to_string()),
        _ => None,
    }
}

/// The selection a `SetTextSelection` request names, as line and byte positions of the run nodes the last report
/// wrote.
fn requested_selection(entries: &[Entry], selection: &TextSelection) -> Option<(TextPos, TextPos)> {
    let pos = |end: &TextPosition| {
        let entry = entries.iter().find(|entry| entry.node == end.node)?;
        let within = entry.text.char_indices().nth(end.character_index).map_or(entry.text.len(), |(byte, _)| byte);
        Some(TextPos::new(entry.line, entry.bytes.start + within))
    };
    Some((pos(&selection.anchor)?, pos(&selection.focus)?))
}

/// A `SetTextSelection` request on a node whose last report wrote `entries`, as the editor input that selects it:
/// queued in arrival order with the node's other input, so the editor or text part applies it where it came.
pub(crate) fn selection_input(entries: &[Entry], action: &AccessAction) -> Option<NodeInput> {
    let (Action::SetTextSelection, Some(ActionData::SetTextSelection(selection))) = (action.action, &action.data) else { return None };
    let (anchor, focus) = requested_selection(entries, selection)?;
    Some(NodeInput::Ime(EditorInput::select(anchor, focus)))
}
