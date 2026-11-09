//! Filter Gallery: the stack's session rules, the workspace's undo, and the columns and preview job shown in a
//! full-window dialog.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use craft_behavior::edit_history::Step;
use craft_behavior::{EditEvent, EditHistory, Mixed, PixelSize};
use craft_imaging::ui::filter_gallery::{
    EffectChange, EffectLayer, FilterCatalog, FilterCategory, FilterGalleryEvent, FilterGalleryOpen, FilterGalleryOutput, FilterGallerySession,
    FilterGallerySource, FilterGalleryWorkspace, GalleryFilter, GalleryThumb,
};
use craft_imaging::ui::wide_angle::PreviewDone;
use craft_theme::Srgba;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::contract::{Job, Role};
use craft_ui::patterns::form_dialogs::{DialogFields, FieldValue, Param, ParamKind};
use craft_ui::primitives::Revision;
use craft_ui::primitives::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::testing::relations::equals_token;
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::SliderScale;
use egui::{Color32, ColorImage, Key, Modifiers, Vec2};

const WINDOW: Vec2 = Vec2::new(1100.0, 640.0);
const LAYER: PixelSize = PixelSize::new(400, 300);

struct Catalog;

impl Catalog {
    fn all() -> Vec<GalleryFilter> {
        let filter =
            |key: &str, params: Vec<Param>, uses_colors: bool| GalleryFilter::new(key, key.to_owned(), "artistic", params).uses_colors(uses_colors);
        vec![
            filter("cutout", Param::parse_list(r#"{"levels":2..8=6,"mode":"soft|hard","edges":bool}"#), false),
            filter("sponge", Param::parse_list(r#"{"size":0..40=2}"#), true),
            filter(
                "neon",
                vec![
                    Param::new("glowSize", "Glow Size", ParamKind::range(-24.0, 24.0, 5.0, SliderScale::default())),
                    Param::new("glowColor", "Glow Color", ParamKind::color(Some([1.0, 0.0, 0.5]))),
                    Param::new("note", "Note", ParamKind::text("x")),
                ],
                false,
            ),
        ]
    }
}

impl FilterCatalog for Catalog {
    fn categories(&self) -> Vec<FilterCategory> {
        vec![FilterCategory::new("artistic", "Artistic", Self::all().into_iter().map(|filter| filter.key).collect())]
    }

    fn filter(&self, key: &str) -> Option<GalleryFilter> {
        Self::all().into_iter().find(|filter| filter.key == key)
    }
}

fn filters(session: &FilterGallerySession) -> Vec<&str> {
    session.effects.iter().map(|effect| effect.filter.as_str()).collect()
}

/// Applies `change`, records it as the workspace does, and reports whether the stack changed.
fn change(session: &mut FilterGallerySession, history: &mut EditHistory<FilterGallerySession>, selected: &mut usize, change: EffectChange) -> bool {
    let before = session.clone();
    let changed = session.apply(change, &Catalog, selected);
    history.record(&EditEvent::set(Mixed::Same(before), session.clone(), "Filter Gallery".into(), None));
    changed
}

// Picking a filter for the selected layer resets its fields to the filter's defaults and keeps its eye; the same
// filter changes nothing.
#[trial]
fn gallery_thumbnail_click_sets_the_selected_effects_filter() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut selected = 0;
    let fields = DialogFields::from_pairs([("levels", 3.0)]);
    assert!(session.apply(EffectChange::with_fields(0, fields), &Catalog, &mut selected));
    assert!(session.apply(EffectChange::set_visible(0, false), &Catalog, &mut selected));
    assert!(!session.apply(EffectChange::set_filter(0, "cutout"), &Catalog, &mut selected));
    assert_eq!(session.effects[0].fields.get("levels"), Some(&FieldValue::Number(3.0)));
    assert!(session.apply(EffectChange::set_filter(0, "sponge"), &Catalog, &mut selected));
    assert_eq!(session.effects[0].filter, "sponge");
    assert_eq!(session.effects[0].fields, Catalog.filter("sponge").map(|found| found.defaults()).unwrap_or_default());
    assert!(!session.effects[0].visible);
    assert!(!session.apply(EffectChange::set_filter(0, "gone"), &Catalog, &mut selected));
}

// New copies the selected layer above it and selects it; Delete keeps the last layer; Move and the eye change one
// layer; the top row is applied last.
#[trial]
fn gallery_stack_new_delete_reorder_and_eye() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut selected = 0;
    assert!(session.apply(EffectChange::Add, &Catalog, &mut selected));
    assert_eq!(selected, 1);
    assert!(session.apply(EffectChange::set_filter(1, "sponge"), &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Add, &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["cutout", "sponge", "sponge"], 2));
    assert!(session.apply(EffectChange::move_to(2, 0), &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["sponge", "cutout", "sponge"], 0));
    assert!(!session.apply(EffectChange::move_to(0, 0), &Catalog, &mut selected));
    assert!(session.apply(EffectChange::set_visible(1, false), &Catalog, &mut selected));
    let eyes: Vec<bool> = session.effects.iter().map(|effect| effect.visible).collect();
    assert_eq!(eyes, [true, false, true]);
    selected = 2;
    assert!(session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["sponge"], 0));
    assert!(!session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert_eq!(session.effects.len(), 1);
}

// `GalleryFilter::defaults` takes each Range's default, each Choice's default or first option, each Bool, and the
// colour.
#[trial]
fn gallery_settings_come_from_the_catalog_params() {
    let Some(cutout) = Catalog.filter("cutout") else { panic!("cutout is in the catalog") };
    let fields = cutout.defaults();
    assert_eq!(fields.get("levels"), Some(&FieldValue::Number(6.0)));
    assert_eq!(fields.get("mode"), Some(&FieldValue::Text("soft".to_owned())));
    assert_eq!(fields.get("edges"), Some(&FieldValue::Bool(false)));
    let Some(neon) = Catalog.filter("neon") else { panic!("neon is in the catalog") };
    let fields = neon.defaults();
    assert_eq!(fields.get("glowColor"), Some(&FieldValue::Color([1.0, 0.0, 0.5])));
    assert_eq!(fields.get("glowSize"), Some(&FieldValue::Number(5.0)));
    assert_eq!(fields.get("note"), None);
}

// Undo of a Delete gives the layer back and clamps the selection.
#[trial]
fn gallery_undo_restores_the_stack_and_clamps_the_selection() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut history = EditHistory::new(&session);
    let mut selected = 0;
    change(&mut session, &mut history, &mut selected, EffectChange::Add);
    change(&mut session, &mut history, &mut selected, EffectChange::set_filter(1, "sponge"));
    assert!(change(&mut session, &mut history, &mut selected, EffectChange::Delete));
    assert_eq!((filters(&session), selected), (vec!["cutout"], 0));
    assert!(history.step(&mut session, Step::Undo));
    assert_eq!(filters(&session), ["cutout", "sponge"]);
    selected = 5;
    assert!(history.step(&mut session, Step::Undo));
    assert!(history.step(&mut session, Step::Undo));
    selected = selected.min(session.effects.len() - 1);
    assert_eq!((filters(&session), selected), (vec!["cutout"], 0));
}

/// A flat picture the size of the layer, whose render never finishes and counts its requests.
struct Flat {
    renders: Rc<RefCell<Vec<usize>>>,
    done: bool,
}

impl RasterSource for Flat {
    fn size(&self) -> PixelSize {
        LAYER
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_gray(120));
    }
}

impl FilterGallerySource for Flat {
    fn preview(&self) -> &dyn RasterSource {
        self
    }

    fn render(&self, effects: &[EffectLayer], _: PixelRect, _: f64) -> Job<PreviewDone, String> {
        self.renders.borrow_mut().push(effects.len());
        if self.done {
            Job::new(|_, _| Ok(PreviewDone::new(Duration::from_millis(12))))
        } else {
            Job::new(|_, cancel| {
                while !cancel.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(String::new())
            })
        }
    }
}

/// A workspace in a fixture and what its passes reported.
struct Rig {
    fixture: Fixture,
    workspace: Rc<RefCell<FilterGalleryWorkspace>>,
    outputs: Rc<RefCell<Vec<FilterGalleryOutput>>>,
    renders: Rc<RefCell<Vec<usize>>>,
}

fn rig(session: FilterGallerySession, done: bool) -> Rig {
    let workspace = Rc::new(RefCell::new(FilterGalleryWorkspace::new(FilterGalleryOpen::new(LAYER, "Layer 1", session))));
    let outputs = Rc::new(RefCell::new(Vec::new()));
    let renders = Rc::new(RefCell::new(Vec::new()));
    let (shown, sink, seen) = (workspace.clone(), outputs.clone(), renders.clone());
    let source = Flat { renders: seen, done };
    let mut fixture = Fixture::new(move |ui| {
        let thumbs = Thumbnails::<GalleryThumb>::in_context(ui.ctx(), "gallery-test-thumbs", ThumbnailOptions::default());
        let out = shown.borrow_mut().show(ui, &Catalog, &thumbs, &FakeThumbSource::<GalleryThumb>::new(), &source);
        sink.borrow_mut().push(out);
    });
    fixture.room(WINDOW).settle();
    Rig { fixture, workspace, outputs, renders }
}

impl Rig {
    fn events(&self) -> Vec<FilterGalleryEvent> {
        self.outputs.borrow().iter().flat_map(|out| out.events.clone()).collect()
    }
}

// The workspace's `step` clamps the selection to the layers that remain and leaves the `Select` event to the
// pass's comparison with the selection the app knows: it is reported once.
#[trial]
fn gallery_workspace_step_clamps_the_selection_without_queuing_select() {
    let mut rig = rig(FilterGallerySession::new("cutout", &Catalog), true);
    let add = rig.fixture.by_label("New effect layer").expect("the stack offers New");
    rig.fixture.click(&add).settle();
    assert_eq!(rig.workspace.borrow().selected(), 1);
    assert!(rig.events().contains(&FilterGalleryEvent::Select(1)));
    let selects = |rig: &Rig| rig.events().iter().filter(|event| matches!(event, FilterGalleryEvent::Select(0))).count();
    assert_eq!(selects(&rig), 0);

    assert!(rig.workspace.borrow_mut().step(Step::Undo));
    assert_eq!(rig.workspace.borrow().session().effects.len(), 1);
    assert_eq!(rig.workspace.borrow().selected(), 0, "the selection moves onto the layer that remains");
    rig.fixture.settle();
    assert_eq!(selects(&rig), 1, "the next pass reports `Select` from its comparison, once");
    assert!(!rig.workspace.borrow_mut().step(Step::Undo), "nothing left to undo");
}

// The app answers a color request by key and `Srgba`: the selected effect's field takes it as one undo step.
#[trial]
fn gallery_set_color_writes_the_selected_effects_field_as_one_undo_step() {
    let session = FilterGallerySession::new("neon", &Catalog);
    let mut workspace = FilterGalleryWorkspace::new(FilterGalleryOpen::new(LAYER, "Layer", session));
    workspace.set_color("glowColor", Srgba::from_rgb_array([0.2, 0.7, 0.9]));
    assert_eq!(workspace.session().effects[0].fields.get("glowColor"), Some(&FieldValue::Color([0.2, 0.7, 0.9])));
    assert!(workspace.step(Step::Undo));
    assert_eq!(workspace.session().effects[0].fields.get("glowColor"), Some(&FieldValue::Color([1.0, 0.0, 0.5])));
}

// OK and Cancel sit at the top of the settings column, in the platform's order, over the Filter select and the
// stack.
#[trial]
fn gallery_footer_is_at_the_top() {
    let rig = rig(FilterGallerySession::new("cutout", &Catalog), true);
    let rect = |label: &str| rig.fixture.by_label(label).unwrap_or_else(|| panic!("{label} is shown")).rect;
    let (ok, cancel) = (rect("OK"), rect("Cancel"));
    let select = rect("Filter");
    let stack = rect("Effect Layers");
    assert!(ok.bottom() <= select.top() && cancel.bottom() <= select.top(), "{ok:?} {cancel:?} over {select:?}");
    assert!(select.bottom() <= stack.top());
    assert!(equals_token(ok.center().y, cancel.center().y), "one row");
    assert!(ok.right().max(cancel.right()) > WINDOW.x - ok.width(), "the column is at the end edge: {ok:?} {cancel:?}");
}

// Clicking a thumbnail picks its filter; arrows and Enter do the same from the keyboard; each pick is one undo step.
#[trial]
fn gallery_thumbnails_pick_by_pointer_and_keyboard() {
    let mut rig = rig(FilterGallerySession::new("cutout", &Catalog), true);
    let sponge = rig.fixture.by_label("sponge").expect("a tile per filter");
    rig.fixture.click(&sponge).settle();
    assert_eq!(rig.workspace.borrow().session().effects[0].filter, "sponge");
    assert!(rig.events().iter().any(|e| matches!(e, FilterGalleryEvent::Stack(EffectChange::SetFilter { filter, .. }) if filter == "sponge")));

    rig.fixture.key(Key::ArrowRight, Modifiers::NONE).key(Key::Enter, Modifiers::NONE).settle();
    assert_eq!(rig.workspace.borrow().session().effects[0].filter, "neon");
    assert!(rig.workspace.borrow_mut().step(Step::Undo));
    assert_eq!(rig.workspace.borrow().session().effects[0].filter, "sponge");
}

// While a render runs the zoom bar shows its progress; a change replaces the running request with a new one.
#[trial]
fn gallery_preview_shows_progress_and_drops_stale_results() {
    let mut rig = rig(FilterGallerySession::new("cutout", &Catalog), false);
    let asked = rig.renders.borrow().len();
    assert!(asked >= 1, "the first pass asks for a preview");
    assert!(!rig.fixture.by_role(Role::PROGRESS_INDICATOR).is_empty(), "the render shows its progress");

    let neon = rig.fixture.by_label("neon").expect("a tile per filter");
    rig.fixture.click(&neon).settle();
    let renders = rig.renders.borrow().clone();
    assert!(renders.len() > asked, "the change asks again: {renders:?}");
    assert!(!rig.fixture.by_role(Role::PROGRESS_INDICATOR).is_empty(), "the new render is the one shown");
}
