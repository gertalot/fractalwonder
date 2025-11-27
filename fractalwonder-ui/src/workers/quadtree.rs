//! Quadtree data structure for spatial partitioning of the render canvas.
//!
//! Used to track glitched regions and associate them with reference orbits.
//! The quadtree subdivides the canvas into cells, each potentially having
//! its own reference orbit for perturbation computation.

/// Maximum subdivision depth to prevent infinite recursion.
pub const MAX_DEPTH: u32 = 10;

/// Minimum cell dimension (width or height) below which subdivision stops.
pub const MIN_CELL_SIZE: u32 = 16;

/// Rectangular bounds in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Bounds {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Area of this bounds rectangle.
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Check if a point (px, py) is within these bounds.
    /// Uses half-open interval: [x, x+width) × [y, y+height)
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// A cell in the quadtree, representing a rectangular region of the canvas.
#[derive(Debug)]
pub struct QuadtreeCell {
    pub bounds: Bounds,
    pub depth: u32,
    /// Children are stored as [top_left, top_right, bottom_left, bottom_right]
    pub children: Option<Box<[QuadtreeCell; 4]>>,
}

impl QuadtreeCell {
    /// Create a root cell covering the entire canvas.
    pub fn new_root(dimensions: (u32, u32)) -> Self {
        Self {
            bounds: Bounds::new(0, 0, dimensions.0, dimensions.1),
            depth: 0,
            children: None,
        }
    }

    /// Create a cell with specific bounds and depth.
    fn new(bounds: Bounds, depth: u32) -> Self {
        Self {
            bounds,
            depth,
            children: None,
        }
    }

    /// Check if a point is within this cell's bounds.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.bounds.contains(x, y)
    }

    /// Check if this cell is a leaf (has no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    /// Check if this cell can be subdivided (not at limits).
    pub fn can_subdivide(&self) -> bool {
        if self.depth >= MAX_DEPTH {
            return false;
        }
        // After subdivision, children must have at least MIN_CELL_SIZE
        let half_w = self.bounds.width / 2;
        let half_h = self.bounds.height / 2;
        half_w >= MIN_CELL_SIZE && half_h >= MIN_CELL_SIZE
    }

    /// Subdivide this cell into four children.
    ///
    /// For odd dimensions, uses floor division for left/top children
    /// and the remainder goes to right/bottom children, ensuring
    /// perfect area conservation with no gaps or overlaps.
    ///
    /// Returns true if subdivision succeeded, false if at limits.
    pub fn subdivide(&mut self) -> bool {
        if self.children.is_some() {
            return false; // Already subdivided
        }

        if !self.can_subdivide() {
            return false;
        }

        let Bounds {
            x,
            y,
            width,
            height,
        } = self.bounds;

        // Floor division for left/top, remainder for right/bottom
        let left_w = width / 2;
        let right_w = width - left_w;
        let top_h = height / 2;
        let bottom_h = height - top_h;

        let next_depth = self.depth + 1;

        let children = [
            // Top-left
            QuadtreeCell::new(Bounds::new(x, y, left_w, top_h), next_depth),
            // Top-right
            QuadtreeCell::new(Bounds::new(x + left_w, y, right_w, top_h), next_depth),
            // Bottom-left
            QuadtreeCell::new(Bounds::new(x, y + top_h, left_w, bottom_h), next_depth),
            // Bottom-right
            QuadtreeCell::new(
                Bounds::new(x + left_w, y + top_h, right_w, bottom_h),
                next_depth,
            ),
        ];

        self.children = Some(Box::new(children));
        true
    }

    /// Collect all leaf cells in this subtree.
    pub fn collect_leaves<'a>(&'a self, leaves: &mut Vec<&'a QuadtreeCell>) {
        if self.is_leaf() {
            leaves.push(self);
        } else if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_leaves(leaves);
            }
        }
    }

    /// Collect all leaf cells mutably.
    pub fn collect_leaves_mut<'a>(&'a mut self, leaves: &mut Vec<&'a mut QuadtreeCell>) {
        if self.is_leaf() {
            leaves.push(self);
        } else if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                child.collect_leaves_mut(leaves);
            }
        }
    }
}

/// Helper function to recursively subdivide to a given depth.
pub fn subdivide_to_depth(cell: &mut QuadtreeCell, target_depth: u32) {
    if cell.depth >= target_depth {
        return;
    }

    if cell.subdivide() {
        if let Some(children) = &mut cell.children {
            for child in children.iter_mut() {
                subdivide_to_depth(child, target_depth);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Mathematical Invariant 1: Area Conservation
    // =========================================================================

    #[test]
    fn subdivision_conserves_area_for_all_dimensions() {
        let test_sizes = [
            (8, 8),
            (9, 9),
            (15, 16),
            (16, 15),
            (17, 17),
            (33, 33),
            (64, 65),
            (100, 101),
            (602, 559), // Real canvas from plan
        ];

        for (width, height) in test_sizes {
            let mut root = QuadtreeCell::new_root((width, height));
            let parent_area = width * height;

            // Skip if can't subdivide due to MIN_CELL_SIZE
            if !root.can_subdivide() {
                continue;
            }

            root.subdivide();

            let child_area_sum: u32 = root
                .children
                .as_ref()
                .unwrap()
                .iter()
                .map(|c| c.bounds.area())
                .sum();

            assert_eq!(
                child_area_sum, parent_area,
                "{}x{}: children area {} != parent area {}",
                width, height, child_area_sum, parent_area
            );
        }
    }

    // =========================================================================
    // Mathematical Invariant 2: No Gaps/Overlaps
    // =========================================================================

    #[test]
    fn every_point_in_exactly_one_child_exhaustive() {
        // Test dimensions that can actually subdivide (>= 2*MIN_CELL_SIZE)
        let test_sizes = [32, 33, 63, 64, 65, 100];

        for size in test_sizes {
            let mut root = QuadtreeCell::new_root((size, size));

            if !root.can_subdivide() {
                continue;
            }

            root.subdivide();
            let children = root.children.as_ref().unwrap();

            for x in 0..size {
                for y in 0..size {
                    let count = children.iter().filter(|c| c.contains(x, y)).count();
                    assert_eq!(
                        count, 1,
                        "{}x{}: point ({},{}) is in {} children (expected exactly 1)",
                        size, size, x, y, count
                    );
                }
            }
        }
    }

    #[test]
    fn every_point_in_exactly_one_child_rectangular() {
        let test_sizes = [(64, 32), (32, 64), (100, 50), (602, 559)];

        for (width, height) in test_sizes {
            let mut root = QuadtreeCell::new_root((width, height));

            if !root.can_subdivide() {
                continue;
            }

            root.subdivide();
            let children = root.children.as_ref().unwrap();

            // Sample points to avoid O(n²) for large dimensions
            let step = ((width.max(height) / 50) as usize).max(1);

            for x in (0..width).step_by(step) {
                for y in (0..height).step_by(step) {
                    let count = children.iter().filter(|c| c.contains(x, y)).count();
                    assert_eq!(
                        count, 1,
                        "{}x{}: point ({},{}) is in {} children (expected exactly 1)",
                        width, height, x, y, count
                    );
                }
            }
        }
    }

    // =========================================================================
    // Mathematical Invariant 3: Containment
    // =========================================================================

    #[test]
    fn all_children_within_parent_bounds() {
        let test_sizes = [(64, 64), (33, 33), (602, 559)];

        for (width, height) in test_sizes {
            let mut root = QuadtreeCell::new_root((width, height));

            if !root.can_subdivide() {
                continue;
            }

            root.subdivide();
            let children = root.children.as_ref().unwrap();

            for child in children.iter() {
                assert!(
                    child.bounds.x >= root.bounds.x,
                    "Child x {} < parent x {}",
                    child.bounds.x,
                    root.bounds.x
                );
                assert!(
                    child.bounds.y >= root.bounds.y,
                    "Child y {} < parent y {}",
                    child.bounds.y,
                    root.bounds.y
                );
                assert!(
                    child.bounds.x + child.bounds.width <= root.bounds.x + root.bounds.width,
                    "Child right edge exceeds parent"
                );
                assert!(
                    child.bounds.y + child.bounds.height <= root.bounds.y + root.bounds.height,
                    "Child bottom edge exceeds parent"
                );
            }
        }
    }

    // =========================================================================
    // Mathematical Invariant 4: Boundary Alignment
    // =========================================================================

    #[test]
    fn child_boundaries_align_perfectly() {
        let test_sizes = [(32, 32), (33, 33), (64, 65), (602, 559)];

        for (width, height) in test_sizes {
            let mut root = QuadtreeCell::new_root((width, height));

            if !root.can_subdivide() {
                continue;
            }

            root.subdivide();
            let c = root.children.as_ref().unwrap();

            // Index mapping: [TL=0, TR=1, BL=2, BR=3]

            // Horizontal alignment: TL right edge == TR left edge
            assert_eq!(
                c[0].bounds.x + c[0].bounds.width,
                c[1].bounds.x,
                "{}x{}: TL right edge {} != TR left edge {}",
                width,
                height,
                c[0].bounds.x + c[0].bounds.width,
                c[1].bounds.x
            );

            // BL right edge == BR left edge
            assert_eq!(
                c[2].bounds.x + c[2].bounds.width,
                c[3].bounds.x,
                "{}x{}: BL right edge != BR left edge",
                width,
                height
            );

            // Vertical alignment: TL bottom == BL top
            assert_eq!(
                c[0].bounds.y + c[0].bounds.height,
                c[2].bounds.y,
                "{}x{}: TL bottom {} != BL top {}",
                width,
                height,
                c[0].bounds.y + c[0].bounds.height,
                c[2].bounds.y
            );

            // TR bottom == BR top
            assert_eq!(
                c[1].bounds.y + c[1].bounds.height,
                c[3].bounds.y,
                "{}x{}: TR bottom != BR top",
                width,
                height
            );

            // Full horizontal coverage
            assert_eq!(
                c[0].bounds.width + c[1].bounds.width,
                width,
                "{}x{}: horizontal coverage {} != {}",
                width,
                height,
                c[0].bounds.width + c[1].bounds.width,
                width
            );

            // Full vertical coverage
            assert_eq!(
                c[0].bounds.height + c[2].bounds.height,
                height,
                "{}x{}: vertical coverage {} != {}",
                width,
                height,
                c[0].bounds.height + c[2].bounds.height,
                height
            );
        }
    }

    // =========================================================================
    // Mathematical Invariant 5: Recursive Preservation
    // =========================================================================

    #[test]
    fn recursive_subdivision_preserves_area() {
        let mut root = QuadtreeCell::new_root((602, 559));
        let expected_area = 602 * 559;

        subdivide_to_depth(&mut root, 4);

        let mut leaves = Vec::new();
        root.collect_leaves(&mut leaves);

        let total_area: u32 = leaves.iter().map(|l| l.bounds.area()).sum();
        assert_eq!(
            total_area, expected_area,
            "Total leaf area {} != original area {}",
            total_area, expected_area
        );
    }

    #[test]
    fn recursive_subdivision_no_gaps_sampled() {
        let mut root = QuadtreeCell::new_root((602, 559));
        subdivide_to_depth(&mut root, 4);

        let mut leaves = Vec::new();
        root.collect_leaves(&mut leaves);

        // Sample points across the canvas
        for x in (0..602).step_by(7) {
            for y in (0..559).step_by(7) {
                let count = leaves.iter().filter(|l| l.contains(x, y)).count();
                assert_eq!(
                    count, 1,
                    "Point ({},{}) is in {} leaves (expected 1)",
                    x, y, count
                );
            }
        }
    }

    #[test]
    fn recursive_subdivision_preserves_invariants_at_each_level() {
        let mut root = QuadtreeCell::new_root((256, 256));

        for depth in 1..=4 {
            subdivide_to_depth(&mut root, depth);

            let mut leaves = Vec::new();
            root.collect_leaves(&mut leaves);

            // Area conservation
            let total_area: u32 = leaves.iter().map(|l| l.bounds.area()).sum();
            assert_eq!(
                total_area,
                256 * 256,
                "Area not conserved at depth {}",
                depth
            );

            // All leaves at expected depth or stopped early due to limits
            for leaf in &leaves {
                assert!(
                    leaf.depth <= depth,
                    "Leaf depth {} > target {}",
                    leaf.depth,
                    depth
                );
            }
        }
    }

    // =========================================================================
    // Mathematical Invariant 6: Limit Enforcement
    // =========================================================================

    #[test]
    fn cannot_subdivide_past_max_depth() {
        let mut cell = QuadtreeCell::new(Bounds::new(0, 0, 1024, 1024), MAX_DEPTH);
        assert!(!cell.can_subdivide(), "Should not subdivide at MAX_DEPTH");
        assert!(
            !cell.subdivide(),
            "Subdivide should return false at MAX_DEPTH"
        );
    }

    #[test]
    fn cannot_subdivide_below_min_cell_size() {
        // Cell that would create children smaller than MIN_CELL_SIZE
        let small_size = MIN_CELL_SIZE * 2 - 1;
        let cell = QuadtreeCell::new_root((small_size, small_size));
        assert!(
            !cell.can_subdivide(),
            "Should not subdivide when children would be < MIN_CELL_SIZE"
        );
    }

    #[test]
    fn can_subdivide_at_exactly_min_size() {
        // Cell that creates children exactly at MIN_CELL_SIZE
        let exact_size = MIN_CELL_SIZE * 2;
        let mut cell = QuadtreeCell::new_root((exact_size, exact_size));
        assert!(
            cell.can_subdivide(),
            "Should be able to subdivide to exactly MIN_CELL_SIZE"
        );
        assert!(cell.subdivide());

        let children = cell.children.as_ref().unwrap();
        for child in children.iter() {
            assert!(
                child.bounds.width >= MIN_CELL_SIZE && child.bounds.height >= MIN_CELL_SIZE,
                "Child size below minimum"
            );
        }
    }

    #[test]
    fn subdivision_stops_at_limits_during_recursive() {
        let mut root = QuadtreeCell::new_root((128, 128));
        subdivide_to_depth(&mut root, 100); // Way past limits

        let mut leaves = Vec::new();
        root.collect_leaves(&mut leaves);

        for leaf in &leaves {
            assert!(leaf.depth <= MAX_DEPTH, "Leaf exceeded MAX_DEPTH");
            if leaf.depth < MAX_DEPTH {
                // If not at max depth, must be at min size
                assert!(
                    leaf.bounds.width < MIN_CELL_SIZE * 2 || leaf.bounds.height < MIN_CELL_SIZE * 2,
                    "Leaf stopped early but not at size limit"
                );
            }
        }
    }

    // =========================================================================
    // Additional Edge Cases
    // =========================================================================

    #[test]
    fn double_subdivide_returns_false() {
        let mut root = QuadtreeCell::new_root((64, 64));
        assert!(root.subdivide());
        assert!(!root.subdivide(), "Second subdivide should return false");
    }

    #[test]
    fn contains_boundary_conditions() {
        let bounds = Bounds::new(10, 20, 30, 40);

        // Inside
        assert!(bounds.contains(10, 20)); // Top-left corner (inclusive)
        assert!(bounds.contains(25, 40)); // Middle

        // Outside - right and bottom edges are exclusive
        assert!(!bounds.contains(40, 30)); // Right edge (exclusive)
        assert!(!bounds.contains(25, 60)); // Bottom edge (exclusive)
        assert!(!bounds.contains(9, 20)); // Left of bounds
        assert!(!bounds.contains(10, 19)); // Above bounds
    }

    #[test]
    fn collect_leaves_on_leaf_returns_self() {
        let root = QuadtreeCell::new_root((64, 64));
        let mut leaves = Vec::new();
        root.collect_leaves(&mut leaves);
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].bounds, root.bounds);
    }
}
