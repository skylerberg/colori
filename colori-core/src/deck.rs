//! A personal deck: an ordered queue of independently shuffled segments.
//!
//! Cards join the deck a whole workshop at a time, shuffled, at the bottom;
//! they are drawn from the top. So the deck is ordered *between* segments and
//! unordered *within* one — and nobody, the owner included, knows a segment's
//! internal order.
//!
//! Representing it as a queue of bitsets is therefore not a compromise but the
//! exact information state, which is what keeps the engine fast:
//!
//! * a draw is still a uniform sample from a bitset, as it was when the whole
//!   deck was one bag;
//! * determinization has nothing to do for player decks, because there is no
//!   hidden order stored to reshuffle. Storing a concrete card order instead
//!   would be both slower and *unsound* for ISMCTS — the search would see its
//!   own future draws unless every determinization reshuffled the deck.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::fixed_vec::FixedVec;
use crate::unordered_cards::UnorderedCards;

/// How many segments a deck can hold.
///
/// This is the exact structural bound for the standard game rather than a
/// round number: the only way to add a segment is to end a round, so six
/// rounds can create at most six on top of the starting deck. Measured over
/// 8 000 random games, no deck exceeded five, and 99.4% held three or fewer.
///
/// A longer game — `--max-rounds` in solo simulation — can outrun it, and a
/// push onto a full deck then merges into the deepest segment. That loses the
/// order between the last two pushes and nothing else, at the bottom of a deck
/// already deeper than the remaining rounds can draw.
pub const MAX_DECK_SEGMENTS: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Deck {
    /// Front segment first. Never contains an empty segment.
    segments: FixedVec<UnorderedCards, MAX_DECK_SEGMENTS>,
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl Deck {
    pub fn new() -> Self {
        Deck {
            segments: FixedVec::new(),
        }
    }

    /// A deck of one shuffled pile, for setup and tests.
    pub fn from_cards(cards: UnorderedCards) -> Self {
        let mut deck = Self::new();
        deck.push_bottom(cards);
        deck
    }

    pub fn len(&self) -> u32 {
        self.segments.iter().map(|s| s.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// How many independently shuffled piles the deck is made of.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Front segment first.
    pub fn segments(&self) -> &[UnorderedCards] {
        &self.segments
    }

    /// Every card in the deck, with the segment structure flattened away.
    /// For scoring and display, which care what is in the deck and not when it
    /// will come up.
    pub fn cards(&self) -> UnorderedCards {
        self.segments
            .iter()
            .fold(UnorderedCards::new(), |acc, s| acc.union(*s))
    }

    pub fn contains(&self, id: u8) -> bool {
        self.segments.iter().any(|s| s.contains(id))
    }

    /// Remove a specific card, wherever it sits. Used by log replay, which
    /// knows which cards a recorded draw produced.
    pub fn remove(&mut self, id: u8) -> bool {
        for i in 0..self.segments.len() {
            if self.segments[i].contains(id) {
                self.segments[i].remove(id);
                if self.segments[i].is_empty() {
                    self.segments.remove(i);
                }
                return true;
            }
        }
        false
    }

    /// Put `cards` on the bottom as a new shuffled segment. Empty pushes are
    /// dropped, so an empty segment never occupies a slot.
    pub fn push_bottom(&mut self, cards: UnorderedCards) {
        if cards.is_empty() {
            return;
        }
        if self.segments.len() == MAX_DECK_SEGMENTS {
            // See MAX_DECK_SEGMENTS: unreachable in a six-round game. Merging
            // into the deepest segment loses the order between the last two
            // pushes and nothing else.
            let last = self.segments.len() - 1;
            self.segments[last] = self.segments[last].union(cards);
            return;
        }
        self.segments.push(cards);
    }

    /// Draw up to `count` cards into `destination`, taking from the front
    /// segment and moving on when it runs out. Returns how many were drawn,
    /// which is short of `count` only when the deck ran dry — there is no
    /// discard pile to reshuffle.
    pub fn draw_into<R: Rng>(
        &mut self,
        destination: &mut UnorderedCards,
        count: u32,
        rng: &mut R,
    ) -> u32 {
        let mut remaining = count;
        while remaining > 0 && !self.segments.is_empty() {
            let available = self.segments[0].len();
            if available <= remaining {
                *destination = destination.union(self.segments[0]);
                self.segments.remove(0);
                remaining -= available;
            } else {
                let drawn = self.segments[0].draw_multiple(remaining, rng);
                *destination = destination.union(drawn);
                remaining = 0;
            }
        }
        count - remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Card;
    use crate::unordered_cards::set_card_registry;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn cards(ids: &[u8]) -> UnorderedCards {
        let mut set = UnorderedCards::new();
        for &id in ids {
            set.insert(id);
        }
        set
    }

    #[test]
    fn empty_deck_draws_nothing() {
        let mut deck = Deck::new();
        let mut hand = UnorderedCards::new();
        let mut rng = WyRand::seed_from_u64(1);
        assert_eq!(deck.draw_into(&mut hand, 5, &mut rng), 0);
        assert!(hand.is_empty());
        assert!(deck.is_empty());
    }

    #[test]
    fn a_dry_deck_draws_fewer_rather_than_reshuffling() {
        let mut deck = Deck::from_cards(cards(&[1, 2]));
        let mut hand = UnorderedCards::new();
        let mut rng = WyRand::seed_from_u64(1);
        assert_eq!(deck.draw_into(&mut hand, 5, &mut rng), 2);
        assert_eq!(hand.len(), 2);
        assert!(deck.is_empty());
    }

    #[test]
    fn pushing_empty_does_not_create_a_segment() {
        let mut deck = Deck::new();
        deck.push_bottom(UnorderedCards::new());
        assert_eq!(deck.segment_count(), 0);
    }

    /// The whole point of the type: a card put on the bottom is not drawn
    /// while anything above it remains.
    #[test]
    fn the_front_segment_is_exhausted_before_the_next_is_touched() {
        let mut rng = WyRand::seed_from_u64(7);
        for _ in 0..200 {
            let mut deck = Deck::from_cards(cards(&[1, 2, 3]));
            deck.push_bottom(cards(&[10, 11, 12]));

            let mut hand = UnorderedCards::new();
            assert_eq!(deck.draw_into(&mut hand, 3, &mut rng), 3);
            assert_eq!(hand, cards(&[1, 2, 3]), "bottom cards were drawn too early");

            let mut rest = UnorderedCards::new();
            deck.draw_into(&mut rest, 3, &mut rng);
            assert_eq!(rest, cards(&[10, 11, 12]));
        }
    }

    /// A draw that spans a boundary takes all of the front segment and a
    /// uniform sample of the next.
    #[test]
    fn a_draw_spanning_segments_takes_the_front_whole() {
        let mut seen_10 = false;
        let mut seen_11 = false;
        let mut seen_12 = false;
        let mut rng = WyRand::seed_from_u64(99);
        for _ in 0..200 {
            let mut deck = Deck::from_cards(cards(&[1, 2]));
            deck.push_bottom(cards(&[10, 11, 12]));

            let mut hand = UnorderedCards::new();
            assert_eq!(deck.draw_into(&mut hand, 3, &mut rng), 3);
            assert!(hand.contains(1) && hand.contains(2));
            assert_eq!(hand.len(), 3);
            seen_10 |= hand.contains(10);
            seen_11 |= hand.contains(11);
            seen_12 |= hand.contains(12);
            assert_eq!(deck.len(), 2);
            assert_eq!(deck.segment_count(), 1);
        }
        assert!(seen_10 && seen_11 && seen_12, "the spanning draw is not uniform");
    }

    #[test]
    fn removing_the_last_card_of_a_segment_drops_it() {
        let mut deck = Deck::from_cards(cards(&[1]));
        deck.push_bottom(cards(&[2, 3]));
        assert!(deck.remove(1));
        assert_eq!(deck.segment_count(), 1);
        assert_eq!(deck.segments()[0], cards(&[2, 3]));
        assert!(!deck.remove(1));
    }

    #[test]
    fn remove_finds_cards_in_later_segments() {
        let mut deck = Deck::from_cards(cards(&[1, 2]));
        deck.push_bottom(cards(&[3, 4]));
        assert!(deck.remove(4));
        assert_eq!(deck.len(), 3);
        assert!(!deck.contains(4));
        assert_eq!(deck.segment_count(), 2);
    }

    #[test]
    fn overflowing_the_segment_cap_merges_into_the_deepest() {
        let mut deck = Deck::new();
        for i in 0..MAX_DECK_SEGMENTS {
            deck.push_bottom(cards(&[i as u8]));
        }
        assert_eq!(deck.segment_count(), MAX_DECK_SEGMENTS);

        deck.push_bottom(cards(&[100]));
        assert_eq!(deck.segment_count(), MAX_DECK_SEGMENTS);
        assert_eq!(deck.len(), MAX_DECK_SEGMENTS as u32 + 1);
        let deepest = deck.segments()[MAX_DECK_SEGMENTS - 1];
        assert!(deepest.contains(100));
        assert!(deepest.contains(MAX_DECK_SEGMENTS as u8 - 1));
    }

    #[test]
    fn cards_and_len_see_every_segment() {
        let mut deck = Deck::from_cards(cards(&[1, 2]));
        deck.push_bottom(cards(&[3]));
        assert_eq!(deck.cards(), cards(&[1, 2, 3]));
        assert_eq!(deck.len(), 3);
    }

    #[test]
    fn serde_preserves_the_segment_boundaries() {
        set_card_registry(&[Card::BasicRed; 256]);
        let mut deck = Deck::from_cards(cards(&[1, 2]));
        deck.push_bottom(cards(&[3, 4, 5]));

        let json = serde_json::to_string(&deck).unwrap();
        let restored: Deck = serde_json::from_str(&json).unwrap();
        assert_eq!(deck, restored);
        assert_eq!(restored.segment_count(), 2);
        assert_eq!(restored.segments()[0], cards(&[1, 2]));
    }

    #[test]
    fn an_empty_deck_serialises_to_an_empty_list() {
        set_card_registry(&[Card::BasicRed; 256]);
        assert_eq!(serde_json::to_string(&Deck::new()).unwrap(), "[]");
    }
}
