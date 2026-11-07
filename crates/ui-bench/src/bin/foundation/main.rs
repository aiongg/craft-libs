//! Runs the spec 22.7 scenarios on the foundation's `craft-ui` and writes `baseline/foundation.json`.
//!
//! A scenario whose component the families have not delivered yet runs on the smallest stand-in in `kit`,
//! marked `F8-MERGE: <family>`; the combiner replaces the stand-in with the real component.

mod kit;

use std::path::PathBuf;
use std::sync::Arc;

use craft_behavior::{ColumnKey, Selection};
use craft_host::target::ShellInfo;
use craft_ui::contract::{
    Bind, Collapse, ContainerProps, Mount, Overlay, Props as _, SelectMode, Show as _, ShowCollection as _, ShowContainer as _, Sizing,
};
use craft_ui::layout::ColumnGroup;
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::frame::{FrameInput, set_input};
use craft_ui::runtime::{InstallSpec, install};
use craft_ui::text::{NumberField, TextField};
use craft_ui_bench::{CountingAlloc, Library, Scene, SceneBuild, gate_failures, print_table, read_json, run_all, write_json};
use craft_window::system::NoWindows;
use egui::{Context, Event, Modifiers, MouseWheelUnit, Pos2, Ui, Vec2, vec2};
use kit::{BenchCanvas, BenchList, Block, Count, Panel, ROW_H, Row, RowKind};

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// The app's root env, built once: the runtime caches the resolved tokens by the identity of its sizes.
struct Foundation {
    root: RootEnv,
}

impl Library for Foundation {
    fn name(&self) -> &'static str {
        "foundation"
    }

    fn install(&self, ctx: &Context) {
        if let Err(error) = install(ctx, InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows))) {
            panic!("the foundation runtime did not install: {error:?}");
        }
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.all_styles_mut(|style| style.scroll_animation = egui::style::ScrollAnimation::none());
    }

    fn before_frame(&self, ctx: &Context) {
        set_input(ctx, FrameInput::new(self.root.clone()));
    }

    fn build(&self, name: &str) -> SceneBuild {
        let (scene, equivalent): (Box<dyn Scene>, &'static str) = match name {
            "inspector-500-fields" => (
                Box::new(InspectorScene::new()),
                // F8-MERGE: family (Field row): the field's own side label stands in for the `Field` row.
                "500 rows alternating NumberField and TextField, each with its side label (no Field row component yet)",
            ),
            "virtual-list-10000" => (Box::new(ListScene::new(Selected::None)), "BenchList over 10,000 rows, one selected row"),
            "sheet-50x200" => (Box::new(SheetScene), "BenchList of 200 rows, each with 50 cell parts that paint their text"),
            // F8-MERGE: canvas: the canvas component and its declared controls.
            "canvas-50-controls" => (Box::new(CanvasScene), "a leaf with 50 control parts painted as knobs"),
            // F8-MERGE: menu: the menu component.
            "menu-40-rows" => (Box::new(MenuScene), "a container on the menu layer, open from the first frame, over a BenchList of 40 labelled rows"),
            "text-field-100kb" => (Box::new(TextScene::new()), "TextField::lines(30) bound with Bind::bound over a 100 KB String"),
            "list-10000-selected" => (Box::new(ListScene::new(Selected::All)), "BenchList over 10,000 rows, all 10,000 keys in the Selection"),
            "collapse-in-content-sized-panel" => {
                (Box::new(CollapseScene::new()), "a content-sized, animated-collapse container whose open state the scene toggles")
            }
            "auto-height-list-scroll-10000" => {
                (Box::new(AutoHeightScene::new()), "BenchList::auto_height over 10,000 rows of one to four row heights, mouse wheel every frame")
            }
            "second-viewport" => (Box::new(ViewportScene), "a container with Mount::viewport (the fallback layer, as under an embedded backend)"),
            "readout-and-content-column" => (
                Box::new(ReadoutScene),
                // F8-MERGE: layout: the grid's `Content` column and the status bar's end slot.
                "two labelled TextFields in a ColumnGroup whose label text changes width every frame, and an end-aligned readout block",
            ),
            _ => return SceneBuild::Missing { reason: "the scenario list names a scenario this binary does not know" },
        };
        SceneBuild::Scene { scene, equivalent }
    }
}

struct InspectorScene {
    values: Vec<Option<f64>>,
    texts: Vec<String>,
}

impl InspectorScene {
    fn new() -> Self {
        Self { values: (0..500).map(|i| Some(f64::from(i))).collect(), texts: (0..500).map(|i| format!("Value {i}")).collect() }
    }
}

impl Scene for InspectorScene {
    fn size(&self) -> Vec2 {
        vec2(360.0, 900.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let Self { values, texts } = self;
        for (i, (value, text)) in values.iter_mut().zip(texts.iter_mut()).enumerate() {
            let label = format!("Property {i}:");
            if i % 2 == 0 {
                let _ = NumberField::new(label, Bind::bound(value)).id_salt(i).show(ui);
            } else {
                let _ = TextField::new(label, Bind::bound(text)).id_salt(i).show(ui);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Selected {
    None,
    All,
}

struct ListScene {
    source: Count,
    selection: Selection<usize>,
}

impl ListScene {
    fn new(selected: Selected) -> Self {
        let selection = match selected {
            Selected::None => Selection::new(vec![0], Some(0), Some(0)),
            Selected::All => Selection::new((0..10_000).collect(), Some(0), Some(0)),
        };
        Self { source: Count(10_000), selection }
    }
}

impl Scene for ListScene {
    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let list = BenchList::new(SelectMode::Multi).selection(Bind::bound(&mut self.selection));
        let _ = list.id_salt("list").show_items(ui, &self.source, &mut |&key, _| Row::new(RowKind::Plain { key }));
    }
}

const SHEET_COLUMNS: usize = 50;
const SHEET_ROWS: usize = 200;

struct SheetScene;

impl Scene for SheetScene {
    fn size(&self) -> Vec2 {
        vec2(SHEET_COLUMNS as f32 * kit::CELL_W + 40.0, SHEET_ROWS as f32 * ROW_H + 100.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let rows = Count(SHEET_ROWS);
        let _ = BenchList::new(SelectMode::Single)
            .id_salt("sheet")
            .show_items(ui, &rows, &mut |&row, _| Row::new(RowKind::Cells { row, cells: SHEET_COLUMNS }));
    }
}

struct CanvasScene;

impl Scene for CanvasScene {
    fn size(&self) -> Vec2 {
        vec2(900.0, 700.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let _ = BenchCanvas::new(vec2(840.0, 640.0), 50).id_salt("canvas").show(ui);
    }
}

struct MenuScene;

impl Scene for MenuScene {
    fn size(&self) -> Vec2 {
        vec2(800.0, 1400.0)
    }

    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        let rows = Count(40);
        let mut open = true;
        let props = ContainerProps::new().mount(Mount::Layer(Overlay::menu())).sizing(Sizing::Content);
        let _ = Panel::new(props).headerless().bind_open(Bind::bound(&mut open)).id_salt("menu").show(ui, |ui, _| {
            let _ = BenchList::new(SelectMode::Single).id_salt("rows").show_items(ui, &rows, &mut |&key, _| Row::new(RowKind::Plain { key }));
        });
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
        let _ = TextField::new("Notes", Bind::bound(&mut self.text)).lines(30).id_salt("notes").show(ui);
    }
}

/// Frames between two toggles; the animation runs longer, so one is always under way.
const COLLAPSE_TOGGLE_FRAMES: u64 = 30;
const PANEL_ROWS: usize = 12;
const PANEL_ROW: Vec2 = vec2(100.0, 16.0);

struct CollapseScene {
    open: bool,
}

impl CollapseScene {
    fn new() -> Self {
        Self { open: false }
    }
}

impl Scene for CollapseScene {
    fn show(&mut self, ui: &mut Ui, frame: u64) {
        if frame.is_multiple_of(COLLAPSE_TOGGLE_FRAMES) {
            self.open = !self.open;
        }
        let props = ContainerProps::new().collapse(Collapse::new().initially_open(false)).sizing(Sizing::Content);
        let _ = Panel::new(props).bind_open(Bind::bound(&mut self.open)).id_salt("panel").show(ui, |ui, _| {
            for i in 0..PANEL_ROWS {
                let _ = Block::new(PANEL_ROW).id_salt(i).show(ui);
            }
        });
    }
}

struct AutoHeightScene {
    source: Count,
    selection: Selection<usize>,
}

impl AutoHeightScene {
    fn new() -> Self {
        Self { source: Count(10_000), selection: Selection::new(vec![0], Some(0), Some(0)) }
    }
}

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
        let list = BenchList::new(SelectMode::Single).auto_height().selection(Bind::bound(&mut self.selection));
        let _ = list.id_salt("auto").show_items(ui, &self.source, &mut |&key, _| Row::new(RowKind::Auto { key }));
    }
}

struct ViewportScene;

impl Scene for ViewportScene {
    fn show(&mut self, ui: &mut Ui, _frame: u64) {
        ui.label("Main window");
        let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
        let _ = Panel::new(props).id_salt("second").show(ui, |ui, _| {
            for i in 0..20 {
                let _ = Block::new(PANEL_ROW).id_salt(i).show(ui);
            }
        });
    }
}

const POSITION: ColumnKey = ColumnKey::new("position");

struct ReadoutScene;

impl Scene for ReadoutScene {
    fn show(&mut self, ui: &mut Ui, frame: u64) {
        // Each frame's text is a different width.
        let digits = 1 + (frame % 9) as usize;
        let wide = "8".repeat(digits);
        let mut position = String::from("0");
        let mut size = String::from("100");
        ColumnGroup::scope(ui, "grid", |ui, group| {
            let _ = group.width(ui, &POSITION, digits as f32 * 8.0);
            let _ = TextField::new(format!("Position {wide}:"), Bind::bound(&mut position)).id_salt("position").show(ui);
            let _ = TextField::new("Size:", Bind::bound(&mut size)).id_salt("size").show(ui);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let _ = Block::new(vec2(digits as f32 * 8.0, 16.0)).id_salt("readout").show(ui);
        });
    }
}

fn main() -> std::process::ExitCode {
    let report = run_all(&Foundation { root: RootEnv::default() });
    print_table(&report);
    let baseline = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("baseline");
    let outcome = write_json(&report, &baseline.join("foundation.json")).and_then(|()| read_json(&baseline.join("legacy.json")));
    let legacy = match outcome {
        Ok(legacy) => legacy,
        Err(error) => {
            eprintln!("the baselines could not be written or read: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let failures = gate_failures(&report, &legacy);
    for failure in &failures {
        eprintln!("gate: {failure}");
    }
    if failures.is_empty() { std::process::ExitCode::SUCCESS } else { std::process::ExitCode::FAILURE }
}
