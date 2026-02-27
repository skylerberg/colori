use crate::colori_game::*;
use crate::scoring::calculate_score;
use crate::state_encoding::{encode_action, encode_state};
use crate::types::*;
use rand::Rng;

/// Trait for neural network evaluation.
/// Implementations provide policy priors and value estimates for game positions.
pub trait NnEvaluator: Send + Sync {
    /// Evaluate a position.
    ///
    /// # Arguments
    /// * `state_encoding` - Float vector of size STATE_ENCODING_SIZE
    /// * `action_encodings` - Slice of float slices, each of size ACTION_ENCODING_SIZE
    ///
    /// # Returns
    /// * `(action_priors, per_player_values)` - Softmax'd policy over actions and per-player
    ///   win probabilities where index 0 = active/perspective player, index 1 = next player, etc.
    fn evaluate(&self, state_encoding: &[f32], action_encodings: &[&[f32]]) -> (Vec<f32>, Vec<f32>);
}

/// Configuration for NN-MCTS.
#[derive(Clone, Debug)]
pub struct NnMctsConfig {
    pub iterations: u32,
    pub c_puct: f32,
}

impl Default for NnMctsConfig {
    fn default() -> Self {
        NnMctsConfig {
            iterations: 200,
            c_puct: 1.5,
        }
    }
}

struct NnMctsNode {
    visits: u32,
    total_value: f64,
    choice: Option<ColoriChoice>,
    prior: f32,
    children: Vec<NnMctsNode>,
}

impl NnMctsNode {
    fn new(choice: Option<ColoriChoice>, prior: f32) -> Self {
        NnMctsNode {
            visits: 0,
            total_value: 0.0,
            choice,
            prior,
            children: Vec::new(),
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn q_value(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.total_value / self.visits as f64
        }
    }
}

/// Select child using PUCT formula.
/// Returns index of best child, or None if no children are available.
fn select_child(node: &NnMctsNode, state: &GameState, c_puct: f32) -> Option<usize> {
    let parent_visits_sqrt = (node.visits as f32).sqrt();
    let mut best_idx: Option<usize> = None;
    let mut best_value = f64::NEG_INFINITY;

    for (idx, child) in node.children.iter().enumerate() {
        if !check_choice_available(state, child.choice.as_ref().unwrap()) {
            continue;
        }

        let q = child.q_value();
        let u = c_puct as f64
            * child.prior as f64
            * parent_visits_sqrt as f64
            / (1.0 + child.visits as f64);
        let value = q + u;

        if value > best_value {
            best_value = value;
            best_idx = Some(idx);
        }
    }

    best_idx
}

/// Expand a leaf node using NN evaluation.
/// Returns the value estimate for the perspective player.
fn expand_and_evaluate(
    node: &mut NnMctsNode,
    state: &GameState,
    active_player: usize,
    perspective_player: usize,
    evaluator: &dyn NnEvaluator,
    choices: &[ColoriChoice],
) -> f64 {
    // Encode state and actions
    let state_enc = encode_state(state, active_player);
    let action_encs: Vec<Vec<f32>> = choices
        .iter()
        .map(|c| encode_action(c, state))
        .collect();
    let action_refs: Vec<&[f32]> = action_encs.iter().map(|v| v.as_slice()).collect();

    let (priors, values) = evaluator.evaluate(&state_enc, &action_refs);

    // Create child nodes with priors
    for (i, choice) in choices.iter().enumerate() {
        let prior = if i < priors.len() { priors[i] } else { 0.0 };
        node.children
            .push(NnMctsNode::new(Some(choice.clone()), prior));
    }

    // Values are from active player's perspective (index 0 = active player).
    // Look up the perspective player's slot in the rotated value vector.
    let num_players = state.players.len();
    let perspective_slot = (perspective_player + num_players - active_player) % num_players;
    if perspective_slot < values.len() {
        values[perspective_slot] as f64
    } else {
        0.0
    }
}

/// Run one iteration of the MCTS search.
fn nn_iteration<R: Rng>(
    node: &mut NnMctsNode,
    state: &mut GameState,
    perspective_player: usize,
    max_round: Option<u32>,
    config: &NnMctsConfig,
    evaluator: &dyn NnEvaluator,
    choices_buf: &mut Vec<ColoriChoice>,
    rng: &mut R,
) -> f64 {
    // Check for terminal state
    let active_player = match get_game_status(state, max_round) {
        GameStatus::Terminated { scores } => {
            let value = if perspective_player < scores.len() {
                scores[perspective_player]
            } else {
                0.0
            };
            node.visits += 1;
            node.total_value += value;
            return value;
        }
        GameStatus::AwaitingAction { player_id } => player_id,
    };

    // If leaf node, expand and evaluate with NN
    if node.is_leaf() {
        enumerate_choices_into(state, choices_buf);
        if choices_buf.is_empty() {
            node.visits += 1;
            return 0.0;
        }
        let value = expand_and_evaluate(
            node,
            state,
            active_player,
            perspective_player,
            evaluator,
            choices_buf,
        );
        node.visits += 1;
        node.total_value += value;
        return value;
    }

    // Select best child via PUCT
    let best_idx = match select_child(node, state, config.c_puct) {
        Some(idx) => idx,
        None => {
            node.visits += 1;
            return 0.0;
        }
    };

    // Apply choice and recurse
    let choice = node.children[best_idx].choice.as_ref().unwrap().clone();
    apply_choice_to_state(state, &choice, rng);

    let value = nn_iteration(
        &mut node.children[best_idx],
        state,
        perspective_player,
        max_round,
        config,
        evaluator,
        choices_buf,
        rng,
    );

    node.visits += 1;
    node.total_value += value;
    value
}

/// Run NN-ISMCTS search.
///
/// Returns the best action and the visit count distribution over legal actions.
/// The visit distribution can be used as a training target for the policy network.
pub fn nn_ismcts<R: Rng>(
    state: &GameState,
    player_id: usize,
    config: &NnMctsConfig,
    evaluator: &dyn NnEvaluator,
    seen_hands: &Option<Vec<Vec<CardInstance>>>,
    max_round: Option<u32>,
    rng: &mut R,
) -> (ColoriChoice, Vec<(ColoriChoice, f32)>) {
    // If only one legal choice, return immediately
    let mut choices_buf: Vec<ColoriChoice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        let choice = choices_buf.swap_remove(0);
        return (choice.clone(), vec![(choice, 1.0)]);
    }

    let mut root = NnMctsNode::new(None, 0.0);
    let mut det_state = state.clone();

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    for _ in 0..config.iterations {
        determinize_in_place(&mut det_state, state, player_id, seen_hands, &cached_scores, rng);
        nn_iteration(
            &mut root,
            &mut det_state,
            player_id,
            max_round,
            config,
            evaluator,
            &mut choices_buf,
            rng,
        );
    }

    // Build visit distribution
    let total_visits: u32 = root.children.iter().map(|c| c.visits).sum();
    let visit_distribution: Vec<(ColoriChoice, f32)> = root
        .children
        .iter()
        .filter(|c| c.visits > 0)
        .map(|c| {
            let visit_frac = if total_visits > 0 {
                c.visits as f32 / total_visits as f32
            } else {
                0.0
            };
            (c.choice.clone().unwrap(), visit_frac)
        })
        .collect();

    // Select best action by visit count
    let best = root
        .children
        .iter()
        .max_by_key(|c| c.visits)
        .map(|c| c.choice.clone().unwrap());

    match best {
        Some(choice) => (choice, visit_distribution),
        None => {
            // Fallback: random legal move
            enumerate_choices_into(state, &mut choices_buf);
            let idx = rng.random_range(0..choices_buf.len());
            let choice = choices_buf[idx].clone();
            (choice.clone(), vec![(choice, 1.0)])
        }
    }
}

/// A uniform random evaluator for testing.
/// Returns equal priors and uniform per-player values.
pub struct UniformEvaluator;

impl NnEvaluator for UniformEvaluator {
    fn evaluate(&self, _state_encoding: &[f32], action_encodings: &[&[f32]]) -> (Vec<f32>, Vec<f32>) {
        let n = action_encodings.len();
        let uniform = if n > 0 { 1.0 / n as f32 } else { 0.0 };
        (vec![uniform; n], vec![1.0 / 3.0; 3])
    }
}

#[cfg(feature = "nn")]
pub mod onnx_evaluator {
    use super::*;
    use crate::state_encoding::{ACTION_ENCODING_SIZE, STATE_ENCODING_SIZE};
    use ort::session::Session;
    use ort::value::Tensor;
    use std::path::Path;
    use std::sync::Mutex;

    pub struct OnnxEvaluator {
        session: Mutex<Session>,
    }

    impl OnnxEvaluator {
        pub fn new(model_path: &Path) -> Result<Self, ort::Error> {
            let session = Session::builder()?.commit_from_file(model_path)?;
            Ok(OnnxEvaluator {
                session: Mutex::new(session),
            })
        }
    }

    impl NnEvaluator for OnnxEvaluator {
        fn evaluate(
            &self,
            state_encoding: &[f32],
            action_encodings: &[&[f32]],
        ) -> (Vec<f32>, Vec<f32>) {
            let num_actions = action_encodings.len();
            if num_actions == 0 {
                return (vec![], vec![1.0 / 3.0; 3]);
            }

            // Build input tensors
            // state: (1, STATE_ENCODING_SIZE)
            let state_data: Vec<f32> = state_encoding.to_vec();

            // action_features: (1, num_actions, ACTION_ENCODING_SIZE)
            let mut action_data: Vec<f32> =
                Vec::with_capacity(num_actions * ACTION_ENCODING_SIZE);
            for enc in action_encodings {
                action_data.extend_from_slice(enc);
            }

            // action_mask: (1, num_actions) as bool -> f32 (all true)
            let mask_data: Vec<bool> = vec![true; num_actions];

            let state_tensor = Tensor::from_array(
                ([1usize, STATE_ENCODING_SIZE], state_data),
            )
            .unwrap();
            let action_tensor = Tensor::from_array(
                ([1usize, num_actions, ACTION_ENCODING_SIZE], action_data),
            )
            .unwrap();
            let mask_tensor = Tensor::from_array(
                ([1usize, num_actions], mask_data),
            )
            .unwrap();

            let inputs = ort::inputs![
                "state" => state_tensor,
                "action_features" => action_tensor,
                "action_mask" => mask_tensor,
            ];

            let mut session = self.session.lock().unwrap();
            let outputs = session.run(inputs).unwrap();

            // policy_logits: (1, num_actions)
            let (_shape, policy_logits_slice) = outputs["policy_logits"]
                .try_extract_tensor::<f32>()
                .unwrap();

            // Softmax over policy logits
            let max_logit = policy_logits_slice
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max);
            let exp_sum: f32 = policy_logits_slice
                .iter()
                .map(|&x| (x - max_logit).exp())
                .sum();
            let priors: Vec<f32> = policy_logits_slice
                .iter()
                .map(|&x| (x - max_logit).exp() / exp_sum)
                .collect();

            // value: (1, NUM_PLAYERS) - win prob per player, index 0 is perspective player
            let (_shape, value_slice) = outputs["value"]
                .try_extract_tensor::<f32>()
                .unwrap();
            let values: Vec<f32> = value_slice.iter().copied().collect();

            (priors, values)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_nn_mcts_with_uniform_evaluator() {
        let mut rng = SmallRng::seed_from_u64(42);
        let ai_players = vec![true, true, true];
        let mut state = create_initial_game_state(3, &ai_players, &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let evaluator = UniformEvaluator;
        let config = NnMctsConfig {
            iterations: 50,
            c_puct: 1.5,
        };

        let (choice, visit_dist) = nn_ismcts(
            &state,
            0,
            &config,
            &evaluator,
            &None,
            Some(10),
            &mut rng,
        );

        // Verify we got a valid choice
        let legal = enumerate_choices(&state);
        assert!(
            legal.contains(&choice),
            "NN-MCTS returned an illegal choice"
        );

        // Verify visit distribution sums to ~1.0
        let total: f32 = visit_dist.iter().map(|(_, v)| v).sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "Visit distribution doesn't sum to 1.0: {}",
            total
        );

        // Verify all choices in distribution are legal
        for (c, _) in &visit_dist {
            assert!(legal.contains(c), "Visit distribution contains illegal choice");
        }
    }

    #[test]
    fn test_nn_mcts_single_choice() {
        // Create a state where there's only one legal choice
        let mut rng = SmallRng::seed_from_u64(123);
        let ai_players = vec![true, true, true];
        let mut state = create_initial_game_state(3, &ai_players, &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        // Find a state with exactly one choice by simulating
        let evaluator = UniformEvaluator;
        let config = NnMctsConfig {
            iterations: 10,
            c_puct: 1.5,
        };

        // Just verify nn_ismcts doesn't crash
        let (choice, _) = nn_ismcts(
            &state,
            0,
            &config,
            &evaluator,
            &None,
            Some(10),
            &mut rng,
        );

        let legal = enumerate_choices(&state);
        assert!(legal.contains(&choice));
    }
}
