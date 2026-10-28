//! Live regions: a container whose text a screen reader announces when it appears.
//!
//! AccessKit inherits `live` from the parent and announces every labelled descendant when it is
//! added, so a toast would also read out its "Dismiss" button. [`region`] puts the content in a
//! child container whose `live` is off: only the region's own label (the text of the nodes handed
//! to [`Labels`]) is announced.

use craft_theme::Tone;
use egui::accesskit::{Live, Role};
use egui::{Id, InnerResponse, Response, Ui, UiBuilder};

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

/// The text nodes that name a live region.
#[derive(Debug, Default)]
pub struct Labels(Vec<Id>);

impl Labels {
    /// Adds the text `response` drew; the region is announced as its text, in the order added.
    pub fn add(&mut self, response: &Response) {
        self.0.push(response.id);
    }
}

/// Shows `add_contents` in a live region of `role` (`Status`, `Alert`, ...). The region is named
/// by the texts the closure adds to the [`Labels`]; everything else drawn inside is silent.
pub fn region<R>(ui: &mut Ui, role: Role, politeness: Politeness, add_contents: impl FnOnce(&mut Ui, &mut Labels) -> R) -> InnerResponse<R> {
    let mut labels = Labels::default();
    let mut ids = None;
    let inner = ui.scope_builder(UiBuilder::new().accessibility_role(role), |ui| {
        let region = ui.unique_id();
        let silent = ui.scope_builder(UiBuilder::new(), |ui| (ui.unique_id(), add_contents(ui, &mut labels)));
        let (silent_id, result) = silent.inner;
        ids = Some((region, silent_id));
        result
    });
    if let Some((region, silent)) = ids {
        let ctx = ui.ctx();
        ctx.accesskit_node_builder(silent, |node| node.set_live(Live::Off));
        ctx.accesskit_node_builder(region, |node| {
            node.set_live(politeness.live());
            for id in &labels.0 {
                node.push_labelled_by(id.accesskit_id());
            }
        });
    }
    inner
}
