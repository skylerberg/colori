use crate::apply_choice::apply_choice;
use crate::draw_phase::execute_draw_phase;
use crate::scoring::compute_terminal_rewards;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::{Rng, SeedableRng};
use wyrand::WyRand;


pub use crate::choices::{check_choice_available, enumerate_choices, enumerate_choices_into};
pub use crate::rollout::{apply_rollout_step, apply_heuristic_rollout_step};

// ── Apply choice with AI post-processing ──

/// Apply the choice and, if it ended the round, run the next draw phase.
pub fn apply_choice_to_state<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    choice: &Choice,
    rng: &mut R,
) {
    apply_choice(state, player_index, choice, rng);

    if matches!(state.phase, GamePhase::Draw) {
        execute_draw_phase(state, rng);
    }
}

// ── Simultaneous action phase ──

pub fn is_taking_turn(state: &GameState, player_index: usize) -> bool {
    matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player_index].ended)
}

/// Each player's view of an action phase: the state as the phase began, with
/// only that player's own choices applied since.
///
/// Everyone acts at once, and the search models each opponent as unable to
/// see the searcher's moves. An AI is held to the same by searching its view
/// rather than the live state, where turns others have already finished would
/// tell it what they did.
pub struct ActionPhaseViews {
    views: Vec<GameState>,
}

impl ActionPhaseViews {
    pub fn new(state: &GameState) -> Self {
        assert!(
            matches!(state.phase, GamePhase::Action { .. }),
            "Action phase views start from an action phase"
        );
        Self {
            views: state.players.iter().map(|_| state.clone()).collect(),
        }
    }

    pub fn view(&self, player_index: usize) -> &GameState {
        &self.views[player_index]
    }

    /// Bring `player_index`'s view up to date with a choice they made.
    pub fn apply(&mut self, player_index: usize, choice: &Choice) {
        // Nothing in a turn is random, so this generator decides nothing.
        let mut rng = WyRand::seed_from_u64(0);
        apply_choice_to_state(&mut self.views[player_index], player_index, choice, &mut rng);
    }
}

// ── Game status ──

#[derive(Debug)]
pub enum GameStatus {
    /// In the action phase everyone still taking their turn may act; this
    /// names the lowest such seat, for drivers that take one decision at a time.
    AwaitingAction { player_index: usize },
    Terminated { scores: [f64; MAX_PLAYERS] },
}

pub fn get_game_status(state: &GameState, max_round: Option<u32>) -> GameStatus {
    if let Some(mr) = max_round {
        if state.round > mr {
            return GameStatus::Terminated {
                scores: compute_terminal_rewards(&state.players),
            };
        }
    }

    match &state.phase {
        GamePhase::Buyers { buyers_state } => GameStatus::AwaitingAction {
            player_index: buyers_state.current_player_index,
        },
        GamePhase::Draft { draft_state } => GameStatus::AwaitingAction {
            player_index: draft_state.current_player_index,
        },
        GamePhase::Action { action_state } => GameStatus::AwaitingAction {
            player_index: action_state
                .first_unended()
                .expect("The round ends as the last turn does"),
        },
        GamePhase::GameOver => GameStatus::Terminated {
            scores: compute_terminal_rewards(&state.players),
        },
        GamePhase::Draw => GameStatus::AwaitingAction { player_index: 0 },
    }
}

// ── Determinization ──

pub fn determinize_in_place<R: Rng>(
    det: &mut GameState,
    source: &GameState,
    perspective_player: usize,
    cached_whole_points: &[u32; MAX_PLAYERS],
    rng: &mut R,
) {
    det.clone_from(source);

    // Initialize cached scores from pre-computed values
    for (i, p) in det.players.iter_mut().enumerate() {
        p.cached_whole_points = cached_whole_points[i];
    }

    if let GamePhase::Draft { ref mut draft_state } = det.phase {
        let num_players = det.players.len();

        // Positional deduction: at pick_number P, the perspective player has
        // seen P+1 distinct original hands through draft rotation. Mark the
        // corresponding current-hand positions as known.
        let mut known_hands = [false; 4];
        known_hands[perspective_player] = true;
        let pick = draft_state.pick_number as usize;
        let limit = pick.min(num_players - 1);
        for m in 0..=limit {
            known_hands[(perspective_player + m) % num_players] = true;
        }

        // Record hand sizes before pooling unknown hands
        let mut hand_sizes = [0u32; 4];
        for i in 0..num_players {
            hand_sizes[i] = draft_state.hands[i].len();
        }

        // Pool cards from unknown hands, redistribute via random draw
        let mut pool = UnorderedCards::new();
        let mut unknown_players = [0usize; 4];
        let mut unknown_count = 0usize;
        for i in 0..num_players {
            if !known_hands[i] {
                unknown_players[unknown_count] = i;
                unknown_count += 1;
                pool = pool.union(draft_state.hands[i]);
                draft_state.hands[i] = UnorderedCards::new();
            }
        }

        if unknown_count > 0 {
            pool = pool.union(det.draft_deck);
            det.draft_deck = UnorderedCards::new();

            for k in 0..unknown_count {
                let pi = unknown_players[k];
                let size = hand_sizes[pi];
                draft_state.hands[pi] = pool.draw_multiple(size, &mut *rng);
            }

            det.draft_deck = pool;
        }

        // No shuffle calls needed - bitset draw is already uniform random
    }
    // No shuffle calls needed for player decks, sell_card_deck, or draft_deck
    // because draw() from bitsets is inherently random
}
