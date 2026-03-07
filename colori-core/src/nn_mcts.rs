use crate::atomic::{
    apply_atomic_choice, atomic_choice_to_index, enumerate_atomic_choices,
    enumerate_atomic_legal_mask, index_to_atomic_choice, AtomicChoice, NUM_ATOMIC_ACTIONS,
};
use crate::colori_game::determinize_in_place;
use crate::encoding::{encode_legal_mask, encode_observation, OBS_SIZE};
use crate::scoring::calculate_score;
use crate::types::*;
use ort::session::Session;
use rand::Rng;
use std::path::Path;

#[derive(Clone)]
pub struct NnMctsConfig {
    pub simulations: u32,
    pub c_puct: f32,
    pub model_path: String,
    pub determinize_draft_deck: bool,
}

impl Default for NnMctsConfig {
    fn default() -> Self {
        NnMctsConfig {
            simulations: 200,
            c_puct: 1.5,
            model_path: String::new(),
            determinize_draft_deck: false,
        }
    }
}

pub struct NnModel {
    session: Session,
}

impl NnModel {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let session = Session::builder()?.commit_from_file(path)?;
        Ok(NnModel { session })
    }

    /// Run inference, returns (policy probabilities, value).
    pub fn predict(
        &mut self,
        obs: &[f32; OBS_SIZE],
        legal_mask: &[f32; NUM_ATOMIC_ACTIONS],
    ) -> ([f32; NUM_ATOMIC_ACTIONS], f32) {
        use ort::value::Tensor;

        let obs_tensor =
            Tensor::from_array(([1usize, OBS_SIZE], obs.to_vec())).unwrap();
        let mask_tensor =
            Tensor::from_array(([1usize, NUM_ATOMIC_ACTIONS], legal_mask.to_vec())).unwrap();

        let outputs = self
            .session
            .run(ort::inputs![obs_tensor, mask_tensor])
            .unwrap();

        // Output 0: log_policy [1, 300], Output 1: value [1, 1]
        let (_, log_policy_data): (_, &[f32]) = outputs[0].try_extract_tensor().unwrap();
        let (_, value_data): (_, &[f32]) = outputs[1].try_extract_tensor().unwrap();

        let mut policy = [0.0f32; NUM_ATOMIC_ACTIONS];
        let log_policy_slice = log_policy_data;

        // Convert log probabilities to probabilities
        let max_log = log_policy_slice
            .iter()
            .zip(legal_mask.iter())
            .filter(|(_, &m)| m > 0.0)
            .map(|(&l, _)| l)
            .fold(f32::NEG_INFINITY, f32::max);

        let mut sum = 0.0f32;
        for i in 0..NUM_ATOMIC_ACTIONS {
            if legal_mask[i] > 0.0 {
                policy[i] = (log_policy_slice[i] - max_log).exp();
                sum += policy[i];
            }
        }
        if sum > 0.0 {
            for p in policy.iter_mut() {
                *p /= sum;
            }
        }

        let v = value_data[0];
        (policy, v)
    }
}

struct PuctNode {
    action: usize,
    prior: f32,
    visit_count: u32,
    value_sum: f32,
    children: Vec<PuctNode>,
    is_expanded: bool,
    player: usize,
}

impl PuctNode {
    fn new(action: usize, prior: f32, player: usize) -> Self {
        PuctNode {
            action,
            prior,
            visit_count: 0,
            value_sum: 0.0,
            children: Vec::new(),
            is_expanded: false,
            player,
        }
    }

    fn q_value(&self) -> f32 {
        if self.visit_count == 0 {
            0.0
        } else {
            self.value_sum / self.visit_count as f32
        }
    }

    fn ucb_score(&self, parent_visits: u32, c_puct: f32) -> f32 {
        self.q_value()
            + c_puct * self.prior * (parent_visits as f32).sqrt() / (1 + self.visit_count) as f32
    }

}

fn best_legal_child_idx(
    node: &PuctNode,
    legal_mask: &[bool; NUM_ATOMIC_ACTIONS],
    c_puct: f32,
) -> Option<usize> {
    let mut best_score = f32::NEG_INFINITY;
    let mut best_idx = None;
    for (i, child) in node.children.iter().enumerate() {
        if !legal_mask[child.action] {
            continue;
        }
        let score = child.ucb_score(node.visit_count, c_puct);
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }
    best_idx
}

fn current_player_of(state: &GameState) -> usize {
    match &state.phase {
        GamePhase::Draft { draft_state } => draft_state.current_player_index,
        GamePhase::Action { action_state } => action_state.current_player_index,
        _ => 0,
    }
}

/// Run neural network MCTS to select an action.
/// Returns the chosen AtomicChoice.
pub fn nn_mcts<R: Rng>(
    state: &GameState,
    player_index: usize,
    model: &mut NnModel,
    config: &NnMctsConfig,
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    rng: &mut R,
) -> AtomicChoice {
    let mut root = PuctNode::new(0, 1.0, player_index);

    // Get initial policy from network
    let mut obs = [0.0f32; OBS_SIZE];
    let mut legal_mask = [0.0f32; NUM_ATOMIC_ACTIONS];
    encode_observation(state, player_index, &mut obs);
    encode_legal_mask(state, &mut legal_mask);

    let (policy, _) = model.predict(&obs, &legal_mask);

    // Expand root
    let legal_actions = enumerate_atomic_choices(state);
    expand_node(&mut root, &legal_actions, &policy, player_index);

    // Cache scores for determinization
    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    let mut det_state = state.clone();

    for _ in 0..config.simulations {
        // Determinize
        determinize_in_place(
            &mut det_state,
            state,
            player_index,
            known_draft_hands,
            &cached_scores,
            config.determinize_draft_deck,
            rng,
        );

        let mut node_path: Vec<usize> = Vec::new();
        let mut current = &mut root;

        // Select (filter children by legality in this determinization)
        while current.is_expanded && !current.children.is_empty() {
            let legal_mask = enumerate_atomic_legal_mask(&det_state);
            let idx = match best_legal_child_idx(current, &legal_mask, config.c_puct) {
                Some(idx) => idx,
                None => break, // No children legal in this determinization; treat as leaf
            };
            node_path.push(idx);
            let action_idx = current.children[idx].action;
            let choice = index_to_atomic_choice(action_idx);
            apply_atomic_choice(&mut det_state, &choice, rng);
            current = &mut current.children[idx];
        }

        // Evaluate
        let value = if matches!(det_state.phase, GamePhase::GameOver) {
            let rewards = crate::scoring::compute_terminal_rewards(&det_state.players);
            (rewards[player_index] * 2.0 - 1.0) as f32
        } else {
            let current_player = current_player_of(&det_state);

            let mut obs_leaf = [0.0f32; OBS_SIZE];
            let mut mask_leaf = [0.0f32; NUM_ATOMIC_ACTIONS];
            encode_observation(&det_state, current_player, &mut obs_leaf);
            encode_legal_mask(&det_state, &mut mask_leaf);

            let (leaf_policy, leaf_value) = model.predict(&obs_leaf, &mask_leaf);

            // Expand (guard against double expansion from different determinizations)
            if !current.is_expanded {
                let leaf_actions = enumerate_atomic_choices(&det_state);
                expand_node(current, &leaf_actions, &leaf_policy, current_player);
            }

            // Adjust value to searching player's perspective
            if current_player != player_index {
                -leaf_value
            } else {
                leaf_value
            }
        };

        // Backpropagate (negate value at opponent nodes)
        root.visit_count += 1;
        root.value_sum += if root.player == player_index { value } else { -value };
        let mut node = &mut root;
        for &idx in &node_path {
            node = &mut node.children[idx];
            node.visit_count += 1;
            node.value_sum += if node.player == player_index { value } else { -value };
        }

        // Reset determinized state for next simulation
        det_state.clone_from(state);
    }

    // Select action with most visits
    let mut best_visits = 0;
    let mut best_action = 0;
    for child in &root.children {
        if child.visit_count > best_visits {
            best_visits = child.visit_count;
            best_action = child.action;
        }
    }

    index_to_atomic_choice(best_action)
}

fn expand_node(
    node: &mut PuctNode,
    legal_actions: &[AtomicChoice],
    policy: &[f32; NUM_ATOMIC_ACTIONS],
    child_player: usize,
) {
    node.is_expanded = true;
    let total_prior: f32 = legal_actions
        .iter()
        .map(|a| policy[atomic_choice_to_index(a)])
        .sum();

    for action in legal_actions {
        let idx = atomic_choice_to_index(action);
        let prior = if total_prior > 0.0 {
            policy[idx] / total_prior
        } else {
            1.0 / legal_actions.len() as f32
        };
        node.children.push(PuctNode::new(idx, prior, child_player));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puct_node_basics() {
        let mut node = PuctNode::new(0, 0.5, 0);
        assert_eq!(node.q_value(), 0.0);
        assert!(!node.is_expanded);

        node.visit_count = 10;
        node.value_sum = 5.0;
        assert_eq!(node.q_value(), 0.5);
    }
}
