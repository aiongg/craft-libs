//! The role-based button row every dialog footer uses: [`ButtonRole`], [`Platform`], the order
//! rules ([`ButtonRole::slot`], [`order`]) and [`FooterButtons`].

use craft_theme::ButtonVariant;
use egui::os::OperatingSystem;
use egui::{AsIdSalt, Context, Id, IdSalt, Response, Sense, Ui, vec2};

use crate::composites::measure;
use crate::controls::Button;
use crate::theme::{Tokens, TokensOf};

/// What a dialog button does, which decides where the platform's button order puts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonRole {
    /// The default answer (OK, Save, Yes), drawn as the primary button.
    Default,
    /// Another answer that closes the dialog (Don't Save, No). A row may hold two.
    Alternate,
    /// Closes the dialog without acting.
    Cancel,
    /// Acts and keeps the dialog open (Apply).
    Apply,
}

/// Whose button order a row follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// macOS: the default action sits in the trailing corner.
    Mac,
    /// Windows and Linux: the default action comes first.
    Other,
}

impl Platform {
    /// The platform `ctx` runs on.
    pub fn of(ctx: &Context) -> Self {
        if ctx.os() == OperatingSystem::Mac { Platform::Mac } else { Platform::Other }
    }
}

impl ButtonRole {
    /// Position in reading order on `platform`. Windows and Linux put the default action first
    /// (OK Cancel Apply, Yes No Cancel); macOS puts it last, in the corner, with Cancel beside it
    /// and the other answers further from it (Don't Save, Cancel, Save).
    pub fn slot(self, platform: Platform) -> u8 {
        match (self, platform) {
            (Self::Default, Platform::Other) | (Self::Alternate, Platform::Mac) => 0,
            (Self::Alternate, Platform::Other) | (Self::Cancel, Platform::Mac) => 1,
            (Self::Cancel, Platform::Other) | (Self::Apply, Platform::Mac) => 2,
            (Self::Apply, Platform::Other) | (Self::Default, Platform::Mac) => 3,
        }
    }
}

/// The indices of `buttons` in reading order on `platform`; buttons of one role keep their listed
/// order.
pub fn order<T>(buttons: &[(ButtonRole, T)], platform: Platform) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..buttons.len()).collect();
    indices.sort_by_key(|&i| buttons[i].0.slot(platform));
    indices
}

/// One button of a [`FooterButtons`] row.
#[derive(Clone, Debug)]
pub struct FooterButton {
    role: ButtonRole,
    label: String,
    min_width: f32,
    enabled: bool,
}

impl FooterButton {
    pub fn new(role: ButtonRole, label: impl Into<String>) -> Self {
        Self { role, label: label.into(), min_width: 0.0, enabled: true }
    }

    /// A least width above `dialog.button_min`.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn role(&self) -> ButtonRole {
        self.role
    }
}

/// The button that was clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clicked {
    /// Its place in the listed buttons.
    pub index: usize,
    pub role: ButtonRole,
    /// Which of the buttons of its role it is, in listed order (the second Alternate is 1).
    pub nth: usize,
}

/// What [`FooterButtons::show`] saw.
#[derive(Debug)]
#[non_exhaustive]
pub struct FooterButtonsResponse {
    /// The button clicked this pass, by click, Enter or Space on the focused button.
    pub clicked: Option<Clicked>,
    /// Whether a button of the row has the keyboard focus.
    pub focused: bool,
    /// The enabled [`Default`](ButtonRole::Default) button.
    pub default: Option<Response>,
    /// Each button's response, in listed order.
    pub responses: Vec<Response>,
}

/// A row of role buttons at the trailing edge, in the platform's order, `dialog.button_gap` apart
/// and at least `dialog.button_min` wide. The row is laid out in reading order, so Tab, arrow
/// keys and AccessKit visit the buttons in the order they appear.
///
/// - The [`Default`](ButtonRole::Default) button is the primary one and is described as the
///   default to a screen reader ([`default_hint`](FooterButtons::default_hint)).
/// - Arrow keys move the focus between buttons; Space and Enter press the focused one. A disabled
///   button takes no focus and no click.
/// - A role appears once, except [`Alternate`](ButtonRole::Alternate), which may appear twice.
///
/// [`Dialog`](super::Dialog) draws its footer with it; a full-window dialog shows it on its own.
#[must_use = "show the row with `show`"]
pub struct FooterButtons {
    id_salt: IdSalt,
    buttons: Vec<FooterButton>,
    platform: Option<Platform>,
    default_hint: String,
}

impl FooterButtons {
    /// A row of `buttons`, listed in any order. `id_salt` keeps the row's width and the buttons'
    /// state when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, buttons: Vec<FooterButton>) -> Self {
        debug_assert!(
            [ButtonRole::Default, ButtonRole::Cancel, ButtonRole::Apply].iter().all(|role| buttons.iter().filter(|b| b.role == *role).count() <= 1)
                && buttons.iter().filter(|b| b.role == ButtonRole::Alternate).count() <= 2,
            "craft-ui: a footer holds each role once, and Alternate at most twice"
        );
        Self { id_salt: IdSalt::new(id_salt), buttons, platform: None, default_hint: "Default".to_owned() }
    }

    /// The order to follow; the context's platform by default.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// The description of the default button for a screen reader; "Default" by default.
    pub fn default_hint(mut self, hint: impl Into<String>) -> Self {
        self.default_hint = hint.into();
        self
    }

    /// Shows the row against the trailing edge of the room `ui` has.
    pub fn show(self, ui: &mut Ui) -> FooterButtonsResponse {
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), tokens.size.control.md));
        let (shown, _) =
            measure::trailing_row(ui, id.with("row"), rect, Sense::hover(), tokens.component.dialog.button_gap, 0.0, |row| self.place(row, id));
        shown
    }

    /// Adds the buttons to `row`, a row in reading order (the footer's trailing row).
    pub(super) fn place(&self, row: &mut Ui, id: Id) -> FooterButtonsResponse {
        let tokens = Tokens::of(row);
        let min = tokens.component.dialog.button_min;
        let platform = self.platform.unwrap_or_else(|| Platform::of(row.ctx()));
        let roles: Vec<(ButtonRole, ())> = self.buttons.iter().map(|b| (b.role, ())).collect();
        let nth = |i: usize| roles[..i].iter().filter(|(role, ())| *role == roles[i].0).count();

        let mut slots: Vec<Option<Response>> = vec![None; self.buttons.len()];
        for i in order(&roles, platform) {
            let b = &self.buttons[i];
            let variant = if b.role == ButtonRole::Default { ButtonVariant::Primary } else { ButtonVariant::Secondary };
            let response = row.add(Button::new(b.label.as_str()).variant(variant).enabled(b.enabled).min_width(b.min_width.max(min)).id_salt((
                id,
                b.role,
                nth(i),
            )));
            if b.role == ButtonRole::Default && b.enabled {
                let hint = self.default_hint.clone();
                row.ctx().accesskit_node_builder(response.id, |node| node.set_description(hint));
            }
            slots[i] = Some(response);
        }
        let responses: Vec<Response> = slots.into_iter().flatten().collect();
        let clicked = responses.iter().position(Response::clicked).map(|index| Clicked { index, role: roles[index].0, nth: nth(index) });
        let default = self.buttons.iter().position(|b| b.role == ButtonRole::Default && b.enabled).map(|i| responses[i].clone());
        FooterButtonsResponse { clicked, focused: responses.iter().any(Response::has_focus), default, responses }
    }
}
