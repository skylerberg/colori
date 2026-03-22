use crate::game_log::DrawEvent;
use crate::types::{CardInstance, GameState};
use crate::unordered_cards::UnorderedCards;

/// Record a PlayerDeckDraw event by diffing workshop_cards before and after a draw.
/// `before` should be the player's workshop_cards snapshot taken before draw_from_deck.
pub fn record_player_deck_draw(state: &mut GameState, player_index: usize, before: UnorderedCards) {
    if let Some(log) = &mut state.draw_log {
        let drawn = state.players[player_index].workshop_cards.difference(before);
        let cards: Vec<CardInstance> = drawn
            .iter()
            .map(|id| CardInstance {
                instance_id: id as u32,
                card: state.card_lookup[id as usize],
            })
            .collect();
        if !cards.is_empty() {
            log.push(DrawEvent::PlayerDeckDraw {
                player_index,
                cards,
            });
        }
    }
}
