use crate::buyers_phase::initialize_buyers_phase;
use crate::draw_log_helpers::{is_replaying, record_player_deck_draw, replay_player_deck_draw};
use crate::types::{GameState, PlayerState};
use rand::Rng;

/// How many cards a player starts the action phase holding in the workshop.
pub const WORKSHOP_SIZE: u32 = 5;

/// How many cards the draw phase adds.
///
/// The workshop is topped up to [`WORKSHOP_SIZE`] rather than dealt a fixed
/// five, because cards kept in the draft pool now arrive in the workshop at
/// the end of the previous round. Keeping one therefore trades against a fresh
/// draw instead of adding to the hand.
fn top_up_count(player: &PlayerState) -> u32 {
    WORKSHOP_SIZE.saturating_sub(player.workshop_cards.len())
}

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    if is_replaying(state) {
        for i in 0..num_players {
            replay_player_deck_draw(state, i);
        }
    } else {
        for i in 0..num_players {
            let before = state.players[i].workshop_cards;
            let player = &mut state.players[i];
            let count = top_up_count(player);
            player.deck.draw_into(&mut player.workshop_cards, count, rng);
            record_player_deck_draw(state, i, before);
        }
    }

    initialize_buyers_phase(state, rng);
}
