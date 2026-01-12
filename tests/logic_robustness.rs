#[cfg(test)]
mod tests {
    use gpui_chart::scales::ChartScale;
    use gpui_chart::data_types::{AxisRange, VecDataSource, PlotDataSource};

    #[test]
    fn test_scale_zero_domain() {
        // Case: Min == Max (e.g. only one data point at y=10.0)
        let domain = (10.0, 10.0);
        let range = (0.0, 100.0);
        
        // This should probably internally apply padding or handle it gracefully
        // If it returns NaN or Inf, it causes rendering bugs/panics
        let scale = ChartScale::new_linear(domain, range);
        
        let mapped = scale.map(10.0);
        assert!(!mapped.is_nan(), "Mapped value should not be NaN for zero domain");
        assert!(!mapped.is_infinite(), "Mapped value should not be Inf for zero domain");
        
        // Ideally, if min==max, it should center the point?
        // Let's assert it produces a valid coordinate within range
        assert!(mapped >= 0.0 && mapped <= 100.0, "Mapped value {} should be within range [0, 100]", mapped);
    }

    #[test]
    fn test_empty_datasource() {
        let source = VecDataSource::new(vec![]);
        
        // Should return None, not panic or Some(Infinity)
        let bounds = source.get_bounds();
        assert!(bounds.is_none(), "Empty source should return None bounds");
        
        let y_range = source.get_y_range(0.0, 10.0);
        assert!(y_range.is_none(), "Empty source should return None y_range");
        
        // Iterating should yield nothing, not panic
        let iter_count = source.iter_range(0.0, 10.0).count();
        assert_eq!(iter_count, 0);
    }

    #[test]
    fn test_axis_range_robustness() {
        let mut axis = AxisRange::new(0.0, 0.0); // Flat range
        let ticks = axis.ticks(10);
        
        // Ticks generation should not crash on flat range
        assert!(!ticks.is_empty(), "Should generate at least one tick even for flat range (or handle it)");
        // If min==max, linear scale might fail to generate ticks.
    }
}