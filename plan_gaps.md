# Technical Specification: Logical Time System (Gaps/Exclusions)

This document details the implementation of the X-axis exclusion system for the `gpui_chart` component. The goal is to allow compression of periods without data (market closures, weekends) to provide a continuous view.

## 1. Core Concepts

### Real Time vs. Logical Time
*   **Real Time (`real_ms`)**: Raw timestamp `i64` (UTC milliseconds) from the data.
*   **Logical Time (`logical_ms`)**: Compressed time after subtracting all preceding exclusion zones. This value is used for calculating X coordinates (pixels).

### Unit and Precision
*   Systematic use of `i64` in milliseconds to avoid floating-point rounding errors on gap boundaries.

## 2. Data Model

### Exclusion Rules (`ExclusionRule`)
The application defines rules which are then converted into concrete segments by the chart.
*   **Fixed**: `(start_ms, end_ms)` - Single period (e.g., specific holiday).
*   **Recurring (Temporal)**: `(days, start_time, end_time, timezone)` - Weekly recurrence. Handles overnight spans if `end_time < start_time`.
*   **Recurring (Numeric)**: `(modulo, offset, width)` - For non-temporal axes.

### Transformation Index (`GapIndex`)
An optimized structure for frequent mapping:
*   Stores a sorted list of `GapSegment { start_real, end_real, cumulative_gap_duration }`.
*   **Normalization**: During "Build", overlapping or contiguous segments are merged to ensure a list of disjoint segments.

## 3. Mapping Algorithms ($O(\log n)$)

*   **`to_logical(real_ms)`**:
    1.  Find the number of "empty" milliseconds before `real_ms` via binary search in the `GapIndex`.
    2.  Return `real_ms - total_gap_before`.
*   **`to_real(logical_ms)`**:
    1.  Binary search to find how many gap segments must be "re-injected" to compensate for compression.
    2.  Return `logical_ms + calculated_offset`.

## 4. Specific Behaviors

### Visual Rendering
*   **Compression**: Excluded zones have a width of 0 pixels.
*   **Continuity**: Plots (lines, areas) directly connect the point before the gap to the point after the gap.
*   **Ticks (Labels)**: Candidate ticks falling into a gap are removed (No "magnet" effect).
*   **Boundaries**: Inclusivity `[Start, End[` (Gap start is hidden, end is the first visible point).

### Aggregation (Polars Engine)
*   **Cut-off (Scenario B)**: An aggregation bucket (e.g., 1h candle) must never span across a gap.
*   If a gap occurs in the middle of an aggregation period, the bucket is truncated. A new candle starts exactly at the end of the gap.

### Interaction
*   **Cursor/Tooltip**: The cursor instantly jumps over the gap when moving the mouse.
*   **Stability**: When changing gap configuration, the view is recentered on the same **Real Time** to avoid a disorienting visual jump.

## 5. Implementation Plan

### Phase 1: Core (`src/gaps.rs`)
- Implement `GapIndex` and binary search functions.
- Implement segment merging (Sweep-line algorithm).
- Unit tests on complex cases (DST, overlaps).

### Phase 2: Scales (`src/scales.rs`)
- Integrate `GapIndex` into `ContinuousScale` calculation.
- Ensure `Navigator` uses the same logic.

### Phase 3: Aggregation (`src/aggregation.rs`)
- Modify Polars bucket creation logic to inject gap boundaries as break points.

### Phase 4: Axis & UI (`src/axis_renderer.rs`)
- Filter ticks and grid lines.

### Phase 5: Performance
- Benchmark $O(\log n)$ mapping on 10,000 segments.
- Verify 60 FPS fluidity during intensive scrolling.