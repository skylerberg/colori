use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn shuffle_in_place<T, R: Rng>(array: &mut Vec<T>, rng: &mut R) {
    let len = array.len();
    if len <= 1 {
        return;
    }
    for i in (1..len).rev() {
        let j = rng.random_range(0..=i);
        array.swap(i, j);
    }
}

pub fn draw_from_deck<R: Rng>(
    deck: &mut UnorderedCards,
    discard: &mut UnorderedCards,
    dest: &mut UnorderedCards,
    count: usize,
    rng: &mut R,
) {
    let mut remaining = count as u32;
    while remaining > 0 {
        if deck.is_empty() {
            if discard.is_empty() {
                break;
            }
            *deck = *discard;
            *discard = UnorderedCards::new();
        }
        let available = deck.len().min(remaining);
        let drawn = deck.draw_multiple(available, rng);
        *dest = dest.union(drawn);
        remaining -= available;
    }
}
