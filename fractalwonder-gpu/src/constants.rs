// fractalwonder-gpu/src/constants.rs

/// Sentinel value indicating a pixel was not computed in an Adam7 pass.
/// Used by GpuPerturbationRenderer for progressive rendering.
pub const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFF;
