//! GPU BLA table serialization.

use bytemuck::{Pod, Zeroable};
use fractalwonder_compute::BlaEntry;

/// GPU-serializable BLA entry (64 bytes, 16 f32-equivalent values).
/// Layout: A (6), B (6), r_sq (3), l (1) = 16 values
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuBlaEntry {
    // Coefficient A (HDRComplex) - multiplies delta_z
    pub a_re_head: f32,
    pub a_re_tail: f32,
    pub a_re_exp: i32,
    pub a_im_head: f32,
    pub a_im_tail: f32,
    pub a_im_exp: i32,

    // Coefficient B (HDRComplex) - multiplies delta_c
    pub b_re_head: f32,
    pub b_re_tail: f32,
    pub b_re_exp: i32,
    pub b_im_head: f32,
    pub b_im_tail: f32,
    pub b_im_exp: i32,

    // Validity radius squared (HDRFloat)
    pub r_sq_head: f32,
    pub r_sq_tail: f32,
    pub r_sq_exp: i32,

    // Iterations to skip
    pub l: u32,
}

impl GpuBlaEntry {
    /// Convert from CPU BlaEntry to GPU format.
    pub fn from_bla_entry(entry: &BlaEntry) -> Self {
        Self {
            a_re_head: entry.a.re.head,
            a_re_tail: entry.a.re.tail,
            a_re_exp: entry.a.re.exp,
            a_im_head: entry.a.im.head,
            a_im_tail: entry.a.im.tail,
            a_im_exp: entry.a.im.exp,
            b_re_head: entry.b.re.head,
            b_re_tail: entry.b.re.tail,
            b_re_exp: entry.b.re.exp,
            b_im_head: entry.b.im.head,
            b_im_tail: entry.b.im.tail,
            b_im_exp: entry.b.im.exp,
            r_sq_head: entry.r_sq.head,
            r_sq_tail: entry.r_sq.tail,
            r_sq_exp: entry.r_sq.exp,
            l: entry.l,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_bla_entry_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<GpuBlaEntry>(), 64);
    }

    #[test]
    fn gpu_bla_entry_from_bla_entry_preserves_values() {
        let entry = BlaEntry::from_orbit_point(1.5, 0.5, 1.0, 0.0);
        let gpu_entry = GpuBlaEntry::from_bla_entry(&entry);

        // Coefficient A (real)
        assert_eq!(gpu_entry.a_re_head, entry.a.re.head);
        assert_eq!(gpu_entry.a_re_tail, entry.a.re.tail);
        assert_eq!(gpu_entry.a_re_exp, entry.a.re.exp);

        // Coefficient A (imaginary)
        assert_eq!(gpu_entry.a_im_head, entry.a.im.head);
        assert_eq!(gpu_entry.a_im_tail, entry.a.im.tail);
        assert_eq!(gpu_entry.a_im_exp, entry.a.im.exp);

        // Coefficient B (real)
        assert_eq!(gpu_entry.b_re_head, entry.b.re.head);
        assert_eq!(gpu_entry.b_re_tail, entry.b.re.tail);
        assert_eq!(gpu_entry.b_re_exp, entry.b.re.exp);

        // Coefficient B (imaginary)
        assert_eq!(gpu_entry.b_im_head, entry.b.im.head);
        assert_eq!(gpu_entry.b_im_tail, entry.b.im.tail);
        assert_eq!(gpu_entry.b_im_exp, entry.b.im.exp);

        // Validity radius squared
        assert_eq!(gpu_entry.r_sq_head, entry.r_sq.head);
        assert_eq!(gpu_entry.r_sq_tail, entry.r_sq.tail);
        assert_eq!(gpu_entry.r_sq_exp, entry.r_sq.exp);

        // Iterations to skip
        assert_eq!(gpu_entry.l, entry.l);
    }
}
