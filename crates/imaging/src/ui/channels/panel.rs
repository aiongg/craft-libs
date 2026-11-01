//! `ChannelsPanel`: list wiring, events, selection and thumbnails.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::canvas::textures::{ThumbSource, Thumbnails};
use craft_ui::list::ThumbSize;
use egui::{AsIdSalt, Context, Id, IdSalt, Ui};

use super::labels::ChannelsLabels;
use super::model::{ChannelRow, ChannelsOutput};
use super::style::ChannelsStyle;

/// The Channels panel over the app's rows: composite, colour, layer-mask, alpha and spot
/// channels and Quick Mask, with thumbnails, eyes, the footer and the menus. It holds no
/// document; what the user did comes back as [`ChannelsEvent`](super::ChannelsEvent)s.
///
/// ```ignore
/// let rows = channel_rows(&doc, &view);                     // the app, once per pass
/// let out = ChannelsPanel::new("channels", &rows).thumbnails(&thumbs, &DocChannelThumbs { doc }).show(ui);
/// for event in out.events {
///     match event {
///         ChannelsEvent::Target(key) => app.run("channel.target", key),
///         ChannelsEvent::LoadSelection { key, operation } => app.load_selection(key, operation.name()),
///         _ => {}
///     }
/// }
/// ```
///
/// - **Layout**: the rows fill the panel's height above a
///   [`PanelFooter`](craft_ui::composites::PanelFooter); a [`List`](craft_ui::list::List) of
///   `ListRow`s in the layer form (`rows::channel_row`), every row one height
///   (`ListRowStyle::height` of the thumbnail side). The thumbnail side is
///   [`thumbnail_size`](Self::thumbnail_size)'s, else `channels.thumb` (28 at Compact, 30 at
///   Comfortable).
/// - **Selection**: the list shows the rows the app marks `targeted` as selected (several while
///   the composite is targeted), so it runs in Multiple mode over a selection rebuilt from the
///   rows each pass; its own changes are not kept. A click (Shift or not) reports
///   [`Target`](super::ChannelsEvent::Target); Cmd/Ctrl-click reports
///   [`LoadSelection`](super::ChannelsEvent::LoadSelection) with
///   `SelectionOperation::from_modifiers` instead; ↑ ↓ report `Target` for the new lead;
///   Cmd/Ctrl+Enter on the lead reports `LoadSelection`.
/// - **Eyes**: the list's toggle column; a click or a sweep reports
///   [`Visibility`](super::ChannelsEvent::Visibility); the eye's click doesn't target.
/// - **Double click and Enter**: on the thumbnail of a row with options, or Enter on it,
///   [`Options`](super::ChannelsEvent::Options); on the name of an alpha or spot row, rename in
///   place (also F2): Enter or a click elsewhere commits, Esc cancels; an empty or unchanged name
///   reports nothing, else [`Rename`](super::ChannelsEvent::Rename).
/// - **Drag**: any row drags with the list's ghost. Alpha and spot rows reorder: the list's
///   marker shows before or after another alpha or spot row and before Quick Mask, and nowhere
///   else ([`Reorder::accepts`](craft_ui::behavior::Reorder::accepts));
///   [`Reorder`](super::ChannelsEvent::Reorder) reports it (Cmd/Ctrl+] and [ too). Any row drops
///   on the footer's Delete, New and Load buttons. A release elsewhere does nothing; Esc cancels.
/// - **Menus**: a right-click or the Menu key opens the row's context menu
///   (`menu::row_menu_entries`); the panel menu is
///   [`panel_menu`](Self::panel_menu).
/// - **Tab**: one stop for the list, then each footer button.
/// - **AccessKit**: a `ListBox` named "Channels"; rows named by the channel name, selected from
///   `targeted`, described "Hidden" when not visible; the eye cells named "Visibility {row}";
///   thumbnails add no nodes.
/// - **Right to left**: the eye, thumbnail and chord follow the list's direction rules; the
///   chord sits at the trailing end.
#[must_use = "show the panel with `show`"]
pub struct ChannelsPanel<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    id_salt: IdSalt,
    rows: &'a [ChannelRow<K>],
    thumbnails: Option<(&'a Thumbnails<K>, &'a dyn ThumbSource<K>)>,
    thumbnail_size: Option<ThumbSize>,
    labels: Option<ChannelsLabels>,
}

/// One pass's shared state of the panel, lent to the rows.
pub(super) struct ChannelsContext<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    /// The context, for `Thumbnails::state` inside the list's row builder.
    pub(super) ctx: Context,
    /// The panel's id: the list, the menu and a pending rename keep state under it.
    pub(super) id: Id,
    /// The thumbnail cache and source; `None` draws placeholders only.
    pub(super) thumbnails: Option<(&'a Thumbnails<K>, &'a dyn ThumbSource<K>)>,
    /// The thumbnail side in points.
    pub(super) thumb_side: f32,
    /// The panel's words.
    pub(super) labels: &'a ChannelsLabels,
    /// The panel's style.
    pub(super) style: ChannelsStyle,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> ChannelsPanel<'a, K> {
    /// The panel showing `rows` in their order; `id_salt` keeps its scroll position, rename, drag
    /// and menu.
    pub fn new(id_salt: impl AsIdSalt, rows: &'a [ChannelRow<K>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rows, thumbnails: None, thumbnail_size: None, labels: None }
    }

    /// The thumbnail cache and the app's source of the channels' pictures, keyed by the rows'
    /// keys. Without it the rows draw placeholders.
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        self.thumbnails = Some((thumbnails, source));
        self
    }

    /// The user's Panel Options thumbnail size (Photoshop's; PhotoCraft sets none): `Small`,
    /// `Medium` and `Large` use `list.thumb.*`, `None` hides the thumbnails. Without it the side is
    /// `channels.thumb`.
    pub fn thumbnail_size(mut self, size: ThumbSize) -> Self {
        self.thumbnail_size = Some(size);
        self
    }

    /// The panel's words; the installed words by default.
    pub fn labels(mut self, labels: ChannelsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the panel over the available rect.
    pub fn show(self, ui: &mut Ui) -> ChannelsOutput<K> {
        // pc: crates/ui-egui/src/panels/channels_panel.rs:285-515 `show` (rows, clicks, drag, footer);
        // pc: crates/ui-egui/src/panels/channels_panel.rs:91-157 (click, Cmd-click, double click, rename);
        // pc: crates/ui-egui/src/panels/channels_panel.rs:207-215 `drop_insertion` (alpha targets).
        todo!()
    }
}

/// The thumbnail side for `size` (`None`: the style's `channels.thumb`): `list.thumb.*` for
/// `Small`, `Medium` and `Large`, 0 for `ThumbSize::None`, the points of `Px`.
pub(super) fn thumb_side(size: Option<ThumbSize>, style: &ChannelsStyle, row_style: &craft_ui::list::ListRowStyle) -> f32 {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:375 (28 Pro, 30 Studio: `channels.thumb`).
    todo!()
}
