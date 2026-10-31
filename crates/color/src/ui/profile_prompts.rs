//! Colour profile prompts (F04.12): the Embedded Profile Mismatch and Missing Profile questions
//! when a file opens ([`ProfileMismatchDialog`]), and a document's profile as the status bar and
//! the document's title say it ([`ProfileStatus`]).
//!
//! The answer is the app's to carry out (convert, assign, discard) and, with "Remember this
//! choice", to store as its Color Settings policy for that colour mode; the dialog only asks.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, StackDialog, dialog_id};

craft_ui::words! {
    /// The profile prompts' words, English by default.
    #[non_exhaustive]
    pub struct ProfileLabels {
        mismatch_title: "profile.mismatchTitle" = "Embedded Profile Mismatch";
        missing_title: "profile.missingTitle" = "Missing Profile";
        /// `{name}` the file name, `{mode}` the mode word ("RGB").
        mismatch_message: "profile.mismatchMessage" = "The document “{name}” has an embedded color profile that does not match the current {mode} working space.";
        missing_message: "profile.missingMessage" = "The document “{name}” does not have an embedded {mode} profile.";
        /// `{working}` is the working space's name.
        working: "profile.working" = "Working: {working}";
        /// `{profile}` is the embedded profile's name.
        use_embedded: "profile.useEmbedded" = "Use the embedded profile ({profile})";
        /// `{profile}` is the working space's name.
        convert: "profile.convert" = "Convert to the working space ({profile})";
        discard: "profile.discard" = "Discard the embedded profile (don't color manage)";
        leave: "profile.leave" = "Leave as is (don't color manage)";
        assign_working: "profile.assignWorking" = "Assign working space";
        remember: "profile.remember" = "Remember this choice for future images";
        remember_tip: "profile.rememberTip" = "Turn the prompt back on with Ask On Mismatch in Edit › Color Settings.";
        rgb: "profile.rgb" = "RGB";
        cmyk: "profile.cmyk" = "CMYK";
        gray: "profile.gray" = "Gray";
        /// `{mode}` the mode word.
        untagged: "profile.untagged" = "Untagged {mode}";
        invalid: "profile.invalid" = "Invalid {mode} profile";
        unnamed: "profile.unnamed" = "Unnamed {mode} profile";
    }
}

/// A document's colour mode, as the prompts word it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProfileMode {
    Rgb,
    Cmyk,
    Gray,
}

/// A document's profile as Photoshop's status bar (Document Profile) says it: "Untagged RGB" (no
/// profile), "Invalid RGB profile" (one that does not parse or link), "Unnamed RGB profile" (no
/// description), else the profile's description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileStatus {
    Untagged,
    Invalid,
    Unnamed,
    Named(String),
}

impl ProfileStatus {
    /// The status of a document whose embedded profile bytes are `icc` (`None` without one).
    // new: Photoshop's Document Profile wording; PhotoCraft reads the description only.
    pub fn of(icc: Option<&[u8]>) -> ProfileStatus {
        todo!()
    }

    /// The text: the labels' template with the mode word, or the name.
    // new: Photoshop's Document Profile wording.
    pub fn label(&self, mode: ProfileMode, labels: &ProfileLabels) -> String {
        todo!()
    }
}

/// What the file asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileQuestion {
    /// The embedded profile differs from the working space; both names shown.
    Mismatch { embedded: String, working: String },
    /// The file has no profile.
    Missing { working: String },
}

/// The user's choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProfileAction {
    /// Mismatch: keep the embedded profile.
    UseEmbedded,
    /// Mismatch: convert the pixels to the working space.
    ConvertToWorking,
    /// Mismatch: drop the profile (not colour managed).
    Discard,
    /// Missing: leave it untagged.
    LeaveAsIs,
    /// Missing: tag it with the working space.
    AssignWorking,
}

/// The answer to a [`ProfileMismatchDialog`]: the action, and whether to remember it for this
/// mode (the app sets its policy and turns Ask On Mismatch off).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProfileAnswer {
    pub action: ProfileAction,
    pub remember: bool,
}

/// Photoshop's Embedded Profile Mismatch and Missing Profile prompt on the
/// [`DialogHost`](craft_ui::dialog::DialogHost).
///
/// - The message (the file name last, so nothing in it reads as a placeholder; the missing form
///   adds "Working: …" on its own line), then a radio group: Use the embedded profile (name),
///   Convert to the working space (name), Discard the embedded profile; or for a missing
///   profile Leave as is, Assign working space. The first is chosen at open unless the app
///   gives its policy's action ([`action`](Self::action)).
/// - With [`rememberable`](Self::rememberable) (a mismatch in a mode that has a policy), the
///   "Remember this choice for future images" check box under the radios, with its tooltip.
/// - OK answers [`ProfileAnswer`]; Cancel answers `DialogResult::Cancel`: the app stops opening
///   this file and every file queued behind it (PhotoCraft).
/// - Keyboard: arrows move in the radio group, Space toggles the box, Enter is OK, Esc Cancel.
pub struct ProfileMismatchDialog {
    id: DialogId,
    file_name: String,
    mode: ProfileMode,
    question: ProfileQuestion,
    action: ProfileAction,
    rememberable: bool,
    remember: bool,
    state: DialogState<()>,
}

impl ProfileMismatchDialog {
    /// The prompt for `file_name` in `mode`.
    pub fn new(file_name: impl Into<String>, mode: ProfileMode, question: ProfileQuestion) -> ProfileMismatchDialog {
        let action = match question {
            ProfileQuestion::Mismatch { .. } => ProfileAction::UseEmbedded,
            ProfileQuestion::Missing { .. } => ProfileAction::LeaveAsIs,
        };
        ProfileMismatchDialog {
            id: dialog_id("profile-mismatch"),
            file_name: file_name.into(),
            mode,
            question,
            action,
            rememberable: false,
            remember: false,
            state: DialogState::default(),
        }
    }

    /// The action chosen at open (the app's policy for this mode).
    pub fn action(mut self, action: ProfileAction) -> ProfileMismatchDialog {
        self.action = action;
        self
    }

    /// Offers "Remember this choice"; only for a mismatch (ignored for a missing profile).
    pub fn rememberable(mut self, on: bool) -> ProfileMismatchDialog {
        self.rememberable = on;
        self
    }
}

impl StackDialog for ProfileMismatchDialog {
    type Result = ProfileAnswer;

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs/open_prompt.rs:78 `show` (message and names), :127 `answer`,
    // :152 `answer_remembering`; crates/ui-egui/src/prefs/prefs_ui/dialogs.rs:287 `mismatch_body`
    // (radios, the box only when `!missing` and a policy key exists). On `Dialog` with a
    // `RadioSet` and a `Checkbox`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<ProfileAnswer>> {
        todo!()
    }
}
