//! [`PasswordDialog`]: the password for a protected file, a permissions password, or a new
//! password with its strength (N9.05, from PdfCraft).

use std::any::Any;
use std::cell::Cell;
use std::fmt;

use craft_theme::StatusKind;
use egui::{Context, Id, TextWrapMode, vec2};

use super::dialog::{Dialog, InitialFocus, Outcome};
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::composites::{Field, FieldGroup};
use crate::controls::{IconButton, TextField};
use crate::icons::{Icon, IconSize};
use crate::localized::Labels as _;
use crate::primitives::{Content, Icon as IconView, Text};

crate::words! {
    /// The password dialog's words, English by default (PdfCraft's, after Acrobat). `{name}` is
    /// the file's name.
    #[non_exhaustive]
    pub struct PasswordLabels {
        open_title: "password.openTitle" = "Password required";
        open_message: "password.openMessage" = "“{name}” is protected. Enter a password to open it.";
        open: "password.open" = "Open";
        permissions_title: "password.permissionsTitle" = "Permissions password";
        permissions_message: "password.permissionsMessage" = "“{name}” has security restrictions. Enter its permissions password to lift them.";
        unlock: "password.unlock" = "Unlock";
        set_title: "password.setTitle" = "Set Password";
        type_password: "password.type" = "Type Password";
        retype_password: "password.retype" = "Re-type Password";
        mismatch: "password.mismatch" = "The passwords don't match.";
        /// The field's placeholder.
        hint: "password.hint" = "Password";
        show: "password.show" = "Show password";
        hide: "password.hide" = "Hide password";
        /// `{strength}` is one of the three below.
        strength: "password.strength" = "Strength: {strength}";
        weak: "password.weak" = "Weak";
        medium: "password.medium" = "Medium";
        strong: "password.strong" = "Strong";
        /// The app's answer to a wrong password, for apps that have no wording of their own.
        wrong: "password.wrong" = "That password is not correct.";
    }
}

/// A password, kept in memory only: `Debug` never prints it (debug output ends up in logs), and
/// its bytes are overwritten when it is dropped. Only this buffer is overwritten: while the
/// dialog is open the text is also in the text field's `String`, the edit event's value and
/// egui's text-edit state (its undo history), and those copies are freed without being zeroed.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps `text` as a secret.
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// The password, for handing to the decryptor. Don't store or log it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// True when no characters were typed.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/files.rs:131 Secret::fmt
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0.is_empty() { "\"\"" } else { "…" })
    }
}

impl Drop for Secret {
    // new: the String's own buffer is zeroed (`into_bytes` keeps it), then kept alive by `black_box`
    // so the writes are not removed as dead stores.
    fn drop(&mut self) {
        let mut bytes = std::mem::take(&mut self.0).into_bytes();
        bytes.fill(0);
        std::hint::black_box(&bytes);
    }
}

/// A rough strength, as Acrobat's meter shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PasswordStrength {
    /// Under 8 characters or a single character class.
    Weak,
    /// At least 8 characters and two classes.
    Medium,
    /// At least 12 characters and three classes.
    Strong,
}

/// Strong: at least 12 characters and three of the four classes (lower case, upper case, ASCII
/// digits, anything not alphanumeric); Medium: at least 8 and two classes; else Weak.
// pdfcraft: crates/ui-egui/src/dialogs/protect.rs:57 strength
pub fn password_strength(password: &str) -> PasswordStrength {
    let classes = [
        password.chars().any(char::is_lowercase),
        password.chars().any(char::is_uppercase),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ]
    .into_iter()
    .filter(|class| *class)
    .count();
    let length = password.chars().count();
    if length >= 12 && classes >= 3 {
        PasswordStrength::Strong
    } else if length >= 8 && classes >= 2 {
        PasswordStrength::Medium
    } else {
        PasswordStrength::Weak
    }
}

/// What the dialog asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasswordPurpose {
    /// The password that opens `name` (Acrobat's Password dialog on open); OK is Open.
    Open {
        /// The file's name, shown in the message.
        name: String,
    },
    /// The permissions (owner) password that lifts `name`'s restrictions; OK is Unlock.
    Permissions {
        /// The file's name, shown in the message.
        name: String,
    },
    /// A new password typed twice, with the strength under the first field (Protect Using
    /// Password); OK stays disabled until both match and are not empty.
    Set,
}

/// What a [`PasswordDialog`] answers: the password and the tag the app gave the dialog.
pub struct PasswordAnswer {
    /// The password the user typed.
    pub secret: Secret,
    tag: Option<Box<dyn Any + Send>>,
}

impl PasswordAnswer {
    /// The tag the app gave the dialog, if it is a `T`.
    pub fn tag<T: Any>(&self) -> Option<&T> {
        self.tag.as_deref()?.downcast_ref()
    }

    /// Takes the tag out, as `PromptAnswer::take_tag` does; `None` when it isn't a `T` (it stays).
    pub fn take_tag<T: Any>(&mut self) -> Option<T> {
        match self.tag.take()?.downcast::<T>() {
            Ok(tag) => Some(*tag),
            Err(other) => {
                self.tag = Some(other);
                None
            }
        }
    }
}

impl fmt::Debug for PasswordAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PasswordAnswer").field("secret", &self.secret).field("tag", &self.tag.is_some()).finish()
    }
}

/// A 400 pt wide question with a lock icon, the title, the message, the masked field, an error
/// line and the buttons (OK at the platform's default place, Cancel).
///
/// The field takes focus when the dialog opens, after the eye button was pressed, and whenever
/// nothing in the window has focus; it asks only then: asking every pass interrupts the input
/// method, and on Linux typed keys go through it (storytold/pdfcraft#2818), and Tab can still reach the
/// eye button and the buttons. Enter submits from the field; Esc and the close box cancel. The
/// eye button beside the (first) field shows or hides the text (its name says what it does:
/// "Show password" / "Hide password"); the field's AccessKit node is a password field while the
/// text is hidden.
///
/// A wrong password: the app reopens the dialog with its message,
/// `dialogs.open(PasswordDialog::open(name).tag(id).error(labels.wrong))`; the field starts empty.
pub struct PasswordDialog {
    id: DialogId,
    purpose: PasswordPurpose,
    input: Secret,
    confirm: Secret,
    shown: bool,
    state: DialogState<()>,
    error: Option<String>,
    tag: Option<Box<dyn Any + Send>>,
}

impl PasswordDialog {
    /// A dialog for `purpose`, with no error line and no tag.
    pub fn new(purpose: PasswordPurpose) -> Self {
        Self {
            id: dialog_id("password"),
            purpose,
            input: Secret::default(),
            confirm: Secret::default(),
            shown: false,
            state: DialogState::new(),
            error: None,
            tag: None,
        }
    }

    /// [`PasswordPurpose::Open`].
    pub fn open(name: impl Into<String>) -> Self {
        Self::new(PasswordPurpose::Open { name: name.into() })
    }

    /// [`PasswordPurpose::Permissions`].
    pub fn permissions(name: impl Into<String>) -> Self {
        Self::new(PasswordPurpose::Permissions { name: name.into() })
    }

    /// The line under the field, in the error colour.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// Carried back in the answer, so the app knows which file the password is for.
    pub fn tag<T: Any + Send>(mut self, tag: T) -> Self {
        self.tag = Some(Box::new(tag));
        self
    }

    /// The purpose the dialog was made for.
    pub fn purpose(&self) -> &PasswordPurpose {
        &self.purpose
    }
}

impl StackDialog for PasswordDialog {
    type Result = PasswordAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // pdfcraft: crates/ui-egui/src/dialogs/prompts.rs:299 password_modal, :348 password, :362 unlock
    // pdfcraft: crates/ui-egui/src/dialogs/protect.rs:93 body (Set: two fields and the strength line)
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PasswordAnswer>> {
        let labels = PasswordLabels::of_ctx(ctx);
        let (title, message, ok) = match &self.purpose {
            PasswordPurpose::Open { name } => {
                (labels.open_title.clone(), Some(craft_i18n::fmt(&labels.open_message, &[("name", name.as_str())])), Some(labels.open.clone()))
            }
            PasswordPurpose::Permissions { name } => (
                labels.permissions_title.clone(),
                Some(craft_i18n::fmt(&labels.permissions_message, &[("name", name.as_str())])),
                Some(labels.unlock.clone()),
            ),
            PasswordPurpose::Set => (labels.set_title.clone(), None, None),
        };
        let setting = matches!(self.purpose, PasswordPurpose::Set);
        let id = self.id;
        let can_submit = Cell::new(!setting || (!self.input.is_empty() && self.input == self.confirm));
        let mut footer = Footer::standard().ok_enabled_with(|| can_submit.get());
        if let Some(ok) = ok {
            footer = footer.ok(ok);
        }
        let Self { input, confirm, shown: reveal, error, state, .. } = self;
        let mut first_field: Option<Id> = None;
        let mut eye_pressed = false;
        let response = Dialog::with_id(id, title)
            .stack(stack)
            .width(Width::Fixed(crate::theme::current(ctx).component.password.width))
            .footer(footer)
            .initial_focus(InitialFocus::FirstField)
            .show(ctx, state, &mut (), |ui, ()| {
                if let Some(message) = &message {
                    ui.horizontal_top(|ui| {
                        ui.add(IconView::new(Icon::Lock).size(IconSize::Lg).content(Content::Accent));
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Wrap);
                            ui.add(Text::new(message.as_str()));
                        });
                    });
                    ui.add_space(ui.spacing().item_spacing.y);
                }
                let strength = (setting && !input.is_empty()).then(|| {
                    let (word, status) = match password_strength(input.expose()) {
                        PasswordStrength::Weak => (&labels.weak, StatusKind::Bad),
                        PasswordStrength::Medium => (&labels.medium, StatusKind::Warn),
                        PasswordStrength::Strong => (&labels.strong, StatusKind::Good),
                    };
                    (craft_i18n::fmt(&labels.strength, &[("strength", word.as_str())]), status)
                });
                let mismatch = (setting && !confirm.is_empty() && *input != *confirm).then(|| labels.mismatch.clone());
                FieldGroup::new(id.with("fields")).show(ui, |ui| {
                    let mut field = if setting { Field::new(labels.type_password.as_str()) } else { Field::unlabelled() };
                    if let Some((strength, status)) = strength {
                        field = field.help(strength).help_status(status);
                    } else if !setting {
                        field = field.error(error.as_deref());
                    }
                    field.show(ui, |ui, invalid| {
                        // The eye button comes first, so that its press masks or shows the text in the
                        // same pass; the row runs from the reading end.
                        let size = vec2(ui.available_width(), ui.spacing().interact_size.y);
                        ui.allocate_ui_with_layout(size, crate::direction::trailing(ui), |ui| {
                            let button = ui.add(
                                IconButton::new(Icon::Eye, labels.show.as_str())
                                    .when_on(Icon::EyeOff, labels.hide.as_str())
                                    .toggle(&mut *reveal)
                                    .id_salt("eye"),
                            );
                            eye_pressed = button.clicked();
                            let edit = TextField::new(id.with("password"), labels.hint.as_str(), input.expose())
                                .placeholder(if setting { "" } else { labels.hint.as_str() })
                                .password(!*reveal)
                                .invalid(invalid)
                                .width(ui.available_width())
                                .show(ui);
                            if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                                *input = Secret::new(value.clone());
                            }
                            first_field = Some(edit.response.id);
                            (edit, button)
                        })
                        .inner
                    });
                    if setting {
                        Field::new(labels.retype_password.as_str()).error(mismatch.as_deref()).show(ui, |ui, invalid| {
                            let edit = TextField::new(id.with("confirm"), labels.retype_password.as_str(), confirm.expose())
                                .password(!*reveal)
                                .invalid(invalid)
                                .show(ui);
                            if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                                *confirm = Secret::new(value.clone());
                            }
                            edit
                        });
                    }
                });
                can_submit.set(!setting || (!input.is_empty() && *input == *confirm));
            });
        if response.active
            && let Some(field) = first_field
            && (eye_pressed || ctx.memory(|m| m.focused()).is_none())
        {
            ctx.memory_mut(|m| m.request_focus(field));
        }
        match response.outcome? {
            Outcome::Ok => Some(DialogResult::Ok(PasswordAnswer { secret: Secret::new(std::mem::take(&mut self.input.0)), tag: self.tag.take() })),
            _ => Some(DialogResult::Cancel),
        }
    }
}
