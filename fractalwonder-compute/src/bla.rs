//! Bivariate Linear Approximation for iteration skipping.

use crate::ReferenceOrbit;

/// Single BLA entry: skips `l` iterations.
/// Applies: δz_new = A·δz + B·δc
#[derive(Clone, Debug, PartialEq)]
pub struct BlaEntry {
    pub a_re: f64,
    pub a_im: f64,
    pub b_re: f64,
    pub b_im: f64,
    pub l: u32,
    pub r_sq: f64,
}

impl BlaEntry {
    /// Create a single-iteration BLA from a reference orbit point Z = (z_re, z_im).
    pub fn from_orbit_point(z_re: f64, z_im: f64) -> Self {
        let epsilon = 2.0_f64.powi(-53);
        let z_mag = (z_re * z_re + z_im * z_im).sqrt();
        let r = epsilon * z_mag;

        Self {
            a_re: 2.0 * z_re,
            a_im: 2.0 * z_im,
            b_re: 1.0,
            b_im: 0.0,
            l: 1,
            r_sq: r * r,
        }
    }

    /// Merge two BLAs: x (first) then y (second).
    /// Result skips l_x + l_y iterations.
    pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: f64) -> BlaEntry {
        // A_merged = A_y * A_x (complex multiplication)
        let a_re = y.a_re * x.a_re - y.a_im * x.a_im;
        let a_im = y.a_re * x.a_im + y.a_im * x.a_re;

        // B_merged = A_y * B_x + B_y
        let b_re = (y.a_re * x.b_re - y.a_im * x.b_im) + y.b_re;
        let b_im = (y.a_re * x.b_im + y.a_im * x.b_re) + y.b_im;

        // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
        let r_x = x.r_sq.sqrt();
        let r_y = y.r_sq.sqrt();
        let b_x_mag = (x.b_re * x.b_re + x.b_im * x.b_im).sqrt();
        let a_x_mag = (x.a_re * x.a_re + x.a_im * x.a_im).sqrt();

        let r_adjusted = (r_y - b_x_mag * dc_max).max(0.0) / a_x_mag.max(1e-300);
        let r = r_x.min(r_adjusted);

        BlaEntry {
            a_re,
            a_im,
            b_re,
            b_im,
            l: x.l + y.l,
            r_sq: r * r,
        }
    }
}

/// BLA table for a reference orbit, organized as a binary tree.
pub struct BlaTable {
    pub entries: Vec<BlaEntry>,
    pub level_offsets: Vec<usize>,
    pub num_levels: usize,
    #[allow(dead_code)] // Will be used in validity checks in future optimizations
    dc_max: f64,
}

impl BlaTable {
    /// Compute BLA table from a reference orbit.
    pub fn compute(orbit: &ReferenceOrbit, dc_max: f64) -> Self {
        let m = orbit.orbit.len();
        if m == 0 {
            return Self {
                entries: vec![],
                level_offsets: vec![0],
                num_levels: 0,
                dc_max,
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
            dc_max,
        }
    }

    /// Find the largest valid BLA at reference index `m` for current |δz|².
    /// Returns None if no BLA is valid (fallback to standard iteration).
    pub fn find_valid(&self, m: usize, dz_mag_sq: f64) -> Option<&BlaEntry> {
        if self.entries.is_empty() {
            return None;
        }

        // When |δz|² = 0, all BLAs are valid (linear approx is exact)
        // Start from highest level for maximum skip
        if dz_mag_sq == 0.0 {
            let highest_level = self.num_levels - 1;
            let level_start = self.level_offsets[highest_level];
            let skip_size = 1usize << highest_level;
            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            let level_end = if highest_level + 1 < self.level_offsets.len() {
                self.level_offsets[highest_level + 1]
            } else {
                self.entries.len()
            };

            if entry_idx < level_end {
                return Some(&self.entries[entry_idx]);
            }
        }

        // Search from highest level (largest skips) down to level 0
        for level in (0..self.num_levels).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1usize << level; // 2^level iterations per entry at this level

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
            if dz_mag_sq < entry.r_sq {
                return Some(entry);
            }
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

        assert!((entry.a_re - 2.0).abs() < 1e-14);
        assert!((entry.a_im - 1.0).abs() < 1e-14);
        assert!((entry.b_re - 1.0).abs() < 1e-14);
        assert!((entry.b_im - 0.0).abs() < 1e-14);
        assert_eq!(entry.l, 1);

        let z_mag = (1.0_f64 * 1.0 + 0.5 * 0.5).sqrt();
        let epsilon = 2.0_f64.powi(-53);
        let expected_r_sq = (epsilon * z_mag).powi(2);
        assert!((entry.r_sq - expected_r_sq).abs() < 1e-40);
    }

    #[test]
    fn bla_entry_merge_two_single_iterations() {
        // Two single-iteration BLAs should merge into one that skips 2
        let x = BlaEntry::from_orbit_point(1.0, 0.0); // Z = 1
        let y = BlaEntry::from_orbit_point(0.5, 0.0); // Z = 0.5

        let dc_max = 0.001; // Small delta_c
        let merged = BlaEntry::merge(&x, &y, dc_max);

        // l should be 2
        assert_eq!(merged.l, 2);

        // A_merged = A_y * A_x = (1.0, 0) * (2.0, 0) = (2.0, 0)
        // (complex multiply: (1)(2) - (0)(0) = 2, (1)(0) + (0)(2) = 0)
        assert!((merged.a_re - 2.0).abs() < 1e-14);
        assert!((merged.a_im - 0.0).abs() < 1e-14);

        // B_merged = A_y * B_x + B_y = (1,0)*(1,0) + (1,0) = (2,0)
        assert!((merged.b_re - 2.0).abs() < 1e-14);
        assert!((merged.b_im - 0.0).abs() < 1e-14);
    }

    #[test]
    fn bla_table_level_0_has_one_entry_per_orbit_point() {
        // Create a simple reference orbit
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        let dc_max = 0.01;
        let table = BlaTable::compute(&orbit, dc_max);

        // Level 0 should have orbit.len() entries
        assert!(table.entries.len() >= orbit.orbit.len());

        // First entry should match first orbit point
        let z0 = orbit.orbit[0];
        let expected = BlaEntry::from_orbit_point(z0.0, z0.1);
        assert_eq!(table.entries[0].l, expected.l);
        assert!((table.entries[0].a_re - expected.a_re).abs() < 1e-14);
    }

    #[test]
    fn bla_table_has_multiple_levels() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 16);

        let table = BlaTable::compute(&orbit, 0.01);

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

        let table = BlaTable::compute(&orbit, 0.01);

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
        let table = BlaTable::compute(&orbit, 0.01);

        // With |δz|² = 1.0 (huge), no BLA should be valid
        let result = table.find_valid(0, 1.0);
        assert!(result.is_none(), "Large |δz| should invalidate all BLAs");
    }

    #[test]
    fn bla_table_find_valid_returns_some_for_tiny_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, 0.01);

        // With |δz|² = 0 (at start), some BLA should be valid
        let result = table.find_valid(0, 0.0);
        assert!(result.is_some(), "Zero |δz| should allow BLA");

        // Should skip multiple iterations
        let bla = result.unwrap();
        assert!(bla.l >= 1);
    }
}
