//! The Image Size dialog.

use craft_ui::composites::{FormGrid, LabelAlign};
use craft_ui::controls::{Checkbox, IconButton, NumberField, Select};
use craft_ui::dialog::{Dialog, DialogState, Footer, Outcome};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::LocaleNumbers;
use craft_ui::primitives::Text;
use egui::Ui;

use super::model::{ImageSizeDraft, Mock, Resample, SizeUnit};

/// Opens the dialog's draft from the document.
pub fn open_image_size(mock: &mut Mock) {
    mock.open_image_size();
}

/// Draws the dialog while `mock.dialog` is set, centred over the preview's visible rect. The
/// library dialog is modal to the whole window, so the gallery's own panels are blocked while it
/// is open.
pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut DialogState<ImageSizeDraft>) {
    let ctx = ui.ctx().clone();
    let Some(draft) = mock.dialog.as_mut() else {
        if state.is_open() {
            state.close();
        }
        return;
    };
    let id = ui.scope_id().with("image-size-dialog");
    let shown = Dialog::new(id, "Image Size").center_over(ui.clip_rect()).footer(Footer::standard()).show(&ctx, state, draft, body);
    match shown.outcome {
        Some(Outcome::Ok) => mock.apply_size(),
        Some(Outcome::Cancel | Outcome::Close) => mock.dialog = None,
        _ => {}
    }
}

fn megabytes([w, h]: [u32; 2]) -> String {
    format!("{:.1}M", f64::from(w) * f64::from(h) * 4.0 / (1024.0 * 1024.0))
}

fn body(ui: &mut Ui, draft: &mut ImageSizeDraft) {
    let numbers = (*LocaleNumbers::of(ui)).clone().ungrouped();
    let [w, h] = draft.pixel_size();
    let (resample, original) = (draft.resample, draft.original);
    let mut widths = (None, None);
    let mut heights = (None, None);
    let mut link = draft.link;
    let mut resolution = None;
    let mut method = None;
    let mut resample_on = resample;
    FormGrid::new("image-size").label_align(LabelAlign::End).min_label_width(90.0).show(ui, |form| {
        form.value("Image Size:", format!("{} (was {})", megabytes([w, h]), megabytes(original)));
        form.value("Dimensions:", format!("{w} px \u{d7} {h} px"));
        form.space(6.0);
        for axis in 0..2 {
            let (label, id, value, unit) =
                if axis == 0 { ("Width:", "width", draft.width, draft.unit_w) } else { ("Height:", "height", draft.height, draft.unit_h) };
            let (name, unit_name) = if axis == 0 { ("Width", "Width unit") } else { ("Height", "Height unit") };
            let (edit_out, unit_out) = if axis == 0 { (&mut widths.0, &mut widths.1) } else { (&mut heights.0, &mut heights.1) };
            form.row(label, |ui, _| {
                let field = match unit {
                    SizeUnit::Pixels => NumberField::new(id, name, value, &numbers).range(1.0..=300_000.0).decimals(0..=0),
                    SizeUnit::Percent => NumberField::new(id, name, value, &numbers).range(1.0..=100_000.0).decimals(0..=2),
                };
                let edit = field.enabled(resample).show(ui);
                *edit_out = value_of(&edit.event);
                let mut chosen = unit;
                let mut select = Select::new(id.to_owned() + "-unit", unit_name, &mut chosen).enabled(resample).fit(true);
                for u in SizeUnit::ALL {
                    select = select.option(u, u.label());
                }
                *unit_out = value_of(&select.show(ui).event).filter(|u| *u != unit);
                if axis == 0 {
                    let icon = if link { Icon::Link } else { Icon::Unlink };
                    ui.add(IconButton::new(icon, "Constrain proportions").toggle(&mut link).enabled(resample));
                }
                edit
            });
        }
        form.row("Resolution:", |ui, _| {
            let edit = NumberField::new("resolution", "Resolution", draft.resolution, &numbers).range(1.0..=30_000.0).decimals(0..=0).show(ui);
            resolution = value_of(&edit.event);
            ui.add(Text::new("Pixels/Inch"));
            edit
        });
        form.space(4.0);
        form.unlabelled(|ui| {
            let check = Checkbox::bool("Resample", &mut resample_on).show(ui);
            let mut chosen = draft.method;
            let mut select = Select::new("resample-method", "Resample method", &mut chosen).enabled(resample).fit(true);
            for m in Resample::ALL {
                select = select.option(m, m.label());
            }
            method = value_of(&select.show(ui).event);
            check
        });
    });
    draft.link = link;
    if resample_on != resample {
        draft.set_resample(resample_on);
    }
    if let Some(unit) = widths.1 {
        draft.set_unit(0, unit);
    }
    if let Some(unit) = heights.1 {
        draft.set_unit(1, unit);
    }
    if let Some(v) = widths.0 {
        draft.set_width(v);
    }
    if let Some(v) = heights.0 {
        draft.set_height(v);
    }
    if let Some(v) = resolution {
        draft.resolution = v;
    }
    if let Some(m) = method {
        draft.method = m;
    }
}

fn value_of<T: Copy>(event: &Option<EditEvent<T>>) -> Option<T> {
    event.as_ref().and_then(|e| e.value().copied())
}

#[cfg(test)]
mod tests {
    use craft_theme::{ModeChoice, SystemAppearance};
    use craft_ui::inspect::AccessTree;
    use craft_ui::theme::install;
    use egui::accesskit::{Action, ActionRequest, Role, TreeId};
    use egui::{Context, Event, Key, Modifiers, RawInput, Rect, vec2};

    use super::super::AppPreview;
    use crate::Appearance;

    struct Fixture {
        ctx: Context,
        appearance: Appearance,
        preview: AppPreview,
    }

    impl Fixture {
        fn open() -> Self {
            let ctx = Context::default();
            ctx.enable_accesskit();
            let appearance = Appearance { theme: "craft.artcraft".to_owned(), mode: ModeChoice::Dark, ..Appearance::default() };
            appearance.configure(&ctx, None, false, SystemAppearance::default());
            let mut fixture = Self { ctx, appearance, preview: AppPreview::default() };
            super::open_image_size(&mut fixture.preview.mock);
            fixture.settle();
            fixture
        }

        fn frame(&mut self, events: Vec<Event>) -> Option<AccessTree> {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))), events, ..RawInput::default() };
            let base = self.appearance.base(SystemAppearance::default());
            let output = self.ctx.run_ui(raw, |ui| {
                install(ui.ctx(), &base);
                self.preview.show(ui);
            });
            let tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
            output.drop_without_applying_deltas();
            tree
        }

        fn settle(&mut self) -> AccessTree {
            for _ in 0..4 {
                self.frame(Vec::new());
            }
            self.frame(Vec::new()).expect("an AccessKit tree")
        }

        fn click(&mut self, name: &str) {
            let tree = self.settle();
            let found: Vec<_> = tree
                .walk(tree.root())
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == Role::Button))
                .collect();
            assert_eq!(found.len(), 1, "one button named {name:?}");
            let request = ActionRequest { action: Action::Click, target_tree: TreeId::ROOT, target_node: found[0], data: None };
            self.frame(vec![Event::AccessKitActionRequest(request)]);
            self.settle();
        }

        fn key(&mut self, key: Key) {
            let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
            self.frame(vec![event(true), event(false)]);
            self.settle();
        }

        /// Types a width into the focused Width field and leaves it.
        fn type_width(&mut self, text: &str) {
            self.frame(vec![Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }]);
            self.frame(vec![Event::Text(text.to_owned())]);
            self.key(Key::Tab);
        }
    }

    #[test]
    fn ok_applies_the_size_and_the_link_moves_the_height() {
        let mut f = Fixture::open();
        let [w, h] = f.preview.mock.doc.size;
        f.type_width("600");
        let draft = f.preview.mock.dialog.clone().expect("the dialog is open");
        assert_eq!(draft.width, 600.0);
        assert_eq!(draft.height, (600.0 * f64::from(h) / f64::from(w)).round(), "the linked height follows");
        f.click("OK");
        assert!(f.preview.mock.dialog.is_none());
        assert_eq!(f.preview.mock.doc.size[0], 600);
        assert!(f.preview.mock.status.starts_with("600 x "), "status: {}", f.preview.mock.status);
    }

    #[test]
    fn cancel_and_escape_drop_the_draft() {
        let mut f = Fixture::open();
        let (size, status) = (f.preview.mock.doc.size, f.preview.mock.status.clone());
        f.type_width("600");
        f.click("Cancel");
        assert!(f.preview.mock.dialog.is_none());
        assert_eq!((f.preview.mock.doc.size, &f.preview.mock.status), (size, &status));

        let mut f = Fixture::open();
        f.type_width("600");
        f.key(Key::Escape);
        assert!(f.preview.mock.dialog.is_none());
        assert_eq!(f.preview.mock.doc.size, size);
    }
}
