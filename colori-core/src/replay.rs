use crate::colori_game::apply_choice_to_state;
use crate::draft_phase::{simultaneous_pick, advance_draft};
use crate::draw_phase::execute_draw_phase;
use crate::game_log::{DrawEvent, DrawLog, StructuredLogEntry};
use crate::scoring::calculate_score;
use crate::types::{Choice, GamePhase, GameState};
use std::collections::VecDeque;
use wyrand::WyRand;
use rand::SeedableRng;

/// Reconstruct a full GameState from a raw JSON initial_state value.
/// Rebuilds card_lookup and sell_card_lookup from the card instances in the state.
pub fn reconstruct_initial_state(initial_state_json: &serde_json::Value) -> GameState {
    let mut state: GameState = serde_json::from_value(initial_state_json.clone())
        .expect("Failed to deserialize initial state");

    state.card_lookup = crate::unordered_cards::get_card_registry();
    state.sell_card_lookup = crate::unordered_cards::get_sell_card_registry();

    for player in state.players.iter_mut() {
        player.cached_score = calculate_score(player);
    }

    state
}

/// Incremental game log replayer that handles simultaneous draft correctly.
///
/// The web version uses simultaneous draft (all players pick, then hands rotate),
/// but the game log records picks sequentially by player index. This replayer
/// uses simultaneous_pick + advance_draft to match the web version's behavior.
pub struct GameReplay {
    pub state: GameState,
    rng: WyRand,
    num_players: usize,
    draft_picks_in_round: usize,
}

impl GameReplay {
    /// Create a new replayer from an initial state JSON and initial draw events.
    pub fn new(initial_state_json: &serde_json::Value, initial_draws: &[DrawEvent]) -> Self {
        let mut state = reconstruct_initial_state(initial_state_json);
        let mut rng = WyRand::seed_from_u64(0);

        let queue: VecDeque<DrawEvent> = initial_draws.iter().cloned().collect();
        state.draw_log = Some(DrawLog::Replaying(queue));
        execute_draw_phase(&mut state, &mut rng);
        state.draw_log = None;

        let num_players = state.players.len();
        GameReplay { state, rng, num_players, draft_picks_in_round: 0 }
    }

    /// Apply a single game log entry to advance the state.
    pub fn apply_entry(&mut self, entry: &StructuredLogEntry) {
        let queue: VecDeque<DrawEvent> = entry.draws.iter().cloned().collect();
        self.state.draw_log = Some(DrawLog::Replaying(queue));

        if self.num_players == 1 {
            // Solo mode: use apply_choice_to_state so phantom draft removals
            // are replayed correctly via player_pick's draw log handling.
            self.draft_picks_in_round = 0;
            apply_choice_to_state(&mut self.state, &entry.choice, &mut self.rng);
        } else if let Choice::DraftPick { card } = &entry.choice {
            simultaneous_pick(&mut self.state, entry.player_index, *card);
            self.draft_picks_in_round += 1;
            if self.draft_picks_in_round >= self.num_players {
                advance_draft(&mut self.state);
                self.draft_picks_in_round = 0;
            }
        } else {
            self.draft_picks_in_round = 0;
            apply_choice_to_state(&mut self.state, &entry.choice, &mut self.rng);
        }

        self.state.draw_log = None;
    }

    /// Fix current_player_index for MCTS/enumerate_choices when the next entry
    /// is a DraftPick. Call this after the last apply_entry before running MCTS.
    pub fn fix_current_player_for_next(&mut self, next_entry: &StructuredLogEntry) {
        if let GamePhase::Draft { ref mut draft_state } = self.state.phase {
            if let Choice::DraftPick { .. } = &next_entry.choice {
                draft_state.current_player_index = next_entry.player_index;
            }
        }
    }
}

/// Replay a game log up to (but not including) the entry at `stop_before_index`.
/// Returns the GameState just before that choice is made.
pub fn replay_to(
    initial_state_json: &serde_json::Value,
    initial_draws: &[DrawEvent],
    entries: &[StructuredLogEntry],
    stop_before_index: usize,
) -> GameState {
    let mut replay = GameReplay::new(initial_state_json, initial_draws);

    for entry in &entries[..stop_before_index] {
        replay.apply_entry(entry);
    }

    if stop_before_index < entries.len() {
        replay.fix_current_player_for_next(&entries[stop_before_index]);
    }

    replay.state
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

    #[test]
    fn test_replay_mid_draft_sets_correct_current_player() {
        use crate::colori_game::enumerate_choices;

        let contents = include_str!("../tests/colori-log-2026-03-23-Player 1-AI Player 2.json");
        let raw: serde_json::Value = serde_json::from_str(contents).unwrap();
        let game: StructuredGameLog = serde_json::from_str(contents).unwrap();
        let initial_state_json = raw.get("initialState").unwrap().clone();

        for (i, entry) in game.entries.iter().enumerate() {
            if let Choice::DraftPick { card } = &entry.choice {
                let state = replay_to(&initial_state_json, &game.initial_draws, &game.entries, i);
                if let GamePhase::Draft { ref draft_state } = state.phase {
                    assert_eq!(
                        draft_state.current_player_index, entry.player_index,
                        "At entry {} (seq {}, round {}): current_player_index is {}, expected {}",
                        i, entry.seq, entry.round, draft_state.current_player_index, entry.player_index,
                    );

                    let choices = enumerate_choices(&state);
                    assert!(
                        choices.iter().any(|c| matches!(c, Choice::DraftPick { card: c } if c == card)),
                        "At entry {} (seq {}): card {:?} not in available choices",
                        i, entry.seq, card,
                    );
                }
            }
        }
    }

    #[test]
    fn test_game_replay_incremental_matches_replay_to() {
        let contents = include_str!("../tests/colori-log-2026-03-22-Player 1-AI Player 2.json");
        let raw: serde_json::Value = serde_json::from_str(contents).unwrap();
        let game: StructuredGameLog = serde_json::from_str(contents).unwrap();
        let initial_state_json = raw.get("initialState").unwrap().clone();

        // Verify that GameReplay produces the same state as replay_to at key points
        let mut replay = GameReplay::new(&initial_state_json, &game.initial_draws);
        for check_at in [8, 14, 22] {
            while replay.state.round <= game.entries[check_at - 1].round {
                let entry_idx = game.entries.iter().position(|e| {
                    // Find the current position by checking how many entries we've applied
                    false
                });
                break;
            }
        }

        // Simpler: just verify key entry points match
        let mut replay = GameReplay::new(&initial_state_json, &game.initial_draws);
        for i in 0..8 {
            replay.apply_entry(&game.entries[i]);
        }
        let state_at_8 = replay_to(&initial_state_json, &game.initial_draws, &game.entries, 8);

        // Compare drafted cards
        let replay_drafted: Vec<_> = replay.state.players[0].drafted_cards.iter().collect();
        let direct_drafted: Vec<_> = state_at_8.players[0].drafted_cards.iter().collect();
        assert_eq!(replay_drafted, direct_drafted);
    }
}
