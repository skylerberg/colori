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
    for _ in 0..count {
        if deck.is_empty() {
            if discard.is_empty() {
                break;
            }
            *deck = *discard;
            *discard = UnorderedCards::new();
        }
        if let Some(id) = deck.draw(rng) {
            dest.insert(id);
        }
    }
}
