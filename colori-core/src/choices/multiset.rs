use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use smallvec::SmallVec;

/// Count occurrences of each card type in an UnorderedCards bitset,
/// returning a sorted array of (Card, count) pairs and the number of distinct types.
pub(super) fn count_card_types(
    mask: UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([Card; 46], [u8; 46], usize) {
    let mut card_types = [Card::BasicRed; 46];
    let mut type_counts = [0u8; 46];
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
/// Produces unique sorted SmallVec<[Card; 4]> entries without needing deduplication.
pub(super) fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current_subset: &mut SmallVec<[Card; 4]>,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(SmallVec<[Card; 4]>) -> Choice,
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
        current_subset.push(card);
    }
    current_subset.truncate(base_len);
}
