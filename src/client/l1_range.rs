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
        std::cmp::max(origin - l1_range_blocks, self.l1_start as u64)
    }

    pub fn l1_equals(&self, other: &L1Range) -> bool {
        self.l1_start == other.l1_start && self.l1_end == other.l1_end
    }
}
