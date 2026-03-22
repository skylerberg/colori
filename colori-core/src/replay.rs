use crate::colori_game::apply_choice_to_state;
use crate::draft_phase::{simultaneous_pick, advance_draft};
use crate::draw_phase::execute_draw_phase;
use crate::game_log::{DrawEvent, DrawLog, StructuredLogEntry};
use crate::scoring::calculate_score;
use crate::types::{Choice, GameState};
use std::collections::VecDeque;
use wyrand::WyRand;
use rand::SeedableRng;

/// Reconstruct a full GameState from a raw JSON initial_state value.
/// Rebuilds card_lookup and sell_card_lookup from the card instances in the state.
pub fn reconstruct_initial_state(initial_state_json: &serde_json::Value) -> GameState {
    let mut state: GameState = serde_json::from_value(initial_state_json.clone())
        .expect("Failed to deserialize initial state");

    // The GameState uses UnorderedCards which serialize as Vec<CardInstance>.
    // During deserialization, the global card registry is populated.
    // We just need to copy it to the state's lookup.
    state.card_lookup = crate::unordered_cards::get_card_registry();
    state.sell_card_lookup = crate::unordered_cards::get_sell_card_registry();

    // Compute cached scores
    for player in state.players.iter_mut() {
        player.cached_score = calculate_score(player);
    }

    state
}

/// Replay a game log up to (but not including) the entry at `stop_before_index`.
/// Returns the GameState just before that choice is made.
pub fn replay_to(
    initial_state_json: &serde_json::Value,
    initial_draws: &[DrawEvent],
    entries: &[StructuredLogEntry],
    stop_before_index: usize,
) -> GameState {
    let mut state = reconstruct_initial_state(initial_state_json);
    let mut rng = WyRand::seed_from_u64(0); // Dummy rng, not used in replay mode

    // Apply initial draw phase using recorded events
    let queue: VecDeque<DrawEvent> = initial_draws.iter().cloned().collect();
    state.draw_log = Some(DrawLog::Replaying(queue));
    execute_draw_phase(&mut state, &mut rng);
    state.draw_log = None;

    // Apply each entry up to (but not including) stop_before_index.
    // DraftPick entries need special handling: the web version uses simultaneous
    // draft (all players pick, then hands rotate), but the game log records picks
    // sequentially by player index. We use simultaneous_pick + advance_draft to
    // match the web version's behavior.
    let num_players = state.players.len();
    let mut draft_picks_in_round = 0;
    for entry in &entries[..stop_before_index] {
        let queue: VecDeque<DrawEvent> = entry.draws.iter().cloned().collect();
        state.draw_log = Some(DrawLog::Replaying(queue));

        if let Choice::DraftPick { card } = &entry.choice {
            simultaneous_pick(&mut state, entry.player_index, *card);
            draft_picks_in_round += 1;
            if draft_picks_in_round >= num_players {
                advance_draft(&mut state);
                draft_picks_in_round = 0;
            }
        } else {
            draft_picks_in_round = 0;
            apply_choice_to_state(&mut state, &entry.choice, &mut rng);
        }

        state.draw_log = None;
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_log::StructuredGameLog;

    #[test]
    fn test_replay_draft_produces_correct_cards() {
        let contents = include_str!("../tests/colori-log-2026-03-22-Player 1-AI Player 2.json");
        let raw: serde_json::Value = serde_json::from_str(contents).unwrap();
        let game: StructuredGameLog = serde_json::from_str(contents).unwrap();
        let initial_state_json = raw.get("initialState").unwrap().clone();

        // Replay to entry 8 (first action entry after round 1 draft completes)
        let state = replay_to(&initial_state_json, &game.initial_draws, &game.entries, 8);

        let p0_drafted: Vec<_> = state.players[0].drafted_cards.iter()
            .map(|id| state.card_lookup[id as usize])
            .collect();
        assert!(p0_drafted.contains(&crate::types::Card::UltramarineCanvas));
        assert!(p0_drafted.contains(&crate::types::Card::GumArabic));
        assert!(p0_drafted.contains(&crate::types::Card::Weld));
        assert!(p0_drafted.contains(&crate::types::Card::Woad));
        assert_eq!(p0_drafted.len(), 4);

        // Replay to entry 22 (first action entry after round 2 draft)
        let state = replay_to(&initial_state_json, &game.initial_draws, &game.entries, 22);

        let p0_drafted: Vec<_> = state.players[0].drafted_cards.iter()
            .map(|id| state.card_lookup[id as usize])
            .collect();
        assert!(p0_drafted.contains(&crate::types::Card::Elderberry));
        assert!(p0_drafted.contains(&crate::types::Card::Woad));
        assert!(p0_drafted.contains(&crate::types::Card::PersianBerries));
        assert!(p0_drafted.contains(&crate::types::Card::Turnsole));
        assert_eq!(p0_drafted.len(), 4);
    }
}
