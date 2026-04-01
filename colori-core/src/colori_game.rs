use crate::apply_choice::apply_choice;
use crate::draw_phase::execute_draw_phase;
use crate::scoring::compute_terminal_rewards;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;


pub use crate::choices::{check_choice_available, enumerate_choices, enumerate_choices_into};
pub use crate::rollout::{apply_rollout_step, apply_heuristic_rollout_step, apply_solver_rollout_step};

// ── Apply choice with AI post-processing ──

pub fn apply_choice_to_state<R: Rng>(state: &mut GameState, choice: &Choice, rng: &mut R) {
    apply_choice(state, choice, rng);

    if matches!(choice, Choice::EndTurn) {
        if matches!(state.phase, GamePhase::Draw) {
            execute_draw_phase(state, rng);
        }
    }
}

// ── Game status ──

#[derive(Debug)]
pub enum GameStatus {
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
        GamePhase::Draft { draft_state } => GameStatus::AwaitingAction {
            player_index: draft_state.current_player_index,
        },
        GamePhase::Action { action_state } => GameStatus::AwaitingAction {
            player_index: action_state.current_player_index,
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
    cached_scores: &[u32; MAX_PLAYERS],
    rng: &mut R,
) {
    det.clone_from(source);

    // Initialize cached scores from pre-computed values
    for (i, p) in det.players.iter_mut().enumerate() {
        p.cached_score = cached_scores[i];
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
