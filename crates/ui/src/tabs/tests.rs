use super::*;

fn total(fit: &TabFit) -> f32 {
    fit.shown.iter().map(|&(_, width)| width).sum::<f32>() + fit.overflow_width
}

#[test]
fn natural_widths_and_common_cap_preserve_tab_order() {
    let f = fit(&[60.0, 70.0, 80.0], 0, 400.0, 40.0, 18.0);
    assert_eq!(f.shown, [(0, 60.0), (1, 70.0), (2, 80.0)]);
    assert!(f.overflow.is_empty());
    assert_eq!(f.overflow_width, 0.0);
    let f = fit(&[50.0, 70.0, 120.0], 0, 200.0, 40.0, 18.0);
    assert!(f.overflow.is_empty());
    assert_eq!(f.shown[0], (0, 50.0));
    assert_eq!(f.shown[1], (1, 70.0));
    assert!((f.shown[2].1 - 80.0).abs() < 0.01);
    assert!((total(&f) - 200.0).abs() < 0.01);
}

#[test]
fn overflow_retains_selection_and_partitions_all_tabs() {
    let natural = [80.0, 90.0, 85.0, 95.0];
    for selected in [0, 1, 2, 3, usize::MAX] {
        for available in [0.0, 1.0, 5.0, 18.0, 19.0, 55.0, 120.0, 200.0, 500.0] {
            let f = fit(&natural, selected, available, 40.0, 18.0);
            let selected = selected.min(natural.len() - 1);
            assert!(f.shown.iter().any(|&(i, _)| i == selected), "{selected} at {available}: {f:?}");
            assert!(!f.overflow.contains(&selected));
            assert!(f.shown.windows(2).all(|w| w[0].0 < w[1].0));
            assert!(f.overflow.windows(2).all(|w| w[0] < w[1]));
            let mut all: Vec<usize> = f.shown.iter().map(|&(i, _)| i).chain(f.overflow.iter().copied()).collect();
            all.sort_unstable();
            assert_eq!(all, [0, 1, 2, 3]);
            assert!(total(&f) <= available + 0.001, "{available}: {f:?}");
        }
    }
}

#[test]
fn one_tab_never_reserves_an_unused_overflow_button() {
    let f = fit(&[80.0], 99, 10.0, 40.0, 18.0);
    assert_eq!(f.shown, [(0, 10.0)]);
    assert!(f.overflow.is_empty());
    assert_eq!(f.overflow_width, 0.0);
}

#[test]
fn malformed_dimensions_remain_finite_and_within_the_strip() {
    for available in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0, 0.01, 10.0, 20.0, 1e20, f32::MAX] {
        for minimum in [f32::NAN, f32::INFINITY, -5.0, 0.0, 40.0, f32::MAX] {
            for overflow in [f32::NAN, f32::INFINITY, -5.0, 0.0, 18.0, f32::MAX] {
                for natural in [&[][..], &[f32::NAN, 50.0][..], &[f32::MAX, -5.0, 30.0, f32::INFINITY][..]] {
                    let f = fit(natural, usize::MAX, available, minimum, overflow);
                    assert!(f.shown.iter().all(|&(_, w)| w.is_finite() && w >= 0.0), "{f:?}");
                    assert!(f.overflow_width.is_finite() && f.overflow_width >= 0.0);
                    assert!(total(&f) <= extent(available) + 0.125, "{available} {minimum} {overflow}: {f:?}");
                    assert_eq!(f.shown.len() + f.overflow.len(), natural.len());
                }
            }
        }
    }
}

#[test]
fn many_tabs_keep_the_last_selected_without_quadratic_membership_scans() {
    let widths = vec![60.0; 10_000];
    let f = fit(&widths, widths.len() - 1, 240.0, 40.0, 18.0);
    assert!(f.shown.iter().any(|&(i, w)| i == widths.len() - 1 && w > 0.0));
    assert_eq!(f.shown.len() + f.overflow.len(), widths.len());
    assert!(total(&f) <= 240.001);
}

#[test]
fn whole_tabs_keep_their_width_and_the_selected_tab_replaces_the_last_that_fits() {
    let natural = [60.0, 70.0, 80.0, 50.0, 90.0, 40.0];
    // 200 points: 22 for the button leaves 178.
    let f = fit_whole(&natural, 0, 200.0, 22.0);
    assert_eq!(f.shown, [(0, 60.0), (1, 70.0)]);
    assert_eq!(f.overflow, [2, 3, 4, 5]);
    assert_eq!(f.overflow_width, 22.0);
    // The last tab, selected, takes the place of tab 1 and keeps its order.
    let f = fit_whole(&natural, 5, 200.0, 22.0);
    assert_eq!(f.shown, [(0, 60.0), (1, 70.0), (5, 40.0)]);
    assert_eq!(f.overflow, [2, 3, 4]);
    let f = fit_whole(&natural, 4, 200.0, 22.0);
    assert_eq!(f.shown, [(0, 60.0), (4, 90.0)]);
    assert_eq!(f.overflow, [1, 2, 3, 5]);
    // Everything fits: no button.
    let f = fit_whole(&natural, 4, 390.0, 22.0);
    assert_eq!(f.shown.len(), 6);
    assert!(f.overflow.is_empty() && f.overflow_width == 0.0);
}

#[test]
fn only_a_selected_tab_wider_than_the_strip_is_narrowed() {
    let f = fit_whole(&[60.0, 300.0, 70.0], 1, 200.0, 22.0);
    assert_eq!(f.shown, [(1, 178.0)]);
    assert_eq!(f.overflow, [0, 2]);
    for selected in [0, 1, 2, usize::MAX] {
        for available in [0.0, 1.0, 21.0, 22.0, 100.0, 200.0, 500.0] {
            let natural = [60.0, 300.0, 70.0];
            let f = fit_whole(&natural, selected, available, 22.0);
            let selected = selected.min(2);
            assert!(f.shown.iter().all(|&(i, w)| i == selected || w == natural[i]), "{selected} at {available}: {f:?}");
            assert!(f.shown.iter().any(|&(i, _)| i == selected));
            assert_eq!(f.shown.len() + f.overflow.len(), 3);
            assert!(total(&f) <= available + 0.001, "{available}: {f:?}");
        }
    }
    assert_eq!(fit_whole(&[80.0], 0, 10.0, 22.0).shown, [(0, 10.0)]);
    let f = fit_whole(&[f32::NAN, f32::INFINITY, -5.0, 30.0], usize::MAX, f32::NAN, f32::MAX);
    assert!(f.shown.iter().all(|&(_, w)| w.is_finite() && w >= 0.0) && total(&f) == 0.0);
}
