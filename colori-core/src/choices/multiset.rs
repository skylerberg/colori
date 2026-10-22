use crate::types::*;
use crate::unordered_cards::UnorderedCards;

/// Count occurrences of each card type in an UnorderedCards bitset,
/// returning a sorted array of (Card, count) pairs and the number of distinct types.
///
/// A workshop holds at most five cards, so this is far more room than a tally
/// can need; `NUM_CARDS` is what bounds it whatever the card set.
pub(super) fn count_card_types(
    mask: UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([Card; NUM_CARDS], [u8; NUM_CARDS], usize) {
    let mut card_types = [Card::BasicRed; NUM_CARDS];
    let mut type_counts = [0u8; NUM_CARDS];
    let mut len = 0usize;
    let mut seen: u64 = 0;
    for id in mask.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit == 0 {
            seen |= bit;
            card_types[len] = card;
            type_counts[len] = 1;
            len += 1;
        } else {
            for i in 0..len {
                if card_types[i] == card {
                    type_counts[i] += 1;
                    break;
                }
            }
        }
    }
    // Sort by card discriminant for deterministic output
    for i in 1..len {
        let mut j = i;
        while j > 0 && (card_types[j] as usize) < (card_types[j - 1] as usize) {
            card_types.swap(j, j - 1);
            type_counts.swap(j, j - 1);
            j -= 1;
        }
    }
    (card_types, type_counts, len)
}

/// Enumerate all non-empty subsets of a card-type multiset whose size is within
/// `fewest..=most`. Produces unique sorted WorkshopCards entries without needing
/// deduplication.
pub(super) fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    fewest: usize,
    most: usize,
    current_subset: &mut WorkshopCards,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(WorkshopCards) -> Choice,
) {
    // Nothing further down can reach the floor once too few cards are left.
    let available: usize = counts.iter().map(|&c| c as usize).sum();
    if current_subset.len() + available < fewest {
        return;
    }
    if types.is_empty() || most == 0 {
        if !current_subset.is_empty() {
            choices.push(make_choice(current_subset.clone()));
        }
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = most.min(count);
    let base_len = current_subset.len();
    for take in 0..=max_take {
        enumerate_multiset_subsets(
            &types[1..],
            &counts[1..],
            fewest,
            most - take,
            current_subset,
            choices,
            make_choice,
        );
        // One more copy goes on for the next pass; the last push would only
        // overrun the picks granted before being truncated again.
        if take < max_take {
            current_subset.push(card);
        }
    }
    current_subset.truncate(base_len);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sizes of the subsets offered for a workshop of `types`, in the order
    /// they are offered.
    fn subset_sizes(types: &[Card], counts: &[u8], fewest: usize, most: usize) -> Vec<usize> {
        let mut choices = Vec::new();
        enumerate_multiset_subsets(
            types,
            counts,
            fewest,
            most,
            &mut WorkshopCards::new(),
            &mut choices,
            &|card_types| Choice::Workshop { card_types },
        );
        choices
            .into_iter()
            .map(|choice| match choice {
                Choice::Workshop { card_types } => card_types.len(),
                _ => unreachable!("only workshop choices are made here"),
            })
            .collect()
    }

    /// Three copies of one card is the most of any card a pick can take, and
    /// three is all a choice holds: the last copy the recursion pushes is never
    /// part of a subset, so it must not be pushed at all.
    #[test]
    fn taking_the_most_copies_stays_within_a_choice() {
        let mut every = subset_sizes(&[Card::Realgar], &[3], 1, 3);
        every.sort_unstable();
        assert_eq!(every, [1, 2, 3]);

        assert_eq!(subset_sizes(&[Card::Realgar], &[3], 3, 3), [3]);
    }

    #[test]
    fn every_take_up_to_the_picks_is_offered() {
        let mut every = subset_sizes(&[Card::Realgar, Card::Woad], &[2, 1], 1, 2);
        every.sort_unstable();
        every.dedup();
        assert_eq!(every, [1, 2]);

        let mut exact = subset_sizes(&[Card::Realgar, Card::Woad], &[2, 1], 2, 2);
        exact.sort_unstable();
        exact.dedup();
        assert_eq!(exact, [2]);
    }

    /// Only the sizes between the two bounds are offered, and a floor there are
    /// not enough cards for offers nothing rather than a short subset.
    #[test]
    fn only_takes_between_the_bounds_are_offered() {
        let types = [Card::Realgar, Card::Woad, Card::Azurite];
        let mut between = subset_sizes(&types, &[1, 1, 1], 2, 3);
        between.sort_unstable();
        assert_eq!(between, [2, 2, 2, 3]);

        // Two copies of one card are still one type, so the floor can be met
        // from a single type's copies.
        assert_eq!(subset_sizes(&[Card::Realgar], &[3], 2, 2), [2]);
        assert!(subset_sizes(&types, &[1, 1, 1], 4, 4).is_empty());
    }
}
