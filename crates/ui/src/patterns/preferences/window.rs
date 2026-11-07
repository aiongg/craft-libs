//! `PreferencesWindow`: the preferences body in a modeless `Dialog` that opens in its own OS window where windows
//! exist.

#[cfg(feature = "testing")]
mod stories;

use super::body::{self, BodyOut};
use super::model::{Closed, PreferencesModel, PreferencesOutput};
use super::words::PreferencesWords;
use crate::buttons::Button;
use crate::contract::{Bind, Props as _, SaveKey, Show as _, ShowContainer as _, clicked};
use crate::dialog::{Dialog, DialogOutput};
use crate::notices::{Notice, NoticeLayout};
use crate::runtime::env::{Env, Label};
use crate::runtime::overlay::CloseReason;
use crate::runtime::words::CommonWords;
use crate::sections::{DialogRole, FooterButton};
use craft_behavior::{EditEvent, GestureId, Mixed};
use craft_i18n::matcher::Matcher;
use craft_theme::variant::Tone;
use egui::Ui;
use std::cell::Cell;
use std::sync::Arc;

/// What the window keeps between passes; the host owns it and binds it with [`PreferencesWindow::new`].
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PreferencesState {
    /// The id of the page the user picked. While a search hides it another page shows, and this one returns when the
    /// search is cleared.
    pub selected: String,
    /// The text in the search field. Empty lists every page.
    pub search: String,
    /// The message of a failed Apply, shown over the buttons until the next edit or Apply.
    pub error: Option<String>,
}

/// The preferences window: a search field over a page list and the shown page, with OK, Cancel and Apply under them.
///
/// The host keeps the model ([`PreferencesModel`]) and calls [`show`](PreferencesWindow::show) on every pass the window
/// should exist; it stops calling to close the window, and does so when the output says `closed`.
///
/// - **Pages and search.** The host builds the pages each pass; the window lists the pages that match the search (the
///   empty ones dimmed) and shows the picked one, or the first listed when the search hides it. Typing in the search
///   field filters; Up, Down and Enter in it go to the page list.
/// - **Rows.** Each row is the library's control for its `RowKind`; an edit goes to [`PreferencesModel::set`] and
///   nothing is saved before OK or Apply. Reset Section resets the shown page's rows in the draft.
/// - **Buttons.** OK applies and closes, Apply applies and stays (enabled while the draft differs), Cancel drops the
///   draft. A failed apply keeps the window open and shows its message over the buttons. Enter is OK and Escape
///   cancels, once the search text, a field's own gesture and a popup have had them.
/// - **Nested dialogs.** While [`PreferencesModel::nested_open`] is true the window takes no input and refuses to
///   close; [`PreferencesModel::nested`] draws the nested dialogs inside the window's own viewport.
/// - **Placement.** The window opens in its own OS window where there are any, else as a modeless dialog layer
///   ([`embedded`](PreferencesWindow::embedded) forces the layer). [`remember`](PreferencesWindow::remember) keeps
///   its place and size across runs.
#[must_use = "show the window with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct PreferencesWindow<'a> {
    state: Bind<'a, PreferencesState>,
    /// The window's title; the translated "Preferences" without it.
    #[prop(into)]
    title: Option<Label>,
    /// Draws the window as a dialog layer even where OS windows exist.
    #[prop(flag)]
    embedded: bool,
    /// Keeps the window's place and size across runs under `key`.
    #[prop]
    remember: Option<SaveKey>,
}

impl<'a> PreferencesWindow<'a> {
    /// A window over the host's `state`.
    pub fn new(state: Bind<'a, PreferencesState>) -> Self {
        PreferencesWindow { state, title: None, embedded: false, remember: None }
    }

    /// Shows the window for this pass over `host`. The output's `closed` tells the host to stop showing it.
    pub fn show<H: PreferencesModel + ?Sized>(self, ui: &mut Ui, host: &mut H) -> PreferencesOutput {
        let PreferencesWindow { mut state, title, embedded, remember } = self;
        let env = Env::of(ui);
        let words = env.words::<PreferencesWords>();
        let common = env.words::<CommonWords>();
        let units = env.units();
        let before = match state.reborrow().get() {
            Some(Mixed::Same(held)) => held,
            _ => PreferencesState::default(),
        };
        let mut work = before.clone();
        let matcher = Matcher::new(&env.folder(), work.search.trim());
        let snap = body::snapshot(&*host, &matcher, &work.selected);

        let nested = host.nested_open();
        let can_ok = host.can_ok() && !nested;
        let differs = host.differs() && !nested;
        let title = title.unwrap_or_else(|| words.title.clone());
        let mut ok = FooterButton::new(common.ok.clone(), DialogRole::Accept).enabled(can_ok);
        if !can_ok {
            ok = ok.disabled_reason(words.cannot_ok.clone());
        }
        let buttons = [ok, FooterButton::new(common.cancel.clone(), DialogRole::Cancel)];

        let mut open = true;
        let apply_clicked = Cell::new(false);
        let error = work.error.clone();
        let mut body_out = BodyOut::default();
        let mut dialog = Dialog::new(title.clone()).modeless().resizable(true).close_on_enter(can_ok);
        if !embedded {
            dialog = dialog.viewport();
        }
        if let Some(key) = remember {
            dialog = dialog.remember(key);
        }
        let shown = dialog
            .opened(Bind::bound(&mut open))
            .enabled(!nested)
            .buttons(buttons)
            .footer(|ui, _| {
                if let Some(message) = &error {
                    Notice::new(Tone::Bad, Label::from(message.clone())).layout(NoticeLayout::Inline).show(ui);
                }
                if clicked(&Button::new(words.apply.clone()).enabled(differs).show(ui)) {
                    apply_clicked.set(true);
                }
            })
            .show(ui, |ui, _| {
                body_out = BodyOut::default();
                body::show_body(ui, &mut *host, &snap, &mut work, &words, &units, &mut body_out);
                host.nested(ui.ctx());
            });

        let DialogOutput { response, closed, .. } = shown.output;
        let mut result = PreferencesOutput::new(response);
        let edits = body_out.edits();
        let edited = !edits.is_empty() || body_out.reset;
        for (key, value) in edits {
            host.set(&key, value);
        }
        if body_out.reset
            && let Some(page) = &snap.shown
        {
            host.reset_page(&page.id);
        }
        if edited {
            work.error = None;
        }
        if apply_clicked.get() && !nested {
            match host.apply() {
                Ok(()) => {
                    work.error = None;
                    result.applied = true;
                }
                Err(message) => work.error = Some(message),
            }
        }
        match closed.filter(|_| !nested) {
            Some(CloseReason::Enter | CloseReason::Activated) => match host.apply() {
                Ok(()) => {
                    result.applied = true;
                    result.closed = Some(Closed::Ok);
                }
                Err(message) => work.error = Some(message),
            },
            Some(_) => {
                host.cancel();
                result.closed = Some(Closed::Cancel);
            }
            None => {}
        }
        if result.closed.is_some() {
            work.search.clear();
            work.error = None;
        }
        result.page = snap.shown.map(|page| page.id);
        if work != before {
            let name: Arc<str> = title.as_str().into();
            let _ = state.reborrow().apply(&EditEvent::set(GestureId::default(), Mixed::Same(before), work, name, None));
        }
        result
    }
}
