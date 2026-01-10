use crate::data_types::AxisRange;

/// ViewController gère la logique métier des interactions (zoom, pan, redimensionnement)
/// indépendamment de l'infrastructure GPUI pour faciliter les tests.
pub struct ViewController;

impl ViewController {
    /// Calcule et applique un déplacement (pan) sur un axe basé sur un delta de pixels.
    pub fn pan_axis(range: &mut AxisRange, delta_pixels: f32, total_pixels: f32, is_y: bool) {
        if total_pixels <= 0.0 {
            return;
        }
        let span = range.span();
        let ratio = span / total_pixels as f64;
        
        // En Y, un delta positif (vers le bas) doit diminuer les valeurs (données).
        // En X, un delta positif (vers la droite) doit diminuer les valeurs de l'origine (décalage vers la gauche).
        // Mais ici AxisRange::pan ajoute delta_data.
        // Si on déplace la souris de 10px vers la droite, on veut "tirer" le graphique, 
        // donc les valeurs du domaine doivent DIMINUER pour montrer ce qu'il y a à gauche.
        let delta_data = if is_y {
            delta_pixels as f64 * ratio
        } else {
            -delta_pixels as f64 * ratio
        };
        
        range.pan(delta_data);
        range.clamp();
    }

    /// Zoom sur un axe à un point pivot précis (exprimé en pourcentage du domaine).
    pub fn zoom_axis_at(range: &mut AxisRange, pivot_pct: f64, factor: f64) {
        let mut new_factor = factor;
        const MIN_SPAN: f64 = 1e-9;
        
        if range.span() * factor < MIN_SPAN {
            new_factor = MIN_SPAN / range.span();
        }

        let pivot_data = range.min + range.span() * pivot_pct;
        range.zoom_at(pivot_data, pivot_pct, new_factor);
        range.clamp();
    }

    /// Redimensionne deux panneaux adjacents en ajustant leurs poids respectifs.
    /// Garantit qu'un panneau ne disparaisse pas complètement (poids minimum).
    pub fn resize_panes(weights: &mut [f32], index: usize, delta_pixels: f32, total_height: f32) {
        if index + 1 >= weights.len() || total_height <= 0.0 {
            return;
        }

        let total_weight: f32 = weights.iter().sum();
        let dw = (delta_pixels / total_height) * total_weight;
        
        const MIN_WEIGHT: f32 = 0.05;
        let w1 = weights[index];
        let w2 = weights[index + 1];

        // On limite dw pour respecter les minima des deux côtés
        let actual_dw = if dw > 0.0 {
            // On descend le splitter : w1 augmente, w2 diminue
            dw.min(w2 - MIN_WEIGHT).max(0.0)
        } else {
            // On monte le splitter : w1 diminue, w2 augmente
            dw.max(-(w1 - MIN_WEIGHT)).min(0.0)
        };

        weights[index] = w1 + actual_dw;
        weights[index + 1] = w2 - actual_dw;
    }

    /// Calcule les nouvelles bornes pour un auto-fit avec une marge optionnelle.
    pub fn compute_auto_fit(min: f64, max: f64, margin_pct: f64) -> (f64, f64) {
        if min == f64::INFINITY || max == f64::NEG_INFINITY {
            return (0.0, 100.0);
        }
        
        let span = if (max - min).abs() < f64::EPSILON {
            1.0 // Évite un span nul
        } else {
            max - min
        };

        (
            min - span * margin_pct,
            max + span * margin_pct
        )
    }

    /// Applique un auto-fit sur un axe donné.
    pub fn auto_fit_axis(range: &mut AxisRange, data_min: f64, data_max: f64, margin_pct: f64) {
        let (new_min, new_max) = Self::compute_auto_fit(data_min, data_max, margin_pct);
        range.min = new_min;
        range.max = new_max;
        range.clamp();
    }

    /// Centre l'axe sur une valeur donnée, en respectant optionnellement des limites strictes.
    pub fn move_to_center(range: &mut AxisRange, center_data: f64, clamp_to: Option<(f64, f64)>) {
        let span = range.span();
        range.min = center_data - span / 2.0;
        range.max = center_data + span / 2.0;
        
        if let Some((limit_min, limit_max)) = clamp_to {
            let limit_span = limit_max - limit_min;
            if span <= limit_span {
                if range.min < limit_min {
                    range.min = limit_min;
                    range.max = limit_min + span;
                } else if range.max > limit_max {
                    range.max = limit_max;
                    range.min = limit_max - span;
                }
            } else {
                range.min = limit_min;
                range.max = limit_max;
            }
        }
        
        range.clamp();
    }

    /// Calcule un facteur de zoom basé sur un delta de pixels.
    pub fn compute_zoom_factor(delta: f32, sensitivity: f32) -> f64 {
        let factor = 1.0 + (delta.abs() / sensitivity) as f64;
        if delta > 0.0 { 1.0 / factor } else { factor }
    }

    /// Calcule la nouvelle vitesse avec friction pour l'inertie.
    pub fn apply_friction(velocity: &mut f64, friction: f64, dt: f64) {
        *velocity *= friction.powf(dt * 60.0);
        if velocity.abs() < 0.1 {
            *velocity = 0.0;
        }
    }

    /// Mappe une position en pixels vers une valeur dans un domaine donné.
    pub fn map_pixels_to_value(pixels: f32, total_pixels: f32, min_val: f64, max_val: f64, invert: bool) -> f64 {
        if total_pixels <= 0.0 { return min_val; }
        let pct = (pixels / total_pixels).clamp(0.0, 1.0) as f64;
        let effective_pct = if invert { 1.0 - pct } else { pct };
        min_val + (max_val - min_val) * effective_pct
    }
}
