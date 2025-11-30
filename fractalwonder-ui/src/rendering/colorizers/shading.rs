//! Slope shading for 3D lighting effect on iteration height field.

/// Mirror a coordinate at boundaries for seamless edge handling.
fn mirror_coord(coord: i32, max: usize) -> usize {
    if coord < 0 {
        (-coord).min(max as i32 - 1) as usize
    } else if coord >= max as i32 {
        let reflected = 2 * max as i32 - coord - 2;
        reflected.max(0) as usize
    } else {
        coord as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_coord_in_bounds() {
        assert_eq!(mirror_coord(5, 10), 5);
        assert_eq!(mirror_coord(0, 10), 0);
        assert_eq!(mirror_coord(9, 10), 9);
    }

    #[test]
    fn mirror_coord_negative() {
        assert_eq!(mirror_coord(-1, 10), 1);
        assert_eq!(mirror_coord(-2, 10), 2);
    }

    #[test]
    fn mirror_coord_beyond_max() {
        assert_eq!(mirror_coord(10, 10), 8);
        assert_eq!(mirror_coord(11, 10), 7);
    }
}
