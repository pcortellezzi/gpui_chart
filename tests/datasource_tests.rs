use gpui_chart::data_types::{
    ColorOp, PlotData, PlotDataSource, PlotPoint, StreamingDataSource, VecDataSource,
};

#[test]
fn test_streaming_datasource_capacity() {
    let mut source = StreamingDataSource::new(10);
    for i in 0..15 {
        source.add_data(PlotData::Point(PlotPoint {
            x: i as f64,
            y: i as f64,
            color_op: ColorOp::None,
        }));
    }

    assert_eq!(source.len(), 10);
    // The first data points (0-4) must have been removed
    let bounds = source.get_bounds().unwrap();
    assert_eq!(bounds.0, 5.0, "x_min should be 5 after eviction");
    assert_eq!(bounds.1, 14.0, "x_max should be 14");
}

#[test]
fn test_datasource_y_range() {
    let mut data = Vec::new();
    for i in 0..100 {
        data.push(PlotData::Point(PlotPoint {
            x: i as f64,
            y: if i == 50 { 1000.0 } else { i as f64 },
            color_op: ColorOp::None,
        }));
    }
    let source = VecDataSource::new(data);

    // Range NOT containing the peak
    let range1 = source.get_y_range(0.0, 10.0).unwrap();
    assert_eq!(range1.0, 0.0);
    assert_eq!(range1.1, 10.0);

    // Range containing the peak
    let range2 = source.get_y_range(45.0, 55.0).unwrap();
    assert_eq!(range2.1, 1000.0);
}

#[test]
fn test_streaming_cache_rebuild() {
    let mut source = StreamingDataSource::new(1000);
    // Add enough points to fill chunks
    for i in 0..600 {
        source.add_data(PlotData::Point(PlotPoint {
            x: i as f64,
            y: i as f64,
            color_op: ColorOp::None,
        }));
    }

    let bounds = source.get_bounds().unwrap();
    assert_eq!(bounds.0, 0.0);
    assert_eq!(bounds.1, 599.0);
}
