/// Contains the range of L1 blocks on which the L2 state was updated
/// Start and End are inclusive
/// Start and End blocks of L1 and L2 are synchronized,
///     meaning that l1_start is the block on which l2_start state was updated
///     and l1_end is the block on which l2_end state was updated
#[derive(Debug, Clone)]
pub struct L1Range {
    pub l1_start: i64,
    pub l1_end: i64,
    pub l2_start: i64,
    pub l2_end: i64,
}

impl L1Range {
    pub fn new(l1_start: i64, l1_end: i64, l2_start: i64, l2_end: i64) -> Self {
        Self { l1_start, l1_end, l2_start, l2_end }
    }

    pub fn next_end(&self, origin: u64, l1_range_blocks: u64) -> u64 {
        std::cmp::min(origin + l1_range_blocks, self.l1_end as u64)
    }

    pub fn prev_start(&self, origin: u64, l1_range_blocks: u64) -> u64 {
        std::cmp::max(
            origin.saturating_sub(l1_range_blocks),
            self.l1_start as u64,
        )
    }

    pub fn l1_equals(&self, other: &L1Range) -> bool {
        self.l1_start == other.l1_start && self.l1_end == other.l1_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let range = L1Range::new(10, 20, 100, 200);
        assert_eq!(range.l1_start, 10);
        assert_eq!(range.l1_end, 20);
        assert_eq!(range.l2_start, 100);
        assert_eq!(range.l2_end, 200);
    }

    #[test]
    fn test_next_end_within_range() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 20u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 70); // origin + l1_range_blocks = 50 + 20 = 70 < 100
    }

    #[test]
    fn test_next_end_exceeds_range() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 90u64;
        let l1_range_blocks = 20u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 100); // origin + l1_range_blocks = 90 + 20 = 110 > 100, so min is 100
    }

    #[test]
    fn test_next_end_at_boundary() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 80u64;
        let l1_range_blocks = 20u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 100); // origin + l1_range_blocks = 80 + 20 = 100 == 100
    }

    #[test]
    fn test_next_end_origin_at_start() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 10u64;
        let l1_range_blocks = 5u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 15); // origin + l1_range_blocks = 10 + 5 = 15 < 100
    }

    #[test]
    fn test_prev_start_within_range() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 20u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 30); // origin - l1_range_blocks = 50 - 20 = 30 > 10
    }

    #[test]
    fn test_prev_start_below_range() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 15u64;
        let l1_range_blocks = 20u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 10); // origin - l1_range_blocks = 15 - 20 = -5 (underflow), but max is 10
    }

    #[test]
    fn test_prev_start_at_boundary() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 30u64;
        let l1_range_blocks = 20u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 10); // origin - l1_range_blocks = 30 - 20 = 10 == 10
    }

    #[test]
    fn test_prev_start_origin_at_end() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 100u64;
        let l1_range_blocks = 30u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 70); // origin - l1_range_blocks = 100 - 30 = 70 > 10
    }

    #[test]
    fn test_l1_equals_same_range() {
        let range1 = L1Range::new(10, 20, 100, 200);
        let range2 = L1Range::new(10, 20, 300, 400);

        assert!(range1.l1_equals(&range2)); // Same l1_start and l1_end, different l2 values
    }

    #[test]
    fn test_l1_equals_different_l1_start() {
        let range1 = L1Range::new(10, 20, 100, 200);
        let range2 = L1Range::new(15, 20, 100, 200);

        assert!(!range1.l1_equals(&range2)); // Different l1_start
    }

    #[test]
    fn test_l1_equals_different_l1_end() {
        let range1 = L1Range::new(10, 20, 100, 200);
        let range2 = L1Range::new(10, 25, 100, 200);

        assert!(!range1.l1_equals(&range2)); // Different l1_end
    }

    #[test]
    fn test_l1_equals_different_both() {
        let range1 = L1Range::new(10, 20, 100, 200);
        let range2 = L1Range::new(15, 25, 100, 200);

        assert!(!range1.l1_equals(&range2)); // Different l1_start and l1_end
    }

    #[test]
    fn test_l1_equals_identical() {
        let range1 = L1Range::new(10, 20, 100, 200);
        let range2 = L1Range::new(10, 20, 100, 200);

        assert!(range1.l1_equals(&range2)); // Completely identical
    }

    #[test]
    fn test_next_end_with_zero_range_blocks() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 0u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 50); // origin + 0 = 50 < 100
    }

    #[test]
    fn test_prev_start_with_zero_range_blocks() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 0u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 50); // origin - 0 = 50 > 10
    }

    #[test]
    fn test_next_end_with_large_range_blocks() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 1000u64;

        let result = range.next_end(origin, l1_range_blocks);
        assert_eq!(result, 100); // origin + 1000 = 1050 > 100, so min is 100
    }

    #[test]
    fn test_prev_start_with_large_range_blocks() {
        let range = L1Range::new(10, 100, 100, 200);
        let origin = 50u64;
        let l1_range_blocks = 1000u64;

        let result = range.prev_start(origin, l1_range_blocks);
        assert_eq!(result, 10); // origin - 1000 would underflow, but max is 10
    }
}
