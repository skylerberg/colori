use crate::deck_utils::draw_from_deck;
use crate::draw_log_helpers::{is_replaying, record_player_deck_draw, replay_player_deck_draw};
use crate::draft_phase::initialize_draft;
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{CardInstance, DraftState, GamePhase, GameState, MAX_PLAYERS};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    if is_replaying(state) {
        // Replay mode: consume recorded draw events
        for i in 0..num_players {
            replay_player_deck_draw(state, i);
        }
        replay_draft_deals(state);
    } else {
        // Normal mode: use rng, optionally record
        for i in 0..num_players {
            let before = state.players[i].workshop_cards;
            let player = &mut state.players[i];
            draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, 5, rng);
            record_player_deck_draw(state, i, before);
        }
        initialize_draft(state, rng);

        // Record draft hands that were dealt
        if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
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
}

/// In replay mode, consume DraftDeal events to set up draft hands.
fn replay_draft_deals(state: &mut GameState) {
    let num_players = state.players.len();
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];

    // Pop DraftDeal events from the replay queue
    loop {
        let is_draft_deal = matches!(
            &state.draw_log,
            Some(DrawLog::Replaying(q)) if matches!(q.front(), Some(DrawEvent::DraftDeal { .. }))
        );
        if !is_draft_deal {
            break;
        }
        let event = match &mut state.draw_log {
            Some(DrawLog::Replaying(queue)) => queue.pop_front(),
            _ => break,
        };
        if let Some(DrawEvent::DraftDeal { player_index, cards }) = event {
            for card in &cards {
                let id = card.instance_id as u8;
                state.draft_deck.remove(id);
                hands[player_index].insert(id);
            }
        }
    }

    // Check if any hands are empty (same logic as initialize_draft)
    if (0..num_players).any(|i| hands[i].is_empty()) {
        for i in 0..num_players {
            state.destroyed_pile = state.destroyed_pile.union(hands[i]);
        }
        crate::action_phase::initialize_action_phase(state);
        return;
    }

    let draft_state = DraftState {
        pick_number: 0,
        current_player_index: ((state.round - 1) as usize) % num_players,
        hands,
        num_hands: num_players,
    };
    state.phase = GamePhase::Draft { draft_state };
}
