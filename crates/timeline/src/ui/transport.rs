//! A monitor's transport bar and the compact media controls (N1.06).

use super::style::TimelineStyle;
use craft_theme::ControlSize;
use craft_ui::canvas::Orientation;
use craft_ui::controls::{Button, Checkbox, IconButton, Slider};
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::{Toolbar, ToolbarItem};
use craft_ui::popover::{Popover, PopoverAnchor};
use craft_ui::primitives::Corner;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _, role_format, scale, tabular};
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Layout, Rect, Response, Sense, Ui, UiBuilder, vec2};

/// How far the arrows and the seek keys move the position, in seconds.
const NUDGE: f64 = 0.25;

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
        toolbar: "transport.toolbar" = "Transport";
        more: "transport.more" = "More transport controls";
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

impl TransportButton {
    /// The button's icon. The library has no transport glyphs yet: these are its nearest icons,
    /// named for in the button's tooltip.
    pub(super) fn icon(&self, playing: bool) -> Icon {
        match self {
            Self::AddMarker => Icon::Flag,
            Self::MarkIn => Icon::TriangleRight,
            Self::MarkOut => Icon::Triangle,
            Self::GoToIn => Icon::ChevronsLeft,
            Self::StepBack => Icon::ChevronLeft,
            Self::PlayStop if playing => Icon::Pause,
            Self::PlayStop => Icon::Play,
            Self::StepForward => Icon::ChevronRight,
            Self::GoToOut => Icon::ChevronsRight,
            Self::Insert => Icon::NewLayer,
            Self::Overwrite => Icon::SwapHorizontal,
            Self::Lift => Icon::Scissors,
            Self::Extract => Icon::SliceKnife,
            Self::ExportFrame => Icon::Image,
            Self::Loop => Icon::Refresh,
            Self::Custom { icon, .. } => *icon,
        }
    }

    fn label<'a>(&'a self, labels: &'a TransportLabels) -> &'a str {
        match self {
            Self::AddMarker => &labels.add_marker,
            Self::MarkIn => &labels.mark_in,
            Self::MarkOut => &labels.mark_out,
            Self::GoToIn => &labels.go_to_in,
            Self::StepBack => &labels.step_back,
            Self::PlayStop => &labels.play_stop,
            Self::StepForward => &labels.step_forward,
            Self::GoToOut => &labels.go_to_out,
            Self::Insert => &labels.insert,
            Self::Overwrite => &labels.overwrite,
            Self::Lift => &labels.lift,
            Self::Extract => &labels.extract,
            Self::ExportFrame => &labels.export_frame,
            Self::Loop => &labels.loop_playback,
            Self::Custom { label, .. } => label,
        }
    }
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
        let Self { id_salt, buttons, mut settings, playing, looping, editable, labels } = self;
        let labels = labels.unwrap_or_else(|| TransportLabels::of(ui));
        let tokens = Tokens::of(ui);
        let toolbar = Toolbar::style(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let hidden: Vec<TransportButton> = settings.as_ref().map(|s| s.hidden.clone()).unwrap_or_default();
        let shown: Vec<&TransportButton> = buttons.iter().filter(|button| !hidden.contains(button)).collect();

        let (row, response) = ui.allocate_exact_size(vec2(ui.available_width(), toolbar.button), Sense::hover());
        let rtl = direction::is_rtl(ui);
        let with_editor = editable && settings.is_some() && row.width() >= toolbar.button;
        let editor_rect =
            Rect::from_min_size(if rtl { row.left_top() } else { row.right_top() - vec2(toolbar.button, 0.0) }, vec2(toolbar.button, toolbar.button));
        let mut bar_rect = row;
        if with_editor {
            if rtl {
                bar_rect.min.x = editor_rect.right();
            } else {
                bar_rect.max.x = editor_rect.left();
            }
        }
        // The buttons sit centred in the room the editor leaves.
        let count = shown.len() as f32;
        let natural = count * toolbar.button + (count - 1.0).max(0.0) * toolbar.gap;
        let inset = ((bar_rect.width() - natural) / 2.0).max(0.0);
        let bar_rect = bar_rect.shrink2(vec2(inset, 0.0));

        let mut on = looping;
        let mut pressed = None;
        {
            let mut items: Vec<ToolbarItem<'_>> = Vec::with_capacity(shown.len());
            let mut toggle = Some(&mut on);
            for button in &shown {
                let item = ToolbarItem::new(button.icon(playing), button.label(&labels));
                items.push(match (button, toggle.take()) {
                    (TransportButton::Loop, Some(on)) => item.toggle(on),
                    (_, other) => {
                        toggle = other;
                        item
                    }
                });
            }
            let mut bar = ui.new_child(UiBuilder::new().max_rect(bar_rect));
            let out = Toolbar::new(id.with("bar"), Orientation::Horizontal)
                .label(labels.toolbar.as_str())
                .more_label(labels.more.as_str())
                .overflow(true)
                .group(items)
                .show(&mut bar);
            if let Some((_, item)) = out.clicked {
                pressed = shown.get(item).map(|button| (*button).clone());
            }
        }

        let mut settings_changed = false;
        if with_editor && let Some(settings) = settings.take() {
            let open_key = id.with("editor-open");
            let mut open = ui.data(|d| d.get_temp::<bool>(open_key)).unwrap_or(false);
            let toggle = IconButton::new(Icon::Plus, labels.button_editor.as_str()).with_id(id.with("editor")).show_at(ui, editor_rect);
            if toggle.clicked() {
                open = !open;
            }
            Popover::new(id.with("editor-popover"), PopoverAnchor::Control(toggle.clone()))
                .toggle(toggle.id)
                .label(labels.button_editor.as_str())
                .show(ui, &mut open, |ui, _| {
                    for (at, button) in buttons.iter().enumerate() {
                        let mut visible = !settings.hidden.contains(button);
                        if Checkbox::bool(button.label(&labels), &mut visible).id_salt(at).show(ui).response.changed() {
                            settings.hidden.retain(|hidden| hidden != button);
                            if !visible {
                                settings.hidden.push(button.clone());
                            }
                            settings_changed = true;
                        }
                    }
                    ui.separator();
                    if ui.add(Button::new(labels.reset.as_str())).clicked() {
                        settings_changed |= !settings.hidden.is_empty();
                        settings.hidden.clear();
                    }
                });
            ui.data_mut(|d| d.insert_temp(open_key, open));
        }
        TransportBarOutput { response, pressed, settings_changed }
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
        let Self { id_salt, position, duration, playing, trim, labels } = self;
        let labels = labels.unwrap_or_else(|| TransportLabels::of(ui));
        let tokens = Tokens::of(ui);
        let slider = Slider::style(&tokens, ControlSize::Md);
        let numbers = LocaleNumbers::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let playable = duration.is_finite() && duration > 0.0;
        let position = if position.is_finite() { position.clamp(0.0, if playable { duration } else { 0.0 }) } else { 0.0 };

        let text = clock(&numbers, position);
        let format = tabular(role_format(&tokens, TimelineStyle::of(&tokens).transport_text, tokens.color.text.normal.to_egui()));
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::single_section(text.clone(), format.clone())));
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let (row, response) = ui.allocate_exact_size(vec2(ui.available_width(), side), Sense::click_and_drag());
        let radius = Corner::Control.radius(&tokens);
        ui.painter().rect_filled(row, radius, tokens.color.surface.panel.to_egui());

        let mut out = MediaControlsOutput { response, toggle: false, nudge: None, seek: None };
        let mut content = ui.new_child(UiBuilder::new().max_rect(row).layout(Layout::left_to_right(egui::Align::Center)));
        let ui = &mut content;
        let play = if playing { Icon::Pause } else { Icon::Play };
        out.toggle = ui.add(IconButton::new(play, labels.play_pause_media.as_str()).with_id(id.with("play"))).clicked();
        if ui.add(IconButton::new(Icon::ChevronLeft, labels.back_quarter.as_str()).with_id(id.with("back"))).clicked() {
            out.nudge = Some(-NUDGE);
        }
        if ui.add(IconButton::new(Icon::ChevronRight, labels.forward_quarter.as_str()).with_id(id.with("forward"))).clicked() {
            out.nudge = Some(NUDGE);
        }
        let width = (ui.available_width() - galley.size().x - ui.spacing().item_spacing.x).max(0.0);
        let range = if playable { 0.0..=duration } else { 0.0..=1.0 };
        let seek = ui.add_enabled_ui(playable, |ui| {
            Slider::new(id.with("seek"), labels.seek.as_str(), position, range).step(NUDGE).width(width).value_text(text.as_str()).show(ui)
        });
        out.seek = seek.inner.event;
        if let (true, Some((start, end))) = (playable, trim) {
            let rect = seek.inner.response.rect;
            let (x0, x1) = (rect.left() + slider.thumb_width / 2.0, rect.right() - slider.thumb_width / 2.0);
            let stroke = scale::Stroke::Hairline.egui(ui.ctx(), tokens.color.status.warn.fg.to_egui());
            for t in [start, end].into_iter().filter(|t| *t > 0.0 && *t < duration) {
                let x = egui::lerp(x0..=x1, (t / duration) as f32);
                let half = slider.thumb / 2.0;
                ui.painter().line_segment([egui::pos2(x, rect.center().y - half), egui::pos2(x, rect.center().y + half)], stroke);
            }
        }
        let (label, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
        ui.painter().galley(label.min, galley, format.color);
        out
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

/// `1:05.25`: minutes, seconds and hundredths, in the locale's digits.
fn clock(numbers: &LocaleNumbers, seconds: f64) -> String {
    let hundredths = (seconds.max(0.0) * 100.0).round() as u64;
    let (minutes, rest) = (hundredths / 6000, (hundredths % 6000) as f64 / 100.0);
    let pad = if rest < 10.0 { numbers.format(0.0, 0..=0) } else { String::new() };
    format!("{}:{pad}{}", numbers.format(minutes as f64, 0..=0), numbers.format(rest, 2..=2))
}
