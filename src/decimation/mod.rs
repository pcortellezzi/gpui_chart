pub mod common;
pub mod bucketing;
pub mod min_max;
pub mod m4;
pub mod lttb;
pub mod ohlcv;

// Re-export public functions to maintain API compatibility
pub use bucketing::{
    calculate_gap_aware_buckets, calculate_gap_aware_buckets_data,
    calculate_stable_buckets, calculate_stable_buckets_data, calculate_stable_buckets_generic
};
pub use min_max::{
    decimate_min_max_arrays_par, decimate_min_max_arrays_par_into,
    decimate_min_max_slice, decimate_min_max_slice_into,
    decimate_min_max_generic
};
pub use m4::{
    decimate_m4_arrays_par, decimate_m4_arrays_par_into,
    decimate_m4_slice, decimate_m4_slice_into,
    decimate_m4_generic
};
pub use lttb::{
    decimate_lttb_arrays, decimate_lttb_arrays_into,
    decimate_lttb_slice, decimate_lttb_generic,
    decimate_ilttb_arrays_par, decimate_ilttb_arrays_par_into
};
pub use ohlcv::{
    decimate_ohlcv_arrays_par, decimate_ohlcv_arrays_par_into,
    decimate_ohlcv_slice_into
};
pub use common::aggregate_chunk;
