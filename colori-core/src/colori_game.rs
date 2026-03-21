use crate::apply_choice::apply_choice;
use crate::draw_phase::execute_draw_phase;
use crate::scoring::compute_terminal_rewards;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;


pub use crate::choices::{check_choice_available, enumerate_choices, enumerate_choices_into};
pub use crate::rollout::{apply_rollout_step, apply_heuristic_rollout_step};

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
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
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
        let direction = 1;

        // Determine which hands are known
        let mut known_hands = [false; 4];
        known_hands[perspective_player] = true;

        if let Some(ref sh) = known_draft_hands {
            // Track which drafted cards we've accounted for per player
            let mut persp_accounted = UnorderedCards::new();
            let mut receiver_accounted = [UnorderedCards::new(); MAX_PLAYERS];

            for round in 0..sh.len() {
                let hand = &sh[round];
                if hand.is_empty() {
                    continue;
                }

                // Convert known_draft_hands[round] to bitset
                let mut current_hand = UnorderedCards::new();
                for c in hand.iter() {
                    current_hand.insert(c.instance_id as u8);
                }
                let mut receiver = perspective_player;

                // Remove perspective player's pick at this round
                let persp_drafted = source.players[perspective_player].drafted_cards;
                let pick_mask = current_hand.intersection(persp_drafted).difference(persp_accounted);
                if let Some(persp_pick) = pick_mask.iter().next() {
                    persp_accounted.insert(persp_pick);
                    current_hand.remove(persp_pick);
                } else {
                    continue;
                }

                // Chain through subsequent players
                for step in 0..(num_players - 1) {
                    receiver = ((receiver as i32 + direction) as usize + num_players) % num_players;
                    if receiver == perspective_player {
                        break;
                    }

                    let pick_round = round + step + 1;
                    if pick_round > draft_state.pick_number as usize {
                        break;
                    }

                    if pick_round >= draft_state.pick_number as usize
                        && draft_state.current_player_index <= receiver
                    {
                        break;
                    }

                    let recv_drafted = source.players[receiver].drafted_cards;
                    let recv_pick_mask = current_hand
                        .intersection(recv_drafted)
                        .difference(receiver_accounted[receiver]);
                    if let Some(recv_pick) = recv_pick_mask.iter().next() {
                        receiver_accounted[receiver].insert(recv_pick);
                        current_hand.remove(recv_pick);
                        known_hands[receiver] = true;
                    } else {
                        break;
                    }
                }
            }
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
