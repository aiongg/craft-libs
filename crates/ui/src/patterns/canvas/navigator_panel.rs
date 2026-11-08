//! `NavigatorPanel`: the Navigator panel's body, the preview box over the zoom row, bound to one view.

use crate::canvas::{NavigationOptions, Navigator, NavigatorPreview, NavigatorZoomRow, ProxyColor, ViewEase};
use crate::contract::{Bind, NoValue, Outcome, Props as _, Show as _, ShowContainer as _};
use crate::layout_components::Stack;
use crate::runtime::env::Label;
use craft_behavior::kurbo::Size;
use craft_behavior::{CanvasView, EditEvent};
use egui::{Response, Ui};

/// What the Navigator panel reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// The view change of this pass, from the preview box or the zoom row, as one change. With [`ease`](Self::ease)
    /// set, it is the change to the target of a zoom, which the bound view has not taken.
    pub view: Option<EditEvent<CanvasView>>,
    /// How [`view`](Self::view) eases to its target: a zoom with Animated Zoom on. The app starts a
    /// [`ZoomAnimation`] from the bound view with the ease's anchor and then writes the target.
    ///
    /// [`ZoomAnimation`]: crate::canvas::ZoomAnimation
    pub ease: Option<ViewEase>,
}

impl Outcome for NavigatorPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The Navigator panel (Photoshop's Window, Navigator): the [`Navigator`] preview box over the [`NavigatorZoomRow`],
/// both bound to one view.
///
/// - Both parts edit the one bound view; a zoom that eases (Animated Zoom) is reported, not written, by either.
/// - Tab goes from the preview box to the zoom row's percent field, then through the row in reading order.
/// - `enabled(false)` disables the preview box and every control of the zoom row.
#[must_use = "show the panel with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NavigatorPanel<'a> {
    label: Label,
    view: Bind<'a, CanvasView>,
    document: Size,
    /// The picture in the preview box.
    #[prop]
    preview: NavigatorPreview<'a>,
    /// The app's navigation options, for both parts.
    #[prop]
    navigation: NavigationOptions,
    /// The colour of the proxy rectangle.
    #[prop]
    proxy: ProxyColor,
    /// Whether the preview box and the zoom row take input.
    #[prop]
    enabled: bool,
}

impl<'a> NavigatorPanel<'a> {
    /// A Navigator panel named `label` that edits `view`, over a document of `document` units.
    pub fn new(label: impl Into<Label>, view: Bind<'a, CanvasView>, document: Size) -> Self {
        NavigatorPanel {
            label: label.into(),
            view,
            document,
            preview: NavigatorPreview::None,
            navigation: NavigationOptions::default(),
            proxy: ProxyColor::Default,
            enabled: true,
        }
    }

    /// Shows the panel.
    pub fn show(self, ui: &mut Ui) -> NavigatorPanelOutput {
        let NavigatorPanel { label, mut view, document, preview, navigation, proxy, enabled } = self;
        // The body runs once per show; the picture's painter is taken there.
        let mut preview = Some(preview);
        let mut change: Option<EditEvent<CanvasView>> = None;
        let mut ease: Option<ViewEase> = None;
        let shown = Stack::new().show(ui, |ui, _| {
            let picture = preview.take().unwrap_or(NavigatorPreview::None);
            let preview = Navigator::new(label.clone(), view.reborrowed(), document)
                .preview(picture)
                .navigation(navigation)
                .proxy(proxy)
                .enabled(enabled)
                .show(ui);
            let row = NavigatorZoomRow::new(view.reborrowed(), document).navigation(navigation).enabled(enabled).show(ui);
            for (event, eases) in [(preview.view.event.clone(), preview.ease), (row.view, row.ease)] {
                if let Some(event) = event {
                    change = EditEvent::fold(change.take(), event);
                }
                ease = eases.or(ease);
            }
        });
        NavigatorPanelOutput { response: shown.output, view: change, ease }
    }
}
