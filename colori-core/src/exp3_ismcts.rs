use crate::colori_game::{
    apply_choice_to_state, apply_rollout_step,
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
pub struct Exp3Config {
    pub iterations: u32,
    pub gamma: f64,
    pub max_rollout_steps: u32,
}

impl Default for Exp3Config {
    fn default() -> Self {
        Exp3Config {
            iterations: 100,
            gamma: 0.1,
            max_rollout_steps: 1000,
        }
    }
}

impl<'de> Deserialize<'de> for Exp3Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Exp3ConfigHelper {
            #[serde(default = "default_iterations")]
            iterations: u32,
            #[serde(default = "default_gamma")]
            gamma: f64,
            #[serde(default = "default_max_rollout_steps")]
            max_rollout_steps: u32,
        }

        fn default_iterations() -> u32 { 100 }
        fn default_gamma() -> f64 { 0.1 }
        fn default_max_rollout_steps() -> u32 { 1000 }

        let helper = Exp3ConfigHelper::deserialize(deserializer)?;
        Ok(Exp3Config {
            iterations: helper.iterations,
            gamma: helper.gamma,
            max_rollout_steps: helper.max_rollout_steps,
        })
    }
}

struct Exp3Node {
    visit_count: u32,
    cumulative_reward: f64,
    log_weight: f64,
    player_index: usize,
    choice: Option<Choice>,
    availability_count: u32,
    children: Vec<Exp3Node>,
}

impl Exp3Node {
    fn new(player_index: usize, choice: Option<Choice>) -> Self {
        Exp3Node {
            visit_count: 0,
            cumulative_reward: 0.0,
            log_weight: 0.0,
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
        available: &mut Vec<bool>,
        rng: &mut R,
    ) {
        let len = choices.len();
        for i in (1..len).rev() {
            let j = rng.random_range(0..=i);
            choices.swap(i, j);
        }

        available.clear();
        available.resize(self.children.len(), false);

        let mut added_new_node = false;

        for choice in choices.iter() {
            if let Some(idx) = self
                .children
                .iter()
                .position(|c| c.choice.as_ref() == Some(choice))
            {
                if !available[idx] {
                    self.children[idx].availability_count += 1;
                    available[idx] = true;
                }
            } else if self.is_root() || !added_new_node {
                let mut new_node = Exp3Node::new(active_player, Some(choice.clone()));
                new_node.availability_count = 1;
                available.push(true);
                self.children.push(new_node);
                added_new_node = true;
            }
        }
    }
}

/// EXP3 selection: returns (selected_index, selection_probability).
/// Unvisited children are picked uniformly at random.
/// Visited children use EXP3 probability distribution.
fn exp3_select<R: Rng>(
    node: &Exp3Node,
    available: &[bool],
    gamma: f64,
    rng: &mut R,
) -> Option<(usize, f64)> {
    let mut available_indices: SmallVec<[usize; 16]> = SmallVec::new();
    let mut unvisited_indices: SmallVec<[usize; 16]> = SmallVec::new();

    for (idx, &is_available) in available.iter().enumerate() {
        if !is_available {
            continue;
        }
        available_indices.push(idx);
        if node.children[idx].visit_count == 0 {
            unvisited_indices.push(idx);
        }
    }

    if available_indices.is_empty() {
        return None;
    }

    // If there are unvisited children, pick one uniformly at random
    if !unvisited_indices.is_empty() {
        let pick = rng.random_range(0..unvisited_indices.len());
        let idx = unvisited_indices[pick];
        let prob = 1.0 / unvisited_indices.len() as f64;
        return Some((idx, prob));
    }

    let k = available_indices.len() as f64;

    // Compute log-sum-exp for numerical stability
    let max_log_weight = available_indices
        .iter()
        .map(|&idx| node.children[idx].log_weight)
        .fold(f64::NEG_INFINITY, f64::max);

    let sum_exp: f64 = available_indices
        .iter()
        .map(|&idx| (node.children[idx].log_weight - max_log_weight).exp())
        .sum();

    // Compute probabilities: p_i = (1-γ) * w_i/Σw + γ/K
    let mut probs: SmallVec<[f64; 16]> = SmallVec::new();
    for &idx in &available_indices {
        let normalized_weight =
            (node.children[idx].log_weight - max_log_weight).exp() / sum_exp;
        let p = (1.0 - gamma) * normalized_weight + gamma / k;
        probs.push(p);
    }

    // Sample from the distribution
    let r: f64 = rng.random_range(0.0..1.0);
    let mut cumulative = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        cumulative += p;
        if r < cumulative {
            return Some((available_indices[i], p));
        }
    }

    // Fallback to last available (rounding)
    let last = available_indices.len() - 1;
    Some((available_indices[last], probs[last]))
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

fn record_outcome(node: &mut Exp3Node, scores: &[f64]) {
    let reward = if node.player_index < scores.len() {
        scores[node.player_index]
    } else {
        0.0
    };
    node.cumulative_reward += reward;
    node.visit_count += 1;
}

pub fn exp3_ismcts<R: Rng>(
    state: &GameState,
    player_index: usize,
    config: &Exp3Config,
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    max_rollout_round: Option<u32>,
    rng: &mut R,
) -> Choice {
    let mut choices_buf: Vec<Choice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let mut root = Exp3Node::new(player_index, None);
    let mut det_state = state.clone();

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    let mut availability_buf: Vec<bool> = Vec::new();

    for _ in 0..config.iterations {
        determinize_in_place(&mut det_state, state, player_index, known_draft_hands, &cached_scores, rng);
        iteration(&mut root, &mut det_state, max_rollout_round, config, &mut choices_buf, &mut availability_buf, rng);
    }

    if root.children.is_empty() {
        enumerate_choices_into(state, &mut choices_buf);
        let idx = rng.random_range(0..choices_buf.len());
        return choices_buf[idx].clone();
    }

    let mut best_child: Option<&Exp3Node> = None;
    for child in root.children.iter() {
        if best_child.is_none() || child.visit_count > best_child.unwrap().visit_count {
            best_child = Some(child);
        }
    }

    best_child.unwrap().choice.clone().unwrap()
}

fn iteration<R: Rng>(
    node: &mut Exp3Node,
    state: &mut GameState,
    max_rollout_round: Option<u32>,
    config: &Exp3Config,
    choices_buf: &mut Vec<Choice>,
    availability_buf: &mut Vec<bool>,
    rng: &mut R,
) -> SmallVec<[f64; 4]> {
    let active_player = match get_game_status(state, max_rollout_round) {
        GameStatus::Terminated { scores } => {
            record_outcome(node, &scores);
            return scores;
        }
        GameStatus::AwaitingAction { player_index } => player_index,
    };

    enumerate_choices_into(state, choices_buf);
    node.expand(choices_buf, active_player, availability_buf, rng);

    let (best_idx, selection_prob) =
        match exp3_select(node, availability_buf, config.gamma, rng) {
            Some(result) => result,
            None => {
                let empty_scores = SmallVec::new();
                record_outcome(node, &empty_scores);
                return empty_scores;
            }
        };

    let choice = node.children[best_idx].choice.clone().unwrap();
    apply_choice_to_state(state, &choice, rng);

    let should_rollout = node.children[best_idx].visit_count == 0;

    let scores = if should_rollout {
        let scores = rollout(state, max_rollout_round, config.max_rollout_steps, rng);
        record_outcome(&mut node.children[best_idx], &scores);
        scores
    } else {
        let child = &mut node.children[best_idx];
        iteration(child, state, max_rollout_round, config, choices_buf, availability_buf, rng)
    };

    // EXP3 weight update for the selected child
    let k = availability_buf.iter().filter(|&&a| a).count() as f64;
    if k > 0.0 && selection_prob > 0.0 {
        let reward = if node.children[best_idx].player_index < scores.len() {
            scores[node.children[best_idx].player_index]
        } else {
            0.0
        };
        node.children[best_idx].log_weight +=
            config.gamma * (reward / selection_prob) / k;
    }

    record_outcome(node, &scores);
    scores
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

        let config = Exp3Config {
            iterations: 10,
            ..Exp3Config::default()
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

            let choice = exp3_ismcts(&state, player_index, &config, &None, None, &mut rng);

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: EXP3 ISMCTS choice {choice:?} \
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
    fn test_exp3_ismcts_valid_moves_2_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(2, seed);
        }
    }

    #[test]
    fn test_exp3_ismcts_valid_moves_3_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(3, seed);
        }
    }

    #[test]
    fn test_exp3_ismcts_valid_moves_4_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(4, seed);
        }
    }
}
