#[cfg(test)]
mod tests {
    use gpui_chart::scales::ChartScale;
    use gpui_chart::gaps::{GapIndexBuilder, ExclusionRule};
    use std::sync::Arc;

    #[test]
    fn test_scale_with_gaps_offset() {
        // Domain [0, 100]. Range [0, 100].
        // Gap [40, 60]. Length 20.
        // Effective logical length: 80.
        // Expected scale factor: 100px / 80 = 1.25.
        
        let mut builder = GapIndexBuilder::new();
        builder.add_rule(ExclusionRule::Fixed { start: 40, end: 60 });
        let gaps = Arc::new(builder.build(0, 100));

        let scale = ChartScale::new_linear((0.0, 100.0), (0.0, 100.0));
        
        // Before gaps: 1.0 scale
        assert_eq!(scale.map(80.0), 80.0);

        // Apply gaps
        let scale_with_gaps = scale.with_gaps(Some(gaps.clone()));

        // Test value 80. 
        // Real 80 -> Logical 60.
        // Expected pixel: 60 * 1.25 = 75.0.
        
        let mapped = scale_with_gaps.map(80.0);
        
        // If the bug exists, mapped will be 60.0 (using 1.0 scale).
        assert!((mapped - 75.0).abs() < 0.001, "Expected 75.0, got {}", mapped);
        
        // Test value 100.
        // Real 100 -> Logical 80.
        // Expected pixel: 80 * 1.25 = 100.0.
        let mapped_max = scale_with_gaps.map(100.0);
        assert!((mapped_max - 100.0).abs() < 0.001, "Expected 100.0, got {}", mapped_max);
    }
}
