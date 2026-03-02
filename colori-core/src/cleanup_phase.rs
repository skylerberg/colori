use crate::action_phase::end_round;
use crate::types::{CleanupState, GamePhase, GameState};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn initialize_cleanup_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    state.phase = GamePhase::Cleanup {
        cleanup_state: CleanupState {
            current_player_index: starting_player,
        },
    };
    advance_cleanup_to_next_nonempty(state, rng);
}

fn advance_cleanup_to_next_nonempty<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    loop {
        let current = get_cleanup_state(state).current_player_index;
        if !state.players[current].workshop_cards.is_empty() {
            return; // This player has workshop cards; wait for their choice
        }
        // Advance to next player
        let next = (current + 1) % num_players;
        if next == starting_player {
            // All players done, end round
            end_round(state, rng);
            return;
        }
        get_cleanup_state_mut(state).current_player_index = next;
    }
}

pub fn resolve_keep_workshop_cards<R: Rng>(
    state: &mut GameState,
    keep_ids: UnorderedCards,
    rng: &mut R,
) {
    let current = get_cleanup_state(state).current_player_index;
    let player = &mut state.players[current];

    // Discard cards NOT in keep_ids
    let to_discard = player.workshop_cards.difference(keep_ids);
    player.discard = player.discard.union(to_discard);
    player.workshop_cards = keep_ids;

    // Advance to next player
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    let next = (current + 1) % num_players;
    if next == starting_player {
        end_round(state, rng);
    } else {
        get_cleanup_state_mut(state).current_player_index = next;
        advance_cleanup_to_next_nonempty(state, rng);
    }
}

#[inline]
fn get_cleanup_state(state: &GameState) -> &CleanupState {
    match &state.phase {
        GamePhase::Cleanup { cleanup_state } => cleanup_state,
        _ => panic!("Expected cleanup phase"),
    }
}

#[inline]
fn get_cleanup_state_mut(state: &mut GameState) -> &mut CleanupState {
    match &mut state.phase {
        GamePhase::Cleanup { cleanup_state } => cleanup_state,
        _ => panic!("Expected cleanup phase"),
    }
}
