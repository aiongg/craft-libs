//! `ParamDialog`: a dialog from a parameter list, with an optional preview, and `HostedParamDialog`, the same dialog
//! owned by a `DialogHost`. A command form is a `ParamDialog` without a preview.

use super::fields::DialogFields;
use super::host_requests::HostRequests;
use super::param::{ColorRequest, EyedropperRequest, Param, ParamKind, PathRequest, open_fields};
use super::preview::{PreviewPane, PreviewZoom};
use super::rows::{ParamRows, ParamRowsOutput};
use crate::choice::Checkbox;
use crate::contract::{Bind, NoValue, Outcome, Props as _, Remember as _, SaveKey, Show as _, ShowContainer as _};
use crate::dialog::{Dialog, DialogId, DialogOutput, DialogResult, DialogStack, StackDialog, dialog_id};
use crate::layout_components::{Row, Stack};
use crate::runtime::env::{Env, Label};
use crate::runtime::memory::{MemoryStage, unique};
use crate::runtime::overlay::CloseReason;
use crate::runtime::words::CommonWords;
use crate::sections::{DialogRole, FooterBar, FooterButton};
use craft_behavior::{CanvasTransform, EditEvent, GestureId, Mixed};
use egui::load::SizedTexture;
use egui::{Response, Ui, Vec2};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

#[cfg(feature = "testing")]
mod stories;

craft_ui::words! {
    /// The words of the parameter dialog beside the common OK and Cancel.
    pub struct ParamDialogWords {
        reset: "formDialogs.reset" = "Reset";
        preview: "formDialogs.preview" = "Preview";
    }
}

/// Where the preview pane sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParamLayout {
    /// The preview pane to the right of the rows, under the Preview check box and the zoom row.
    #[default]
    Beside,
    /// The preview pane under the rows.
    Below,
}

/// What a parameter dialog answers with at OK.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ParamResult {
    /// The dialog's key: the command the values are for.
    pub key: String,
    /// The values the user confirmed, keyed by parameter.
    pub fields: DialogFields,
    remember: Option<(SaveKey, DialogFields)>,
}

impl ParamResult {
    /// Stores the values of a dialog built with [`ParamDialog::remember`] for the next open. The app calls it after
    /// its command succeeds, so a failed command leaves the previous values; without a remember key it does nothing.
    pub fn remember(&self, ui: &Ui) {
        if let Some((key, values)) = &self.remember {
            let mut stage = MemoryStage::open(ui.ctx(), unique(("craft_ui::param_result", key.as_str())));
            *stage.saved::<Option<DialogFields>>(key) = Some(values.clone());
            stage.write_back();
        }
    }
}

/// What a parameter dialog keeps while it is open: the Preview check box and the view of the preview pane.
#[derive(Clone)]
struct Live {
    on: bool,
    view: CanvasTransform,
}

impl Default for Live {
    fn default() -> Self {
        Live { on: true, view: CanvasTransform::new(1.0, Vec2::ZERO) }
    }
}

/// The values of a dialog that has no binding for them, between passes.
#[derive(Clone, Default)]
struct Held(DialogFields);

/// What a parameter dialog reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParamDialogOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The answer, the pass the dialog closed as OK.
    pub result: Option<ParamResult>,
    /// The rows' output of this pass, with the colour, path and eyedropper requests; `None` when the dialog is closed.
    pub rows: Option<ParamRowsOutput>,
    /// The values after the pass when they differ from the binding's or the dialog opened in this pass: a `values`
    /// binding that does not write through applies them, the opening snapshot included.
    pub values: Option<DialogFields>,
    /// Whether the Preview check box is on: the host renders the preview while it is.
    pub preview_on: bool,
}

impl Outcome for ParamDialogOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A dialog from a parameter list: one row per [`Param`], the Preview check box with a preview pane when the host
/// gives a picture, and OK, Cancel and a footer extra.
///
/// - Keys: Tab walks the fields, Enter is OK unless a field owns it, Escape cancels. Holding Alt turns Cancel into
///   Reset, which restores the values from when the dialog opened (or `reset_to`) and keeps the dialog open.
/// - Values: the `values` binding holds them; without one the dialog keeps its own between passes. The first pass
///   fills the parameters' defaults and, with `remember`, the values saved by [`ParamResult::remember`].
/// - Preview: the host renders from the reported values while `preview_on` and hands the picture to `preview`. Drag
///   pans the picture, the zoom buttons scale it.
/// - A command form is a dialog with no preview and no remember key.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct ParamDialog<'a> {
    key: String,
    title: Label,
    params: Cow<'a, [Param]>,
    /// Binds the values to `values`.
    #[prop(name = "values")]
    bound: Option<Bind<'a, DialogFields>>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// Saves the values under `key` when the app calls [`ParamResult::remember`], and opens with them.
    #[prop]
    remember: Option<SaveKey>,
    /// Whether OK can be pressed; Enter does nothing while it cannot.
    #[prop]
    ok_enabled: bool,
    /// Why OK is disabled: the tooltip of the disabled button.
    #[prop]
    ok_reason: Option<Label>,
    /// The accepting button's label when it is not OK.
    #[prop]
    ok_label: Option<Label>,
    /// The second button's label when it is not Cancel (Close, say).
    #[prop]
    cancel_label: Option<Label>,
    /// Whether holding Alt turns the second button into Reset; the default is true.
    #[prop]
    alt_resets: bool,
    /// Whether the dialog holds one outer gesture over its fields, from open to close.
    #[prop]
    outer_gesture: bool,
    /// Whether the body is one collection that owns the scroll (a browse list): the dialog's own body scroll is off.
    #[prop]
    collection_body: bool,
    /// The values Reset sets, instead of the values from when the dialog opened. Keys the target lacks keep their
    /// value.
    #[prop]
    reset_to: Option<DialogFields>,
    /// Where the preview pane sits.
    #[prop]
    layout: ParamLayout,
    /// Whether the title row has a close action.
    #[prop]
    closable: bool,
    /// The picture the preview pane shows; without one the dialog has no preview.
    #[prop]
    preview: Option<SizedTexture>,
    /// Controls before the buttons in the footer (Color Table's Load and Save).
    #[prop(func = "FnMut(&mut Ui)")]
    footer_extra: Option<FooterExtra<'a>>,
    /// A section under the rows that only reads the values (a monitor note).
    #[prop(func = "Fn(&mut Ui, &DialogFields)")]
    below: Option<BelowRows<'a>>,
    shaped: Option<Shaped<'a>>,
}

/// What `ParamDialog::shaped` applies to the built [`Dialog`].
struct Shaped<'a>(Box<dyn FnOnce(Dialog<'a>) -> Dialog<'a> + 'a>);

/// What `ParamDialog::footer_extra` draws before the buttons.
pub struct FooterExtra<'a>(Box<dyn FnMut(&mut Ui) + 'a>);

impl<'a> FooterExtra<'a> {
    /// The extra that runs `add`.
    pub fn new(add: impl FnMut(&mut Ui) + 'a) -> Self {
        FooterExtra(Box::new(add))
    }
}

/// What `ParamDialog::below` draws under the rows.
pub struct BelowRows<'a>(Box<dyn Fn(&mut Ui, &DialogFields) + 'a>);

impl<'a> BelowRows<'a> {
    /// The section that runs `draw`.
    pub fn new(draw: impl Fn(&mut Ui, &DialogFields) + 'a) -> Self {
        BelowRows(Box::new(draw))
    }
}

impl<'a> ParamDialog<'a> {
    /// A dialog for `params`, titled `title`, answering for the command `key`.
    pub fn new(key: impl Into<String>, title: impl Into<Label>, params: impl Into<Cow<'a, [Param]>>) -> Self {
        ParamDialog {
            key: key.into(),
            title: title.into(),
            params: params.into(),
            bound: None,
            open: None,
            remember: None,
            ok_enabled: true,
            ok_reason: None,
            ok_label: None,
            cancel_label: None,
            alt_resets: true,
            outer_gesture: false,
            collection_body: false,
            reset_to: None,
            layout: ParamLayout::default(),
            closable: true,
            preview: None,
            footer_extra: None,
            below: None,
            shaped: None,
        }
    }

    /// Changes the built [`Dialog`] before it shows, for the `Dialog` settings a form track needs and `ParamDialog`
    /// has no setter for (`Dialog::overlay`, say). It runs after the settings `ParamDialog` makes itself.
    #[must_use]
    pub fn shaped(mut self, shape: impl FnOnce(Dialog<'a>) -> Dialog<'a> + 'a) -> Self {
        self.shaped = Some(Shaped(Box::new(shape)));
        self
    }

    /// The part of `fields` a remembering dialog saves: numbers, angles, choices and check boxes of the remembered
    /// rows and their unit choices; colours, grids, points, channels and free text start from their defaults.
    pub fn remembered(params: &[Param], fields: &DialogFields) -> DialogFields {
        let kept = |param: &Param| {
            param.is_remembered()
                && matches!(
                    param.kind,
                    ParamKind::Range { .. }
                        | ParamKind::Angle { .. }
                        | ParamKind::Int { .. }
                        | ParamKind::Number { .. }
                        | ParamKind::Bool { .. }
                        | ParamKind::Choice { .. }
                )
        };
        let units = params.iter().filter(|param| param.is_remembered()).filter_map(Param::unit_key);
        let keys: Vec<&str> = params.iter().filter(|param| kept(param)).map(|param| param.key.as_str()).chain(units).collect();
        let mut values = fields.clone();
        values.retain_fitting(&keys, |key, value| params.iter().any(|param| param.accepts(key, value)));
        values
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> ParamDialogOutput {
        let ParamDialog {
            key,
            title,
            params,
            bound,
            open,
            remember,
            ok_enabled,
            ok_reason,
            ok_label,
            cancel_label,
            alt_resets,
            outer_gesture,
            collection_body,
            reset_to,
            layout,
            closable,
            preview,
            mut footer_extra,
            below,
            shaped,
        } = self;
        let env = Env::of(ui);
        let words = env.words::<ParamDialogWords>();
        let common = env.words::<CommonWords>();
        let unbound = bound.is_none();
        let mut bound = bound;
        let mut before = DialogFields::new();
        let mut written = DialogFields::new();
        let work = RefCell::new(DialogFields::new());
        let opened = Cell::new(false);

        let mut dialog = Dialog::new(title.clone()).closable(closable).close_on_enter(ok_enabled).outer_gesture(outer_gesture);
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        if collection_body {
            dialog = dialog.collection_body();
        }
        if let Some(Shaped(shape)) = shaped {
            dialog = shape(dialog);
        }
        let dialog = dialog.footer(|ui, footer| {
            if let Some(FooterExtra(extra)) = footer_extra.as_mut() {
                extra(ui);
            }
            let mut accept = FooterButton::new(ok_label.clone().unwrap_or_else(|| common.ok.clone()), DialogRole::Accept).enabled(ok_enabled);
            if let (false, Some(reason)) = (ok_enabled, &ok_reason) {
                accept = accept.disabled_reason(reason.clone());
            }
            let reset = ResetFooter::new(ui, cancel_label.clone(), alt_resets);
            let changed = work.borrow().changed();
            match reset.buttons(ui, accept, changed) {
                Some(ResetPick::Accept) => footer.request_close_as(CloseReason::Activated),
                Some(ResetPick::Reset) => ResetFooter::restore(&mut work.borrow_mut(), reset_to.as_ref()),
                Some(ResetPick::Cancel) => footer.request_close_as(CloseReason::Requested),
                None => {}
            }
        });

        let mut live = Live::default();
        let mut rows = None;
        let shown = dialog.show(ui, |ui, cx| {
            let first = cx.appeared();
            before = match bound.as_mut().and_then(|bind| bind.reborrow().read(cx)) {
                Some(Mixed::Same(held)) => held,
                _ => DialogFields::new(),
            };
            *work.borrow_mut() = if unbound && !first { cx.memory::<Held>().0.clone() } else { before.clone() };
            if first {
                let mut values = work.borrow_mut();
                if let Some(key) = &remember
                    && let Some(mut saved) = cx.saved::<Option<DialogFields>>(key).clone()
                {
                    saved = Self::remembered(&params, &saved);
                    for (name, value) in saved.iter() {
                        values.set(name, value.clone());
                    }
                }
                open_fields(&params, &mut values);
                opened.set(true);
            }
            live = cx.memory::<Live>().clone();
            let mut show_rows = |ui: &mut Ui| {
                let mut values = work.borrow_mut();
                rows = Some(ParamRows::new(title.clone(), &params, Bind::bound(&mut *values)).show(ui));
            };
            match (&preview, layout) {
                (Some(texture), ParamLayout::Beside) => {
                    Row::new().show(ui, |ui, _| {
                        show_rows(ui);
                        Stack::new().show(ui, |ui, _| show_preview(ui, &words, texture, &mut live));
                    });
                }
                (Some(texture), ParamLayout::Below) => {
                    Stack::new().show(ui, |ui, _| {
                        show_rows(ui);
                        show_preview(ui, &words, texture, &mut live);
                    });
                }
                (None, _) => show_rows(ui),
            }
            if let Some(BelowRows(draw)) = &below {
                draw(ui, &work.borrow());
            }
            *cx.memory::<Live>() = live.clone();
            if unbound {
                cx.memory::<Held>().0 = work.borrow().clone();
            }
            if let Some(bind) = bound.as_mut() {
                let after = work.borrow().clone();
                written = after.clone();
                if after != before || opened.get() {
                    let name: Arc<str> = title.as_str().into();
                    let _ = bind.reborrow().write(cx, &EditEvent::set(GestureId::default(), Mixed::Same(before.clone()), after, name, None));
                }
            }
        });

        let DialogOutput { response, closed, .. } = shown.output;
        let after = work.into_inner();
        let result = match closed {
            Some(CloseReason::Enter | CloseReason::Activated) if ok_enabled => {
                let kept = remember.map(|key| (key, Self::remembered(&params, &after)));
                Some(ParamResult { key, fields: after.clone(), remember: kept })
            }
            _ => None,
        };
        let values = (after != before || opened.get()).then_some(after);
        if let (Some(after), Some(bind)) = (&values, bound.as_mut())
            && *after != written
        {
            let name: Arc<str> = title.as_str().into();
            let _ = bind.reborrow().apply(&EditEvent::set(GestureId::default(), Mixed::Same(written), after.clone(), name, None));
        }
        ParamDialogOutput { response, closed, result, rows, values, preview_on: preview.is_some() && live.on }
    }
}

/// What a footer with a Reset alternative answered with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResetPick {
    /// The accepting button.
    Accept,
    /// The second button while Alt is held: the dialog resets its values and stays open.
    Reset,
    /// The second button while Alt is not held: the dialog closes without an answer.
    Cancel,
}

/// The footer of a dialog whose Cancel turns into Reset while Alt is held: Reset restores the values from when the
/// dialog opened (or a target) and keeps the dialog open. `ParamDialog` and every other dialog with this behaviour
/// build their footer with it.
///
/// ```ignore
/// let footer = ResetFooter::new(ui, None, true);
/// match footer.buttons(ui, FooterButton::new(common.ok.clone(), DialogRole::Accept), values.changed()) {
///     Some(ResetPick::Reset) => ResetFooter::restore(&mut values, None),
///     _ => {}
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ResetFooter {
    resetting: bool,
    cancel: Label,
    reset: Label,
}

impl ResetFooter {
    /// The footer for this pass: `cancel` names the second button when it is not Cancel; with `alt_resets` false
    /// the second button stays Cancel whatever is held.
    pub fn new(ui: &Ui, cancel: Option<Label>, alt_resets: bool) -> Self {
        let env = Env::of(ui);
        ResetFooter {
            resetting: alt_resets && crate::runtime::pointer::frame(ui.ctx()).mods.alt,
            cancel: cancel.unwrap_or_else(|| env.words::<CommonWords>().cancel.clone()),
            reset: env.words::<ParamDialogWords>().reset.clone(),
        }
    }

    /// Whether the second button is Reset in this pass.
    pub fn resetting(&self) -> bool {
        self.resetting
    }

    /// Lays out `accept` and the second button, which is Reset (enabled while `changed`) or Cancel, and answers with
    /// the one pressed.
    pub fn buttons(&self, ui: &mut Ui, accept: FooterButton, changed: bool) -> Option<ResetPick> {
        let second = if self.resetting {
            FooterButton::new(self.reset.clone(), DialogRole::Other).enabled(changed)
        } else {
            FooterButton::new(self.cancel.clone(), DialogRole::Cancel)
        };
        match FooterBar::buttons([accept, second]).show_footer(ui).picked {
            Some(0) => Some(ResetPick::Accept),
            Some(_) if self.resetting => {
                crate::runtime::frame::request_repaint(ui.ctx());
                Some(ResetPick::Reset)
            }
            Some(_) => Some(ResetPick::Cancel),
            None => None,
        }
    }

    /// Sets `values` to `target`, key by key (keys the target lacks keep their value), or to the values they
    /// opened with when there is no target.
    pub fn restore(values: &mut DialogFields, target: Option<&DialogFields>) {
        match target {
            Some(target) => {
                for (key, value) in target.iter() {
                    values.set(key, value.clone());
                }
            }
            None => {
                values.reset();
            }
        }
    }
}

/// The Preview check box, the zoom row and the pane.
fn show_preview(ui: &mut Ui, words: &ParamDialogWords, texture: &SizedTexture, live: &mut Live) {
    Checkbox::new(Bind::bound(&mut live.on)).label(words.preview.clone()).show(ui);
    let zoom = PreviewZoom::new(live.view).show(ui);
    let pane = PreviewPane::new(words.preview.clone(), *texture, live.view).show(ui);
    if let Some(view) = zoom.view.or(pane.view) {
        live.view = view;
    }
}

/// Builds the dialog over the host-owned values of a [`HostedParamDialog`] for one pass.
type Build = dyn for<'b> Fn(&'b mut DialogFields) -> ParamDialog<'b> + Send;

/// A [`ParamDialog`] the [`DialogHost`](crate::dialog::DialogHost) owns: the host keeps the values, and `build` makes
/// the dialog over them each pass.
///
/// ```ignore
/// host.open(HostedParamDialog::new("radial-blur", DialogFields::new(), |fields| {
///     ParamDialog::new("radial-blur", "Radial Blur", params.clone()).values(Bind::bound(fields)).preview(texture)
/// }));
/// ```
pub struct HostedParamDialog {
    id: DialogId,
    key: String,
    instance: Option<DialogId>,
    fields: DialogFields,
    build: Box<Build>,
    requests: HostRequests,
}

impl HostedParamDialog {
    /// A hosted dialog for the command `key`, opening with `fields`; `build` makes the dialog over the values.
    pub fn new(
        key: impl Into<String>,
        fields: DialogFields,
        build: impl for<'b> Fn(&'b mut DialogFields) -> ParamDialog<'b> + Send + 'static,
    ) -> Self {
        let key = key.into();
        HostedParamDialog { id: dialog_id(key.as_str()), key, instance: None, fields, build: Box::new(build), requests: HostRequests::new() }
    }

    /// Names this dialog among others made with the same key (one per document, say), so they keep separate state.
    #[must_use]
    pub fn instance(mut self, instance: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = dialog_id((self.key.as_str(), instance));
        self.instance = Some(self.id);
        self
    }

    /// The values the dialog shows.
    pub fn fields(&self) -> &DialogFields {
        &self.fields
    }

    /// The values the dialog shows, for the app to set (a script, a colour picker's answer).
    pub fn fields_mut(&mut self) -> &mut DialogFields {
        &mut self.fields
    }

    /// The colour parameter whose button was pressed, once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.requests.take_color()
    }

    /// The Browse button that was pressed, once.
    pub fn pick_path(&mut self) -> Option<PathRequest> {
        self.requests.take_path()
    }

    /// The eyedropper toggle that was pressed, once.
    pub fn pick_eyedropper(&mut self) -> Option<EyedropperRequest> {
        self.requests.take_eyedropper()
    }
}

impl StackDialog for HostedParamDialog {
    type Result = ParamResult;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<ParamResult>> {
        let shown = (self.build)(&mut self.fields).show(ui);
        if let Some(mut rows) = shown.rows {
            self.requests.absorb(&mut rows);
        }
        match (shown.result, shown.closed) {
            (Some(result), _) => Some(DialogResult::Ok(result)),
            (None, Some(_)) => Some(DialogResult::Cancel),
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn instance(&self) -> Option<DialogId> {
        self.instance
    }
}
