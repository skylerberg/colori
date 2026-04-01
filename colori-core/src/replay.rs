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
