use crate::colori_game::{
    abstract_choice, apply_choice_to_state, apply_rollout_step, deduplicate_choices,
    determinize_in_place, enumerate_choices_canonical_into, enumerate_choices_into,
    get_game_status, resolve_abstract_choice, GameStatus,
};
use crate::scoring::calculate_score;
use crate::types::*;
use rand::Rng;
use serde::Deserialize;
use smallvec::SmallVec;

#[derive(Clone, Debug)]
pub struct MctsConfig {
    pub iterations: u32,
    pub exploration_constant: f64,
    pub max_rollout_steps: u32,
    pub canonical_ordering: bool,
    pub abstract_choices: bool,
}

impl Default for MctsConfig {
    fn default() -> Self {
        MctsConfig {
            iterations: 100,
            exploration_constant: std::f64::consts::SQRT_2,
            max_rollout_steps: 1000,
            canonical_ordering: false,
            abstract_choices: false,
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
            #[serde(default)]
            canonical_ordering: bool,
            #[serde(default)]
            abstract_choices: bool,
        }

        fn default_iterations() -> u32 { 100 }
        fn default_exploration_constant() -> f64 { std::f64::consts::SQRT_2 }
        fn default_max_rollout_steps() -> u32 { 1000 }

        let helper = MctsConfigHelper::deserialize(deserializer)?;
        Ok(MctsConfig {
            iterations: helper.iterations,
            exploration_constant: helper.exploration_constant,
            max_rollout_steps: helper.max_rollout_steps,
            canonical_ordering: helper.canonical_ordering,
            abstract_choices: helper.abstract_choices,
        })
    }
}

struct MctsNode {
    games: u32,
    cumulative_reward: f64,
    player_id: usize,
    choice: Option<AbstractChoice>,
    availability_count: u32,
    children: Vec<MctsNode>,
}

impl MctsNode {
    fn new(player_id: usize, choice: Option<AbstractChoice>) -> Self {
        MctsNode {
            games: 0,
            cumulative_reward: 0.0,
            player_id,
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
        choices: &mut Vec<ColoriChoice>,
        active_player: usize,
        state: &GameState,
        rng: &mut R,
    ) {
        // Shuffle choices in place
        let len = choices.len();
        for i in (1..len).rev() {
            let j = rng.random_range(0..=i);
            choices.swap(i, j);
        }

        let mut added_new_node = false;
        // Track which abstract choices we've already incremented availability for
        // to avoid double-counting duplicates within the same determinization
        let mut seen_this_expand: SmallVec<[usize; 32]> = SmallVec::new();

        for choice in choices.iter() {
            let abs = abstract_choice(choice, state);
            if let Some(idx) = self.children.iter().position(|c| c.choice.as_ref() == Some(&abs))
            {
                if !seen_this_expand.contains(&idx) {
                    self.children[idx].availability_count += 1;
                    seen_this_expand.push(idx);
                }
            } else if self.is_root() || !added_new_node {
                let mut new_node = MctsNode::new(active_player, Some(abs));
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
    games: u32,
    total_game_count: u32,
    c: f64,
) -> f64 {
    let win_rate = cumulative_reward / games as f64;
    win_rate + c * ((total_game_count as f64).ln() / games as f64).sqrt()
}

fn upper_confidence_bound_with_ln(
    cumulative_reward: f64,
    games: u32,
    ln_total: f64,
    c: f64,
) -> f64 {
    let win_rate = cumulative_reward / games as f64;
    win_rate + c * (ln_total / games as f64).sqrt()
}

pub fn ismcts<R: Rng>(
    state: &GameState,
    player_id: usize,
    config: &MctsConfig,
    seen_hands: &Option<Vec<Vec<CardInstance>>>,
    max_round: Option<u32>,
    rng: &mut R,
) -> ColoriChoice {
    // If there's only one legal choice, return it immediately without searching
    let mut choices_buf: Vec<ColoriChoice> = Vec::new();
    if config.canonical_ordering {
        enumerate_choices_canonical_into(state, &mut choices_buf);
    } else {
        enumerate_choices_into(state, &mut choices_buf);
    }
    if config.abstract_choices {
        deduplicate_choices(&mut choices_buf, state);
    }
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let mut root = MctsNode::new(player_id, None);
    let mut det_state = state.clone();

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    for _ in 0..config.iterations {
        determinize_in_place(&mut det_state, state, player_id, seen_hands, &cached_scores, rng);
        iteration(&mut root, &mut det_state, max_round, config, &mut choices_buf, rng);
    }

    if root.children.is_empty() {
        if config.canonical_ordering {
            enumerate_choices_canonical_into(state, &mut choices_buf);
        } else {
            enumerate_choices_into(state, &mut choices_buf);
        }
        let idx = rng.random_range(0..choices_buf.len());
        return choices_buf[idx].clone();
    }

    let mut best_child: Option<&MctsNode> = None;
    for child in root.children.iter() {
        if best_child.is_none() || child.games > best_child.unwrap().games {
            best_child = Some(child);
        }
    }

    // Resolve the best AbstractChoice back to a concrete ColoriChoice using the original state
    let best_abs = best_child.unwrap().choice.as_ref().unwrap();
    resolve_abstract_choice(best_abs, state).unwrap()
}

fn iteration<R: Rng>(
    node: &mut MctsNode,
    state: &mut GameState,
    max_round: Option<u32>,
    config: &MctsConfig,
    choices_buf: &mut Vec<ColoriChoice>,
    rng: &mut R,
) -> SmallVec<[f64; 4]> {
    let active_player = match get_game_status(state, max_round) {
        GameStatus::Terminated { scores } => {
            record_outcome(node, &scores);
            return scores;
        }
        GameStatus::AwaitingAction { player_id } => player_id,
    };

    // Expand
    if !(node.is_root() && !node.children.is_empty()) {
        if config.canonical_ordering {
            enumerate_choices_canonical_into(state, choices_buf);
        } else {
            enumerate_choices_into(state, choices_buf);
        }
        if config.abstract_choices {
            deduplicate_choices(choices_buf, state);
        }
        node.expand(choices_buf, active_player, state, rng);
    }

    // Select
    let best_idx = match select(node, state, config.exploration_constant) {
        Some(idx) => idx,
        None => {
            let empty_scores = SmallVec::new();
            record_outcome(node, &empty_scores);
            return empty_scores;
        }
    };

    // Resolve AbstractChoice to concrete ColoriChoice and apply
    let abs = node.children[best_idx].choice.as_ref().unwrap();
    let choice = resolve_abstract_choice(abs, state).unwrap();
    apply_choice_to_state(state, &choice, rng);

    let should_rollout = node.children[best_idx].games == 0;

    let scores = if should_rollout {
        let scores = rollout(state, max_round, config.max_rollout_steps, rng);
        record_outcome(&mut node.children[best_idx], &scores);
        scores
    } else {
        let child = &mut node.children[best_idx];
        iteration(child, state, max_round, config, choices_buf, rng)
    };

    record_outcome(node, &scores);
    scores
}

fn select(node: &MctsNode, state: &GameState, c: f64) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_value = f64::NEG_INFINITY;

    let root_ln = if node.is_root() { (node.games as f64).ln() } else { 0.0 };

    for (idx, child) in node.children.iter().enumerate() {
        if resolve_abstract_choice(child.choice.as_ref().unwrap(), state).is_none() {
            continue;
        }

        let value = if child.games == 0 {
            f64::INFINITY
        } else if node.is_root() {
            upper_confidence_bound_with_ln(child.cumulative_reward, child.games, root_ln, c)
        } else {
            let total_game_count = child.availability_count.max(1);
            upper_confidence_bound(child.cumulative_reward, child.games, total_game_count, c)
        };

        if value > best_value {
            best_value = value;
            best_idx = Some(idx);
        }
    }

    best_idx
}

#[inline]
fn is_terminal(state: &GameState, max_round: Option<u32>) -> bool {
    matches!(state.phase, GamePhase::GameOver)
        || max_round.is_some_and(|mr| state.round > mr)
}

#[inline]
fn compute_terminal_scores(state: &GameState) -> SmallVec<[f64; 4]> {
    let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| p.cached_score as f64).collect();
    let max_score = scores.iter().cloned().fold(0.0f64, f64::max);
    let num_winners = scores.iter().filter(|&&s| s == max_score).count() as f64;
    scores.iter().map(|&s| if s == max_score { 1.0 / num_winners } else { 0.0 }).collect()
}

fn rollout<R: Rng>(state: &mut GameState, max_round: Option<u32>, max_rollout_steps: u32, rng: &mut R) -> SmallVec<[f64; 4]> {
    for _ in 0..max_rollout_steps {
        if is_terminal(state, max_round) {
            return compute_terminal_scores(state);
        }
        apply_rollout_step(state, rng);
    }

    if is_terminal(state, max_round) {
        return compute_terminal_scores(state);
    }

    SmallVec::new()
}

fn record_outcome(node: &mut MctsNode, scores: &[f64]) {
    let reward = if node.player_id < scores.len() {
        scores[node.player_id]
    } else {
        0.0
    };
    node.cumulative_reward += reward;
    node.games += 1;
}
