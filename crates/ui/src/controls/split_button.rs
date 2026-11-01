#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`SplitButton`]: an action button with an arrow part that opens a menu of variants or presets.

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::{ButtonVariant, ControlSize};
use crate::icons::Icon;
use crate::menu::{MenuEntry, MenuResponse};

/// What the button reported this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct SplitButtonResponse<T> {
    /// The main part: `clicked()` runs the action.
    pub main: Response,
    /// The arrow part.
    pub arrow: Response,
    /// The menu, and the row chosen from it.
    pub menu: MenuResponse<T>,
}

/// A button whose end opens a menu, after VectorCraft's Image Trace button and Paragraph panel
/// buttons (Illustrator's Image Trace ▾).
///
/// ```ignore
/// let presets = trace_presets.iter().map(|p| MenuItem::new(&p.name, p.id).into()).collect::<Vec<_>>();
/// let out = SplitButton::new("trace", "Image Trace", &presets).arrow_label("Tracing Presets").show(ui);
/// if out.main.clicked() { trace(current) }
/// if let Some(id) = out.menu.chosen { trace(id) }
/// ```
///
/// - **Look.** A [`Button`](crate::controls::Button) of the variant and size whose end is an
///   arrow part 18 points wide (VectorCraft's `SPLIT_ARROW`; `split_button.arrow_width`) with a
///   chevron down, split by a 1-point divider inset 5 points top and bottom in the button's
///   border colour. Each part has its own hover and pressed state layer; the outer corners are
///   the button's, the inner ones square.
/// - **Menu.** A press on the arrow opens a [`PressMenu`](crate::menu::PressMenu) of `entries`
///   under the button, aligned to its start edge, so press-drag-release chooses in one gesture;
///   a press on the main part runs the action and never opens the menu. While the menu is open,
///   a press outside it (the main part included) only closes it (lead call L21: the closing
///   press does nothing else); a press on the arrow closes it, as an anchor toggles its menu.
/// - **Keys.** One Tab stop: Space and Enter run the action; Alt+↓ (and ↓) open the menu with
///   its first row active. Esc closes the menu and leaves the focus on the button.
/// - **AccessKit.** The main part is a `Button` named by the label; the arrow a child `Button`
///   named by [`arrow_label`](Self::arrow_label) (the label alone without it) with `has_popup`
///   menu and `expanded` while open.
pub struct SplitButton<'a, T> {
    id_salt: IdSalt,
    label: String,
    entries: &'a [MenuEntry<T>],
    arrow_label: Option<String>,
    icon: Option<Icon>,
    variant: ButtonVariant,
    size: ControlSize,
    enabled: bool,
}

impl<'a, T: Clone + Send + Sync + 'static> SplitButton<'a, T> {
    /// A split button showing `label` (as shown, translated) whose arrow opens `entries`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, entries: &'a [MenuEntry<T>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            entries,
            arrow_label: None,
            icon: None,
            variant: ButtonVariant::Secondary,
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// The arrow's tooltip and accessible name ("Tracing Presets").
    pub fn arrow_label(mut self, label: impl Into<String>) -> Self {
        self.arrow_label = Some(label.into());
        self
    }

    /// A leading icon on the main part.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // vectorcraft: crates/ui-egui/src/widgets/buttons.rs:65 split_button_named
    pub fn show(self, ui: &mut Ui) -> SplitButtonResponse<T> {
        todo!()
    }
}
