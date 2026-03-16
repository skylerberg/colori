use crate::colori_game::{
    apply_choice_to_state, apply_rollout_step,
    determinize_in_place, enumerate_choices_into,
};
use crate::draft_phase::player_pick;
use crate::scoring::{calculate_score, compute_heuristic_rewards, compute_terminal_rewards, HeuristicParams};
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
    pub heuristic_params: HeuristicParams,
}

impl Default for MctsConfig {
    fn default() -> Self {
        MctsConfig {
            iterations: 100,
            exploration_constant: std::f64::consts::SQRT_2,
            max_rollout_steps: 1000,
            use_heuristic_eval: true,
            heuristic_params: HeuristicParams::default(),
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
            #[serde(default)]
            heuristic_params: HeuristicParams,
        }

        fn default_iterations() -> u32 { 100 }
        fn default_exploration_constant() -> f64 { std::f64::consts::SQRT_2 }
        fn default_max_rollout_steps() -> u32 { 1000 }
        fn default_use_heuristic_eval() -> bool { true }

        let helper = MctsConfigHelper::deserialize(deserializer)?;
        Ok(MctsConfig {
            iterations: helper.iterations,
            exploration_constant: helper.exploration_constant,
            max_rollout_steps: helper.max_rollout_steps,
            use_heuristic_eval: helper.use_heuristic_eval,
            heuristic_params: helper.heuristic_params,
        })
    }
}

struct MctsNode {
    visit_count: u32,
    cumulative_reward: f64,
    player_index: usize,
    choice: Option<Choice>,
    availability_count: u32,
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
            children: Vec::new(),
        }
    }

    fn is_root(&self) -> bool {
        self.choice.is_none()
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

fn upper_confidence_bound_with_ln(
    cumulative_reward: f64,
    visit_count: u32,
    ln_total: f64,
    c: f64,
) -> f64 {
    let win_rate = cumulative_reward / visit_count as f64;
    win_rate + c * (ln_total / visit_count as f64).sqrt()
}

// ── DUCT (Decoupled UCT) for opponent draft modeling ──

const NUM_CARDS: usize = 46;

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
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    max_rollout_round: Option<u32>,
    rng: &mut R,
) -> Choice {
    // If there's only one legal choice, return it immediately without searching
    let mut choices_buf: Vec<Choice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let mut root = MctsNode::new(player_index, None);
    let mut det_state = state.clone();

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    let mut availability_buf: Vec<bool> = Vec::new();

    let mut opponent_stats = OpponentDraftStats::new();
    let mut pick_log: Vec<(u32, usize, Card)> = Vec::new();

    let (effective_max_rollout_round, use_heuristic) = if config.use_heuristic_eval && {
        if let Some(score_threshold) = config.heuristic_params.heuristic_score_threshold {
            let max_score = state.players.iter().map(|p| p.cached_score).max().unwrap_or(0);
            (max_score as f64) < score_threshold
        } else {
            state.round <= config.heuristic_params.heuristic_round_threshold
        }
    } {
        let heuristic_round = state.round + config.heuristic_params.heuristic_lookahead;
        let effective = max_rollout_round.map_or(heuristic_round, |mr| mr.min(heuristic_round));
        (Some(effective), true)
    } else {
        (max_rollout_round, false)
    };

    for _ in 0..config.iterations {
        pick_log.clear();
        determinize_in_place(&mut det_state, state, player_index, known_draft_hands, &cached_scores, rng);
        advance_past_opponent_draft_picks(
            &mut det_state, player_index, &mut opponent_stats,
            &mut pick_log, config.exploration_constant, rng,
        );
        let scores = iteration_simultaneous(
            &mut root, &mut det_state, player_index,
            &mut opponent_stats, &mut pick_log,
            effective_max_rollout_round, use_heuristic, config, &mut choices_buf, &mut availability_buf, rng,
        );
        for &(pick_round, player, card) in &pick_log {
            let reward = scores[player];
            opponent_stats.record_outcome(pick_round as usize, player, card, reward);
        }
    }

    if root.children.is_empty() {
        enumerate_choices_into(state, &mut choices_buf);
        let idx = rng.random_range(0..choices_buf.len());
        return choices_buf[idx].clone();
    }

    let mut best_child: Option<&MctsNode> = None;
    for child in root.children.iter() {
        if best_child.is_none() || child.visit_count > best_child.unwrap().visit_count {
            best_child = Some(child);
        }
    }

    best_child.unwrap().choice.clone().unwrap()
}

fn eval_scores(state: &GameState, use_heuristic: bool, params: &HeuristicParams) -> [f64; MAX_PLAYERS] {
    if use_heuristic {
        compute_heuristic_rewards(&state.players, &state.sell_card_display, &state.card_lookup, params)
    } else {
        compute_terminal_rewards(&state.players)
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
    rng: &mut R,
) -> [f64; MAX_PLAYERS] {
    let active_player = if matches!(state.phase, GamePhase::GameOver) {
        let scores = compute_terminal_rewards(&state.players);
        record_outcome(node, &scores);
        return scores;
    } else if max_rollout_round.is_some_and(|mr| state.round > mr) {
        let scores = eval_scores(state, use_heuristic, &config.heuristic_params);
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
        match select(node, availability_buf, config.exploration_constant)
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

    let scores = if should_rollout {
        let scores = rollout(state, max_rollout_round, config.max_rollout_steps, use_heuristic, &config.heuristic_params, rng);
        record_outcome(&mut node.children[best_idx], &scores);
        scores
    } else {
        let child = &mut node.children[best_idx];
        iteration_simultaneous(
            child, state, perspective_player,
            opponent_stats, pick_log,
            max_rollout_round, use_heuristic, config, choices_buf, availability_buf, rng,
        )
    };

    record_outcome(node, &scores);
    scores
}

fn select(
    node: &MctsNode,
    available: &[bool],
    c: f64,
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
        } else if node.is_root() {
            upper_confidence_bound_with_ln(child.cumulative_reward, child.visit_count, root_ln, c)
        } else {
            let total_visit_count = child.availability_count.max(1);
            upper_confidence_bound(child.cumulative_reward, child.visit_count, total_visit_count, c)
        };

        if value > best_value {
            best_value = value;
            best_idx = Some(idx);
        }
    }

    best_idx
}

fn rollout<R: Rng>(state: &mut GameState, max_rollout_round: Option<u32>, max_rollout_steps: u32, use_heuristic: bool, params: &HeuristicParams, rng: &mut R) -> [f64; MAX_PLAYERS] {
    for _ in 0..max_rollout_steps {
        if matches!(state.phase, GamePhase::GameOver) {
            return compute_terminal_rewards(&state.players);
        }
        if max_rollout_round.is_some_and(|mr| state.round > mr) {
            return eval_scores(state, use_heuristic, params);
        }
        apply_rollout_step(state, rng);
    }

    if matches!(state.phase, GamePhase::GameOver) {
        return compute_terminal_rewards(&state.players);
    }
    if max_rollout_round.is_some_and(|mr| state.round > mr) {
        return eval_scores(state, use_heuristic, params);
    }

    [0.0; MAX_PLAYERS]
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

            let choice = ismcts(&state, player_index, &config, &None, None, &mut rng);

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

            let choice = ismcts(&state, player_index, &config, &None, None, &mut rng);

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
}
