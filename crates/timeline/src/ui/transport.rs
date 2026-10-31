//! A monitor's transport bar and the compact media controls (N1.06).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
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
/// with an icon name and a label.
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
        icon: String,
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

/// What a transport press asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportAction {
    Pressed(TransportButton),
    /// The button editor changed the hidden set.
    Hidden(Vec<TransportButton>),
}

/// A monitor's transport row: 28-point buttons centred under the picture, Pause in place of Play
/// while playing, Loop pressed while on, and a "+" button editor at the right (a popup with a
/// checkbox per button and Reset) when the row has room.
///
/// - Every button is a Tab stop with its tooltip as its name; Space and Enter press it. The keys
///   in the tooltips (Space, I, O, Left, Right…) are the app's shortcuts, not handled here.
/// - Buttons in `hidden` are left out; the editor reports a new hidden set.
pub struct TransportBar<'a> {
    id_salt: IdSalt,
    buttons: &'a [TransportButton],
    hidden: &'a [TransportButton],
    playing: bool,
    looping: bool,
    editable: bool,
    labels: Option<TransportLabels>,
}

impl<'a> TransportBar<'a> {
    pub fn new(id_salt: impl AsIdSalt, buttons: &'a [TransportButton]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), buttons, hidden: &[], playing: false, looping: false, editable: true, labels: None }
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn looping(mut self, on: bool) -> Self {
        self.looping = on;
        self
    }

    pub fn hidden(mut self, hidden: &'a [TransportButton]) -> Self {
        self.hidden = hidden;
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
    pub fn show(self, ui: &mut Ui) -> (Response, Option<TransportAction>) {
        todo!()
    }
}

/// PowerPoint's media controls under a video or sound on a slide (DeckCraft): play / pause,
/// back and forward a quarter second, a seek track with the trim marks, and the time
/// (`1:05.25` style, monospace). The seek track is a slider (one Tab stop; Left / Right a
/// quarter second, Home / End) reporting an `EditEvent<f64>` gesture in seconds; the buttons are
/// Tab stops. A press on the bar never reaches the canvas under it.
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
    pub fn show(self, ui: &mut Ui) -> MediaControlsResponse {
        todo!()
    }
}

/// What the media controls did this pass.
pub struct MediaControlsResponse {
    pub response: Response,
    pub toggle: bool,
    /// −0.25 or +0.25 seconds.
    pub nudge: Option<f64>,
    pub seek: Option<EditEvent<f64>>,
}
