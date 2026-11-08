//! `NavigatorOptionsDialog`: Panel Options of the Navigator panel.

use super::{ColorRequest, FormDialog, HostAnswer, HostRequests};
use crate::canvas::{CanvasWords, NAVIGATOR_PROXY_COLOR_KEY, NavigatorOptions, ProxyColor};
use crate::contract::{Bind, Entry, NoValue, Outcome, Remember as _};
use crate::contract::{Show as _, ShowContainer as _};
use crate::form::{Field, FieldGroup};
use crate::pickers::Select;
use crate::primitives::IconSpec;
use crate::runtime::env::{Env, Label};
use crate::runtime::overlay::CloseReason;
use egui::{Response, Ui};

#[cfg(feature = "testing")]
mod stories;

/// The custom colour the dialog offered last, kept while it is open.
#[derive(Clone, Default)]
struct LastCustom(Option<[u8; 3]>);

/// What Panel Options reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NavigatorOptionsDialogOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The options, the pass the dialog closed as OK.
    pub accepted: Option<NavigatorOptions>,
    /// The options after the pass when they differ from the bind's at the start of the pass: a bind that does not
    /// write through applies them.
    pub value: Option<NavigatorOptions>,
    /// The colour request of Custom…: the app opens its colour picker and answers with
    /// [`HostRequests::answer_color`], then hands the answers back with [`NavigatorOptionsDialog::answers`].
    pub requests: HostRequests,
}

impl Outcome for NavigatorOptionsDialogOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// Panel Options of the Navigator: the colour of the proxy rectangle.
///
/// - The Color list shows the named colours, each with its swatch, then Custom…. One Tab stop; the arrows pick.
/// - Custom… asks the app for a colour with a request keyed [`NAVIGATOR_PROXY_COLOR_KEY`]; the colour becomes Custom
///   when the app answers, and a cancelled picker leaves the colour as it was.
/// - Enter is OK and Escape cancels. Holding Alt turns Cancel into Reset, which restores the options from when the
///   dialog opened.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NavigatorOptionsDialog<'a> {
    options: Bind<'a, NavigatorOptions>,
    /// The app's answers to earlier colour requests.
    #[prop]
    answers: Vec<HostAnswer>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
}

impl<'a> NavigatorOptionsDialog<'a> {
    /// Panel Options over the options in `options`.
    pub fn new(options: Bind<'a, NavigatorOptions>) -> Self {
        NavigatorOptionsDialog { options, answers: Vec::new(), open: None }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> NavigatorOptionsDialogOutput {
        let NavigatorOptionsDialog { options, answers, open } = self;
        let words = Env::of(ui).words::<CanvasWords>();
        let title = Label::from(words.navigator_options_title.clone());
        let color_label = Label::from(words.navigator_proxy_color.clone());
        let picker_title = Label::from(words.navigator_picker_title.clone());
        let mut form = FormDialog::new(title.clone(), options).answers(answers);
        if let Some(open) = open {
            form = form.opened(open);
        }
        let shown = form.show(ui, |ui, cx, options| {
            for answer in cx.take_answers() {
                if let HostAnswer::Color { key, color } = answer
                    && key == NAVIGATOR_PROXY_COLOR_KEY
                {
                    options.proxy_color = ProxyColor::custom(color);
                }
            }
            if let ProxyColor::Custom(rgb) = options.proxy_color {
                cx.memory::<LastCustom>().0 = Some(rgb);
            }
            let custom = cx.memory::<LastCustom>().0.unwrap_or(ProxyColor::DEFAULT_CUSTOM);
            let entries: Vec<Entry<ProxyColor>> = ProxyColor::ALL
                .into_iter()
                .chain([ProxyColor::Custom(custom)])
                .map(|color| Entry::new(color, color.label(&words)).icon(IconSpec::swatch(color.color())))
                .collect();
            let mut picked = None;
            FieldGroup::new(title.clone()).show(ui, |ui, _| {
                Field::new(color_label.clone()).show(ui, |ui, _| {
                    let list = Select::new(color_label.clone(), Bind::given(options.proxy_color), &entries).show(ui);
                    picked = list.event.as_ref().and_then(|event| event.value().cloned());
                });
            });
            match picked {
                Some(chosen @ ProxyColor::Custom(_)) => {
                    cx.requests().request_color(ColorRequest::new(NAVIGATOR_PROXY_COLOR_KEY, chosen.color()).title(picker_title.clone()));
                }
                Some(named) => options.proxy_color = named,
                None => {}
            }
            true
        });
        NavigatorOptionsDialogOutput {
            response: shown.response,
            closed: shown.closed,
            accepted: shown.accepted,
            value: shown.value,
            requests: shown.requests,
        }
    }
}
