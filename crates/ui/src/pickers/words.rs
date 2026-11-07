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
        /// The calendar button's tooltip and accessible name.
        date_pick: "datePicker.pick" = "Pick a date";
        /// The name of the year page's switch segment.
        date_year: "datePicker.year" = "Year";
        /// The name of the month page's switch segment.
        date_month: "datePicker.month" = "Month";
        /// The name of the day page's switch segment.
        date_day: "datePicker.day" = "Day";
        /// The name of the button that shows the previous page.
        date_previous: "datePicker.previous" = "Previous";
        /// The name of the button that shows the next page.
        date_next: "datePicker.next" = "Next";
        /// The link that picks today.
        date_today: "datePicker.today" = "Today";
        /// The link that clears the date.
        date_clear: "datePicker.clear" = "Clear";
        /// The name of the precision switch.
        date_precision: "datePicker.precision" = "Pick by";
        /// The name of the icon picker's search field.
        icon_search: "iconPicker.search" = "Search icons";
        /// Shown in the icon picker when no icon matches the search.
        icon_no_match: "iconPicker.noMatch" = "No icons match your search.";
    }
}
