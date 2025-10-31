use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
};
use eyre::Result;

/// Convert bytes to a Felt value, handling leading zeros according to RPC spec
///
/// The RPC spec requires that FELT values don't have leading zeros in their hex representation
pub fn as_felt(bytes: &[u8]) -> Result<Felt> {
    // RPC spec FELT regex: leading zeroes are not allowed
    let hex = hex::encode(bytes);
    let hex = hex.chars().skip_while(|c| c == &'0').collect::<String>();
    let hex = format!("0x{hex}");
    let felt = Felt::try_new(&hex)?;
    Ok(felt)
}

/// Approximate the L1 block number for a given L2 block number
pub fn approximate_l1_block(
    l1_range: &L1Range,
    block_number: i64,
) -> Result<i64> {
    let l1_length = l1_range.l1_end - l1_range.l1_start;
    let l2_length = l1_range.l2_end - l1_range.l2_start;
    if l1_length == 0 || l2_length == 0 {
        eyre::bail!("L1 range is empty");
    }
    let l2_to_l1_ratio = l1_length as f64 / l2_length as f64;
    let l1_block = (block_number - l1_range.l2_start) as f64 * l2_to_l1_ratio
        + l1_range.l1_start as f64;
    Ok(l1_block as i64)
}

pub fn find_l1_sub_range(
    full_l1_range: L1Range,
    states: &[(State, u64)],
    target_block_number: i64,
    new_l1_ranges: &mut Vec<L1Range>,
) -> Result<(Option<L1Range>, bool)> {
    let mut prev_state: Option<(State, u64)> = None;
    let mut target_sub_range: Option<L1Range> = None;
    let mut is_target_below_range = false;

    states.iter().for_each(|(state, l1_block_number)| {
        // states are sorted by L1 block number
        if let Some((prev_state, prev_l1_block_number)) = &prev_state {
            new_l1_ranges.push(L1Range::new(
                *prev_l1_block_number as i64,
                *l1_block_number as i64,
                prev_state.block_number,
                state.block_number,
            ));
        } else {
            new_l1_ranges.push(L1Range::new(
                full_l1_range.l1_start,
                *l1_block_number as i64,
                full_l1_range.l2_start,
                state.block_number,
            ));
        }

        if target_block_number < state.block_number
            && target_sub_range.is_none()
        {
            // the found state is above the target block number
            is_target_below_range = true;
            target_sub_range = Some(L1Range::new(
                full_l1_range.l1_start,
                *l1_block_number as i64,
                full_l1_range.l2_start,
                state.block_number,
            ));
        } else if target_block_number == state.block_number {
            // the state block was committed on L1
            target_sub_range = Some(L1Range::new(
                *l1_block_number as i64,
                *l1_block_number as i64,
                state.block_number,
                state.block_number,
            ));
        }
        prev_state = Some((state.clone(), *l1_block_number));
    });
    if let Some((state, l1_block_number)) = &prev_state {
        let range = L1Range::new(
            *l1_block_number as i64,
            full_l1_range.l1_end,
            state.block_number,
            full_l1_range.l2_end,
        );
        if target_sub_range.is_none() {
            // the found state is below the target block number
            is_target_below_range = false;
            target_sub_range = Some(range.clone());
        }
        new_l1_ranges.push(range);
    }
    Ok((target_sub_range, is_target_below_range))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approximate_l1_block_same_range() {
        let l1_range = L1Range::new(1, 100, 1, 100);
        let block_number = 50;
        let l1_block = approximate_l1_block(&l1_range, block_number).unwrap();
        assert_eq!(l1_block, 50);
    }

    #[test]
    fn test_approximate_l1_block_different_range_of_same_length() {
        let l1_range = L1Range::new(1, 100, 101, 200);
        let block_number = 150;
        let l1_block = approximate_l1_block(&l1_range, block_number).unwrap();
        assert_eq!(l1_block, 50);
    }

    #[test]
    fn test_approximate_l1_block_different_range_of_different_length() {
        let l1_range = L1Range::new(1, 100, 1, 200);
        let block_number = 100;
        let l1_block = approximate_l1_block(&l1_range, block_number).unwrap();
        assert_eq!(l1_block, 50);
    }

    #[test]
    fn test_approximate_l1_block_empty_range() {
        let l1_range = L1Range::new(1, 1, 1, 1);
        let block_number = 1;
        let l1_block =
            approximate_l1_block(&l1_range, block_number).unwrap_err();
        assert_eq!(l1_block.to_string(), "L1 range is empty");
    }

    /// ------------------------------------------------------------
    /// Tests for find_l1_sub_range
    /// ------------------------------------------------------------

    // Helper function to create a State for testing
    fn create_state(block_number: i64) -> State {
        let hash = Felt::try_new(&format!("0x{:064x}", block_number)).unwrap();
        State::new(block_number, hash.clone(), hash)
    }

    #[test]
    fn test_find_l1_sub_range_empty_states() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let states: Vec<(State, u64)> = vec![];
        let target_block_number = 150;
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_none());
        assert_eq!(result.1, false);
        assert_eq!(new_l1_ranges.len(), 0);
    }

    #[test]
    fn test_find_l1_sub_range_target_below_first_state() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(150);
        let states = vec![(state1, 50)];
        let target_block_number = 120; // Below state1.block_number
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 10);
        assert_eq!(target_sub_range.l1_end, 50);
        assert_eq!(target_sub_range.l2_start, 100);
        assert_eq!(target_sub_range.l2_end, 150);
        assert_eq!(result.1, true); // is_target_below_range

        // Should have two ranges: one for [l1_start, state1_l1] and one for [state1_l1, l1_end]
        assert_eq!(new_l1_ranges.len(), 2);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 50);
        assert_eq!(new_l1_ranges[1].l1_start, 50);
        assert_eq!(new_l1_ranges[1].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_matches_first_state() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(150);
        let states = vec![(state1, 50)];
        let target_block_number = 150; // Matches state1.block_number
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 50);
        assert_eq!(target_sub_range.l1_end, 50);
        assert_eq!(target_sub_range.l2_start, 150);
        assert_eq!(target_sub_range.l2_end, 150);
        assert_eq!(result.1, false); // Exact match, not below

        // Should have one range: [state1_l1, l1_end]
        assert_eq!(new_l1_ranges.len(), 2);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 50);
        assert_eq!(new_l1_ranges[1].l1_start, 50);
        assert_eq!(new_l1_ranges[1].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_above_all_states() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(150);
        let states = vec![(state1, 50)];
        let target_block_number = 180; // Above state1.block_number
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 50);
        assert_eq!(target_sub_range.l1_end, 100);
        assert_eq!(target_sub_range.l2_start, 150);
        assert_eq!(target_sub_range.l2_end, 200);
        assert_eq!(result.1, false); // is_target_below_range = false (above all states)

        // Should have one range: [state1_l1, l1_end]
        assert_eq!(new_l1_ranges.len(), 2);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 50);
        assert_eq!(new_l1_ranges[1].l1_start, 50);
        assert_eq!(new_l1_ranges[1].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_matches_middle_state() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(120);
        let state2 = create_state(150);
        let state3 = create_state(180);
        let states = vec![(state1, 30), (state2, 60), (state3, 90)];
        let target_block_number = 150; // Matches state2.block_number
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 60);
        assert_eq!(target_sub_range.l1_end, 60);
        assert_eq!(target_sub_range.l2_start, 150);
        assert_eq!(target_sub_range.l2_end, 150);
        assert_eq!(result.1, false); // Exact match

        // check ranges
        assert_eq!(new_l1_ranges.len(), 4);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 30);
        assert_eq!(new_l1_ranges[1].l1_start, 30);
        assert_eq!(new_l1_ranges[1].l1_end, 60);
        assert_eq!(new_l1_ranges[2].l1_start, 60);
        assert_eq!(new_l1_ranges[2].l1_end, 90);
        assert_eq!(new_l1_ranges[3].l1_start, 90);
        assert_eq!(new_l1_ranges[3].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_between_states() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(120);
        let state2 = create_state(180);
        let states = vec![(state1, 30), (state2, 70)];
        let target_block_number = 150; // Between state1 and state2
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        // Target is between states, so state2 is the first state above target
        // The function sets target_sub_range from full_l1_range.l1_start to state2's l1
        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 10);
        assert_eq!(target_sub_range.l1_end, 70);
        assert_eq!(target_sub_range.l2_start, 100);
        assert_eq!(target_sub_range.l2_end, 180);
        assert_eq!(result.1, true); // is_target_below_range = true because target < state2.block_number

        // check ranges
        assert_eq!(new_l1_ranges.len(), 3);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 30);
        assert_eq!(new_l1_ranges[1].l1_start, 30);
        assert_eq!(new_l1_ranges[1].l1_end, 70);
        assert_eq!(new_l1_ranges[2].l1_start, 70);
        assert_eq!(new_l1_ranges[2].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_between_first_second_state() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(150);
        let state2 = create_state(180);
        let states = vec![(state1, 50), (state2, 80)];
        let target_block_number = 165; // Between state1 and state2
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        // state1: target (165) < state1.block_number (150)? No
        // state2: prev_state exists, add [50, 80, 150, 180]
        //         target (165) < state2.block_number (180)? Yes, so set target_sub_range = [10, 80, 100, 180]
        // Finally: add [80, 100, 180, 200]

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 10);
        assert_eq!(target_sub_range.l1_end, 80);
        assert_eq!(target_sub_range.l2_start, 100);
        assert_eq!(target_sub_range.l2_end, 180);
        assert_eq!(result.1, true); // is_target_below_range = true because target < state2.block_number

        // check ranges
        assert_eq!(new_l1_ranges.len(), 3);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 50);
        assert_eq!(new_l1_ranges[1].l1_start, 50);
        assert_eq!(new_l1_ranges[1].l1_end, 80);
        assert_eq!(new_l1_ranges[2].l1_start, 80);
        assert_eq!(new_l1_ranges[2].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_target_before_first_state_multiple_states() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state1 = create_state(150);
        let state2 = create_state(170);
        let state3 = create_state(190);
        let states = vec![(state1, 40), (state2, 60), (state3, 80)];
        let target_block_number = 120; // Below state1.block_number
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        // state1: target (120) < state1.block_number (150)? Yes
        //   Add [10, 40, 100, 150] to new_l1_ranges
        //   Set target_sub_range = [10, 40, 100, 150], is_target_below_range = true
        // state2: prev_state exists, add [40, 60, 150, 170]
        // state3: prev_state exists, add [60, 80, 170, 190]
        // Finally: add [80, 100, 190, 200]

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 10);
        assert_eq!(target_sub_range.l1_end, 40);
        assert_eq!(target_sub_range.l2_start, 100);
        assert_eq!(target_sub_range.l2_end, 150);
        assert_eq!(result.1, true);

        // check ranges
        assert_eq!(new_l1_ranges.len(), 4);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 40);
        assert_eq!(new_l1_ranges[1].l1_start, 40);
        assert_eq!(new_l1_ranges[1].l1_end, 60);
        assert_eq!(new_l1_ranges[2].l1_start, 60);
        assert_eq!(new_l1_ranges[2].l1_end, 80);
        assert_eq!(new_l1_ranges[3].l1_start, 80);
        assert_eq!(new_l1_ranges[3].l1_end, 100);
    }

    #[test]
    fn test_find_l1_sub_range_single_state_target_matches() {
        let full_l1_range = L1Range::new(10, 100, 100, 200);
        let state = create_state(150);
        let states = vec![(state, 50)];
        let target_block_number = 150;
        let mut new_l1_ranges = Vec::new();

        let result = find_l1_sub_range(
            full_l1_range.clone(),
            &states,
            target_block_number,
            &mut new_l1_ranges,
        )
        .unwrap();

        assert!(result.0.is_some());
        let target_sub_range = result.0.unwrap();
        assert_eq!(target_sub_range.l1_start, 50);
        assert_eq!(target_sub_range.l1_end, 50);
        assert_eq!(target_sub_range.l2_start, 150);
        assert_eq!(target_sub_range.l2_end, 150);
        assert_eq!(result.1, false);

        // check ranges
        assert_eq!(new_l1_ranges.len(), 2);
        assert_eq!(new_l1_ranges[0].l1_start, 10);
        assert_eq!(new_l1_ranges[0].l1_end, 50);
        assert_eq!(new_l1_ranges[1].l1_start, 50);
        assert_eq!(new_l1_ranges[1].l1_end, 100);
    }
}
