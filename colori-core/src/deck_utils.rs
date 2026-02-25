use crate::types::{CardInstance, PlayerState};
use rand::Rng;

pub fn shuffle_in_place<T, R: Rng>(array: &mut Vec<T>, rng: &mut R) {
    let len = array.len();
    if len <= 1 {
        return;
    }
    for i in (1..len).rev() {
        let j = rng.gen_range(0..=i);
        array.swap(i, j);
    }
}

pub fn shuffle<T: Clone, R: Rng>(array: &[T], rng: &mut R) -> Vec<T> {
    let mut result = array.to_vec();
    shuffle_in_place(&mut result, rng);
    result
}

pub fn draw_from_deck<R: Rng>(
    player: &mut PlayerState,
    count: usize,
    rng: &mut R,
) -> Vec<CardInstance> {
    let mut drawn = Vec::with_capacity(count);
    for _ in 0..count {
        if player.deck.is_empty() {
            if player.discard.is_empty() {
                break;
            }
            shuffle_in_place(&mut player.discard, rng);
            std::mem::swap(&mut player.deck, &mut player.discard);
        }
        if let Some(card) = player.deck.pop() {
            drawn.push(card);
        }
    }
    drawn
}
