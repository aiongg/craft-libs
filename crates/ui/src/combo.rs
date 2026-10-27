//! Combo boxes and drop-down menus whose menu follows its contents' height while it is open.
//!
//! egui lays a popup out in the size it had on the previous pass, and [`egui::ComboBox`] puts its
//! menu in a [`ScrollArea`] that takes no more than that height. Contents that shrink (a search
//! with no matches) shrink the menu; contents that grow again (the search cleared) only scroll
//! inside it, and the menu stays small until it is closed. [`menu`] offers its contents the full
//! height on every pass, so the menu grows back. Use [`combo`] in place of `ComboBox`, and
//! [`menu`] under a field an app draws itself.

use egui::{
    Align2, AsIdSalt, Atom, AtomExt as _, Button, InnerResponse, Popup, Rect, Response, Role, ScrollArea, Shape, TextWrapMode, Ui, Vec2, WidgetInfo,
    WidgetText, vec2,
};

/// A combo box `width` wide showing `selected`, with `add` as its [`menu`], at most `max_height`
/// tall. Like egui's `ComboBox::from_id_salt(id_salt).selected_text(selected).width(width)
/// .height(max_height).truncate().show_ui(ui, add)`, with a menu that grows when its contents do.
pub fn combo<R>(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    selected: impl Into<WidgetText>,
    width: f32,
    max_height: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    let selected: WidgetText = selected.into();
    let value = selected.text().to_owned();
    ui.push_id(id_salt, |ui| {
        let open = Popup::is_id_open(ui.ctx(), ui.next_auto_id().with("popup"));
        let open_color = ui.visuals().widgets.open.fg_stroke.color;
        let icon = Atom::paint(Vec2::splat(ui.spacing().icon_width), move |ui, args| {
            let rect = Rect::from_center_size(args.rect.center(), vec2(args.rect.width() * 0.7, args.rect.height() * 0.45));
            ui.painter().add(Shape::rotated_triangle(rect, 0.0, if open { open_color } else { args.fallback_text_color }));
        })
        .atom_grow(true)
        .atom_align(if ui.style().right_to_left { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER });
        let button = ui.add(Button::new((selected, icon)).truncate().min_size(vec2(width, 0.0)));
        button.widget_info(|| {
            let mut info = WidgetInfo::new(Role::ComboBox);
            info.enabled = ui.is_enabled();
            info.current_text_value = Some(value.clone());
            info
        });
        let inner = menu(&button, max_height, add);
        InnerResponse::new(inner, button)
    })
    .inner
}

/// `add` in a menu under `button` (which opens and closes it), at least as wide as `button` and at
/// most `max_height` tall, scrolling beyond. Its height follows its contents on every pass.
/// Returns what `add` returned while the menu is open.
pub fn menu<R>(button: &Response, max_height: f32, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let popup = Popup::default_response_id(button);
    let inner = Popup::menu(button)
        .width(button.rect.width())
        .show(|ui| {
            ui.set_min_width(ui.available_width());
            // The popup offers last pass's height. Offer the full height instead: a scroll area
            // takes the height it is offered, then shrinks to its contents.
            ui.set_max_height(max_height);
            ScrollArea::vertical()
                .max_height(max_height)
                .show(ui, |ui| {
                    // Labels widen a narrow menu rather than wrap early, as in egui's combo box.
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    add(ui)
                })
                .inner
        })
        .map(|r| r.inner);
    if inner.is_some() {
        button.ctx.accesskit_node_builder(popup, |node| node.set_role(Role::ListBox));
    }
    inner
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    struct Menu {
        filter: String,
        button: Option<egui::Id>,
    }

    fn popup(h: &Harness<'_, Menu>) -> egui::Id {
        h.state().button.unwrap_or(egui::Id::NULL).with("popup")
    }

    /// The menu's height as egui keeps it between passes.
    fn height(h: &Harness<'_, Menu>) -> f32 {
        egui::AreaState::load(&h.ctx, popup(h)).and_then(|s| s.size).map_or(0.0, |s| s.y)
    }

    #[test]
    fn a_filtered_menu_grows_back_when_the_filter_is_cleared() {
        let mut h = Harness::builder().with_size(egui::vec2(600.0, 800.0)).build_ui_state(
            |ui, m: &mut Menu| {
                let inner = super::combo(ui, "language", "Any language", 200.0, 380.0, |ui| {
                    let name = ui.label("Find a language").id;
                    ui.text_edit_singleline(&mut m.filter).labelled_by(name);
                    let rows = if m.filter.is_empty() { 60 } else { 0 };
                    let row = ui.spacing().interact_size.y;
                    egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show_rows(ui, row, rows, |ui, range| {
                        for n in range {
                            let _ = ui.selectable_label(false, format!("Language {n}"));
                        }
                    });
                });
                ui.ctx().accesskit_node_builder(inner.response.id, |node| node.set_label("Language"));
                m.button = Some(inner.response.id);
            },
            Menu { filter: String::new(), button: None },
        );
        h.run();
        egui::Popup::open_id(&h.ctx, popup(&h));
        h.run();
        let full = height(&h);
        assert!(full > 300.0, "the open menu shows the list: {full}");

        h.state_mut().filter = "xxxx".into();
        h.run();
        let none = height(&h);
        assert!(none < 100.0, "no matches shrink the menu: {none}");

        h.state_mut().filter.clear();
        h.run();
        assert!((height(&h) - full).abs() < 1.0, "cleared, the menu is {} tall again, not {}", full, height(&h));
    }
}
