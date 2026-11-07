//! The completion list of a text control (18.2): the popup shows the candidates the text adapter holds, and a click
//! on a row hands the pick back to the adapter, which accepts it in its next run.

use crate::contract::{Bind, CollectionEvent, Component, Cx, Overlay, OverlayUi, Overlays as _, Props as _, ShowCollection as _};
use crate::controls::select::{OptionEvent, OptionList, OptionRow};
use crate::primitives::TextKey;
use crate::runtime::env::Label;
use craft_behavior::{DisplayOrder, ItemSource, Selection};
use std::borrow::Cow;

/// The labels of the candidates as a collection source, keyed by position.
struct Candidates<'a> {
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Candidates<'_> {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.labels.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Candidates<'_> {}

/// Shows the `completions` popup of a component whose text part is `key`: one row per candidate with the highlighted
/// one selected. A text control that takes completion calls this from its `overlay` hook.
pub(crate) fn show<C: Component, V: 'static>(ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, C>, key: &TextKey<V>) {
    let popup = cx.popup("completions");
    let (candidates, highlight) = cx.text(key).map_or_else(|| (Vec::new(), 0), |text| (text.completions.clone(), text.completion_index));
    let labels: Vec<Label> = candidates.iter().map(|candidate| cx.env.resolve(&candidate.label)).collect();
    let events = ui.show(popup, Overlay::dropdown(), |ui, _| {
        let selection = Selection::new(vec![highlight], Some(highlight), Some(highlight));
        OptionList::new(Bind::given(selection))
            .show_items(ui, &Candidates { labels: &labels }, &mut |at, _| OptionRow::new(labels[*at].as_str()).label(labels[*at].clone()))
            .events
    });
    let picked = events.into_iter().flatten().find_map(|event| match event {
        CollectionEvent::Activate(at) | CollectionEvent::Item(at, OptionEvent::Picked) => Some(at),
        _ => None,
    });
    if let Some(at) = picked {
        cx.pick_completion(at);
    }
}
