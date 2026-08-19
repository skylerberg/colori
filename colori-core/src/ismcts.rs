//! Search configuration, and the opponent draft model.
//!
//! The tree search itself now lives in the shared `mcts` crate; see
//! `crate::mcts_impl` for this game's side of it. What remains here is the
//! configuration every caller passes, and the DUCT bandit over opponent draft
//! picks — which is deliberately not part of the tree, so it is not part of the
//! generic search either.

use crate::draft_phase::player_pick;
use crate::scoring::HeuristicParams;
use crate::types::*;
use rand::Rng;
use rand::RngExt;
use serde::Deserialize;
use smallvec::SmallVec;

#[derive(Clone, Debug)]
pub struct MctsConfig {
    pub iterations: u32,
    pub exploration_constant: f64,
    pub max_rollout_steps: u32,
    pub use_heuristic_eval: bool,
    pub progressive_bias_weight: f64,
    pub heuristic_params: HeuristicParams,
    pub no_rollout: bool,
    pub heuristic_rollout: bool,
    pub heuristic_draft: bool,
    pub early_termination: bool,
    pub time_limit_ms: Option<u64>,
    pub random_first_pick: bool,
    pub force_max_workshop: bool,
}


impl MctsConfig {
    pub fn new(heuristic_params: HeuristicParams) -> Self {
        MctsConfig {
            iterations: 100,
            exploration_constant: 0.75,
            max_rollout_steps: 1000,
            use_heuristic_eval: true,
            progressive_bias_weight: 0.0,
            heuristic_params,
            no_rollout: false,
            heuristic_rollout: true,
            heuristic_draft: false,
            early_termination: true,
            time_limit_ms: None,
            random_first_pick: false,
            force_max_workshop: true,
        }
    }
}

impl<'de> Deserialize<'de> for MctsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct MctsConfigHelper {
            #[serde(default = "default_iterations")]
            iterations: u32,
            #[serde(default = "default_exploration_constant")]
            exploration_constant: f64,
            #[serde(default = "default_max_rollout_steps")]
            max_rollout_steps: u32,
            #[serde(default = "default_use_heuristic_eval")]
            use_heuristic_eval: bool,
            #[serde(default = "default_progressive_bias_weight")]
            progressive_bias_weight: f64,
            heuristic_params: HeuristicParams,
            #[serde(default = "default_heuristic_rollout")]
            heuristic_rollout: bool,
            #[serde(default)]
            heuristic_draft: bool,
            #[serde(default)]
            early_termination: bool,
            #[serde(default)]
            time_limit_ms: Option<u64>,
            #[serde(default)]
            random_first_pick: bool,
        }

        fn default_iterations() -> u32 { 100 }
        fn default_exploration_constant() -> f64 { 0.75 }
        fn default_max_rollout_steps() -> u32 { 1000 }
        fn default_use_heuristic_eval() -> bool { true }
        fn default_progressive_bias_weight() -> f64 { 0.0 }
        fn default_heuristic_rollout() -> bool { true }

        let helper = MctsConfigHelper::deserialize(deserializer)?;
        Ok(MctsConfig {
            iterations: helper.iterations,
            exploration_constant: helper.exploration_constant,
            max_rollout_steps: helper.max_rollout_steps,
            use_heuristic_eval: helper.use_heuristic_eval,
            progressive_bias_weight: helper.progressive_bias_weight,
            heuristic_params: helper.heuristic_params,
            no_rollout: false,
            heuristic_rollout: helper.heuristic_rollout,
            heuristic_draft: helper.heuristic_draft,
            early_termination: helper.early_termination,
            time_limit_ms: helper.time_limit_ms,
            random_first_pick: helper.random_first_pick,
            force_max_workshop: true,
        })
    }
}


fn upper_confidence_bound(
    cumulative_reward: f64,
    visit_count: u32,
    total_visit_count: u32,
    c: f64,
) -> f64 {
    let win_rate = cumulative_reward / visit_count as f64;
    win_rate + c * ((total_visit_count as f64).ln() / visit_count as f64).sqrt()
}

// ── DUCT (Decoupled UCT) for opponent draft modeling ──


const NUM_CARDS: usize = 47;

#[derive(Clone, Copy, Default)]
struct OpponentPickStat {
    visit_count: u32,
    cumulative_reward: f64,
    availability_count: u32,
}

pub(crate) struct OpponentDraftStats {
    // [pick_round][player_index][card as usize] -> per-card stats
    stats: [[[OpponentPickStat; NUM_CARDS]; MAX_PLAYERS]; 4],
}

impl Default for OpponentDraftStats {
    fn default() -> Self {
        Self::new()
    }
}

impl OpponentDraftStats {
    pub(crate) fn new() -> Self {
        OpponentDraftStats {
            stats: [[[OpponentPickStat::default(); NUM_CARDS]; MAX_PLAYERS]; 4],
        }
    }

    fn update_availability(&mut self, pick_round: usize, player: usize, available_cards: &[Card]) {
        let slot = &mut self.stats[pick_round][player];
        for &card in available_cards {
            slot[card as usize].availability_count += 1;
        }
    }

    fn select<R: Rng>(
        &self,
        pick_round: usize,
        player: usize,
        available_cards: &[Card],
        exploration_constant: f64,
        rng: &mut R,
    ) -> Card {
        let slot = &self.stats[pick_round][player];

        let mut best_card: Option<Card> = None;
        let mut best_value = f64::NEG_INFINITY;

        for &card in available_cards {
            let stat = &slot[card as usize];
            let value = if stat.availability_count == 0 || stat.visit_count == 0 {
                f64::INFINITY
            } else {
                let total = stat.availability_count.max(1);
                upper_confidence_bound(
                    stat.cumulative_reward,
                    stat.visit_count,
                    total,
                    exploration_constant,
                )
            };

            if value > best_value || (value == best_value && value == f64::INFINITY && rng.random_bool(0.5)) {
                best_value = value;
                best_card = Some(card);
            }
        }

        best_card.unwrap()
    }

    pub(crate) fn record_outcome(&mut self, pick_round: usize, player: usize, card: Card, reward: f64) {
        let stat = &mut self.stats[pick_round][player][card as usize];
        stat.visit_count += 1;
        stat.cumulative_reward += reward;
    }
}

fn get_opponent_draft_cards(state: &GameState) -> SmallVec<[Card; 8]> {
    let mut cards = SmallVec::new();
    if let GamePhase::Draft { ref draft_state } = state.phase {
        let hand = draft_state.hands[draft_state.current_player_index];
        let mut seen: u64 = 0;
        for id in hand.iter() {
            let card = state.card_lookup[id as usize];
            let bit = 1u64 << (card as u64);
            if seen & bit != 0 { continue; }
            seen |= bit;
            cards.push(card);
        }
    }
    cards
}

fn find_card_id(state: &GameState, card: Card) -> u32 {
    if let GamePhase::Draft { ref draft_state } = state.phase {
        let hand = draft_state.hands[draft_state.current_player_index];
        for id in hand.iter() {
            if state.card_lookup[id as usize] == card {
                return id as u32;
            }
        }
    }
    panic!("Card not found for DUCT choice");
}

pub(crate) fn advance_past_opponent_draft_picks<R: Rng>(
    state: &mut GameState,
    perspective_player: usize,
    opponent_stats: &mut OpponentDraftStats,
    pick_log: &mut Vec<(u32, usize, Card)>,
    exploration_constant: f64,
    rng: &mut R,
) {
    loop {
        let (current_player, pick_number) = match &state.phase {
            GamePhase::Draft { draft_state } => {
                (draft_state.current_player_index, draft_state.pick_number)
            }
            _ => break,
        };

        if current_player == perspective_player {
            break;
        }

        let available = get_opponent_draft_cards(state);
        if available.is_empty() {
            break;
        }

        opponent_stats.update_availability(pick_number as usize, current_player, &available);
        let card = opponent_stats.select(
            pick_number as usize,
            current_player,
            &available,
            exploration_constant,
            rng,
        );

        let card_id = find_card_id(state, card);
        pick_log.push((pick_number, current_player, card));
        player_pick(state, card_id, rng);
    }
}
