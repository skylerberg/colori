use crate::colori_game::*;
use crate::types::*;
use rand::Rng;
use std::collections::HashMap;

struct MctsNode {
    games: u32,
    cumulative_reward: f64,
    player_id: usize,
    choice: Option<ColoriChoice>,
    children: HashMap<String, MctsNode>,
    choice_availability_count: HashMap<String, u32>,
}

impl MctsNode {
    fn new(player_id: usize, choice: Option<ColoriChoice>) -> Self {
        MctsNode {
            games: 0,
            cumulative_reward: 0.0,
            player_id,
            choice,
            children: HashMap::new(),
            choice_availability_count: HashMap::new(),
        }
    }

    fn is_root(&self) -> bool {
        self.choice.is_none()
    }

    fn expand<R: Rng>(&mut self, state: &GameState, mut choices: Vec<ColoriChoice>, rng: &mut R) {
        // Shuffle choices
        let len = choices.len();
        for i in (1..len).rev() {
            let j = rng.gen_range(0..=i);
            choices.swap(i, j);
        }

        let active_player = match get_game_status(state, None) {
            GameStatus::AwaitingAction { player_id } => player_id,
            _ => return,
        };

        let mut added_new_node = false;

        for choice in &choices {
            let key = choice_to_key(choice);

            if let Some(count) = self.choice_availability_count.get_mut(&key) {
                *count += 1;
            } else {
                self.choice_availability_count.insert(key.clone(), 0);
            }

            if self.is_root() || (!added_new_node && !self.children.contains_key(&key)) {
                self.children
                    .insert(key, MctsNode::new(active_player, Some(choice.clone())));
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

const C: f64 = std::f64::consts::SQRT_2;
const MAX_ROLLOUT_STEPS: u32 = 1000;

pub fn ismcts<R: Rng>(
    state: &GameState,
    player_id: usize,
    iterations: u32,
    seen_hands: &Option<Vec<Vec<CardInstance>>>,
    max_round: Option<u32>,
    rng: &mut R,
) -> ColoriChoice {
    let mut root = MctsNode::new(player_id, None);

    for _ in 0..iterations {
        let mut det_state = determinize(state, player_id, seen_hands, rng);
        iteration(&mut root, &mut det_state, max_round, rng);
    }

    if root.children.is_empty() {
        let choices = enumerate_choices(state);
        let idx = rng.gen_range(0..choices.len());
        return choices[idx].clone();
    }

    let mut best_child: Option<&MctsNode> = None;
    for child in root.children.values() {
        if best_child.is_none() || child.games > best_child.unwrap().games {
            best_child = Some(child);
        }
    }

    best_child.unwrap().choice.clone().unwrap()
}

fn iteration<R: Rng>(
    node: &mut MctsNode,
    state: &mut GameState,
    max_round: Option<u32>,
    rng: &mut R,
) -> Vec<f64> {
    let status = get_game_status(state, max_round);
    if let GameStatus::Terminated { scores } = status {
        record_outcome(node, &scores);
        return scores;
    }

    // Expand
    if !(node.is_root() && !node.children.is_empty()) {
        let choices = enumerate_choices(state);
        node.expand(state, choices, rng);
    }

    // Select
    let best_key = select(node, state);
    if best_key.is_none() {
        let empty_scores: Vec<f64> = vec![];
        record_outcome(node, &empty_scores);
        return empty_scores;
    }
    let best_key = best_key.unwrap();

    // Apply choice
    let choice = node.children[&best_key].choice.clone().unwrap();
    apply_choice_to_state(state, &choice, rng);

    let should_rollout = node.children[&best_key].games == 0;

    let scores = if should_rollout {
        let scores = rollout(state, max_round, rng);
        record_outcome(node.children.get_mut(&best_key).unwrap(), &scores);
        scores
    } else {
        let child = node.children.get_mut(&best_key).unwrap();
        iteration(child, state, max_round, rng)
    };

    record_outcome(node, &scores);
    scores
}

fn select(node: &MctsNode, state: &GameState) -> Option<String> {
    let mut best_key: Option<String> = None;
    let mut best_value = f64::NEG_INFINITY;

    for (key, child) in &node.children {
        if !check_choice_available(state, child.choice.as_ref().unwrap()) {
            continue;
        }

        let value = if child.games == 0 {
            f64::INFINITY
        } else {
            let total_game_count = if node.is_root() {
                node.games
            } else {
                *node.choice_availability_count.get(key).unwrap_or(&node.games)
            };
            upper_confidence_bound(child.cumulative_reward, child.games, total_game_count, C)
        };

        if value > best_value {
            best_value = value;
            best_key = Some(key.clone());
        }
    }

    best_key
}

fn rollout<R: Rng>(state: &mut GameState, max_round: Option<u32>, rng: &mut R) -> Vec<f64> {
    for _ in 0..MAX_ROLLOUT_STEPS {
        let status = get_game_status(state, max_round);
        if let GameStatus::Terminated { scores } = status {
            return scores;
        }
        let choice = get_rollout_choice(state, rng);
        apply_choice_to_state(state, &choice, rng);
    }

    let status = get_game_status(state, max_round);
    if let GameStatus::Terminated { scores } = status {
        return scores;
    }

    vec![]
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
