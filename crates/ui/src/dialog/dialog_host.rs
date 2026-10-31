//! [`DialogHost`]: the dialogs an app has open, as values. One `open`, one `take`.
//!
//! The id-only [`DialogStack`] stays the routing structure every [`Dialog`](super::Dialog),
//! [`Prompt`](super::Prompt) and [`Alert`](super::Alert) takes by reference; it is `Clone` and
//! lives in egui memory where a story needs it. The host owns the dialog values on top of it, shows
//! them each pass in the order the stack says, queues the startup modals and keeps the results
//! until the app reads them.

use std::any::{Any, TypeId};
use std::fmt;

use egui::Context;

use super::stack::{DialogId, DialogStack};

/// How a dialog on the [`DialogHost`] finished. `T` is the dialog's own answer type: a struct of
/// values for a form, an enum for a prompt with three buttons. [`Outcome`](super::Outcome) is the
/// per-pass chrome answer underneath (Apply and Reset keep the dialog open and never reach here).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogResult<T> {
    /// OK, Enter, or the button the dialog counts as its answer.
    Ok(T),
    /// Cancel, Esc, the close box, or a parent that closed over it.
    Cancel,
}

impl<T> DialogResult<T> {
    /// The answer, or `None` for Cancel.
    pub fn ok(self) -> Option<T> {
        match self {
            DialogResult::Ok(value) => Some(value),
            DialogResult::Cancel => None,
        }
    }

    pub fn is_cancel(&self) -> bool {
        matches!(self, DialogResult::Cancel)
    }
}

/// A dialog the host owns and shows. Implemented by every step 2 dialog
/// ([`ParamDialog`](super::ParamDialog), [`ImageSizeDialog`](super::ImageSizeDialog),
/// [`StandardPrompt`](super::StandardPrompt), …) and by an app's own dialogs: the app builds the
/// value, [`DialogHost::open`]s it and reads [`DialogHost::take`] on a later pass.
///
/// `show` draws one pass with a [`Dialog`](super::Dialog) (or `Prompt`/`Alert`) that takes
/// `stack` through [`Dialog::stack`](super::Dialog::stack), so the host's ordering applies: only
/// the top dialog is interactive. It returns `Some` on the pass the dialog finished; the host
/// removes it then and stores the result. `Send + Sync` so a host can live in egui memory behind
/// an `Arc<Mutex<_>>` (stories, tests).
pub trait StackDialog: Any + Send + Sync {
    /// What the app reads back. `()` for a notice.
    type Result: 'static;

    /// The id the stack knows the dialog by; stable for the dialog's life
    /// ([`dialog_id`](super::dialog_id) of the dialog's key).
    fn id(&self) -> DialogId;

    /// Draws one pass; `Some` when the dialog finished this pass.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>>;
}

/// Where a startup modal goes in the queue when several apply at launch (spec question 5).
/// Lower shows first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartupStage {
    /// Recover unsaved documents (craft-settings recovery): before anything the user could lose.
    Recovery,
    /// A warning about this run (PhotoCraft's GPU fallback notice).
    Warning,
    /// The update prompt (craft-update): last, it can wait.
    Update,
}

/// Type-erased [`StackDialog`] the host stores.
trait AnyStackDialog: Send + Sync {
    fn id(&self) -> DialogId;
    fn result_type(&self) -> TypeId;
    fn show_erased(&mut self, ctx: &Context, stack: &DialogStack) -> Option<Box<dyn Any>>;
    /// `DialogResult::<D::Result>::Cancel`, boxed, for a dialog closed from outside.
    fn cancelled(&self) -> Box<dyn Any>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<D: StackDialog> AnyStackDialog for D {
    fn id(&self) -> DialogId {
        StackDialog::id(self)
    }

    fn result_type(&self) -> TypeId {
        TypeId::of::<D>()
    }

    fn show_erased(&mut self, ctx: &Context, stack: &DialogStack) -> Option<Box<dyn Any>> {
        self.show(ctx, stack).map(|result| Box::new(result) as Box<dyn Any>)
    }

    fn cancelled(&self) -> Box<dyn Any> {
        Box::new(DialogResult::<D::Result>::Cancel)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// The dialogs an app has open, their results, and the startup queue. The app keeps one and
/// calls [`show`](DialogHost::show) once per pass, after its own UI.
///
/// ```ignore
/// if ui.button("Gaussian Blur…").clicked() {
///     dialogs.open(ParamDialog::new("gaussian-blur", "Gaussian Blur", params));
/// }
/// dialogs.show(ctx);
/// if let Some(DialogResult::Ok(fields)) = dialogs.take::<ParamDialog>() {
///     doc.run("filter.gaussianBlur", fields.values());
/// }
/// ```
///
/// - One modal at a time: the newest open dialog takes the input; older ones are disabled whole
///   ([`DialogStack`]). A child opened with [`open_child`](DialogHost::open_child) sits over its
///   parent and closes with it.
/// - Startup modals [`enqueue`](DialogHost::enqueue) in [`StartupStage`] order and open one at a
///   time, each when the stack is empty, so the GPU warning and the update prompt never overlap
///   (gap 75).
/// - A result stays until [`take`](DialogHost::take)n or until a dialog of the same type is
///   opened again. One open dialog per type: opening a second `ParamDialog` while one is open
///   cancels the first (its result is `Cancel`).
/// - Automation: [`get`](DialogHost::get) and [`get_mut`](DialogHost::get_mut) reach the open
///   dialog's value, so a script sets fields and a test reads them like a user would.
#[derive(Default)]
pub struct DialogHost {
    stack: DialogStack,
    open: Vec<Box<dyn AnyStackDialog>>,
    queued: Vec<(StartupStage, Box<dyn AnyStackDialog>)>,
    results: Vec<(TypeId, Box<dyn Any>)>,
}

impl fmt::Debug for DialogHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DialogHost").field("stack", &self.stack).field("open", &self.open.len()).field("queued", &self.queued.len()).finish()
    }
}

impl DialogHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// The routing stack, for a [`Dialog`](super::Dialog) the app shows itself beside the host's
    /// (`Dialog::new(..).stack(dialogs.stack())`). Push its id with [`DialogStack::push`] and
    /// remove it on the closing outcome, as before.
    pub fn stack(&self) -> &DialogStack {
        &self.stack
    }

    pub fn stack_mut(&mut self) -> &mut DialogStack {
        &mut self.stack
    }

    /// Opens `dialog` on top. Its id goes on the stack now; the dialog shows from the next
    /// [`show`](DialogHost::show). Returns the id.
    pub fn open<D: StackDialog>(&mut self, dialog: D) -> DialogId {
        self.open_with_parent(Box::new(dialog), None)
    }

    /// Opens `dialog` as a child of `parent`: shown over it, disabled with it, closed when the
    /// parent closes (a contour editor from Layer Style).
    pub fn open_child<D: StackDialog>(&mut self, dialog: D, parent: DialogId) -> DialogId {
        self.open_with_parent(Box::new(dialog), Some(parent))
    }

    /// Queues a startup modal. It opens when the stack is empty and no earlier stage waits;
    /// two of one stage open in `enqueue` order.
    pub fn enqueue<D: StackDialog>(&mut self, stage: StartupStage, dialog: D) {
        let at = self.queued.partition_point(|(queued, _)| *queued <= stage);
        self.queued.insert(at, (stage, Box::new(dialog)));
    }

    /// Shows every open dialog for this pass, oldest first, the top one interactive; removes the
    /// ones that finished and stores their results; opens the next queued startup modal when the
    /// stack is empty. Call once per pass, after the app's own UI so dialogs draw above it.
    pub fn show(&mut self, ctx: &Context) {
        self.open_next_queued();
        let mut finished = Vec::new();
        for dialog in &mut self.open {
            if let Some(result) = dialog.show_erased(ctx, &self.stack) {
                finished.push((dialog.id(), result));
            }
        }
        for (id, result) in finished {
            let Some(at) = self.open.iter().position(|dialog| dialog.id() == id) else { continue };
            let dialog = self.open.remove(at);
            self.store_result(dialog.result_type(), result);
            self.stack.remove_tree(id);
            self.cancel_orphans();
        }
        if self.open_next_queued() {
            ctx.request_repaint();
        }
    }

    /// The result of the last `D` that finished, once. `None` while it is open or already taken.
    pub fn take<D: StackDialog>(&mut self) -> Option<DialogResult<D::Result>> {
        let at = self.results.iter().position(|(type_id, _)| *type_id == TypeId::of::<D>())?;
        let (_, result) = self.results.remove(at);
        result.downcast::<DialogResult<D::Result>>().ok().map(|result| *result)
    }

    /// Closes the open `D` with [`DialogResult::Cancel`] (and its children), as Esc would.
    pub fn cancel<D: StackDialog>(&mut self) -> bool {
        let Some(at) = self.position::<D>() else { return false };
        let id = self.open[at].id();
        let dialog = self.open.remove(at);
        self.store_result(dialog.result_type(), dialog.cancelled());
        self.stack.remove_tree(id);
        self.cancel_orphans();
        true
    }

    /// The open `D`, for the app to feed (a preview image) or a script to read.
    // new.
    pub fn get<D: StackDialog>(&self) -> Option<&D> {
        self.open.iter().find(|dialog| dialog.result_type() == TypeId::of::<D>())?.as_any().downcast_ref()
    }

    pub fn get_mut<D: StackDialog>(&mut self) -> Option<&mut D> {
        self.open.iter_mut().find(|dialog| dialog.result_type() == TypeId::of::<D>())?.as_any_mut().downcast_mut()
    }

    fn position<D: StackDialog>(&self) -> Option<usize> {
        self.open.iter().position(|dialog| dialog.result_type() == TypeId::of::<D>())
    }

    fn store_result(&mut self, type_id: TypeId, result: Box<dyn Any>) {
        self.results.retain(|(stored, _)| *stored != type_id);
        self.results.push((type_id, result));
    }

    fn open_with_parent(&mut self, dialog: Box<dyn AnyStackDialog>, parent: Option<DialogId>) -> DialogId {
        let type_id = dialog.result_type();
        self.results.retain(|(stored, _)| *stored != type_id);
        if let Some(at) = self.open.iter().position(|open| open.result_type() == type_id) {
            let old = self.open.remove(at);
            self.stack.remove_tree(old.id());
            self.store_result(type_id, old.cancelled());
            self.cancel_orphans();
        }
        let id = dialog.id();
        self.stack.push(id, parent);
        self.open.push(dialog);
        id
    }

    /// Closes the open dialogs the stack no longer holds (children of a dialog that closed), with
    /// `Cancel`.
    fn cancel_orphans(&mut self) {
        let (kept, orphans): (Vec<_>, Vec<_>) = std::mem::take(&mut self.open).into_iter().partition(|dialog| self.stack.contains(dialog.id()));
        self.open = kept;
        for dialog in orphans {
            self.store_result(dialog.result_type(), dialog.cancelled());
        }
    }

    /// Opens the first queued startup modal if nothing else is open.
    fn open_next_queued(&mut self) -> bool {
        if !self.stack.is_empty() || !self.open.is_empty() || self.queued.is_empty() {
            return false;
        }
        let (_, dialog) = self.queued.remove(0);
        self.open_with_parent(dialog, None);
        true
    }

    /// Whether `D` is open.
    pub fn is_open<D: StackDialog>(&self) -> bool {
        self.get::<D>().is_some()
    }

    /// Whether any dialog is open or a startup modal waits: the app's shortcuts and canvas input
    /// are blocked meanwhile ([`any_modal`](super::any_modal) answers for the frame).
    pub fn is_empty(&self) -> bool {
        self.open.is_empty() && self.queued.is_empty()
    }
}
