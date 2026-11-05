//! Tests of Filter Gallery's catalog and stack.

use craft_theme::Srgba;
use craft_ui::dialog::{DialogFields, FieldValue, LocalHistory, Param, ParamKind, Step};

use super::catalog::{FilterCatalog, FilterCategory, GalleryFilter};
use super::session::{EffectChange, FilterGallerySession};
use super::{FilterGalleryOpen, FilterGalleryWorkspace};

struct Catalog;

impl Catalog {
    fn all() -> Vec<GalleryFilter> {
        let filter = |key: &str, params: Vec<Param>, uses_colors: bool| GalleryFilter {
            key: key.to_owned(),
            label: key.to_owned(),
            category: "artistic".to_owned(),
            params,
            uses_colors,
        };
        vec![
            filter("cutout", Param::parse_list(r#"{"levels":2..8=6,"mode":"soft|hard","edges":bool}"#), false),
            filter("sponge", Param::parse_list(r#"{"size":0..40=2}"#), true),
            filter(
                "neon",
                vec![
                    Param::new("glowSize", "Glow Size", ParamKind::Range { min: -24.0, max: 24.0, default: 5.0, scale: Default::default() }),
                    Param::new("glowColor", "Glow Color", ParamKind::Color { default: Some([1.0, 0.0, 0.5]) }),
                    Param::new("note", "Note", ParamKind::Text { default: "x".to_owned() }),
                ],
                false,
            ),
        ]
    }
}

impl FilterCatalog for Catalog {
    fn categories(&self) -> Vec<FilterCategory> {
        vec![FilterCategory {
            key: "artistic".to_owned(),
            label: "Artistic".to_owned(),
            filters: Self::all().into_iter().map(|filter| filter.key).collect(),
        }]
    }

    fn filter(&self, key: &str) -> Option<GalleryFilter> {
        Self::all().into_iter().find(|filter| filter.key == key)
    }
}

fn change(session: &mut FilterGallerySession, history: &mut LocalHistory<FilterGallerySession>, selected: &mut usize, change: EffectChange) -> bool {
    let changed = session.apply(change, &Catalog, selected);
    history.settle(session, false);
    changed
}

fn filters(session: &FilterGallerySession) -> Vec<&str> {
    session.effects.iter().map(|effect| effect.filter.as_str()).collect()
}

// Picking a filter for the selected layer resets its fields to the filter's defaults and keeps its
// eye; the same filter changes nothing.
#[test]
fn gallery_thumbnail_click_sets_the_selected_effects_filter() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut selected = 0;
    let fields = DialogFields::from_pairs([("levels", 3.0)]);
    assert!(session.apply(EffectChange::Fields { index: 0, fields }, &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Visible { index: 0, visible: false }, &Catalog, &mut selected));
    assert!(!session.apply(EffectChange::SetFilter { index: 0, filter: "cutout".to_owned() }, &Catalog, &mut selected));
    assert_eq!(session.effects[0].fields.get("levels"), Some(&FieldValue::Number(3.0)));
    assert!(session.apply(EffectChange::SetFilter { index: 0, filter: "sponge".to_owned() }, &Catalog, &mut selected));
    assert_eq!(session.effects[0].filter, "sponge");
    assert_eq!(session.effects[0].fields, Catalog.filter("sponge").map(|found| found.defaults()).unwrap_or_default());
    assert!(!session.effects[0].visible);
    assert!(!session.apply(EffectChange::SetFilter { index: 0, filter: "gone".to_owned() }, &Catalog, &mut selected));
}

// New copies the selected layer above it and selects it; Delete keeps the last layer; Move and the
// eye change one layer; the top row is applied last.
#[test]
fn gallery_stack_new_delete_reorder_and_eye() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut selected = 0;
    assert!(session.apply(EffectChange::Add, &Catalog, &mut selected));
    assert_eq!(selected, 1);
    assert!(session.apply(EffectChange::SetFilter { index: 1, filter: "sponge".to_owned() }, &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Add, &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["cutout", "sponge", "sponge"], 2));
    assert!(session.apply(EffectChange::Move { from: 2, to: 0 }, &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["sponge", "cutout", "sponge"], 0));
    assert!(!session.apply(EffectChange::Move { from: 0, to: 0 }, &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Visible { index: 1, visible: false }, &Catalog, &mut selected));
    let eyes: Vec<bool> = session.effects.iter().map(|effect| effect.visible).collect();
    assert_eq!(eyes, [true, false, true]);
    selected = 2;
    assert!(session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert!(session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert_eq!((filters(&session), selected), (vec!["sponge"], 0));
    assert!(!session.apply(EffectChange::Delete, &Catalog, &mut selected));
    assert_eq!(session.effects.len(), 1);
}

// `GalleryFilter::defaults` takes each Range's default, each Choice's default or first option,
// each Bool, and the colour.
#[test]
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
#[test]
fn gallery_undo_restores_the_stack_and_clamps_the_selection() {
    let mut session = FilterGallerySession::new("cutout", &Catalog);
    let mut history = LocalHistory::new(&session);
    let mut selected = 0;
    change(&mut session, &mut history, &mut selected, EffectChange::Add);
    change(&mut session, &mut history, &mut selected, EffectChange::SetFilter { index: 1, filter: "sponge".to_owned() });
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

// The workspace's `step` clamps the selection to the layers that remain and leaves the `Select` event
// to the pass's comparison with the selection the app knows: nothing is queued twice.
#[test]
fn gallery_workspace_step_clamps_the_selection_without_queuing_select() {
    let session = FilterGallerySession::new("cutout", &Catalog);
    let mut workspace = FilterGalleryWorkspace::new(FilterGalleryOpen::new([10, 10], "Layer", session));
    assert!(workspace.session.apply(EffectChange::Add, &Catalog, &mut workspace.selected));
    workspace.history.settle(&workspace.session, false);
    // The pass that made the layer reported its selection.
    workspace.reported = workspace.selected;
    assert_eq!(workspace.selected, 1);

    assert!(workspace.step(Step::Undo));
    assert_eq!(workspace.session.effects.len(), 1);
    assert_eq!(workspace.selected, 0, "the selection moves onto the layer that remains");
    assert!(workspace.queued.is_empty(), "the next pass reports `Select` from its comparison, once");
    assert_ne!(workspace.selected, workspace.reported);
    assert!(!workspace.step(Step::Undo), "nothing left to undo");
}

// The app answers a color request by key and `Srgba`: the selected effect's field takes it as one
// undo step.
#[test]
fn gallery_set_color_writes_the_selected_effects_field_as_one_undo_step() {
    let session = FilterGallerySession::new("neon", &Catalog);
    let mut workspace = FilterGalleryWorkspace::new(FilterGalleryOpen::new([10, 10], "Layer", session));
    workspace.set_color("glowColor", Srgba::from_rgb_array([0.2, 0.7, 0.9]));
    assert_eq!(workspace.session.effects[0].fields.get("glowColor"), Some(&FieldValue::Color([0.2, 0.7, 0.9])));
    assert!(matches!(workspace.queued.as_slice(), [super::FilterGalleryEvent::Stack(EffectChange::Fields { index: 0, .. })]));
    assert!(workspace.step(Step::Undo));
    assert_eq!(workspace.session.effects[0].fields.get("glowColor"), Some(&FieldValue::Color([1.0, 0.0, 0.5])));
}
