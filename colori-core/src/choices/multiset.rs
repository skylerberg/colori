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

/// Enumerate all non-empty subsets of a card-type multiset up to max_size.
/// Produces unique sorted WorkshopCards entries without needing deduplication.
pub(super) fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current_subset: &mut WorkshopCards,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(WorkshopCards) -> Choice,
) {
    if types.is_empty() || max_remaining == 0 {
        if !current_subset.is_empty() {
            choices.push(make_choice(current_subset.clone()));
        }
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = max_remaining.min(count);
    let base_len = current_subset.len();
    for take in 0..=max_take {
        enumerate_multiset_subsets(
            &types[1..],
            &counts[1..],
            max_remaining - take,
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

/// Like `enumerate_multiset_subsets` but only emits subsets of exactly `max_remaining` size.
/// Used when we know the player should always workshop the maximum number of cards.
pub(super) fn enumerate_multiset_subsets_exact(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current_subset: &mut WorkshopCards,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(WorkshopCards) -> Choice,
) {
    if max_remaining == 0 {
        if !current_subset.is_empty() {
            choices.push(make_choice(current_subset.clone()));
        }
        return;
    }
    if types.is_empty() {
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = max_remaining.min(count);
    // Compute total available in remaining types (excluding current)
    let remaining_available: usize = counts[1..].iter().map(|&c| c as usize).sum();
    let base_len = current_subset.len();
    for take in 0..=max_take {
        // Prune: can we still reach exactly max_remaining with remaining types?
        let needed = max_remaining - take;
        if needed <= remaining_available {
            enumerate_multiset_subsets_exact(
                &types[1..],
                &counts[1..],
                needed,
                current_subset,
                choices,
                make_choice,
            );
        }
        // As above: the last push would only overrun the picks granted.
        if take < max_take {
            current_subset.push(card);
        }
    }
    current_subset.truncate(base_len);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sizes of the subsets each enumerator offers for a workshop of
    /// `types`, in the order it offers them.
    fn subset_sizes(types: &[Card], counts: &[u8], picks: usize, exact: bool) -> Vec<usize> {
        let mut choices = Vec::new();
        let make_choice = |card_types| Choice::Workshop { card_types };
        if exact {
            enumerate_multiset_subsets_exact(
                types,
                counts,
                picks,
                &mut WorkshopCards::new(),
                &mut choices,
                &make_choice,
            );
        } else {
            enumerate_multiset_subsets(
                types,
                counts,
                picks,
                &mut WorkshopCards::new(),
                &mut choices,
                &make_choice,
            );
        }
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
        let mut every = subset_sizes(&[Card::Realgar], &[3], 3, false);
        every.sort_unstable();
        assert_eq!(every, [1, 2, 3]);

        assert_eq!(subset_sizes(&[Card::Realgar], &[3], 3, true), [3]);
    }

    #[test]
    fn every_take_up_to_the_picks_is_offered() {
        let mut every = subset_sizes(&[Card::Realgar, Card::Woad], &[2, 1], 2, false);
        every.sort_unstable();
        every.dedup();
        assert_eq!(every, [1, 2]);

        let mut exact = subset_sizes(&[Card::Realgar, Card::Woad], &[2, 1], 2, true);
        exact.sort_unstable();
        exact.dedup();
        assert_eq!(exact, [2]);
    }
}
