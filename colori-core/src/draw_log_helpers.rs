use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{GameState, SellCardInstance};

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

/// Record a buyer claimed off the top of the deck. The display refill has its
/// own event; this is the one nothing else can reconstruct.
pub fn record_buyer_draw(state: &mut GameState, player_index: usize, sell_card: SellCardInstance) {
    if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
        log.push(DrawEvent::BuyerDraw {
            player_index,
            sell_card,
        });
    }
}

/// In replay mode, consume this player's BuyerDraw event and return the card
/// it recorded. Returns None when the recorded draw came up empty.
pub fn replay_buyer_draw(state: &mut GameState, player_index: usize) -> Option<SellCardInstance> {
    let matches_player = matches!(
        &state.draw_log,
        Some(DrawLog::Replaying(queue))
            if matches!(queue.front(), Some(DrawEvent::BuyerDraw { player_index: pi, .. }) if *pi == player_index)
    );
    if !matches_player {
        return None;
    }
    let event = match &mut state.draw_log {
        Some(DrawLog::Replaying(queue)) => queue.pop_front(),
        _ => return None,
    };
    match event {
        Some(DrawEvent::BuyerDraw { sell_card, .. }) => {
            state.sell_card_deck.remove(sell_card.instance_id as u8);
            Some(sell_card)
        }
        _ => None,
    }
}
