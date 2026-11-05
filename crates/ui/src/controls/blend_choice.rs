//! [`BlendChoice`]: one entry of a blend-mode list, shared by the Layers panel's blend row and
//! the imaging editors (Layer Style's blend modes, the fill and adjustment dialogs).

/// One entry of the blend-mode list.
///
/// `craft_layers::ui::tree::BlendChoice` is this type, re-exported where the Layers panel first
/// defined it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlendChoice {
    /// The app's key of the mode, reported back in events.
    pub key: String,
    /// The mode's name as shown.
    pub label: String,
    /// A separator before this entry (Photoshop's groups of modes).
    pub separator_before: bool,
}
