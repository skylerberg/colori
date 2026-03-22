use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{CardInstance, GameState};
use crate::unordered_cards::UnorderedCards;

pub fn record_player_deck_draw(state: &mut GameState, player_index: usize, before: UnorderedCards) {
    if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
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

/// In replay mode, pop the next PlayerDeckDraw event and apply it to the state.
/// Moves the specified cards from the player's deck (or discard) to workshop.
pub fn replay_player_deck_draw(state: &mut GameState, player_index: usize) {
    let event = match &mut state.draw_log {
        Some(DrawLog::Replaying(queue)) => queue.pop_front(),
        _ => return,
    };
    if let Some(DrawEvent::PlayerDeckDraw { cards, .. }) = event {
        let player = &mut state.players[player_index];
        for card in &cards {
            let id = card.instance_id as u8;
            if player.deck.contains(id) {
                player.deck.remove(id);
            } else if player.discard.contains(id) {
                player.discard.remove(id);
            }
            player.workshop_cards.insert(id);
        }
    }
}

/// In replay mode, pop the next SellCardReveal event and apply it.
pub fn replay_sell_card_reveal(state: &mut GameState) -> bool {
    let event = match &mut state.draw_log {
        Some(DrawLog::Replaying(queue)) => {
            // Peek to check if next event is SellCardReveal
            if matches!(queue.front(), Some(DrawEvent::SellCardReveal { .. })) {
                queue.pop_front()
            } else {
                return false;
            }
        }
        _ => return false,
    };
    if let Some(DrawEvent::SellCardReveal { sell_card }) = event {
        let id = sell_card.instance_id as u8;
        state.sell_card_deck.remove(id);
        state.sell_card_display.push(sell_card);
        return true;
    }
    false
}

/// Check if we're in replay mode
pub fn is_replaying(state: &GameState) -> bool {
    matches!(&state.draw_log, Some(DrawLog::Replaying(_)))
}
