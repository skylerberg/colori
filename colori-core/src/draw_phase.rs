use crate::deck_utils::draw_from_deck;
use crate::draft_phase::initialize_draft;
use crate::types::GameState;
use rand::Rng;

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    for i in 0..num_players {
        let drawn = draw_from_deck(&mut state.players[i], 5, rng);
        state.players[i].workshop_cards.extend(drawn);
    }
    initialize_draft(state, rng);
}
