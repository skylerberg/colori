use crate::colori_game::*;
use crate::types::*;
use rand::Rng;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

struct MctsNode {
    games: u32,
    cumulative_reward: f64,
    player_id: usize,
    choice: Option<ColoriChoice>,
    children: FxHashMap<ColoriChoice, MctsNode>,
    choice_availability_count: FxHashMap<ColoriChoice, u32>,
}

impl MctsNode {
    fn new(player_id: usize, choice: Option<ColoriChoice>) -> Self {
        MctsNode {
            games: 0,
            cumulative_reward: 0.0,
            player_id,
            choice,
            children: FxHashMap::default(),
            choice_availability_count: FxHashMap::default(),
        }
    }

    fn is_root(&self) -> bool {
        self.choice.is_none()
    }

    fn expand<R: Rng>(&mut self, state: &GameState, choices: &mut Vec<ColoriChoice>, rng: &mut R) {
        // Shuffle choices in place
        let len = choices.len();
        for i in (1..len).rev() {
            let j = rng.random_range(0..=i);
            choices.swap(i, j);
        }

        let active_player = match get_game_status(state, None) {
            GameStatus::AwaitingAction { player_id } => player_id,
            _ => return,
        };

        let mut added_new_node = false;

        for choice in choices.iter() {
            if let Some(count) = self.choice_availability_count.get_mut(choice) {
                *count += 1;
            } else {
                self.choice_availability_count.insert(choice.clone(), 0);
            }

            if self.is_root() || (!added_new_node && !self.children.contains_key(choice)) {
                self.children
                    .insert(choice.clone(), MctsNode::new(active_player, Some(choice.clone())));
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
    // If there's only one legal choice, return it immediately without searching
    let mut choices_buf: Vec<ColoriChoice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let mut root = MctsNode::new(player_id, None);
    let mut det_state = state.clone();

    for _ in 0..iterations {
        determinize_in_place(&mut det_state, state, player_id, seen_hands, rng);
        iteration(&mut root, &mut det_state, max_round, &mut choices_buf, rng);
    }

    if root.children.is_empty() {
        enumerate_choices_into(state, &mut choices_buf);
        let idx = rng.random_range(0..choices_buf.len());
        return choices_buf[idx].clone();
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
    choices_buf: &mut Vec<ColoriChoice>,
    rng: &mut R,
) -> SmallVec<[f64; 4]> {
    let status = get_game_status(state, max_round);
    if let GameStatus::Terminated { scores } = status {
        record_outcome(node, &scores);
        return scores;
    }

    // Expand
    if !(node.is_root() && !node.children.is_empty()) {
        enumerate_choices_into(state, choices_buf);
        node.expand(state, choices_buf, rng);
    }

    // Select
    let best_key = select(node, state);
    if best_key.is_none() {
        let empty_scores = SmallVec::new();
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
        iteration(child, state, max_round, choices_buf, rng)
    };

    record_outcome(node, &scores);
    scores
}

fn select(node: &MctsNode, state: &GameState) -> Option<ColoriChoice> {
    let mut best_key: Option<ColoriChoice> = None;
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

fn rollout<R: Rng>(state: &mut GameState, max_round: Option<u32>, rng: &mut R) -> SmallVec<[f64; 4]> {
    for _ in 0..MAX_ROLLOUT_STEPS {
        let status = get_game_status(state, max_round);
        if let GameStatus::Terminated { scores } = status {
            return scores;
        }
        apply_rollout_step(state, rng);
    }

    let status = get_game_status(state, max_round);
    if let GameStatus::Terminated { scores } = status {
        return scores;
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
