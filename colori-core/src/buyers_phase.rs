//! The buyers phase: claiming sell cards before the draft.
//!
//! Sell cards are no longer completed straight out of the shared display.
//! Before drafting, players take turns claiming cards into their own buyers
//! section, and only what sits there can be sold to for the rest of the round.
//! A sale empties the slot, and the next buyers phase refills it.
//!
//! Claiming goes round-robin in turn order — one card each, repeatedly — until
//! nobody has an empty slot left, so a run of good cards is shared out rather
//! than swept up by whoever goes first.

use rand::Rng;

use crate::draft_phase::initialize_draft;
use crate::draw_log_helpers::{is_replaying, record_buyer_draw, replay_buyer_draw};
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{BuyersState, GamePhase, GameState, SellCardInstance};

/// How many buyer slots a player has in `round`.
///
/// One to start with, a second from round 2 and a third from round 4.
pub fn buyer_capacity(round: u32) -> usize {
    match round {
        0..=1 => 1,
        2..=3 => 2,
        _ => 3,
    }
}

pub(crate) fn needs_a_buyer(state: &GameState, player_index: usize) -> bool {
    state.players[player_index].buyers.len() < buyer_capacity(state.round)
}

/// Whether there is anything left to claim. Both piles can run dry late in a
/// long game, and a player then simply leaves the slot empty.
pub(crate) fn anything_to_claim(state: &GameState) -> bool {
    !state.sell_card_display.is_empty() || !state.sell_card_deck.is_empty()
}

/// Enter the buyers phase, or skip straight to the draft if nobody can claim.
pub fn initialize_buyers_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;

    state.phase = GamePhase::Buyers {
        buyers_state: BuyersState {
            current_player_index: starting_player,
        },
    };

    // `starting_player` may already be full — round 1 aside, most rounds leave
    // some sections untouched — so the same search that ends the phase also
    // finds the first player who actually picks.
    if !seat_first_claimant(state, starting_player) {
        initialize_draft(state, rng);
    }
}

/// Point the phase at the next player who needs a card, starting the search at
/// `from` and wrapping. Returns false when nobody does, leaving the phase over.
fn seat_first_claimant(state: &mut GameState, from: usize) -> bool {
    if !anything_to_claim(state) {
        return false;
    }
    let num_players = state.players.len();
    for offset in 0..num_players {
        let candidate = (from + offset) % num_players;
        if needs_a_buyer(state, candidate) {
            if let GamePhase::Buyers { ref mut buyers_state } = state.phase {
                buyers_state.current_player_index = candidate;
            }
            return true;
        }
    }
    false
}

fn current_player(state: &GameState) -> usize {
    match &state.phase {
        GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
        _ => panic!("Expected buyers phase"),
    }
}

/// Hand the turn to the next player who needs a card, or start the draft.
fn advance_buyers<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let next = (current_player(state) + 1) % num_players;
    if !seat_first_claimant(state, next) {
        initialize_draft(state, rng);
    }
}

/// Claim a face-up sell card, and reveal its replacement immediately so the
/// next player also picks from five.
pub fn take_buyer<R: Rng>(state: &mut GameState, sell_card_instance_id: u32, rng: &mut R) {
    let player_index = current_player(state);
    claim_from_display(state, player_index, sell_card_instance_id, rng);
    advance_buyers(state, rng);
}

/// Claim the top of the deck unseen.
pub fn draw_buyer<R: Rng>(state: &mut GameState, rng: &mut R) {
    let player_index = current_player(state);
    claim_from_deck(state, player_index, rng);
    advance_buyers(state, rng);
}

/// Move a face-up card into a player's buyers and reveal its replacement,
/// without touching whose turn it is. The rollout shortcuts fill every
/// section at once and so drive the claims directly.
pub(crate) fn claim_from_display<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    sell_card_instance_id: u32,
    rng: &mut R,
) {
    let index = state
        .sell_card_display
        .iter()
        .position(|c| c.instance_id == sell_card_instance_id)
        .expect("Sell card not found in sell card display");
    let claimed = state.sell_card_display.remove(index);
    state.players[player_index].buyers.push(claimed);
    refill_display(state, rng);
}

/// Move the top of the deck into a player's buyers, unseen. A no-op on an
/// empty deck, which leaves the slot empty.
pub(crate) fn claim_from_deck<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    let drawn = if is_replaying(state) {
        replay_buyer_draw(state, player_index)
    } else {
        state.sell_card_deck.draw(rng).map(|id| SellCardInstance {
            instance_id: id as u32,
            sell_card: state.sell_card_lookup[id as usize],
        })
    };

    if let Some(instance) = drawn {
        record_buyer_draw(state, player_index, instance);
        state.players[player_index].buyers.push(instance);
    }
}

fn refill_display<R: Rng>(state: &mut GameState, rng: &mut R) {
    if is_replaying(state) {
        crate::draw_log_helpers::replay_sell_card_reveal(state);
        return;
    }
    let Some(id) = state.sell_card_deck.draw(rng) else {
        return;
    };
    let revealed = SellCardInstance {
        instance_id: id as u32,
        sell_card: state.sell_card_lookup[id as usize],
    };
    if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
        log.push(DrawEvent::SellCardReveal {
            sell_card: revealed,
        });
    }
    state.sell_card_display.push(revealed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_grows_at_rounds_two_and_four() {
        assert_eq!(buyer_capacity(1), 1);
        assert_eq!(buyer_capacity(2), 2);
        assert_eq!(buyer_capacity(3), 2);
        assert_eq!(buyer_capacity(4), 3);
        assert_eq!(buyer_capacity(5), 3);
        assert_eq!(buyer_capacity(6), 3);
    }

    #[test]
    fn capacity_never_exceeds_the_slot_array() {
        for round in 0..64 {
            assert!(buyer_capacity(round) <= crate::types::MAX_BUYERS);
        }
    }
}
