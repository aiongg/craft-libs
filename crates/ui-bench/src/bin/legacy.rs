//! Runs the spec 22.7 scenarios on the legacy UI library (`craft-ui-legacy`) and writes
//! `baseline/legacy.json`.

use std::path::PathBuf;

use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::composites::{Field, FieldGroup, SectionHeader};
use craft_ui::controls::{NumberField, TextField};
use craft_ui::list::{List, ListRow, Table, TableColumn, TableLayout};
use craft_ui::localized::LocaleNumbers;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::patterns::StatusBar;
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{Base, install};
use craft_ui::viewport::ViewportWindow;
use craft_ui_bench::{CountingAlloc, Library, Scene, SceneBuild, print_table, run_all, write_json};
use egui::collapsing_header::CollapsingState;
use egui::{Context, Event, Modifiers, MouseWheelUnit, Pos2, Sense, Ui, Vec2, ViewportBuilder, ViewportId, pos2, vec2};

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

struct Legacy;

impl Library for Legacy {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn install(&self, ctx: &Context) {
        install(ctx, &Base::suite_default());
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.all_styles_mut(|style| style.scroll_animation = egui::style::ScrollAnimation::none());
    }

    fn build(&self, name: &str) -> SceneBuild {
        let (scene, equivalent): (Box<dyn Scene>, &'static str) = match name {
            "inspector-500-fields" => {
                (Box::new(InspectorScene::new()), "Inspector > FieldGroup > 500 Field rows (alternating NumberField and TextField)")
            }
            "virtual-list-10000" => (Box::new(ListScene::new(Selected::None)), "list::List over 10,000 RowInfo, one selected row"),
            "sheet-50x200" => (Box::new(SheetScene::new()), "list::Table of 50 columns x 200 rows, a screen tall enough to draw every cell"),
            "canvas-50-controls" => (Box::new(CanvasScene), "primitives::Handle x 50 over an interactive canvas rect"),
            "menu-40-rows" => (Box::new(MenuScene::new()), "menu::ContextMenu of 40 items, opened on the first frame"),
            "text-field-100kb" => (Box::new(TextScene::new()), "controls::TextField::multiline over a 100 KB String the host owns"),
            "list-10000-selected" => (Box::new(ListScene::new(Selected::All)), "list::List over 10,000 RowInfo, all 10,000 keys in the Selection"),
            "collapse-in-content-sized-panel" => (
                Box::new(CollapseScene::new()),
                "egui::Window sized to content; SectionHeader row with an egui CollapsingState animating the body (legacy has no animated section)",
            ),
            "auto-height-list-scroll-10000" => {
                (Box::new(AutoHeightScene::new()), "list::List::measured over 10,000 rows of 0 to 3 wrapped lines, mouse wheel every frame")
            }
            "second-viewport" => (Box::new(ViewportScene), "viewport::ViewportWindow, embedded (egui's default without a native backend)"),
            "readout-and-content-column" => {
                (Box::new(ReadoutScene), "patterns::StatusBar trailing slot and a FieldGroup label column whose text changes width every frame")
            }
            _ => return SceneBuild::Missing { reason: "the scenario list names a scenario this binary does not know" },
        };
        SceneBuild::Scene { scene, equivalent }
    }
}

struct InspectorScene {
    numbers: LocaleNumbers,
    values: Vec<f64>,
    texts: Vec<String>,
}

impl InspectorScene {
    fn new() -> Self {
        Self {
            numbers: LocaleNumbers::from_tag("en"),
            values: (0..500).map(f64::from).collect(),
            texts: (0..500).map(|i| format!("Value {i}")).collect(),
        }
    }
}

impl Scene for InspectorScene {
    fn size(&self) -> Vec2 {
        vec2(360.0, 900.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let Self { numbers, values, texts } = self;
        craft_ui::patterns::Inspector::new("inspector").target("layer").show(ui, |ui| {
            FieldGroup::new("fields").show(ui, |ui| {
                for i in 0..500 {
                    let label = format!("Property {i}:");
                    if i % 2 == 0 {
                        let _ = Field::new(label).show(ui, |ui, _| NumberField::new(("number", i), "Number", values[i], numbers).show(ui));
                    } else {
                        let _ = Field::new(label).show(ui, |ui, _| TextField::new(("text", i), "Text", &texts[i]).show(ui));
                    }
                }
            });
        });
    }
}

#[derive(Clone, Copy)]
enum Selected {
    None,
    All,
}

struct ListScene {
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
}

impl ListScene {
    fn new(selected: Selected) -> Self {
        let rows: Vec<_> = (0..10_000).map(RowInfo::leaf).collect();
        let selection = match selected {
            Selected::None => Selection::only(0),
            Selected::All => Selection { items: (0..10_000).collect(), lead: Some(0), anchor: Some(0) },
        };
        Self { rows, selection }
    }
}

impl Scene for ListScene {
    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let _ = List::new("list", "Items", self.rows.as_slice())
            .selection(&mut self.selection)
            .max_height(560.0)
            .show(ui, |_, &key| ListRow::new(key, format!("Item {key}")));
    }
}

const SHEET_COLUMNS: usize = 50;
const SHEET_ROWS: usize = 200;

struct SheetScene {
    columns: Vec<TableColumn<usize>>,
    layout: TableLayout<usize>,
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
}

impl SheetScene {
    fn new() -> Self {
        let columns: Vec<_> = (0..SHEET_COLUMNS).map(|c| TableColumn::new(c, format!("C{c}")).min_width(60.0).initial_width(80.0)).collect();
        let layout = TableLayout::new(&columns);
        Self { columns, layout, rows: (0..SHEET_ROWS).map(RowInfo::leaf).collect(), selection: Selection::only(0) }
    }
}

impl Scene for SheetScene {
    fn size(&self) -> Vec2 {
        vec2(SHEET_COLUMNS as f32 * 80.0 + 40.0, 5200.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let Self { columns, layout, rows, selection } = self;
        let _ = Table::new("sheet", "Sheet", columns, layout, rows.as_slice()).selection(selection).row_name(|row| format!("Row {row}")).show(
            ui,
            |ui, row, column, cell| {
                cell.label(ui, format!("{row}:{column}"));
            },
        );
    }
}

struct CanvasScene;

impl Scene for CanvasScene {
    fn size(&self) -> Vec2 {
        vec2(900.0, 700.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let (rect, _) = ui.allocate_exact_size(vec2(840.0, 640.0), Sense::click_and_drag());
        const SHAPES: [HandleShape; 3] = [HandleShape::Square, HandleShape::Round, HandleShape::Rotate];
        for i in 0..50usize {
            let at = pos2(rect.left() + 40.0 + (i % 10) as f32 * 80.0, rect.top() + 40.0 + (i / 10) as f32 * 120.0);
            let _ = Handle::new(SHAPES[i % 3], format!("Handle {i}")).id_salt(i).show(ui, at);
        }
    }
}

struct MenuScene {
    entries: Vec<MenuEntry<usize>>,
}

impl MenuScene {
    fn new() -> Self {
        Self { entries: (0..40).map(|i| MenuItem::new(format!("Command {i}"), i).into()).collect() }
    }
}

impl Scene for MenuScene {
    fn size(&self) -> Vec2 {
        vec2(800.0, 1400.0)
    }

    fn show(&mut self, ui: &mut Ui, frame: u64) {
        let button = ui.add(craft_ui::controls::Button::new("Open"));
        let open = (frame == 0).then(|| OpenAt::Pointer(button.rect.left_bottom()));
        let _ = ContextMenu::new(&self.entries).open_at(open).own_gestures(false).show(ui, &button);
    }
}

struct TextScene {
    text: String,
}

impl TextScene {
    fn new() -> Self {
        let line = "The quick brown fox jumps over the lazy dog, again and again, to fill a line.\n";
        Self { text: line.repeat(100 * 1024 / line.len() + 1) }
    }
}

impl Scene for TextScene {
    fn size(&self) -> Vec2 {
        vec2(900.0, 700.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let _ = TextField::new("notes", "Notes", &self.text).multiline(30).show(ui);
    }
}

struct CollapseScene {
    open: bool,
    animated: bool,
}

impl CollapseScene {
    fn new() -> Self {
        Self { open: false, animated: false }
    }
}

/// Frames between two toggles; the animation runs longer, so one is always under way.
const COLLAPSE_TOGGLE_FRAMES: u64 = 30;

impl Scene for CollapseScene {
    fn show(&mut self, ui: &mut Ui, frame: u64) {
        if !self.animated {
            ui.ctx().all_styles_mut(|style| style.animation_time = 0.5);
            self.animated = true;
        }
        if frame.is_multiple_of(COLLAPSE_TOGGLE_FRAMES) {
            self.open = !self.open;
        }
        let ctx = ui.ctx().clone();
        let Self { open, .. } = self;
        let _ = egui::Window::new("Panel").title_bar(false).resizable(false).auto_sized().show(&ctx, |ui| {
            let mut header_open = *open;
            let _ = SectionHeader::new("section", "Appearance").open(&mut header_open).show(ui, |_| ());
            let id = ui.make_persistent_id("body");
            let mut state = CollapsingState::load_with_default_open(&ctx, id, false);
            state.set_open(*open);
            let _ = state.show_body_unindented(ui, |ui| {
                for i in 0..12 {
                    ui.label(format!("Row {i}"));
                }
            });
            state.store(&ctx);
        });
    }
}

struct AutoHeightScene {
    rows: Vec<RowInfo<usize>>,
    selection: Selection<usize>,
}

impl AutoHeightScene {
    fn new() -> Self {
        Self { rows: (0..10_000).map(RowInfo::leaf).collect(), selection: Selection::only(0) }
    }
}

const WRAPPED: &str = "A longer description of the row that wraps onto more lines when the list is narrow, to give rows different heights.";

impl Scene for AutoHeightScene {
    fn size(&self) -> Vec2 {
        vec2(320.0, 640.0)
    }

    fn events(&mut self, frame: u64, events: &mut Vec<Event>) {
        if frame == 0 {
            events.push(Event::PointerMoved(Pos2::new(100.0, 300.0)));
        }
        events.push(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -40.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        });
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let _ =
            List::new("auto", "Versions", self.rows.as_slice()).selection(&mut self.selection).max_height(600.0).measured().show(ui, |_, &key| {
                let row = ListRow::new(key, format!("Row {key}"));
                match key % 4 {
                    0 => row,
                    lines => row.detail("Today").lines(WRAPPED, lines),
                }
            });
    }
}

struct ViewportScene;

impl Scene for ViewportScene {
    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let ctx = ui.ctx().clone();
        ui.label("Main window");
        let _ = ViewportWindow::new(ViewportId::from_hash_of("second"), ViewportBuilder::default().with_inner_size([320.0, 240.0]))
            .title("Second")
            .show(&ctx, |ui, _| {
                for i in 0..20 {
                    ui.label(format!("Line {i}"));
                }
            });
    }
}

struct ReadoutScene;

impl Scene for ReadoutScene {
    fn show(&mut self, ui: &mut Ui, frame: u64) {
        // Each frame's text is a different width.
        let wide = "8".repeat(1 + (frame % 9) as usize);
        FieldGroup::new("grid").show(ui, |ui| {
            let _ = Field::new(format!("Position {wide}:")).show(ui, |ui, _| TextField::new("position", "Position", "0").show(ui));
            let _ = Field::new("Size:").show(ui, |ui, _| TextField::new("size", "Size", "100").show(ui));
        });
        let readout = format!("X {wide}");
        let _ = StatusBar::new("status").label("Status").show(ui, |slots| {
            slots.leading(|ui| {
                let _ = StatusBar::info(ui, "Ready");
            });
            slots.trailing(move |ui| {
                let _ = StatusBar::info(ui, &readout);
            });
        });
    }
}

fn main() -> std::io::Result<()> {
    let report = run_all(&Legacy);
    print_table(&report);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("baseline/legacy.json");
    write_json(&report, &path)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
