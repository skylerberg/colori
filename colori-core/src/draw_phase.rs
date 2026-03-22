use crate::deck_utils::draw_from_deck;
use crate::draw_log_helpers::record_player_deck_draw;
use crate::draft_phase::initialize_draft;
use crate::game_log::DrawEvent;
use crate::types::{CardInstance, GamePhase, GameState};
use rand::Rng;

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    for i in 0..num_players {
        let before = state.players[i].workshop_cards;
        let player = &mut state.players[i];
        draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, 5, rng);
        record_player_deck_draw(state, i, before);
    }
    initialize_draft(state, rng);

    // Record draft hands that were dealt
    if let Some(log) = &mut state.draw_log {
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..num_players {
                let cards: Vec<CardInstance> = draft_state.hands[i]
                    .iter()
                    .map(|id| CardInstance {
                        instance_id: id as u32,
                        card: state.card_lookup[id as usize],
                    })
                    .collect();
                if !cards.is_empty() {
                    log.push(DrawEvent::DraftDeal {
                        player_index: i,
                        cards,
                    });
                }
            }
        }
    }
}
