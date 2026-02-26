use crate::deck_utils::draw_from_deck;
use crate::draft_phase::initialize_draft;
use crate::types::GameState;
use rand::Rng;

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    for i in 0..num_players {
        let player = &mut state.players[i];
        let current = player.workshop_cards.len() as usize;
        let to_draw = if current >= 5 { 0 } else { 5 - current };
        if to_draw > 0 {
            draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, to_draw, rng);
        }
    }
    initialize_draft(state, rng);
}
