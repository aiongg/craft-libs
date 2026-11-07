//! Built-in words of the presets family.

craft_ui::words! {
    /// The words of the presets family: the browser, its footer, context menu, panel menu and the file entries. Each
    /// id is prefixed by its module. `{n}` stands for a number, `{name}` for a preset's or group's name, `{file}` for
    /// a file name and `{reason}` for one of the `reason_*` words.
    pub struct PresetWords {
        grid_view: "presets.gridView" = "Grid View";
        list_view: "presets.listView" = "List View";
        thumb_size: "presets.thumbSize" = "Thumbnail size";
        footer: "presets.footer" = "Preset actions";
        view: "presets.view" = "View";
        menu: "presets.menu" = "Preset menu";
        search: "presets.search" = "Search";
        clear: "presets.clear" = "Clear";
        name: "presets.name" = "Name";
        rename: "presets.rename" = "Rename…";
        rename_group: "presets.renameGroup" = "Rename Group…";
        delete: "presets.delete" = "Delete";
        delete_group: "presets.deleteGroup" = "Delete Group";
        move_to: "presets.moveTo" = "Move to";
        new_preset: "presets.new" = "New";
        new_group: "presets.newGroup" = "New Group";
        delete_button: "presets.deleteButton" = "Delete";
        count: count "presets.count" = "{n} preset" | "{n} presets";
        empty: "presets.empty" = "No presets";
        no_match: "presets.noMatch" = "No presets match";
        group_count: count "presets.groupCount" = "{n} preset" | "{n} presets";
        loaded: template "presets.loaded" = "Loaded {file}";
        saved: template "presets.saved" = "Saved {file}";
        read_failed: template "presets.readFailed" = "Could not read {file}: {reason}";
        write_failed: template "presets.writeFailed" = "Could not save {file}: {reason}";
        reason_not_found: "presets.reasonNotFound" = "the file was not found";
        reason_denied: "presets.reasonDenied" = "access was denied";
        reason_unsupported: "presets.reasonUnsupported" = "this is not available here";
        reason_other: "presets.reasonOther" = "the file system reported an error";

        custom: "presets.custom" = "Custom";
        mixed: "presets.mixed" = "Mixed";
        built_in: template "presets.builtIn" = "{name} (built-in)";
        name_taken: "presets.nameTaken" = "A preset with this name already exists.";
        name_invalid: "presets.nameInvalid" = "This name can't be used.";
        more: "presets.more" = "Manage presets";
    }
}
