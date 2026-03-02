use crate::colori_game::{
    apply_choice_to_state, apply_rollout_step, check_choice_available,
    determinize_in_place, enumerate_choices_into,
    get_game_status, GameStatus,
};
use crate::scoring::calculate_score;
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
}

impl Default for MctsConfig {
    fn default() -> Self {
        MctsConfig {
            iterations: 100,
            exploration_constant: std::f64::consts::SQRT_2,
            max_rollout_steps: 1000,
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
        }

        fn default_iterations() -> u32 { 100 }
        fn default_exploration_constant() -> f64 { std::f64::consts::SQRT_2 }
        fn default_max_rollout_steps() -> u32 { 1000 }

        let helper = MctsConfigHelper::deserialize(deserializer)?;
        Ok(MctsConfig {
            iterations: helper.iterations,
            exploration_constant: helper.exploration_constant,
            max_rollout_steps: helper.max_rollout_steps,
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
        choices: &mut Vec<Choice>,
        active_player: usize,
        rng: &mut R,
    ) {
        // Shuffle choices in place
        let len = choices.len();
        for i in (1..len).rev() {
            let j = rng.random_range(0..=i);
            choices.swap(i, j);
        }

        let mut added_new_node = false;
        // Track which choices we've already incremented availability for
        // to avoid double-counting duplicates within the same determinization
        let mut seen_this_expand: SmallVec<[usize; 32]> = SmallVec::new();

        for choice in choices.iter() {
            if let Some(idx) = self
                .children
                .iter()
                .position(|c| c.choice.as_ref() == Some(choice))
            {
                if !seen_this_expand.contains(&idx) {
                    self.children[idx].availability_count += 1;
                    seen_this_expand.push(idx);
                }
            } else if self.is_root() || !added_new_node {
                let mut new_node = MctsNode::new(active_player, Some(choice.clone()));
                new_node.availability_count = 1;
                seen_this_expand.push(self.children.len());
                self.children.push(new_node);
                added_new_node = true;
            }
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

    for _ in 0..config.iterations {
        determinize_in_place(&mut det_state, state, player_index, known_draft_hands, &cached_scores, rng);
        iteration(&mut root, &mut det_state, max_rollout_round, config, &mut choices_buf, rng);
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

fn iteration<R: Rng>(
    node: &mut MctsNode,
    state: &mut GameState,
    max_rollout_round: Option<u32>,
    config: &MctsConfig,
    choices_buf: &mut Vec<Choice>,
    rng: &mut R,
) -> SmallVec<[f64; 4]> {
    let active_player = match get_game_status(state, max_rollout_round) {
        GameStatus::Terminated { scores } => {
            record_outcome(node, &scores);
            return scores;
        }
        GameStatus::AwaitingAction { player_index } => player_index,
    };

    // Enumerate choices (needed for both expand and select)
    enumerate_choices_into(state, choices_buf);

    // Expand
    if !(node.is_root() && !node.children.is_empty()) {
        node.expand(choices_buf, active_player, rng);
    }

    // Select
    let best_idx =
        match select(node, state, config.exploration_constant)
        {
            Some(idx) => idx,
            None => {
                let empty_scores = SmallVec::new();
                record_outcome(node, &empty_scores);
                return empty_scores;
            }
        };

    // Apply selected child's choice
    let choice = node.children[best_idx].choice.clone().unwrap();
    apply_choice_to_state(state, &choice, rng);

    let should_rollout = node.children[best_idx].visit_count == 0;

    let scores = if should_rollout {
        let scores = rollout(state, max_rollout_round, config.max_rollout_steps, rng);
        record_outcome(&mut node.children[best_idx], &scores);
        scores
    } else {
        let child = &mut node.children[best_idx];
        iteration(child, state, max_rollout_round, config, choices_buf, rng)
    };

    record_outcome(node, &scores);
    scores
}

fn select(
    node: &MctsNode,
    state: &GameState,
    c: f64,
) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_value = f64::NEG_INFINITY;

    let root_ln = if node.is_root() { (node.visit_count as f64).ln() } else { 0.0 };

    for (idx, child) in node.children.iter().enumerate() {
        let available = check_choice_available(state, child.choice.as_ref().unwrap());
        if !available {
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

#[inline]
fn is_terminal(state: &GameState, max_rollout_round: Option<u32>) -> bool {
    matches!(state.phase, GamePhase::GameOver)
        || max_rollout_round.is_some_and(|mr| state.round > mr)
}

#[inline]
fn compute_terminal_scores(state: &GameState) -> SmallVec<[f64; 4]> {
    let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| p.cached_score as f64).collect();
    let max_score = scores.iter().cloned().fold(0.0f64, f64::max);
    let num_winners = scores.iter().filter(|&&s| s == max_score).count() as f64;
    scores.iter().map(|&s| if s == max_score { 1.0 / num_winners } else { 0.0 }).collect()
}

fn rollout<R: Rng>(state: &mut GameState, max_rollout_round: Option<u32>, max_rollout_steps: u32, rng: &mut R) -> SmallVec<[f64; 4]> {
    for _ in 0..max_rollout_steps {
        if is_terminal(state, max_rollout_round) {
            return compute_terminal_scores(state);
        }
        apply_rollout_step(state, rng);
    }

    if is_terminal(state, max_rollout_round) {
        return compute_terminal_scores(state);
    }

    SmallVec::new()
}

fn record_outcome(node: &mut MctsNode, scores: &[f64]) {
    let reward = if node.player_index < scores.len() {
        scores[node.player_index]
    } else {
        0.0
    };
    node.cumulative_reward += reward;
    node.visit_count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colori_game::{
        apply_choice_to_state, check_choice_available, enumerate_choices_into,
    };
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
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
                GamePhase::Draft { draft_state } => {
                    assert!(
                        !draft_state.waiting_for_pass,
                        "seed={seed}, players={num_players}, \
                         step={step}, round={}: unexpected waiting_for_pass in Draft phase",
                        state.round
                    );
                }
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
}
