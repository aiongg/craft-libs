use super::layout::*;
use crate::behavior::StackKey;
use egui::{Key, Modifiers, Rect, os::OperatingSystem, pos2};

const STRIP: Rect = Rect { min: pos2(100.0, 10.0), max: pos2(500.0, 36.0) };

fn plan(kind: TabKind, rtl: bool, natural: &[f32], selected: usize, leading: f32, trailing: f32) -> StripPlan {
    StripPlan::new(&StripPlanInput {
        kind,
        rect: STRIP,
        rtl,
        leading,
        trailing,
        natural,
        selected: Some(selected),
        min_width: 40.0,
        overflow_width: 20.0,
    })
}

#[test]
fn both_fit_modes_keep_the_selected_tab() {
    let natural = [120.0; 8];
    for kind in [TabKind::Document, TabKind::Panel] {
        for selected in 0..natural.len() {
            let p = plan(kind, false, &natural, selected, 30.0, 10.0);
            assert!(p.shown.iter().any(|t| t.index == selected), "{kind:?} {selected}");
            let mut all: Vec<usize> = p.shown.iter().map(|t| t.index).chain(p.hidden.iter().copied()).collect();
            all.sort_unstable();
            assert_eq!(all, (0..natural.len()).collect::<Vec<_>>());
        }
    }
}

#[test]
fn documents_pin_the_button_to_the_end_and_panels_follow_the_tabs() {
    let natural = [120.0; 6];
    let doc = plan(TabKind::Document, false, &natural, 0, 0.0, 30.0);
    let button = doc.overflow.unwrap();
    assert_eq!(button.right(), STRIP.right() - 30.0);
    assert!(doc.shown.last().unwrap().rect.right() < button.left());
    let panel = plan(TabKind::Panel, false, &[120.0; 12], 0, 0.0, 30.0);
    assert_eq!(panel.overflow.unwrap().left(), panel.shown.last().unwrap().rect.right());
    assert!(plan(TabKind::Document, false, &[60.0, 60.0], 0, 0.0, 0.0).overflow.is_none());
}

#[test]
fn panels_shrink_before_they_overflow_and_documents_do_not() {
    let natural = [150.0, 150.0, 150.0];
    let panel = plan(TabKind::Panel, false, &natural, 0, 0.0, 0.0);
    assert!(panel.hidden.is_empty() && panel.shown.iter().all(|t| t.rect.width() < 150.0));
    let doc = plan(TabKind::Document, false, &natural, 0, 0.0, 0.0);
    assert_eq!(doc.hidden, [2]);
}

#[test]
fn the_host_areas_take_both_ends_and_rtl_mirrors() {
    let natural = [100.0, 100.0];
    let ltr = plan(TabKind::Document, false, &natural, 0, 30.0, 40.0);
    assert_eq!((ltr.leading.left(), ltr.leading.right()), (100.0, 130.0));
    assert_eq!((ltr.trailing.left(), ltr.trailing.right()), (460.0, 500.0));
    assert_eq!(ltr.shown[0].rect.left(), 130.0);
    let rtl = plan(TabKind::Document, true, &natural, 0, 30.0, 40.0);
    assert_eq!((rtl.leading.left(), rtl.leading.right()), (470.0, 500.0));
    assert_eq!((rtl.trailing.left(), rtl.trailing.right()), (100.0, 140.0));
    assert_eq!(rtl.shown[0].rect.right(), 470.0);
    assert!(rtl.shown[1].rect.right() <= rtl.shown[0].rect.left());
    let overflowing = plan(TabKind::Document, true, &[150.0; 4], 0, 30.0, 40.0);
    assert_eq!(overflowing.overflow.unwrap().left(), 140.0);
}

#[test]
fn the_close_side_follows_the_platform_and_the_direction() {
    assert_eq!(close_side(OperatingSystem::Mac, false), CloseSide::Left);
    assert_eq!(close_side(OperatingSystem::Mac, true), CloseSide::Right);
    assert_eq!(close_side(OperatingSystem::Windows, false), CloseSide::Right);
    assert_eq!(close_side(OperatingSystem::Nix, true), CloseSide::Left);
}

#[test]
fn slots_are_host_order_indices_over_shown_and_hidden_tabs() {
    // Tab 3 is selected and shown in the place of a hidden tab.
    let p = plan(TabKind::Document, false, &[100.0; 6], 4, 0.0, 0.0);
    let order: Vec<usize> = p.shown.iter().map(|t| t.index).collect();
    assert!(!p.hidden.is_empty() && order.contains(&4));
    let first = p.shown[0].rect;
    assert_eq!(p.slot_at(false, first.left_top()), 0);
    assert_eq!(p.slot_at(false, pos2(first.center().x + 1.0, 20.0)), p.shown[1].index);
    let last = p.shown.last().unwrap();
    assert_eq!(p.slot_at(false, pos2(STRIP.right(), 20.0)), last.index + 1);
    assert_eq!(plan(TabKind::Document, false, &[], 0, 0.0, 0.0).slot_at(false, pos2(200.0, 20.0)), 0);
}

#[test]
fn slots_and_tab_lookup_in_rtl() {
    let p = plan(TabKind::Document, true, &[100.0; 3], 0, 0.0, 0.0);
    // Tab 0 is rightmost: a pointer at the strip's right edge is before tab 0.
    assert_eq!(p.slot_at(true, pos2(STRIP.right(), 20.0)), 0);
    assert_eq!(p.slot_at(true, pos2(STRIP.left(), 20.0)), 3);
    assert_eq!(p.slot_at(true, pos2(p.shown[0].rect.center().x - 1.0, 20.0)), 1);
    assert_eq!(p.tab_at(pos2(p.shown[1].rect.center().x, 20.0)), Some(1));
    assert_eq!(p.tab_at(pos2(STRIP.left(), 200.0)), None);
}

#[test]
fn the_band_is_the_strip_grown_and_ignored_without_tear_off() {
    let inside = pos2(300.0, STRIP.top() - 24.0);
    assert_eq!(drag_zone(STRIP, inside, true, 24.0), DragZone::Band);
    assert_eq!(drag_zone(STRIP, pos2(300.0, STRIP.top() - 25.0), true, 24.0), DragZone::Outside);
    assert_eq!(drag_zone(STRIP, pos2(STRIP.right() + 25.0, 20.0), true, 24.0), DragZone::Outside);
    assert_eq!(drag_zone(STRIP, pos2(900.0, 900.0), false, 24.0), DragZone::Band);
}

#[test]
fn the_reorder_keys_point_the_way_the_tab_moves() {
    let ctrl = Modifiers::CTRL;
    let shift = Modifiers { shift: true, ..Modifiers::CTRL };
    assert_eq!(reorder_key(Key::CloseBracket, ctrl, false), Some(StackKey::TowardEnd));
    assert_eq!(reorder_key(Key::OpenBracket, ctrl, false), Some(StackKey::TowardStart));
    assert_eq!(reorder_key(Key::CloseBracket, shift, false), Some(StackKey::ToEnd));
    assert_eq!(reorder_key(Key::OpenBracket, shift, false), Some(StackKey::ToStart));
    assert_eq!(reorder_key(Key::CloseBracket, Modifiers::MAC_CMD, true), Some(StackKey::TowardEnd));
    assert_eq!(reorder_key(Key::CloseBracket, Modifiers::NONE, false), None);
    assert_eq!(strip_key(strip_key(StackKey::ToStart)), StackKey::ToStart);
}

#[test]
fn arrows_follow_the_reading_direction_and_home_end_stay_on_the_tabs() {
    let none = Modifiers::NONE;
    assert_eq!(focus_move(Key::ArrowRight, none, false), Some(FocusMove::Next));
    assert_eq!(focus_move(Key::ArrowRight, none, true), Some(FocusMove::Previous));
    assert_eq!(focus_move(Key::ArrowLeft, none, true), Some(FocusMove::Next));
    assert_eq!(focus_move(Key::ArrowRight, Modifiers::SHIFT, false), None);
    assert_eq!(focus_move(Key::Home, none, true), Some(FocusMove::First));
    assert_eq!(focus_move(Key::Tab, none, false), None);
}

#[test]
fn focus_walks_the_tabs_then_the_overflow_button_without_wrapping() {
    use FocusMove::*;
    use FocusSpot::*;
    assert_eq!(step_focus(Tab(0), Next, 3, true), Some(Tab(1)));
    assert_eq!(step_focus(Tab(2), Next, 3, true), Some(Overflow));
    assert_eq!(step_focus(Tab(2), Next, 3, false), None);
    assert_eq!(step_focus(Overflow, Next, 3, true), None);
    assert_eq!(step_focus(Overflow, Previous, 3, true), Some(Tab(2)));
    assert_eq!(step_focus(Tab(0), Previous, 3, true), None);
    assert_eq!(step_focus(Overflow, Last, 3, true), Some(Tab(2)));
    assert_eq!(step_focus(Tab(1), First, 3, true), Some(Tab(0)));
    assert_eq!(step_focus(Tab(0), First, 3, true), None);
    assert_eq!(step_focus(Overflow, Next, 0, true), None);
}

#[test]
fn a_tab_moved_into_the_overflow_leaves_the_focus_on_the_button() {
    let p = plan(TabKind::Document, false, &[150.0; 6], 0, 0.0, 0.0);
    let hidden = p.hidden[0];
    assert_eq!(focus_spot(&p, 0), Some(FocusSpot::Tab(0)));
    assert_eq!(focus_spot(&p, hidden), Some(FocusSpot::Overflow));
    assert_eq!(focus_spot(&plan(TabKind::Document, false, &[50.0], 0, 0.0, 0.0), 9), None);
}

#[test]
fn kind_defaults() {
    assert!(TabKind::Document.shows_close() && !TabKind::Panel.shows_close());
    assert!(TabKind::Document.middle_click_closes() && !TabKind::Panel.middle_click_closes());
    assert!(TabKind::Document.compact_dividers() && !TabKind::Panel.compact_dividers());
    assert_eq!(TabKind::Document.overflow_menu(), OverflowMenu::AllTabs);
    assert_eq!(TabKind::Panel.overflow_menu(), OverflowMenu::HiddenOnly);
}
