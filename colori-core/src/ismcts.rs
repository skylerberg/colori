use crate::colori_game::{
    apply_choice_to_state, apply_rollout_step, apply_heuristic_rollout_step,
    determinize_in_place, enumerate_choices_into,
};
use crate::draft_phase::player_pick;
use crate::scoring::{calculate_score, CardHeuristicTable, compute_heuristic_rewards, compute_terminal_rewards, heuristic_score, HeuristicParams, FirstPickParams, DiffEvalParams, DiffEvalTable, diff_eval_score, compute_diff_eval_rewards};
use crate::types::*;
use rand::Rng;
use rand::RngExt;
use serde::Deserialize;
use smallvec::SmallVec;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct MctsConfig {
    pub iterations: u32,
    pub exploration_constant: f64,
    pub max_rollout_steps: u32,
    pub use_heuristic_eval: bool,
    pub progressive_bias_weight: f64,
    pub heuristic_params: HeuristicParams,
    pub diff_eval_params: Option<Box<DiffEvalParams>>,
    pub no_rollout: bool,
    pub heuristic_rollout: bool,
    pub heuristic_draft: bool,
    pub early_termination: bool,
    pub time_limit_ms: Option<u64>,
    pub random_first_pick: bool,
    pub first_pick_params: Option<Box<FirstPickParams>>,
}

pub struct MctsResult {
    pub choice: Choice,
    pub iterations_used: u32,
    pub reused_iterations: u32,
    pub tree: Option<MctsNode>,
}

pub struct TreeStats {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub avg_branching_factor: f64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        MctsConfig {
            iterations: 100,
            exploration_constant: std::f64::consts::SQRT_2,
            max_rollout_steps: 1000,
            use_heuristic_eval: true,
            progressive_bias_weight: 0.0,
            heuristic_params: HeuristicParams::default(),
            diff_eval_params: None,
            no_rollout: false,
            heuristic_rollout: true,
            heuristic_draft: false,
            early_termination: true,
            time_limit_ms: None,
            random_first_pick: false,
            first_pick_params: None,
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
            #[serde(default)]
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
        fn default_exploration_constant() -> f64 { std::f64::consts::SQRT_2 }
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
            diff_eval_params: None,
            no_rollout: false,
            heuristic_rollout: helper.heuristic_rollout,
            heuristic_draft: helper.heuristic_draft,
            early_termination: helper.early_termination,
            time_limit_ms: helper.time_limit_ms,
            random_first_pick: helper.random_first_pick,
            first_pick_params: None,
        })
    }
}

pub struct MctsNode {
    visit_count: u32,
    cumulative_reward: f64,
    player_index: usize,
    choice: Option<Choice>,
    availability_count: u32,
    ln_availability: f64,
    heuristic_bias: f64,
    children: Vec<MctsNode>,
}

impl MctsNode {
    fn new(player_index: usize, choice: Option<Choice>) -> Self {
        MctsNode {
            visit_count: 0,
            cumulative_reward: 0.0,
            player_index,
            choice,
            availability_count: 0,
            ln_availability: 0.0,
            heuristic_bias: 0.0,
            children: Vec::new(),
        }
    }

    fn is_root(&self) -> bool {
        self.choice.is_none()
    }

    pub fn into_subtree(mut self, choice: &Choice) -> Option<MctsNode> {
        let idx = self.children.iter().position(|c| c.choice.as_ref() == Some(choice))?;
        let mut child = self.children.swap_remove(idx);
        child.choice = None;
        Some(child)
    }

    fn expand<R: Rng>(
        &mut self,
        choices: &[Choice],
        active_player: usize,
        available: &mut Vec<bool>,
        rng: &mut R,
    ) {
        available.clear();
        available.resize(self.children.len(), false);

        // Match choices against existing children, collect unseen indices
        let mut unseen_indices: SmallVec<[usize; 16]> = SmallVec::new();
        for (i, choice) in choices.iter().enumerate() {
            if let Some(idx) = self
                .children
                .iter()
                .position(|c| c.choice.as_ref() == Some(choice))
            {
                if !available[idx] {
                    self.children[idx].availability_count += 1;
                    self.children[idx].ln_availability = (self.children[idx].availability_count as f64).ln();
                    available[idx] = true;
                }
            } else {
                unseen_indices.push(i);
            }
        }

        // Add new nodes: root adds all unseen, non-root adds one at random
        if self.is_root() {
            for &i in &unseen_indices {
                let mut new_node = MctsNode::new(active_player, Some(choices[i].clone()));
                new_node.availability_count = 1;
                available.push(true);
                self.children.push(new_node);
            }
        } else if !unseen_indices.is_empty() {
            let pick = rng.random_range(0..unseen_indices.len());
            let i = unseen_indices[pick];
            let mut new_node = MctsNode::new(active_player, Some(choices[i].clone()));
            new_node.availability_count = 1;
            available.push(true);
            self.children.push(new_node);
        }
    }

    pub fn visit_count(&self) -> u32 {
        self.visit_count
    }

    pub fn average_reward(&self) -> f64 {
        if self.visit_count == 0 {
            0.0
        } else {
            self.cumulative_reward / self.visit_count as f64
        }
    }

    pub fn choice(&self) -> Option<&Choice> {
        self.choice.as_ref()
    }

    pub fn children(&self) -> &[MctsNode] {
        &self.children
    }

    /// Maximum depth from this node to any leaf.
    pub fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            0
        } else {
            1 + self.children.iter().map(|c| c.max_depth()).max().unwrap_or(0)
        }
    }

    /// Compute aggregate tree statistics.
    pub fn tree_stats(&self) -> TreeStats {
        let mut stats = TreeStatsAccum { total_nodes: 0, internal_nodes: 0, total_children: 0, max_depth: 0 };
        self.tree_stats_recurse(&mut stats, 0);
        TreeStats {
            total_nodes: stats.total_nodes,
            max_depth: stats.max_depth,
            avg_branching_factor: if stats.internal_nodes == 0 {
                0.0
            } else {
                stats.total_children as f64 / stats.internal_nodes as f64
            },
        }
    }

    fn tree_stats_recurse(&self, acc: &mut TreeStatsAccum, depth: usize) {
        acc.total_nodes += 1;
        if depth > acc.max_depth {
            acc.max_depth = depth;
        }
        let visited_children: Vec<&MctsNode> = self.children.iter()
            .filter(|c| c.visit_count > 0)
            .collect();
        if !visited_children.is_empty() {
            acc.internal_nodes += 1;
            acc.total_children += visited_children.len();
            for child in visited_children {
                child.tree_stats_recurse(acc, depth + 1);
            }
        }
    }
}

struct TreeStatsAccum {
    total_nodes: usize,
    internal_nodes: usize,
    total_children: usize,
    max_depth: usize,
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

const NUM_CARDS: usize = 48;

#[derive(Clone, Copy, Default)]
struct OpponentPickStat {
    visit_count: u32,
    cumulative_reward: f64,
    availability_count: u32,
}

struct OpponentDraftStats {
    // [pick_round][player_index][card as usize] -> per-card stats
    stats: [[[OpponentPickStat; NUM_CARDS]; MAX_PLAYERS]; 4],
}

impl OpponentDraftStats {
    fn new() -> Self {
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

    fn record_outcome(&mut self, pick_round: usize, player: usize, card: Card, reward: f64) {
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

fn advance_past_opponent_draft_picks<R: Rng>(
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
        player_pick(state, card_id);
    }
}

pub fn ismcts<R: Rng>(
    state: &GameState,
    player_index: usize,
    config: &MctsConfig,
    max_rollout_round: Option<u32>,
    previous_tree: Option<MctsNode>,
    rng: &mut R,
) -> MctsResult {
    // If there's only one legal choice, return it immediately without searching
    let mut choices_buf: Vec<Choice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return MctsResult { choice: choices_buf.swap_remove(0), iterations_used: 0, reused_iterations: 0, tree: None };
    }

    let mut root = previous_tree.unwrap_or_else(|| MctsNode::new(player_index, None));
    let reused_iterations = root.visit_count;
    let mut det_state = state.clone();

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    // If diff eval specifies a progressive bias weight, use it
    let effective_progressive_bias = config.diff_eval_params.as_ref()
        .map(|dep| dep.progressive_bias_weight())
        .filter(|&w| w != 0.0)
        .unwrap_or(config.progressive_bias_weight);
    let config = &MctsConfig {
        progressive_bias_weight: effective_progressive_bias,
        ..config.clone()
    };

    let mut availability_buf: Vec<bool> = Vec::new();
    let card_table = CardHeuristicTable::new(&config.heuristic_params);
    let diff_table = config.diff_eval_params.as_ref().map(|p| DiffEvalTable::new(p));

    let mut opponent_stats = OpponentDraftStats::new();
    let mut pick_log: Vec<(u32, usize, Card)> = Vec::new();

    let (effective_max_rollout_round, use_heuristic) = if config.use_heuristic_eval {
        let (should_use, lookahead) = if let Some(ref diff_params) = config.diff_eval_params {
            (state.round <= diff_params.heuristic_round_threshold(), diff_params.heuristic_lookahead())
        } else if let Some(score_threshold) = config.heuristic_params.heuristic_score_threshold {
            let max_score = state.players.iter().map(|p| p.cached_score).max().unwrap_or(0);
            ((max_score as f64) < score_threshold, config.heuristic_params.heuristic_lookahead)
        } else {
            (state.round <= config.heuristic_params.heuristic_round_threshold, config.heuristic_params.heuristic_lookahead)
        };
        if should_use {
            let heuristic_round = state.round + lookahead;
            let effective = max_rollout_round.map_or(heuristic_round, |mr| mr.min(heuristic_round));
            (Some(effective), true)
        } else {
            (max_rollout_round, false)
        }
    } else {
        (max_rollout_round, false)
    };

    let mut iterations_used = 0u32;
    if let Some(time_limit_ms) = config.time_limit_ms {
        let deadline = Instant::now() + Duration::from_millis(time_limit_ms);
        while Instant::now() < deadline {
            iterations_used += 1;
            pick_log.clear();
            determinize_in_place(&mut det_state, state, player_index, &cached_scores, rng);
            advance_past_opponent_draft_picks(
                &mut det_state, player_index, &mut opponent_stats,
                &mut pick_log, config.exploration_constant, rng,
            );
            let scores = iteration_simultaneous(
                &mut root, &mut det_state, player_index,
                &mut opponent_stats, &mut pick_log,
                effective_max_rollout_round, use_heuristic, config, &mut choices_buf, &mut availability_buf, &card_table, &diff_table, rng,
            );
            for &(pick_round, player, card) in &pick_log {
                let reward = scores[player];
                opponent_stats.record_outcome(pick_round as usize, player, card, reward);
            }
        }
    } else {
        let new_iterations = config.iterations.saturating_sub(reused_iterations);
        for i in 0..new_iterations {
            iterations_used = i + 1;
            pick_log.clear();
            determinize_in_place(&mut det_state, state, player_index, &cached_scores, rng);
            advance_past_opponent_draft_picks(
                &mut det_state, player_index, &mut opponent_stats,
                &mut pick_log, config.exploration_constant, rng,
            );
            let scores = iteration_simultaneous(
                &mut root, &mut det_state, player_index,
                &mut opponent_stats, &mut pick_log,
                effective_max_rollout_round, use_heuristic, config, &mut choices_buf, &mut availability_buf, &card_table, &diff_table, rng,
            );
            for &(pick_round, player, card) in &pick_log {
                let reward = scores[player];
                opponent_stats.record_outcome(pick_round as usize, player, card, reward);
            }

            // Early termination: stop if the leader can't be overtaken
            if config.early_termination {
                let remaining = new_iterations - iterations_used;
                if remaining > 0 && root.children.len() >= 2 {
                    // Cheap check: gap exceeds remaining iterations
                    let (best, second) = top_two_visit_counts(&root.children);
                    if best - second > remaining {
                        break;
                    }
                    // Worst-case UCB simulation: even with optimal rewards for
                    // challengers (1.0) and worst rewards for leader (0.0),
                    // can any challenger overtake the leader by visit count?
                    // Only run when the leader has a meaningful lead (gap > remaining/4)
                    // to avoid wasting time on simulations that won't pass.
                    let gap = best - second;
                    if iterations_used % 1024 == 0
                        && gap > remaining / 4
                        && !can_challenger_overtake(
                            &root.children,
                            root.visit_count,
                            remaining,
                            config.exploration_constant,
                            config.progressive_bias_weight,
                        )
                    {
                        break;
                    }
                }
            }
        }
    }

    if root.children.is_empty() {
        enumerate_choices_into(state, &mut choices_buf);
        let idx = rng.random_range(0..choices_buf.len());
        return MctsResult { choice: choices_buf[idx].clone(), iterations_used, reused_iterations, tree: None };
    }

    let best_choice = root.children.iter()
        .max_by_key(|c| c.visit_count)
        .unwrap()
        .choice.clone().unwrap();

    MctsResult {
        choice: best_choice,
        iterations_used,
        reused_iterations,
        tree: Some(root),
    }
}

fn top_two_visit_counts(children: &[MctsNode]) -> (u32, u32) {
    let mut best = 0u32;
    let mut second = 0u32;
    for child in children {
        if child.visit_count > best {
            second = best;
            best = child.visit_count;
        } else if child.visit_count > second {
            second = child.visit_count;
        }
    }
    (best, second)
}

/// Worst-case simulation to check if any challenger can overtake the visit-count leader.
/// Assumes all non-leader children receive reward 1.0 and the leader receives reward 0.0.
/// Returns true if any challenger could possibly end up with >= the leader's visit count.
fn can_challenger_overtake(
    children: &[MctsNode],
    root_visit_count: u32,
    remaining: u32,
    exploration_constant: f64,
    progressive_bias_weight: f64,
) -> bool {
    let k = children.len();
    if k < 2 || remaining == 0 {
        return false;
    }

    let leader_idx = children.iter()
        .enumerate()
        .max_by_key(|(_, c)| c.visit_count)
        .unwrap()
        .0;

    let mut sim_visits: SmallVec<[u32; 16]> = children.iter().map(|c| c.visit_count).collect();
    let mut sim_cumulative: SmallVec<[f64; 16]> = children.iter().map(|c| c.cumulative_reward).collect();
    let heuristic_biases: SmallVec<[f64; 16]> = children.iter().map(|c| c.heuristic_bias).collect();
    let mut sim_root_visits = root_visit_count;

    let mut max_challenger_visits = 0u32;
    for j in 0..k {
        if j != leader_idx {
            max_challenger_visits = max_challenger_visits.max(sim_visits[j]);
        }
    }

    for step in 0..remaining {
        sim_root_visits += 1;
        let ln_total = (sim_root_visits as f64).ln();

        let mut best_idx = 0;
        let mut best_ucb = f64::NEG_INFINITY;

        for j in 0..k {
            let ucb = if sim_visits[j] == 0 {
                f64::INFINITY
            } else {
                let v = sim_visits[j] as f64;
                sim_cumulative[j] / v
                    + exploration_constant * (ln_total / v).sqrt()
                    + progressive_bias_weight * heuristic_biases[j] / (1.0 + v)
            };
            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = j;
            }
        }

        sim_visits[best_idx] += 1;
        if best_idx != leader_idx {
            sim_cumulative[best_idx] += 1.0;
            max_challenger_visits = max_challenger_visits.max(sim_visits[best_idx]);
        }

        // Fast exit: if the leader's gap exceeds remaining simulation steps,
        // no challenger can catch up even in this worst-case scenario
        let sim_remaining = remaining - step - 1;
        if sim_visits[leader_idx].saturating_sub(max_challenger_visits) > sim_remaining {
            return false;
        }
    }

    max_challenger_visits >= sim_visits[leader_idx]
}

fn eval_scores(
    state: &GameState,
    use_heuristic: bool,
    params: &HeuristicParams,
    card_table: &CardHeuristicTable,
    diff_eval: Option<(&DiffEvalParams, &DiffEvalTable)>,
) -> [f64; MAX_PLAYERS] {
    if use_heuristic {
        if let Some((diff_params, diff_table)) = diff_eval {
            compute_diff_eval_rewards(state, 0, diff_params, diff_table)
        } else {
            compute_heuristic_rewards(&state.players, &state.sell_card_display, &state.card_lookup, params, card_table)
        }
    } else {
        compute_terminal_rewards(&state.players)
    }
}

/// Helper to create diff_eval tuple from config and optional table
#[inline]
fn diff_eval_ref<'a>(config: &'a MctsConfig, diff_table: &'a Option<DiffEvalTable>) -> Option<(&'a DiffEvalParams, &'a DiffEvalTable)> {
    match (&config.diff_eval_params, diff_table) {
        (Some(params), Some(table)) => Some((params.as_ref(), table)),
        _ => None,
    }
}

fn iteration_simultaneous<R: Rng>(
    node: &mut MctsNode,
    state: &mut GameState,
    perspective_player: usize,
    opponent_stats: &mut OpponentDraftStats,
    pick_log: &mut Vec<(u32, usize, Card)>,
    max_rollout_round: Option<u32>,
    use_heuristic: bool,
    config: &MctsConfig,
    choices_buf: &mut Vec<Choice>,
    availability_buf: &mut Vec<bool>,
    card_table: &CardHeuristicTable,
    diff_table: &Option<DiffEvalTable>,
    rng: &mut R,
) -> [f64; MAX_PLAYERS] {
    let active_player = if matches!(state.phase, GamePhase::GameOver) {
        let scores = compute_terminal_rewards(&state.players);
        record_outcome(node, &scores);
        return scores;
    } else if max_rollout_round.is_some_and(|mr| state.round > mr) {
        let scores = eval_scores(state, use_heuristic, &config.heuristic_params, card_table, diff_eval_ref(config, diff_table));
        record_outcome(node, &scores);
        return scores;
    } else {
        match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => 0,
            _ => unreachable!(),
        }
    };

    // Enumerate choices (needed for both expand and select)
    enumerate_choices_into(state, choices_buf);

    // Expand (also populates availability_buf)
    node.expand(choices_buf, active_player, availability_buf, rng);

    // Select
    let best_idx =
        match select(node, availability_buf, config.exploration_constant, config.progressive_bias_weight)
        {
            Some(idx) => idx,
            None => {
                let empty_scores = [0.0; MAX_PLAYERS];
                record_outcome(node, &empty_scores);
                return empty_scores;
            }
        };

    // Apply selected child's choice
    let choice = node.children[best_idx].choice.as_ref().unwrap();
    apply_choice_to_state(state, choice, rng);

    // After applying the perspective player's draft pick, advance past opponents
    advance_past_opponent_draft_picks(
        state, perspective_player, opponent_stats,
        pick_log, config.exploration_constant, rng,
    );

    let should_rollout = node.children[best_idx].visit_count == 0;

    if should_rollout && config.progressive_bias_weight != 0.0 {
        node.children[best_idx].heuristic_bias = if let Some((diff_params, dt)) = diff_eval_ref(config, diff_table) {
            diff_eval_score(
                state,
                perspective_player,
                diff_params,
                dt,
            )
        } else {
            heuristic_score(
                &state.players[perspective_player],
                &state.sell_card_display,
                &state.card_lookup,
                &config.heuristic_params,
                card_table,
            )
        };
    }

    let scores = if should_rollout {
        let de = diff_eval_ref(config, diff_table);
        let scores = if config.no_rollout {
            eval_scores(state, true, &config.heuristic_params, card_table, de)
        } else {
            rollout(state, max_rollout_round, config.max_rollout_steps, use_heuristic, config.heuristic_rollout, config.heuristic_draft, &config.heuristic_params, card_table, de, rng)
        };
        record_outcome(&mut node.children[best_idx], &scores);
        scores
    } else {
        let child = &mut node.children[best_idx];
        iteration_simultaneous(
            child, state, perspective_player,
            opponent_stats, pick_log,
            max_rollout_round, use_heuristic, config, choices_buf, availability_buf, card_table, diff_table, rng,
        )
    };

    record_outcome(node, &scores);
    scores
}

fn select(
    node: &MctsNode,
    available: &[bool],
    c: f64,
    progressive_bias_weight: f64,
) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_value = f64::NEG_INFINITY;

    let root_ln = if node.is_root() { (node.visit_count as f64).ln() } else { 0.0 };

    for (idx, child) in node.children.iter().enumerate() {
        if !available[idx] {
            continue;
        }

        let value = if child.visit_count == 0 {
            f64::INFINITY
        } else {
            let ln_total = if node.is_root() { root_ln } else { child.ln_availability };
            let visit_count_f = child.visit_count as f64;
            let win_rate = child.cumulative_reward / visit_count_f;
            let exploration = c * (ln_total / visit_count_f).sqrt();

            win_rate + exploration
                + progressive_bias_weight * child.heuristic_bias / (1.0 + visit_count_f)
        };

        if value > best_value {
            best_value = value;
            best_idx = Some(idx);
        }
    }

    best_idx
}

fn rollout<R: Rng>(state: &mut GameState, max_rollout_round: Option<u32>, max_rollout_steps: u32, use_heuristic: bool, heuristic_rollout: bool, heuristic_draft: bool, params: &HeuristicParams, card_table: &CardHeuristicTable, diff_eval: Option<(&DiffEvalParams, &DiffEvalTable)>, rng: &mut R) -> [f64; MAX_PLAYERS] {
    for _ in 0..max_rollout_steps {
        if matches!(state.phase, GamePhase::GameOver) {
            return compute_terminal_rewards(&state.players);
        }
        if max_rollout_round.is_some_and(|mr| state.round > mr) {
            return eval_scores(state, use_heuristic, params, card_table, diff_eval);
        }
        if heuristic_rollout {
            apply_heuristic_rollout_step(state, heuristic_draft, rng);
        } else {
            apply_rollout_step(state, heuristic_draft, rng);
        }
    }

    eval_scores(state, use_heuristic, params, card_table, diff_eval)
}

fn record_outcome(node: &mut MctsNode, scores: &[f64; MAX_PLAYERS]) {
    node.cumulative_reward += scores[node.player_index];
    node.visit_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colori_game::{
        apply_choice_to_state, check_choice_available, enumerate_choices_into,
        get_game_status, GameStatus,
    };
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::{create_initial_game_state, create_initial_game_state_with_expansions};
    use crate::types::Expansions;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn run_full_game_validating_choices(num_players: usize, seed: u64) {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

        let config = MctsConfig {
            iterations: 10,
            ..MctsConfig::default()
        };

        execute_draw_phase(&mut state, &mut rng);

        let mut choices_buf: Vec<Choice> = Vec::new();
        let max_steps = 5000;

        for step in 0..max_steps {
            match &state.phase {
                GamePhase::GameOver => return,
                GamePhase::Draw => {
                    execute_draw_phase(&mut state, &mut rng);
                    continue;
                }
                GamePhase::Draft { .. } => {}
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let choice = ismcts(&state, player_index, &config, None, None, &mut rng).choice;

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: ISMCTS choice {choice:?} \
                 not in enumerated choices",
                state.round, state.phase
            );

            assert!(
                check_choice_available(&state, &choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: check_choice_available returned \
                 false for {choice:?}",
                state.round, state.phase
            );

            apply_choice_to_state(&mut state, &choice, &mut rng);
        }

        panic!(
            "seed={seed}, players={num_players}: \
             game did not finish within {max_steps} steps"
        );
    }

    #[test]
    fn test_ismcts_valid_moves_2_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(2, seed);
        }
    }

    #[test]
    fn test_ismcts_valid_moves_3_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(3, seed);
        }
    }

    #[test]
    fn test_ismcts_valid_moves_4_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(4, seed);
        }
    }

    fn run_full_game_validating_choices_with_glass(num_players: usize, seed: u64) {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state_with_expansions(
            num_players,
            &ai_players,
            Expansions { glass: true },
            &mut rng,
        );

        let config = MctsConfig {
            iterations: 10,
            ..MctsConfig::default()
        };

        execute_draw_phase(&mut state, &mut rng);

        let mut choices_buf: Vec<Choice> = Vec::new();
        let max_steps = 5000;

        for step in 0..max_steps {
            match &state.phase {
                GamePhase::GameOver => return,
                GamePhase::Draw => {
                    execute_draw_phase(&mut state, &mut rng);
                    continue;
                }
                GamePhase::Draft { .. } => {}
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let choice = ismcts(&state, player_index, &config, None, None, &mut rng).choice;

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: ISMCTS choice {choice:?} \
                 not in enumerated choices",
                state.round, state.phase
            );

            assert!(
                check_choice_available(&state, &choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: check_choice_available returned \
                 false for {choice:?}",
                state.round, state.phase
            );

            apply_choice_to_state(&mut state, &choice, &mut rng);
        }

        panic!(
            "seed={seed}, players={num_players}: \
             game did not finish within {max_steps} steps"
        );
    }

    #[test]
    fn test_ismcts_valid_moves_2_players_glass() {
        for seed in 0..5 {
            run_full_game_validating_choices_with_glass(2, seed);
        }
    }

    #[test]
    fn test_ismcts_valid_moves_3_players_glass() {
        for seed in 0..5 {
            run_full_game_validating_choices_with_glass(3, seed);
        }
    }

    #[test]
    fn test_ismcts_valid_moves_4_players_glass() {
        for seed in 0..5 {
            run_full_game_validating_choices_with_glass(4, seed);
        }
    }

    fn run_full_game_with_config(num_players: usize, seed: u64, config: &MctsConfig) {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

        execute_draw_phase(&mut state, &mut rng);

        let mut choices_buf: Vec<Choice> = Vec::new();
        let max_steps = 5000;

        for _step in 0..max_steps {
            match &state.phase {
                GamePhase::GameOver => return,
                GamePhase::Draw => {
                    execute_draw_phase(&mut state, &mut rng);
                    continue;
                }
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let choice = ismcts(&state, player_index, config, None, None, &mut rng).choice;

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(choices_buf.contains(&choice));

            apply_choice_to_state(&mut state, &choice, &mut rng);
        }

        panic!("seed={seed}, players={num_players}: game did not finish within {max_steps} steps");
    }

    #[test]
    fn test_ismcts_with_heuristic_rollout() {
        let config = MctsConfig {
            iterations: 10,
            ..MctsConfig::default()
        };
        for num_players in 2..=4 {
            for seed in 0..3 {
                run_full_game_with_config(num_players, seed, &config);
            }
        }
    }

    #[test]
    fn test_ismcts_with_heuristic_rollout_glass() {
        let config = MctsConfig {
            iterations: 10,
            ..MctsConfig::default()
        };
        for seed in 0..3 {
            let mut rng = WyRand::seed_from_u64(seed);
            let ai_players = vec![true; 2];
            let mut state = create_initial_game_state_with_expansions(
                2, &ai_players, Expansions { glass: true }, &mut rng,
            );

            execute_draw_phase(&mut state, &mut rng);

            let mut choices_buf: Vec<Choice> = Vec::new();
            for _step in 0..5000 {
                match &state.phase {
                    GamePhase::GameOver => break,
                    GamePhase::Draw => {
                        execute_draw_phase(&mut state, &mut rng);
                        continue;
                    }
                    _ => {}
                }

                let player_index = match get_game_status(&state, None) {
                    GameStatus::AwaitingAction { player_index } => player_index,
                    GameStatus::Terminated { .. } => break,
                };

                let choice = ismcts(&state, player_index, &config, None, None, &mut rng).choice;
                enumerate_choices_into(&state, &mut choices_buf);
                assert!(choices_buf.contains(&choice));
                apply_choice_to_state(&mut state, &choice, &mut rng);
            }
        }
    }
}
