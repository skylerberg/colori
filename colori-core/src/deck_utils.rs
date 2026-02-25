use crate::types::CardInstance;
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

pub fn shuffle<T: Clone, R: Rng>(array: &[T], rng: &mut R) -> Vec<T> {
    let mut result = array.to_vec();
    shuffle_in_place(&mut result, rng);
    result
}

pub fn draw_from_deck<R: Rng>(
    deck: &mut Vec<CardInstance>,
    discard: &mut Vec<CardInstance>,
    dest: &mut Vec<CardInstance>,
    count: usize,
    rng: &mut R,
) {
    for _ in 0..count {
        if deck.is_empty() {
            if discard.is_empty() {
                break;
            }
            shuffle_in_place(discard, rng);
            std::mem::swap(deck, discard);
        }
        if let Some(card) = deck.pop() {
            dest.push(card);
        }
    }
}
