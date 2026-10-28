//! Live regions: a container whose text a screen reader announces when it appears.
//!
//! AccessKit inherits `live` from the parent and announces every named descendant when it is
//! added, so a toast would also read out its "Dismiss" button. [`region`] puts the content in a
//! child container whose `live` is off, and names the region by the texts handed to [`Labels`]:
//! only that name is announced. The region's node id is the caller's, so a region that moves
//! (a toast whose older neighbour closed) keeps its node and is not announced again.

use craft_theme::Tone;
use egui::accesskit::{Live, Role};
use egui::{Id, InnerResponse, Ui, UiBuilder};

/// How soon an announcement interrupts the screen reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Politeness {
    /// After what is being read.
    Polite,
    /// At once.
    Assertive,
}

impl Politeness {
    fn live(self) -> Live {
        match self {
            Politeness::Polite => Live::Polite,
            Politeness::Assertive => Live::Assertive,
        }
    }
}

/// The role and politeness WAI-ARIA gives a message of `tone`: errors are `Alert` and assertive,
/// every other tone `Status` and polite.
pub fn for_tone(tone: Tone) -> (Role, Politeness) {
    match tone {
        Tone::Bad => (Role::Alert, Politeness::Assertive),
        _ => (Role::Status, Politeness::Polite),
    }
}

/// The texts that name a live region, in reading order.
#[derive(Debug, Default)]
pub struct Labels(Vec<String>);

impl Labels {
    /// Adds a text drawn in the region; the region is announced as its texts, in the order added.
    pub fn add(&mut self, text: impl Into<String>) {
        self.0.push(text.into());
    }

    /// The announcement: "title, line, line". A text that already ends in punctuation is
    /// followed by a space alone.
    pub fn announcement(&self) -> String {
        let mut out = String::new();
        for text in self.0.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            if !out.is_empty() {
                out.push_str(if out.ends_with(['.', ',', ';', ':', '!', '?', '…']) { " " } else { ", " });
            }
            out.push_str(text);
        }
        out
    }
}

/// Shows `add_contents` in a live region of `role` (`Status`, `Alert`, ...) whose node id is
/// `id`. The region is named by the texts the closure adds to the [`Labels`]; everything drawn
/// inside is silent.
///
/// `id` must be unique and must stay with the message across frames: a region whose id changes
/// is a new node, announced again.
pub fn region<R>(ui: &mut Ui, id: Id, role: Role, politeness: Politeness, add_contents: impl FnOnce(&mut Ui, &mut Labels) -> R) -> InnerResponse<R> {
    let mut labels = Labels::default();
    let mut silent = None;
    let inner = ui.scope_builder(UiBuilder::new().scope_id(id).accessibility_role(role), |ui| {
        let inner = ui.scope_builder(UiBuilder::new(), |ui| (ui.unique_id(), add_contents(ui, &mut labels)));
        let (silent_id, result) = inner.inner;
        silent = Some(silent_id);
        result
    });
    let ctx = ui.ctx();
    if let Some(silent) = silent {
        ctx.accesskit_node_builder(silent, |node| node.set_live(Live::Off));
    }
    let name = labels.announcement();
    ctx.accesskit_node_builder(id, |node| {
        node.set_live(politeness.live());
        node.set_label(name);
    });
    inner
}
