//! Built-in words of the pickers family.

craft_ui::words! {
    /// The words of the dropdown pickers.
    pub struct PickerWords {
        /// Shown in a select or tree picker whose value is mixed.
        mixed: "select.mixed" = "Mixed";
        /// The name of the filter field above a searchable select's options.
        search: "select.search" = "Search";
        /// Shown in a searchable select's list when no option matches.
        no_matches: "select.noMatches" = "No matches";
        /// The name of the button in a select's popup header that opens its menu.
        header_menu: "select.headerMenu" = "More options";
        /// The name of the combo box's option list.
        combo_options: "comboBox.options" = "Options";
        /// Shown in a tree picker with no pick.
        tree_placeholder: "treePicker.placeholder" = "Choose…";
    }
}
