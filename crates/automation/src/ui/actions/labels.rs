//! [`ActionsLabels`]: every user-visible word of the Actions panel (`actions.*`), and its count
//! words.

use craft_ui::localized::words::Counted;

craft_ui::words! {
    /// The Actions panel's words, English by default (Photoshop's and PhotoCraft's). Pass a
    /// translated copy with [`ActionsPanel::labels`](super::ActionsPanel::labels); the menus read
    /// the installed words. Step labels are the host's ([`ActionsHost::step_label`](super::ActionsHost::step_label)).
    #[non_exhaustive]
    pub struct ActionsLabels {
        /// The tree's accessible name.
        name: "actions.name" = "Actions";
        /// The empty list's text (with the Record glyph before it).
        empty: "actions.empty" = "Record a sequence of edits, then play it on any document.";
        /// A recording action's accessible name: `{name}` is the action's.
        recording_suffix: "actions.recordingSuffix" = "{name}, recording";
        /// The include cell's accessible name: `{label}` is the row's.
        include: "actions.include" = "Include in playback: {label}";
        include_tooltip: "actions.includeTooltip" = "Include in playback";
        /// The dialog cell's accessible name: `{label}` is the row's.
        dialog: "actions.dialog" = "Show dialog during playback: {label}";
        dialog_tooltip: "actions.dialogTooltip" = "Show dialog during playback";
        /// An action row's trailing text with some steps off: `{enabled}` of `{steps}` (the
        /// counted "3 steps"), PhotoCraft's "2/3 steps".
        partial_steps: "actions.partialSteps" = "{enabled}/{steps}";
        /// An action row's trailing text with a shortcut: `{shortcut}` ("F2"), `{steps}`
        /// (PhotoCraft's `panels/actions.rs:1034`).
        shortcut_steps: "actions.shortcutSteps" = "{shortcut} · {steps}";
        // Footer buttons.
        stop: "actions.stop" = "Stop playing/recording";
        record: "actions.record" = "Begin recording";
        play: "actions.play" = "Play selection";
        new_set: "actions.newSet" = "Create new set";
        new_action: "actions.newAction" = "Create new action";
        delete: "actions.delete" = "Delete";
        button_mode: "actions.buttonMode" = "Button Mode";
        // Panel menu.
        menu_new_action: "actions.menuNewAction" = "New Action…";
        menu_new_set: "actions.menuNewSet" = "New Set…";
        set_options: "actions.setOptions" = "Set Options…";
        menu_play: "actions.menuPlay" = "Play";
        start_recording: "actions.startRecording" = "Start Recording";
        stop_recording: "actions.stopRecording" = "Stop Recording";
        menu_delete: "actions.menuDelete" = "Delete";
        insert_stop: "actions.insertStop" = "Insert Stop…";
        batch: "actions.batch" = "Batch…";
        load: "actions.load" = "Load Actions…";
        save: "actions.save" = "Save Actions…";
        // Context menus.
        save_set: "actions.saveSet" = "Save Set…";
        keyboard_shortcuts: "actions.keyboardShortcuts" = "Keyboard Shortcuts";
        none: "actions.none" = "None";
        shift: "actions.shift" = "Shift";
        /// The modifier check row on macOS.
        command: "actions.command" = "Command";
        /// The modifier check row elsewhere.
        ctrl: "actions.ctrl" = "Ctrl";
        color: "actions.color" = "Color";
        red: "actions.red" = "Red";
        orange: "actions.orange" = "Orange";
        yellow: "actions.yellow" = "Yellow";
        green: "actions.green" = "Green";
        blue: "actions.blue" = "Blue";
        violet: "actions.violet" = "Violet";
        gray: "actions.gray" = "Gray";
    }
}

/// "{n} step" / "{n} steps": an action row's step count.
pub const STEPS: Counted = Counted::new("actions.steps", "{n} step", "{n} steps");
/// "{n} action" / "{n} actions": a set header's count.
pub const ACTIONS: Counted = Counted::new("actions.actions", "{n} action", "{n} actions");
