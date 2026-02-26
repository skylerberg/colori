use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn draw_from_deck<R: Rng>(
    deck: &mut UnorderedCards,
    discard: &mut UnorderedCards,
    dest: &mut UnorderedCards,
    count: usize,
    rng: &mut R,
) {
    let count = count as u32;
    let deck_len = deck.len();
    if deck_len >= count {
        let drawn = deck.draw_multiple(count, rng);
        *dest = dest.union(drawn);
    } else {
        // Take everything from deck directly (no draw_multiple needed)
        *dest = dest.union(*deck);
        *deck = UnorderedCards::new();
        let remaining = count - deck_len;
        if remaining > 0 && !discard.is_empty() {
            *deck = *discard;
            *discard = UnorderedCards::new();
            let available = deck.len().min(remaining);
            if available > 0 {
                let drawn = deck.draw_multiple(available, rng);
                *dest = dest.union(drawn);
            }
        }
    }
}
