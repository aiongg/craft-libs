//! The rename tables of contract-rt.md section 15.11 and the service rows of 15.9.

/// An egui item with a craft-ui replacement (rules 1-11).
pub struct Item {
    pub egui: &'static str,
    /// `""`, `"geom"` or `"input"`: the module below `craft_ui`.
    pub module: &'static str,
    pub new: &'static str,
    pub rule: &'static str,
    /// Rule number, for the ui-contract-internal targets (only rules 1-4 have one).
    pub number: u8,
}

const fn item(egui: &'static str, module: &'static str, new: &'static str, rule: &'static str, number: u8) -> Item {
    Item { egui, module, new, rule, number }
}

pub const ITEMS: &[Item] = &[
    item("Ui", "", "Ui", "01 egui::Ui", 1),
    item("Response", "", "Response", "02 egui::Response", 2),
    item("Id", "", "Id", "03 egui::Id", 3),
    item("IdSalt", "", "IdSalt", "03 egui::Id", 3),
    item("Context", "", "Context", "04 egui::Context", 4),
    item("Pos2", "geom", "Pos2", "05 geometry", 5),
    item("Rect", "geom", "Rect", "05 geometry", 5),
    item("Vec2", "geom", "Vec2", "05 geometry", 5),
    item("Align", "geom", "Align", "05 geometry", 5),
    item("pos2", "geom", "pos2", "05 geometry", 5),
    item("vec2", "geom", "vec2", "05 geometry", 5),
    item("Rangef", "geom", "Rangef", "05 geometry", 5),
    item("lerp", "geom", "lerp", "05 geometry", 5),
    item("Color32", "", "Color32", "06 Color32/CornerRadius", 6),
    item("CornerRadius", "", "CornerRadius", "06 Color32/CornerRadius", 6),
    item("Key", "input", "Key", "07 Key/Modifiers", 7),
    item("Modifiers", "input", "Mods", "07 Key/Modifiers", 7),
    item("PointerButton", "input", "PointerButton", "08 PointerButton/CursorIcon", 8),
    item("CursorIcon", "input", "CursorIcon", "08 PointerButton/CursorIcon", 8),
    item("ViewportId", "", "WindowId", "09 ViewportId", 9),
    item("ViewportCommand", "", "WindowCommand", "10 ViewportCommand", 10),
    item("UiBuilder", "", "UiBuilder", "11 UiBuilder", 11),
];

pub fn item_named(name: &str) -> Option<&'static Item> {
    ITEMS.iter().find(|i| i.egui == name)
}

/// Rule 10: `ViewportCommand` variants and their `WindowCommand` names.
pub const VIEWPORT_VARIANTS: &[(&str, &str)] = &[
    ("Title", "Title"),
    ("InnerSize", "Size"),
    ("MinInnerSize", "MinSize"),
    ("OuterPosition", "Position"),
    ("Visible", "Visible"),
    ("Decorations", "Decorations"),
    ("WindowLevel", "Level"),
    ("Minimized", "Minimized"),
    ("Maximized", "Maximized"),
    ("Fullscreen", "Fullscreen"),
    ("Focus", "Focus"),
    ("StartDrag", "DragMove"),
    ("Close", "Close"),
];

pub fn variant_named(name: &str) -> Option<&'static str> {
    VIEWPORT_VARIANTS.iter().find(|(from, _)| *from == name).map(|(_, to)| *to)
}

/// A method of `ui` renamed by rules 12-15: (old, new, argument count, rule).
pub const UI_METHODS: &[(&str, &str, usize, &str)] = &[
    ("is_sizing_pass", "is_measuring", 0, "12 is_measuring"),
    ("painter", "draw", 0, "13 draw"),
    ("painter_at", "draw_at", 1, "14 draw_at"),
    ("allocate_space", "allocate", 1, "15 allocate"),
];

/// Who a service function's replacement is a method of.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A method of `Ui` only.
    Ui,
    /// A method of `Ui` and of `Cx` ("Ui, Cx" in 15.9).
    UiCx,
    /// A method of `Context`.
    Ctx,
}

pub struct Service {
    pub module: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    pub new: &'static str,
    /// The call's argument count, the context included.
    pub args: usize,
    pub rule: &'static str,
    /// Wrap the second argument as `std::time::Duration::from_secs_f64(..)` (rule 18).
    pub secs: bool,
}

const fn svc(module: &'static str, name: &'static str, kind: Kind, new: &'static str, args: usize) -> Service {
    Service { module, name, kind, new, args, rule: "21 service call", secs: false }
}

const fn svc_rule(module: &'static str, name: &'static str, kind: Kind, new: &'static str, args: usize, rule: &'static str) -> Service {
    Service { module, name, kind, new, args, rule, secs: false }
}

use Kind::{Ctx, Ui, UiCx};

/// Rules 16, 18, 19 and 21: the 15.9 rows whose replacement takes the old arguments minus the context.
pub const SERVICES: &[Service] = &[
    svc_rule("pass", "current_viewport", Ctx, "window", 1, "16 window"),
    svc_rule("frame", "request_repaint", Ctx, "request_repaint", 1, "18 request_repaint"),
    Service {
        module: "frame",
        name: "request_repaint_after",
        kind: Ctx,
        new: "request_repaint_after",
        args: 2,
        rule: "18 request_repaint",
        secs: true,
    },
    svc_rule("measure", "active", UiCx, "is_measuring", 1, "19 measure::active"),
    svc("frame", "clock", Ctx, "time", 1),
    svc("pointer", "frame", Ui, "pointer", 1),
    svc("pointer", "mods", Ctx, "modifiers", 1),
    svc("pointer", "hover_point_in", Ui, "hover_point_in", 2),
    svc("pointer", "hovers", Ui, "hovers", 2),
    svc("pointer", "pressed_part", Ui, "pressed_part", 2),
    svc("pointer", "shapes_at", Ui, "shapes_at", 2),
    svc("primitives", "cancel_press_of", Ui, "cancel_press_of", 2),
    svc("focus", "request", UiCx, "request_focus", 2),
    svc("focus", "focus_in_scope", Ui, "focus_in_scope", 2),
    svc("focus", "focused_rec", Ui, "focused_stop", 1),
    svc("focus", "is_focused", Ui, "is_focused", 2),
    svc("focus", "open_scope", Ui, "open_focus_scope", 1),
    svc("focus", "return_from_pick", Ui, "return_from_pick", 2),
    svc("focus", "running_menu", Ui, "running_menu", 1),
    svc("keys", "held", Ctx, "held_keys", 1),
    svc("keys", "keyed", Ctx, "keyed", 1),
    svc("keys", "take_submit", Ui, "take_submit", 1),
    svc("lock_keys", "caps_lock", Ctx, "caps_lock", 1),
    svc("commands", "fire", UiCx, "fire", 3),
    svc("commands", "resolve", Ui, "resolve_command", 2),
    svc("commands", "shown", Ui, "command_shown", 2),
    svc("commands", "holds", Ui, "hold_commands", 5),
    svc("commands", "holder", Ui, "command_holder", 2),
    svc("commands", "keymap_generation", Ui, "keymap_generation", 1),
    svc("commands", "declare_child", Ui, "declare_command_scope", 3),
    svc("commands", "fire_beneath", Ui, "fire_beneath", 4),
    svc("commands", "resolve_beneath", Ui, "resolve_beneath", 3),
    svc("memory", "with_session", Ui, "with_session", 3),
    svc("memory", "set_saved_value", Ui, "set_saved_value", 3),
    svc("overlay", "raise", Ui, "raise_layer", 2),
    svc("overlay", "opening", Ui, "layer_opening", 2),
    svc("overlay", "is_open", Ui, "layer_open", 2),
    svc("overlay", "entry", Ui, "layer_entry", 2),
    svc("overlay", "any_open", Ui, "any_layer_open", 2),
    svc("overlay", "top", Ui, "top_layer", 1),
    svc("overlay", "drawing_parent", Ui, "layer_parent", 1),
    svc("overlay", "drawing_top_modal", Ui, "in_top_modal", 1),
    svc("overlay", "is_top_modal", Ui, "is_top_modal", 2),
    svc("overlay", "running_dialog", Ui, "running_dialog", 1),
    svc("overlay", "request_close", Ui, "close_layer", 3),
    svc("overlay", "end_stack_dialog", Ui, "end_stack_dialog", 2),
    svc("overlay", "stack_place", Ui, "stack_place", 4),
    svc("overlay", "opened_from", Ui, "opened_from", 2),
    svc("drag", "started", Ui, "drag_started", 1),
    svc("drag", "cancel", Ui, "cancel_drag", 2),
    svc("drag", "mark", Ui, "drop_mark", 1),
    svc("drag", "moves", Ui, "drag_moves", 1),
    svc("drag", "next_ordinal", Ui, "drag_ordinal", 2),
    svc("drag", "winning", Ui, "drag_winning", 3),
    svc("drag", "sourced_by", Ui, "drag_sourced_by", 2),
    svc("drag", "cross_window_pointer", Ui, "drag_pointer", 1),
    svc("edit", "next_gesture_id", UiCx, "next_gesture_id", 1),
    svc("edit", "cancel_all", Ui, "cancel_edits", 2),
    svc("edit", "watch_gestures", Ui, "watch_gestures", 2),
    svc("inspect", "recorded", Ui, "inspect_recorded", 2),
    svc("inspect", "pointed", Ui, "inspect_pointed", 2),
    svc("inspect", "point_at", Ui, "inspect_point_at", 3),
    svc("layout", "ambient_width", Ui, "ambient_width", 3),
    svc("sampler", "pass_tool_gesture", Ui, "pass_tool_gesture", 3),
    svc("contract", "request_rename", Ui, "request_rename", 3),
    svc("windows", "os_of", Ctx, "os", 1),
];

pub fn service_named(module: &str, name: &str) -> Option<&'static Service> {
    SERVICES.iter().find(|s| s.module == module && s.name == name)
}

/// 15.9 rows that are deleted, hand-written or change arguments beyond dropping the context: C1 only reports them.
/// An empty name stands for every function of the module.
pub const HAND_WRITTEN: &[(&str, &str)] = &[
    ("frame", "running"),
    ("frame", "discard_pass"),
    ("frame", "discardable"),
    ("frame", "will_rerun"),
    ("measure", "effect"),
    ("layout_memory", ""),
    ("pointer", "with_pass_pointer"),
    ("pointer", "withhold_members"),
    ("primitives", "member_gesture_running"),
    ("cursor", "request"),
    ("cursor", "request_within"),
    ("focus", "request_within"),
    ("keys", "capture"),
    ("keys", "claim"),
    ("keys", "claim_alt_tap"),
    ("keys", "take_claimed"),
    ("keys", "take_alt_tap"),
    ("keys", "intents_of"),
    ("keys", "settle"),
    ("keys", "with_canvas_window_intents"),
    ("commands", "note_page_root"),
    ("derived", "derived"),
    ("overlay", "take_keys_for_new_dialog"),
    ("toast", ""),
    ("drag", "start"),
    ("ghost", "readout"),
    ("access", "report_sub"),
    ("windows", "shell_of"),
    ("windows", "logic"),
    ("windows", "hold_close"),
    ("windows", "root_close_requested"),
    ("windows", "route_root"),
    ("WindowHost", "of"),
    ("TextureCache", "texture"),
    ("TextureStore", "in_context"),
    ("Thumbnails", "in_context"),
    ("replay", "journal"),
    ("pen", "install"),
];

pub fn is_hand_written(module: &str, name: &str) -> bool {
    HAND_WRITTEN.iter().any(|(m, f)| *m == module && (f.is_empty() || *f == name))
}

/// Method names of rule 24's Fixture methods without a Harness twin (prefix match for `story` and `frame_shapes`).
pub fn fixture_without_twin(method: &str) -> bool {
    method.starts_with("frame_shapes")
        || method.starts_with("story")
        || matches!(method, "take_paint" | "texts" | "shape_bounds" | "ctx" | "cursor_icon" | "hosted" | "probe_purity")
}

/// Skip class of an egui or epaint item name the do-not-touch list names.
pub fn skip_class(name: &str) -> Option<&'static str> {
    match name {
        "TextureHandle" | "ColorImage" | "SizedTexture" | "TextureId" | "TextureOptions" => Some("textures"),
        "Shape" | "Painter" | "Stroke" | "Mesh" => Some("raw-egui-paint"),
        "ViewportBuilder" | "ViewportInfo" => Some("viewport-other"),
        "Layout" | "Align2" | "Grid" => Some("egui-layout-widgets"),
        "Event" | "RawInput" | "TouchPhase" | "MouseWheelUnit" => Some("egui-input-types"),
        _ => None,
    }
}

/// Method names the do-not-touch list leaves, by class (for a `ui` or `ctx` receiver where the class says so).
pub fn method_class(name: &str) -> Option<&'static str> {
    match name {
        "load_texture" => Some("textures"),
        "slot_here" | "take_after" => Some("picture-slot"),
        "send_viewport_cmd_to" => Some("viewport-other"),
        "take_command" | "take_step" => Some("take-command-step"),
        "hover_pos" | "highlight" | "request_focus" | "context_menu" | "layer_id" => Some("response-methods"),
        _ => None,
    }
}

/// Egui widget and layout helpers of `ui` that rules 12-15 do not touch.
pub fn ui_widget_method(name: &str) -> bool {
    matches!(
        name,
        "label"
            | "button"
            | "horizontal"
            | "horizontal_wrapped"
            | "horizontal_centered"
            | "horizontal_top"
            | "vertical"
            | "vertical_centered"
            | "vertical_centered_justified"
            | "with_layout"
            | "layout"
            | "spacing"
            | "spacing_mut"
            | "allocate_exact_size"
            | "allocate_ui"
            | "allocate_ui_with_layout"
    )
}
