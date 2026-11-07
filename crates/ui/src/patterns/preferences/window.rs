//! `PreferencesWindow`: the preferences body in a modeless `Dialog` that opens in its own OS window where windows
//! exist.

#[cfg(feature = "testing")]
mod stories;

use super::body::{self, BodyOut};
use super::model::{Closed, PreferencesModel, PreferencesOutput};
use super::words::PreferencesWords;
use crate::contract::{Bind, Props as _, SaveKey, Show as _, ShowContainer as _};
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
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// The footer indices of the OK and Apply buttons, in the order they are given to the dialog.
const OK_AT: usize = 0;
const APPLY_AT: usize = 1;

/// What the window keeps between passes; the host owns it and binds it with [`PreferencesWindow::new`].
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PreferencesState {
    /// The id of the page the user picked. While a search hides it another page shows, and this one returns when the
    /// search is cleared.
    pub selected: String,
    /// The text in the search field. Empty lists every page.
    pub search: String,
    /// The message of a failed Apply, shown in the footer until the next edit or Apply.
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
///   draft. A failed apply keeps the window open and shows its message in the footer. Enter is OK and Escape
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
        let apply = FooterButton::new(words.apply.clone(), DialogRole::Other).enabled(differs).stays_open(true);
        let buttons = [ok, apply, FooterButton::new(common.cancel.clone(), DialogRole::Cancel)];

        let mut open = true;
        let error = work.error.clone();
        let failure = RefCell::new(None);
        let applied_by_ok = Cell::new(false);
        let edited = Cell::new(false);
        let mut body_out = BodyOut::default();
        let host_cell = RefCell::new(&mut *host);
        let mut dialog = Dialog::new(title.clone()).collection_body().modeless().resizable(true).close_on_enter(can_ok).close_guard(|at| {
            if at != OK_AT {
                return true;
            }
            match host_cell.borrow_mut().apply() {
                Ok(()) => {
                    applied_by_ok.set(true);
                    true
                }
                Err(message) => {
                    *failure.borrow_mut() = Some(message);
                    false
                }
            }
        });
        if !embedded {
            dialog = dialog.viewport();
        }
        if let Some(key) = remember {
            dialog = dialog.remember(key);
        }
        if let Some(message) = &error {
            dialog = dialog.footer(|ui, _| {
                Notice::new(Tone::Bad, Label::from(message.clone())).layout(NoticeLayout::Inline).show(ui);
            });
        }
        let shown = dialog.opened(Bind::bound(&mut open)).enabled(!nested).buttons(buttons).show(ui, |ui, _| {
            body_out = BodyOut::default();
            let mut host = host_cell.borrow_mut();
            body::show_body(ui, &mut **host, &snap, &mut work, &words, &units, &mut body_out);
            let edits = body_out.edits();
            edited.set(!edits.is_empty() || body_out.reset);
            for (key, value) in edits {
                host.set(&key, value);
            }
            if body_out.reset
                && let Some(page) = &snap.shown
            {
                host.reset_page(&page.id);
            }
            host.nested(ui.ctx());
        });
        drop(host_cell);

        let DialogOutput { response, closed, picked, .. } = shown.output;
        let mut result = PreferencesOutput::new(response);
        if edited.get() {
            work.error = None;
        }
        if let Some(message) = failure.into_inner() {
            work.error = Some(message);
        }
        if picked == Some(APPLY_AT) && !nested {
            match host.apply() {
                Ok(()) => {
                    work.error = None;
                    result.applied = true;
                }
                Err(message) => work.error = Some(message),
            }
        }
        match closed.filter(|_| !nested) {
            Some(CloseReason::Activated) => {
                result.applied = applied_by_ok.get();
                result.closed = Some(Closed::Ok);
            }
            Some(CloseReason::Enter) => match host.apply() {
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
