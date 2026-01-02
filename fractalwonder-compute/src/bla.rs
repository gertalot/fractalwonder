//! Bivariate Linear Approximation for iteration skipping.
//!
//! Uses HDRFloat for coefficients to prevent overflow at deep zoom levels
//! where BLA coefficients can exceed f64 range (10^308).

use crate::ReferenceOrbit;
use fractalwonder_core::{HDRComplex, HDRFloat};

/// Single BLA entry: skips `l` iterations.
///
/// Position formula: δz_new = A·δz + B·δc
/// Derivative formula: δρ_new = C·δρ + D·δz + E·δc
///
/// Note: C = A mathematically (both derive from 2·Z_m with identical merge formulas),
/// so C is not stored separately.
#[derive(Clone, Debug)]
pub struct BlaEntry {
    /// Complex coefficient A (multiplies δz for position, δρ for derivative)
    pub a: HDRComplex,
    /// Complex coefficient B (multiplies δc for position)
    pub b: HDRComplex,
    /// Complex coefficient D (δz contribution to δρ)
    pub d: HDRComplex,
    /// Complex coefficient E (δc contribution to δρ)
    pub e: HDRComplex,
    /// Number of iterations to skip
    pub l: u32,
    /// Validity radius squared
    pub r_sq: HDRFloat,
}

/// BLA entry with f64 coefficients for fast-path rendering.
/// Created from HDR entry when coefficients fit in f64 range.
///
/// Note: C = A mathematically, so not stored separately.
#[derive(Clone, Debug)]
pub struct BlaEntryF64 {
    /// Complex coefficient A as (re, im)
    pub a: (f64, f64),
    /// Complex coefficient B as (re, im)
    pub b: (f64, f64),
    /// Complex coefficient D as (re, im) - δz contribution to δρ
    pub d: (f64, f64),
    /// Complex coefficient E as (re, im) - δc contribution to δρ
    pub e: (f64, f64),
    /// Number of iterations to skip
    pub l: u32,
    /// Validity radius squared
    pub r_sq: f64,
}

impl BlaEntryF64 {
    /// Try to convert from HDR entry. Returns None if any coefficient overflows f64.
    pub fn try_from_hdr(entry: &BlaEntry) -> Option<Self> {
        let a_re = entry.a.re.to_f64();
        let a_im = entry.a.im.to_f64();
        let b_re = entry.b.re.to_f64();
        let b_im = entry.b.im.to_f64();
        let d_re = entry.d.re.to_f64();
        let d_im = entry.d.im.to_f64();
        let e_re = entry.e.re.to_f64();
        let e_im = entry.e.im.to_f64();
        let r_sq = entry.r_sq.to_f64();

        // Check for overflow (inf) or underflow to zero when non-zero
        if !a_re.is_finite() || !a_im.is_finite() {
            return None;
        }
        if !b_re.is_finite() || !b_im.is_finite() {
            return None;
        }
        if !d_re.is_finite() || !d_im.is_finite() {
            return None;
        }
        if !e_re.is_finite() || !e_im.is_finite() {
            return None;
        }
        if !r_sq.is_finite() {
            return None;
        }

        Some(Self {
            a: (a_re, a_im),
            b: (b_re, b_im),
            d: (d_re, d_im),
            e: (e_re, e_im),
            l: entry.l,
            r_sq,
        })
    }
}

impl BlaEntry {
    /// Create a single-iteration BLA from a reference orbit point.
    ///
    /// Single-step coefficients:
    ///   A = 2·Z_m (multiplies δz for position, δρ for derivative)
    ///   B = 1 (multiplies δc for position)
    ///   D = 2·Der_m (δz contribution to δρ)
    ///   E = 0 (δc contribution to δρ, zero at single step)
    pub fn from_orbit_point(z_re: f64, z_im: f64, der_re: f64, der_im: f64) -> Self {
        let epsilon = 2.0_f64.powi(-53);
        let z_mag = (z_re * z_re + z_im * z_im).sqrt();
        let r = epsilon * z_mag;

        Self {
            a: HDRComplex {
                re: HDRFloat::from_f64(2.0 * z_re),
                im: HDRFloat::from_f64(2.0 * z_im),
            },
            b: HDRComplex {
                re: HDRFloat::from_f64(1.0),
                im: HDRFloat::ZERO,
            },
            d: HDRComplex {
                re: HDRFloat::from_f64(2.0 * der_re),
                im: HDRFloat::from_f64(2.0 * der_im),
            },
            e: HDRComplex::ZERO,
            l: 1,
            r_sq: HDRFloat::from_f64(r * r),
        }
    }

    /// Merge two BLAs: x (first) then y (second).
    /// Result skips l_x + l_y iterations.
    ///
    /// All arithmetic uses HDRFloat to prevent overflow at deep zoom levels.
    /// dc_max must be HDRFloat to prevent underflow at deep zoom (10^-270 underflows in f64).
    pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: &HDRFloat) -> BlaEntry {
        // A_merged = A_y * A_x (HDRComplex multiplication - no overflow!)
        let a = y.a.mul(&x.a);

        // B_merged = A_y * B_x + B_y
        let b = y.a.mul(&x.b).add(&y.b);

        // Derivative coefficients
        // D_merged = C_y·D_x + D_y·A_x (note: C_y = A_y)
        let d = y.a.mul(&x.d).add(&y.d.mul(&x.a));

        // E_merged = C_y·E_x + D_y·B_x + E_y (note: C_y = A_y)
        let e = y.a.mul(&x.e).add(&y.d.mul(&x.b)).add(&y.e);

        // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
        let r_x = x.r_sq.sqrt();
        let r_y = y.r_sq.sqrt();
        let b_x_mag = x.b.norm_hdr();
        let a_x_mag = x.a.norm_hdr();

        // All HDRFloat arithmetic - no f64 overflow/underflow possible
        let b_dc = b_x_mag.mul(dc_max);
        let r_adjusted_num = r_y.sub(&b_dc);

        // max(0, r_adjusted_num) - check if negative via sign
        let r_adjusted = if r_adjusted_num.is_negative() || r_adjusted_num.is_zero() {
            HDRFloat::ZERO
        } else {
            // Avoid division by zero
            if a_x_mag.is_zero() {
                HDRFloat::ZERO
            } else {
                r_adjusted_num.div(&a_x_mag)
            }
        };

        // min(r_x, r_adjusted)
        let r = r_x.min(&r_adjusted);

        BlaEntry {
            a,
            b,
            d,
            e,
            l: x.l + y.l,
            r_sq: r.square(),
        }
    }
}

/// BLA table for a reference orbit, organized as a binary tree.
#[derive(Clone)]
pub struct BlaTable {
    pub entries: Vec<BlaEntry>,
    pub level_offsets: Vec<usize>,
    pub num_levels: usize,
    /// Maximum |delta_c| for BLA validity checks.
    /// Uses HDRFloat to prevent underflow at deep zoom (f64 underflows below ~10^-308).
    dc_max: HDRFloat,
}

impl BlaTable {
    /// Compute BLA table from a reference orbit.
    ///
    /// dc_max must be HDRFloat to prevent underflow at deep zoom levels
    /// where the viewport width (10^-270) underflows in f64.
    pub fn compute(orbit: &ReferenceOrbit, dc_max: &HDRFloat) -> Self {
        let m = orbit.orbit.len();
        if m == 0 {
            return Self {
                entries: vec![],
                level_offsets: vec![0],
                num_levels: 0,
                dc_max: *dc_max,
            };
        }

        let num_levels = ((m as f64).log2().ceil() as usize).max(1) + 1;
        let mut entries = Vec::with_capacity(2 * m);
        let mut level_offsets = Vec::with_capacity(num_levels);

        // Level 0: single-iteration BLAs
        level_offsets.push(0);
        for &(z_re, z_im) in &orbit.orbit {
            entries.push(BlaEntry::from_orbit_point(z_re, z_im));
        }

        // Build higher levels by merging pairs
        let mut prev_level_size = m;
        for _level in 1..num_levels {
            let prev_offset = *level_offsets.last().unwrap();
            level_offsets.push(entries.len());

            let this_level_size = prev_level_size.div_ceil(2);

            for i in 0..this_level_size {
                let x_idx = prev_offset + 2 * i;
                let y_idx = prev_offset + 2 * i + 1;

                if y_idx >= entries.len() {
                    // Odd number: copy last entry unchanged
                    entries.push(entries[x_idx].clone());
                } else {
                    let merged = BlaEntry::merge(&entries[x_idx], &entries[y_idx], dc_max);
                    entries.push(merged);
                }
            }

            prev_level_size = this_level_size;
            if this_level_size <= 1 {
                break;
            }
        }

        let num_levels_actual = level_offsets.len();
        Self {
            entries,
            level_offsets,
            num_levels: num_levels_actual,
            dc_max: *dc_max,
        }
    }

    /// Get dc_max for validity checks during find_valid.
    pub fn dc_max(&self) -> &HDRFloat {
        &self.dc_max
    }

    /// Find the largest valid BLA at reference index `m` for current |δz|².
    /// Returns None if no BLA is valid (fallback to standard iteration).
    ///
    /// Uses HDRFloat for dz_mag_sq to prevent f64 underflow at deep zoom levels
    /// where |δz|² can be as small as 10^-1800.
    pub fn find_valid(
        &self,
        m: usize,
        dz_mag_sq: &HDRFloat,
        dc_max: &HDRFloat,
    ) -> Option<&BlaEntry> {
        if self.entries.is_empty() {
            return None;
        }

        // Maximum allowed |B| * dc_max to prevent coefficient explosion.
        // After BLA: dz_new = A*dz + B*dc. If |B*dc| becomes large, accumulated
        // errors cause all pixels to escape at identical iterations (uniform color).
        // Threshold of 2^0 = 1 is conservative to preserve pixel differences.
        let max_b_dc_exp = 0;

        // Search from highest level (largest skips) down to level 0
        for level in (0..=self.num_levels.saturating_sub(1)).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1usize << level; // 2^level iterations per entry at this level

            // BUG FIX: Only use higher-level BLA when m is aligned to skip_size.
            // Level n entry i was built from orbit points Z_{i*2^n} to Z_{i*2^n + 2^n - 1}.
            // These coefficients are only valid when applied starting at m = i * 2^n.
            // If m is not aligned, the BLA would use wrong orbit points.
            if !m.is_multiple_of(skip_size) {
                continue;
            }

            // Index within this level for reference index m
            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            // Check bounds
            let level_end = if level + 1 < self.level_offsets.len() {
                self.level_offsets[level + 1]
            } else {
                self.entries.len()
            };

            if entry_idx >= level_end {
                continue;
            }

            let entry = &self.entries[entry_idx];

            // Check validity: |δz|² < r²
            // Using HDRFloat comparison: a < b iff (a - b).is_negative()
            let diff = dz_mag_sq.sub(&entry.r_sq);
            let validity_check = diff.is_negative();

            // Check B coefficient: |B| * dc_max must not be too large
            let b_norm = entry.b.norm_hdr();
            let b_dc = b_norm.mul(dc_max);
            let b_check = b_dc.exp <= max_b_dc_exp;

            if validity_check && b_check {
                return Some(entry);
            }
            // If B is too large or validity check fails, try lower level
        }

        None
    }

    /// Find the largest valid BLA at reference index `m` for current |δz|², returning f64 coefficients.
    /// Returns None if no BLA is valid or if coefficients overflow f64 range.
    ///
    /// This is the f64-optimized version for moderate zoom levels where BLA coefficients
    /// fit in f64 range. Falls back gracefully when coefficients overflow.
    pub fn find_valid_f64(&self, m: usize, dz_mag_sq: f64, dc_max: f64) -> Option<BlaEntryF64> {
        if self.entries.is_empty() {
            return None;
        }

        let max_b_dc_exp = 0;

        for level in (0..=self.num_levels.saturating_sub(1)).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1usize << level;

            if !m.is_multiple_of(skip_size) {
                continue;
            }

            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            let level_end = if level + 1 < self.level_offsets.len() {
                self.level_offsets[level + 1]
            } else {
                self.entries.len()
            };

            if entry_idx >= level_end {
                continue;
            }

            let entry = &self.entries[entry_idx];

            // Convert r_sq to f64 for comparison
            let r_sq_f64 = entry.r_sq.to_f64();
            if !r_sq_f64.is_finite() || r_sq_f64 <= 0.0 {
                continue;
            }

            // Validity check: |δz|² < r²
            if dz_mag_sq >= r_sq_f64 {
                continue;
            }

            // B coefficient check: |B| * dc_max must not be too large
            let b_norm = entry.b.norm_hdr();
            let dc_max_hdr = HDRFloat::from_f64(dc_max);
            let b_dc = b_norm.mul(&dc_max_hdr);
            if b_dc.exp > max_b_dc_exp {
                continue;
            }

            // Try to convert to f64 - returns None if overflow
            if let Some(f64_entry) = BlaEntryF64::try_from_hdr(entry) {
                return Some(f64_entry);
            }
            // If conversion failed, try lower level
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReferenceOrbit;
    use fractalwonder_core::BigFloat;

    #[test]
    fn bla_entry_from_orbit_point() {
        // Z = (1.0, 0.5), ε = 2^-53
        // A = 2Z = (2.0, 1.0)
        // B = 1
        // r = ε·|Z| = ε·√(1 + 0.25) ≈ ε·1.118
        let entry = BlaEntry::from_orbit_point(1.0, 0.5);

        assert!((entry.a.re.to_f64() - 2.0).abs() < 1e-14);
        assert!((entry.a.im.to_f64() - 1.0).abs() < 1e-14);
        assert!((entry.b.re.to_f64() - 1.0).abs() < 1e-14);
        assert!((entry.b.im.to_f64() - 0.0).abs() < 1e-14);
        assert_eq!(entry.l, 1);

        let z_mag = (1.0_f64 * 1.0 + 0.5 * 0.5).sqrt();
        let epsilon = 2.0_f64.powi(-53);
        let expected_r_sq = (epsilon * z_mag).powi(2);
        assert!((entry.r_sq.to_f64() - expected_r_sq).abs() < 1e-40);
    }

    #[test]
    fn bla_entry_merge_two_single_iterations() {
        // Two single-iteration BLAs should merge into one that skips 2
        let x = BlaEntry::from_orbit_point(1.0, 0.0); // Z = 1
        let y = BlaEntry::from_orbit_point(0.5, 0.0); // Z = 0.5

        let dc_max = HDRFloat::from_f64(0.001); // Small delta_c
        let merged = BlaEntry::merge(&x, &y, &dc_max);

        // l should be 2
        assert_eq!(merged.l, 2);

        // A_merged = A_y * A_x = (1.0, 0) * (2.0, 0) = (2.0, 0)
        // (complex multiply: (1)(2) - (0)(0) = 2, (1)(0) + (0)(2) = 0)
        assert!((merged.a.re.to_f64() - 2.0).abs() < 1e-14);
        assert!((merged.a.im.to_f64() - 0.0).abs() < 1e-14);

        // B_merged = A_y * B_x + B_y = (1,0)*(1,0) + (1,0) = (2,0)
        assert!((merged.b.re.to_f64() - 2.0).abs() < 1e-14);
        assert!((merged.b.im.to_f64() - 0.0).abs() < 1e-14);
    }

    #[test]
    fn bla_table_level_0_has_one_entry_per_orbit_point() {
        // Create a simple reference orbit
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        let dc_max = HDRFloat::from_f64(0.01);
        let table = BlaTable::compute(&orbit, &dc_max);

        // Level 0 should have orbit.len() entries
        assert!(table.entries.len() >= orbit.orbit.len());

        // First entry should match first orbit point
        let z0 = orbit.orbit[0];
        let expected = BlaEntry::from_orbit_point(z0.0, z0.1);
        assert_eq!(table.entries[0].l, expected.l);
        assert!((table.entries[0].a.re.to_f64() - expected.a.re.to_f64()).abs() < 1e-14);
    }

    #[test]
    fn bla_table_has_multiple_levels() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 16);

        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

        // 16 entries -> should have ~log2(16)+1 = 5 levels
        // Level 0: 16 entries (skip 1)
        // Level 1: 8 entries (skip 2)
        // Level 2: 4 entries (skip 4)
        // Level 3: 2 entries (skip 8)
        // Level 4: 1 entry (skip 16)
        assert!(
            table.num_levels >= 4,
            "Expected at least 4 levels, got {}",
            table.num_levels
        );
        assert!(table.level_offsets.len() >= 4);

        // Total entries should be ~2M
        assert!(table.entries.len() >= 16 + 8 + 4 + 2);
    }

    #[test]
    fn bla_table_higher_level_entries_skip_more() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 16);

        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

        // Level 0 entries skip 1
        assert_eq!(table.entries[0].l, 1);

        // Level 1 entries skip 2
        let level1_start = table.level_offsets[1];
        assert_eq!(table.entries[level1_start].l, 2);

        // Level 2 entries skip 4
        if table.level_offsets.len() > 2 {
            let level2_start = table.level_offsets[2];
            assert_eq!(table.entries[level2_start].l, 4);
        }
    }

    #[test]
    fn bla_table_find_valid_returns_none_for_large_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

        // With |δz|² = 1.0 (huge), no BLA should be valid
        let large_dz = HDRFloat::from_f64(1.0);
        let result = table.find_valid(0, &large_dz, table.dc_max());
        assert!(result.is_none(), "Large |δz| should invalidate all BLAs");
    }

    #[test]
    fn bla_table_find_valid_returns_some_for_tiny_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        // Use small dc_max so merged entries have r > 0
        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

        // At m=0, Z_0 = 0 so r = 0, no BLA valid there.
        // At m=1 onwards, |Z_m| > 0 so r > 0 and BLA can be valid.
        // Test at m=1 where the reference orbit has non-zero magnitude.
        let result = table.find_valid(1, &HDRFloat::ZERO, table.dc_max());
        assert!(
            result.is_some(),
            "Zero |δz| at m=1 should allow BLA (r > 0)"
        );

        // Should skip at least 1 iteration
        let bla = result.unwrap();
        assert!(bla.l >= 1);
    }

    #[test]
    fn bla_find_valid_at_origin_returns_none() {
        // At m=0, the reference orbit starts at Z_0 = 0, so r = ε * 0 = 0.
        // This means no BLA is valid at m=0, which is correct.
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

        // Even with |δz|² = 0, BLA at m=0 should be None (r = 0)
        let result = table.find_valid(0, &HDRFloat::ZERO, table.dc_max());
        assert!(result.is_none(), "BLA at m=0 should be None since Z_0 = 0");
    }

    #[test]
    fn bla_entry_f64_from_hdr_entry() {
        let entry = BlaEntry::from_orbit_point(1.0, 0.5);
        let f64_entry = BlaEntryF64::try_from_hdr(&entry);

        assert!(f64_entry.is_some());
        let f64_entry = f64_entry.unwrap();

        assert!((f64_entry.a.0 - 2.0).abs() < 1e-14);
        assert!((f64_entry.a.1 - 1.0).abs() < 1e-14);
        assert!((f64_entry.b.0 - 1.0).abs() < 1e-14);
        assert!((f64_entry.b.1 - 0.0).abs() < 1e-14);
        assert_eq!(f64_entry.l, 1);
    }

    #[test]
    fn bla_entry_f64_returns_none_on_overflow() {
        // Create entry with HDRFloat coefficients that overflow f64
        let huge = HDRFloat {
            head: 1.0,
            tail: 0.0,
            exp: 1100,
        }; // ~2^1100, way beyond f64
        let entry = BlaEntry {
            a: HDRComplex {
                re: huge,
                im: HDRFloat::ZERO,
            },
            b: HDRComplex {
                re: HDRFloat::from_f64(1.0),
                im: HDRFloat::ZERO,
            },
            d: HDRComplex::ZERO,
            e: HDRComplex::ZERO,
            l: 1,
            r_sq: HDRFloat::from_f64(1e-10),
        };

        let f64_entry = BlaEntryF64::try_from_hdr(&entry);
        assert!(
            f64_entry.is_none(),
            "Should return None when coefficient overflows f64"
        );
    }

    #[test]
    fn bla_table_find_valid_f64_returns_some_for_tiny_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

        // At m=1, Z_m != 0, so r > 0 and BLA should be valid
        let dc_max = 1e-10;
        let result = table.find_valid_f64(1, 0.0, dc_max);
        assert!(result.is_some(), "Zero |δz|² at m=1 should allow f64 BLA");

        let bla = result.unwrap();
        assert!(bla.l >= 1);
        assert!(bla.a.0.is_finite());
        assert!(bla.r_sq > 0.0);
    }

    #[test]
    fn bla_table_find_valid_f64_returns_none_for_large_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

        // With |δz|² = 1.0 (huge), no BLA should be valid
        let result = table.find_valid_f64(0, 1.0, 0.01);
        assert!(
            result.is_none(),
            "Large |δz| should invalidate all f64 BLAs"
        );
    }
}
