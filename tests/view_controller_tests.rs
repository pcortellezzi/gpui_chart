use gpui_chart::view_controller::ViewController;
use gpui_chart::data_types::AxisRange;

#[test]
fn test_pan_axis_x() {
    let mut range = AxisRange::new(0.0, 100.0);
    // Déplacement de 10 pixels vers la droite sur 100 pixels total
    // ratio = 100 / 100 = 1.0. delta_data = -10.0
    ViewController::pan_axis(&mut range, 10.0, 100.0, false);
    assert_eq!(range.min, -10.0);
    assert_eq!(range.max, 90.0);
}

#[test]
fn test_pan_axis_y() {
    let mut range = AxisRange::new(0.0, 100.0);
    // Déplacement de 10 pixels vers le bas (positif en GPUI)
    // ratio = 100 / 100 = 1.0. delta_data = 10.0
    ViewController::pan_axis(&mut range, 10.0, 100.0, true);
    assert_eq!(range.min, 10.0);
    assert_eq!(range.max, 110.0);
}

#[test]
fn test_zoom_axis_at() {
    let mut range = AxisRange::new(0.0, 100.0);
    // Zoom x2 (factor 0.5) au centre (0.5)
    ViewController::zoom_axis_at(&mut range, 0.5, 0.5);
    assert_eq!(range.min, 25.0);
    assert_eq!(range.max, 75.0);
}

#[test]
fn test_resize_panes() {
    let mut weights = vec![1.0, 1.0];
    // Descendre le splitter de 100px sur 400px total -> +0.5 de poids
    ViewController::resize_panes(&mut weights, 0, 100.0, 400.0);
    assert_eq!(weights[0], 1.5);
    assert_eq!(weights[1], 0.5);
}

#[test]
fn test_resize_panes_limit() {
    let mut weights = vec![1.0, 1.0];
    // Essayer de réduire le second panneau en dessous du minimum
    ViewController::resize_panes(&mut weights, 0, 1000.0, 400.0);
    assert!(weights[1] >= 0.05);
    // Somme des poids doit rester constante
    assert!((weights[0] + weights[1] - 2.0).abs() < 1e-6);
}

#[test]
fn test_compute_auto_fit() {
    let (min, max) = ViewController::compute_auto_fit(10.0, 20.0, 0.1);
    assert_eq!(min, 9.0);
    assert_eq!(max, 21.0);
}

#[test]
fn test_move_to_center() {
    let mut range = AxisRange::new(40.0, 60.0); // span 20
    ViewController::move_to_center(&mut range, 100.0, None);
    assert_eq!(range.min, 90.0);
    assert_eq!(range.max, 110.0);
}

#[test]
fn test_move_to_center_clamped() {
    let mut range = AxisRange::new(40.0, 60.0); // span 20
    // Move to 10, but clamp to [0, 100]
    ViewController::move_to_center(&mut range, 5.0, Some((0.0, 100.0)));
    assert_eq!(range.min, 0.0);
    assert_eq!(range.max, 20.0);
}
