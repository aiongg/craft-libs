//! Built-in words of the dialog family.

craft_ui::words! {
    /// The words of the standard prompts: titles, messages and answers.
    pub struct PromptLabels {
        cancel: "prompt.cancel" = "Cancel";
        ok: "prompt.ok" = "OK";
        confirm_delete_title: "prompt.confirmDeleteTitle" = "Delete";
        confirm_delete_one: template "prompt.confirmDeleteOne" = "Delete {name}?";
        confirm_delete_many: template "prompt.confirmDeleteMany" = "Delete {things}? {names}";
        items: count "prompt.items" = "{n} item" | "{n} items";
        delete: "prompt.delete" = "Delete";
        damaged_title: "prompt.damagedTitle" = "Damaged file";
        damaged_message: template "prompt.damagedMessage" = "“{name}” has damaged data. Open it anyway?";
        open_anyway: "prompt.openAnyway" = "Open Anyway";
        changed_on_disk_title: "prompt.changedOnDiskTitle" = "File changed";
        changed_on_disk_message: template "prompt.changedOnDiskMessage" = "“{name}” was changed by another app after it was opened or saved here.";
        changed_on_disk_detail: "prompt.changedOnDiskDetail" = "Replace it with this document, or cancel and use Save As to keep both.";
        replace: "prompt.replace" = "Replace";
        replace_file_title: "prompt.replaceFileTitle" = "Replace file";
        replace_file_message: template "prompt.replaceFileMessage" = "“{name}” already exists. Replace it?";
        replace_file_detail: template "prompt.replaceFileDetail" = "The name in the save dialog was “{answered}”.";
        trash_mask_title: "prompt.trashMaskTitle" = "Delete mask";
        trash_mask_message: "prompt.trashMaskMessage" = "Apply the mask to the layer before removing it?";
        trash_vector_mask_title: "prompt.trashVectorMaskTitle" = "Delete vector mask";
        trash_vector_mask_message: "prompt.trashVectorMaskMessage" = "Delete the vector mask?";
        /// The title of a custom prompt the app gave none.
        custom_title: "prompt.customTitle" = "Confirm";
        /// The header of the collapsible section of a custom prompt.
        details: "prompt.details" = "Details";
        apply: "prompt.apply" = "Apply";
        discard: "prompt.discard" = "Delete";
        dont_show_again: "prompt.dontShowAgain" = "Don't show again";
    }
}

craft_ui::words! {
    /// The unsaved-changes question's words. Windows and Linux ask Yes, No and Cancel; macOS asks Save, Don't Save
    /// and Cancel.
    pub struct UnsavedLabels {
        title: "unsaved.title" = "Unsaved changes";
        closing: template "unsaved.closing" = "Do you want to save the changes you made to “{name}” before closing?";
        quitting: template "unsaved.quitting" = "Do you want to save the changes you made to “{name}” before quitting?";
        reverting: template "unsaved.reverting" = "Revert “{name}” to the last saved version? Your changes will be lost.";
        save: "unsaved.save" = "Save";
        dont_save: "unsaved.dontSave" = "Don't Save";
        yes: "unsaved.yes" = "Yes";
        no: "unsaved.no" = "No";
        revert: "unsaved.revert" = "Revert";
        apply_to_all: "unsaved.applyToAll" = "Apply to All";
    }
}

craft_ui::words! {
    /// The recovery dialog's words.
    pub struct RecoveryLabels {
        title: "recovery.title" = "Recover Unsaved Work";
        /// `{name}` is the document and `{age}` when it was autosaved ("5 minutes ago").
        message: template "recovery.message" = "“{name}” has unsaved changes from an earlier run, autosaved {age}.";
        recover: "recovery.recover" = "Recover";
        discard: "recovery.discard" = "Discard";
        recently: "recovery.recently" = "recently";
        minutes_ago: count "recovery.minutesAgo" = "{n} minute ago" | "{n} minutes ago";
        hours_ago: count "recovery.hoursAgo" = "{n} hour ago" | "{n} hours ago";
        days_ago: count "recovery.daysAgo" = "{n} day ago" | "{n} days ago";
    }
}
