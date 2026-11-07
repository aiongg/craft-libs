//! Built-in words of the form dialogs family.

craft_ui::words! {
    /// The words of the parameter rows: the buttons and names a row adds beside its control. Each id is prefixed by
    /// its module. `{name}` stands for the row's label, `{row}` and `{column}` for a grid cell's place.
    pub struct ParamWords {
        browse: "formDialogs.browse" = "Browse…";
        randomize: "formDialogs.randomize" = "Random";
        eyedropper: "formDialogs.eyedropper" = "Eyedropper";
        custom: "formDialogs.custom" = "Custom";
        unit: "formDialogs.unit" = "Unit";
        angle_field: template "formDialogs.angleField" = "{name} in degrees";
        point_x: template "formDialogs.pointX" = "{name} X";
        point_y: template "formDialogs.pointY" = "{name} Y";
        grid_cell: template "formDialogs.gridCell" = "{name} {row}, {column}";
        choose_color: "formDialogs.chooseColor" = "Choose…";
        color_unset: "formDialogs.colorUnset" = "Not set";
    }
}
