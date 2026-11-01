//! A monitor's transport bar and the compact media controls (N1.06).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use egui::{AsIdSalt, IdSalt, Response, Ui};

craft_ui::words! {
    /// The buttons' tooltips and names, English by default (FilmCraft's, with Premiere's keys).
    #[non_exhaustive]
    pub struct TransportLabels {
        add_marker: "transport.addMarker" = "Add Marker (M)";
        mark_in: "transport.markIn" = "Mark In (I)";
        mark_out: "transport.markOut" = "Mark Out (O)";
        go_to_in: "transport.goToIn" = "Go to In (Shift+I)";
        step_back: "transport.stepBack" = "Step Back 1 Frame (Left)";
        play_stop: "transport.playStop" = "Play-Stop Toggle (Space)";
        step_forward: "transport.stepForward" = "Step Forward 1 Frame (Right)";
        go_to_out: "transport.goToOut" = "Go to Out (Shift+O)";
        insert: "transport.insert" = "Insert (,)";
        overwrite: "transport.overwrite" = "Overwrite (.)";
        lift: "transport.lift" = "Lift (;)";
        extract: "transport.extract" = "Extract (')";
        export_frame: "transport.exportFrame" = "Export Frame";
        loop_playback: "transport.loop" = "Loop";
        button_editor: "transport.buttonEditor" = "Button Editor";
        reset: "transport.reset" = "Reset";
        play_pause_media: "transport.playPauseMedia" = "Play or pause media";
        back_quarter: "transport.backQuarter" = "Back 0.25 seconds";
        forward_quarter: "transport.forwardQuarter" = "Forward 0.25 seconds";
        seek: "transport.seek" = "Seek";
    }
}

/// A transport button. The app's own buttons (Toggle Proxies, a camera angle) are `Custom`
/// with an icon and a label in the app's words.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportButton {
    AddMarker,
    MarkIn,
    MarkOut,
    GoToIn,
    StepBack,
    /// Shows Pause while playing.
    PlayStop,
    StepForward,
    GoToOut,
    Insert,
    Overwrite,
    Lift,
    Extract,
    ExportFrame,
    /// A toggle; shown pressed while on.
    Loop,
    Custom {
        id: String,
        icon: Icon,
        label: String,
    },
}

impl TransportButton {
    /// Premiere's Program monitor row; FilmCraft adds Toggle Proxies as a custom button before
    /// Loop (`monitor.rs:1022`).
    pub const PROGRAM: [TransportButton; 12] = [
        TransportButton::AddMarker,
        TransportButton::MarkIn,
        TransportButton::MarkOut,
        TransportButton::GoToIn,
        TransportButton::StepBack,
        TransportButton::PlayStop,
        TransportButton::StepForward,
        TransportButton::GoToOut,
        TransportButton::Lift,
        TransportButton::Extract,
        TransportButton::ExportFrame,
        TransportButton::Loop,
    ];

    /// Premiere's Source monitor row (`monitor.rs:1007`).
    pub const SOURCE: [TransportButton; 11] = [
        TransportButton::AddMarker,
        TransportButton::MarkIn,
        TransportButton::MarkOut,
        TransportButton::GoToIn,
        TransportButton::StepBack,
        TransportButton::PlayStop,
        TransportButton::StepForward,
        TransportButton::GoToOut,
        TransportButton::Insert,
        TransportButton::Overwrite,
        TransportButton::ExportFrame,
    ];
}

/// The buttons a monitor's row leaves out, chosen in its button editor; the app saves one per
/// monitor (Premiere keeps the Program and Source rows apart).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TransportBarSettings {
    pub hidden: Vec<TransportButton>,
}

/// A monitor's transport row (Premiere's, as FilmCraft draws it), through the kernel's
/// horizontal [`Toolbar`](craft_ui::patterns::Toolbar): `style.transport_button` squares
/// ([`TimelineStyle`](super::TimelineStyle)) centred under the picture, Pause in place of Play
/// while playing, Loop pressed while on, and a "+" button editor at the right (a popover with a
/// [`Checkbox`](craft_ui::controls::Checkbox) per button and Reset) when the row has room.
///
/// - Keyboard and AccessKit are the toolbar's: a `Toolbar` whose buttons are named by their
///   tooltips; Space and Enter press. The keys in the tooltips (Space, I, O, Left, Right…) are
///   the app's shortcuts, not handled here.
/// - Buttons in [`TransportBarSettings::hidden`] are left out; the editor changes the settings
///   and sets [`TransportBarOutput::settings_changed`].
pub struct TransportBar<'a> {
    id_salt: IdSalt,
    buttons: &'a [TransportButton],
    settings: Option<&'a mut TransportBarSettings>,
    playing: bool,
    looping: bool,
    editable: bool,
    labels: Option<TransportLabels>,
}

impl<'a> TransportBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, buttons: &'a [TransportButton]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), buttons, settings: None, playing: false, looping: false, editable: true, labels: None }
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn looping(mut self, on: bool) -> Self {
        self.looping = on;
        self
    }

    /// The row's saved settings; without them every button shows and the editor is off.
    pub fn settings(mut self, settings: &'a mut TransportBarSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Show the "+" button editor (on by default).
    pub fn editable(mut self, on: bool) -> Self {
        self.editable = on;
        self
    }

    pub fn labels(mut self, labels: TransportLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/monitor.rs:1003 `transport`, `:1146`
    // `transport_button`, `:1000` `TRANSPORT_BUTTON`.
    pub fn show(self, ui: &mut Ui) -> TransportBarOutput {
        todo!()
    }
}

/// What the transport row did this pass.
#[must_use]
pub struct TransportBarOutput {
    pub response: Response,
    /// The button pressed this pass.
    pub pressed: Option<TransportButton>,
    /// The button editor changed [`TransportBarSettings`]; the app saves them.
    pub settings_changed: bool,
}

/// PowerPoint's media controls under a video or sound on a slide (DeckCraft): play / pause,
/// back and forward a quarter second, a seek track with the trim marks, and the time
/// (`1:05.25` style in the mono face, digits through `LocaleNumbers`). The seek track is the
/// kernel's [`Slider`](craft_ui::controls::Slider) (one Tab stop; Left / Right a quarter
/// second, Home / End) reporting an `EditEvent<f64>` gesture in seconds; the buttons are
/// [`IconButton`](craft_ui::controls::IconButton) Tab stops. A press on the bar never reaches
/// the canvas under it. AccessKit: the slider's value text is the time; the buttons are named by
/// [`TransportLabels`].
pub struct MediaControls {
    id_salt: IdSalt,
    position: f64,
    duration: f64,
    playing: bool,
    trim: Option<(f64, f64)>,
    labels: Option<TransportLabels>,
}

impl MediaControls {
    /// At `position` of `duration` seconds.
    pub fn new(id_salt: impl AsIdSalt, position: f64, duration: f64) -> Self {
        Self { id_salt: IdSalt::new(id_salt), position, duration, playing: false, trim: None, labels: None }
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// The trimmed start and end (seconds), marked on the track in amber.
    pub fn trim(mut self, start: f64, end: f64) -> Self {
        self.trim = Some((start, end));
        self
    }

    pub fn labels(mut self, labels: TransportLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // deckcraft: crates/ui-egui/src/media.rs:472 `control_bar`, `:404` `clock`.
    pub fn show(self, ui: &mut Ui) -> MediaControlsOutput {
        todo!()
    }
}

/// What the media controls did this pass.
#[must_use]
pub struct MediaControlsOutput {
    pub response: Response,
    pub toggle: bool,
    /// −0.25 or +0.25 seconds.
    pub nudge: Option<f64>,
    pub seek: Option<EditEvent<f64>>,
}
