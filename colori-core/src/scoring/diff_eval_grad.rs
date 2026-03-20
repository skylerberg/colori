use crate::types::*;

use super::diff_eval::*;

/// Accumulated gradients, same shape as DiffEvalParams.
/// Heap-allocated to avoid stack overflows with large parameter arrays.
#[derive(Debug, Clone)]
pub struct DiffEvalGradients {
    pub grads: Box<[f64; NUM_PARAMS]>,
}

impl DiffEvalGradients {
    pub fn zeros() -> Self {
        let grads: Box<[f64; NUM_PARAMS]> = vec![0.0f64; NUM_PARAMS].into_boxed_slice().try_into()
            .unwrap_or_else(|_| unreachable!());
        DiffEvalGradients { grads }
    }

    pub fn accumulate(&mut self, other: &DiffEvalGradients) {
        for i in 0..NUM_PARAMS {
            self.grads[i] += other.grads[i];
        }
    }

    pub fn scale(&mut self, factor: f64) {
        for g in self.grads.iter_mut() {
            *g *= factor;
        }
    }
}

const LEAKY_ALPHA: f64 = 0.01;

/// Compute forward pass and cache intermediates, then compute backward pass.
/// Returns gradients w.r.t. all MLP parameters for a single evaluation.
/// `grad_output` is the gradient flowing back from the loss (typically from softmax+CE).
///
/// Takes the full game state and perspective player, matching the new diff_eval_score API.
pub fn diff_eval_backward(
    state: &GameState,
    perspective_player: usize,
    params: &DiffEvalParams,
    _table: &DiffEvalTable,
    grad_output: f64,
) -> DiffEvalGradients {
    let w = &params.weights;
    let mut grads = DiffEvalGradients::zeros();
    let g = &mut grads.grads;

    // ── Step 1: Compute features (f32) and convert to f64 for gradient computation ──
    let features_f32 = super::diff_eval::compute_features(state, perspective_player);
    let mut mlp_inputs = [0.0f64; MLP_INPUT_SIZE];
    for i in 0..MLP_INPUT_SIZE {
        mlp_inputs[i] = features_f32[i] as f64;
    }

    // ── Step 2: MLP forward to cache pre-activations and hidden outputs ──

    // Hidden layer 1: MLP_INPUT_SIZE -> MLP_HIDDEN_SIZE (LeakyReLU)
    let mut pre_act1 = [0.0f64; MLP_HIDDEN_SIZE];
    let mut hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = w[MLP_B1 + row];
        sum += super::simd_ops::dot_f64(&w[MLP_W1 + row * MLP_INPUT_SIZE..MLP_W1 + (row + 1) * MLP_INPUT_SIZE], &mlp_inputs);
        pre_act1[row] = sum;
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_ALPHA * sum };
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE -> MLP_HIDDEN2_SIZE (LeakyReLU)
    let mut pre_act2 = [0.0f64; MLP_HIDDEN2_SIZE];
    let mut hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for row in 0..MLP_HIDDEN2_SIZE {
        let mut sum = w[MLP_B2 + row];
        sum += super::simd_ops::dot_f64(&w[MLP_W2 + row * MLP_HIDDEN_SIZE..MLP_W2 + (row + 1) * MLP_HIDDEN_SIZE], &hidden1);
        pre_act2[row] = sum;
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_ALPHA * sum };
    }

    // ── Step 3: MLP backward ──

    // Output layer: output = W3 * hidden2 + B3
    g[MLP_B3] += grad_output;
    for i in 0..MLP_HIDDEN2_SIZE {
        g[MLP_W3 + i] += grad_output * hidden2[i];
    }

    // Backward through hidden layer 2
    let mut d_hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for i in 0..MLP_HIDDEN2_SIZE {
        d_hidden2[i] = grad_output * w[MLP_W3 + i];
    }

    // Backward through LeakyReLU 2
    let mut d_pre_act2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for i in 0..MLP_HIDDEN2_SIZE {
        d_pre_act2[i] = if pre_act2[i] > 0.0 { d_hidden2[i] } else { LEAKY_ALPHA * d_hidden2[i] };
    }

    // Backward through W2 * hidden1 + B2
    for row in 0..MLP_HIDDEN2_SIZE {
        g[MLP_B2 + row] += d_pre_act2[row];
        for col in 0..MLP_HIDDEN_SIZE {
            g[MLP_W2 + row * MLP_HIDDEN_SIZE + col] += d_pre_act2[row] * hidden1[col];
        }
    }

    // Gradient w.r.t. hidden1
    let mut d_hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for col in 0..MLP_HIDDEN_SIZE {
        for row in 0..MLP_HIDDEN2_SIZE {
            d_hidden1[col] += d_pre_act2[row] * w[MLP_W2 + row * MLP_HIDDEN_SIZE + col];
        }
    }

    // Backward through LeakyReLU 1
    let mut d_pre_act1 = [0.0f64; MLP_HIDDEN_SIZE];
    for i in 0..MLP_HIDDEN_SIZE {
        d_pre_act1[i] = if pre_act1[i] > 0.0 { d_hidden1[i] } else { LEAKY_ALPHA * d_hidden1[i] };
    }

    // Backward through W1 * x + B1
    for row in 0..MLP_HIDDEN_SIZE {
        g[MLP_B1 + row] += d_pre_act1[row];
        for col in 0..MLP_INPUT_SIZE {
            g[MLP_W1 + row * MLP_INPUT_SIZE + col] += d_pre_act1[row] * mlp_inputs[col];
        }
    }

    grads
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::diff_eval::{DiffEvalParams, DiffEvalTable, diff_eval_score};
    use crate::setup::create_initial_game_state;
    use crate::draw_phase::execute_draw_phase;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn make_test_state() -> GameState {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        state
    }

    /// Verify hand-rolled gradients match finite-difference approximation.
    #[test]
    fn test_gradient_vs_finite_difference() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let state = make_test_state();
        let perspective_player = 0;

        // Compute analytical gradients
        let grads = diff_eval_backward(&state, perspective_player, &params, &table, 1.0);

        // Compare against finite differences.
        // Sample MLP params across the range.
        let eps = 1e-5;
        let mut max_rel_error = 0.0f64;
        let mut worst_param = 0;

        // Sample every 500th param (there are 173,697 diff params)
        let mut params_to_check: Vec<usize> = Vec::new();
        let mut i = 0;
        while i < NUM_DIFF_PARAMS {
            params_to_check.push(i);
            i += 500;
        }
        // Always include the last few params
        if NUM_DIFF_PARAMS > 3 {
            for j in (NUM_DIFF_PARAMS - 3)..NUM_DIFF_PARAMS {
                if !params_to_check.contains(&j) {
                    params_to_check.push(j);
                }
            }
        }

        for &i in &params_to_check {
            let mut params_plus = params.clone();
            params_plus.weights[i] += eps;
            let score_plus = diff_eval_score(&state, perspective_player, &params_plus, &table);

            let mut params_minus = params.clone();
            params_minus.weights[i] -= eps;
            let score_minus = diff_eval_score(&state, perspective_player, &params_minus, &table);

            let fd_grad = (score_plus - score_minus) / (2.0 * eps);
            let analytical_grad = grads.grads[i];

            let abs_diff = (fd_grad - analytical_grad).abs();
            let denom = fd_grad.abs().max(analytical_grad.abs()).max(1e-8);
            let rel_error = abs_diff / denom;

            if rel_error > max_rel_error {
                max_rel_error = rel_error;
                worst_param = i;
            }

            // Allow small absolute errors for near-zero gradients
            if abs_diff > 1e-4 {
                assert!(
                    rel_error < 0.02,
                    "Gradient mismatch at param {}: analytical={:.8}, fd={:.8}, rel_error={:.6}",
                    i, analytical_grad, fd_grad, rel_error
                );
            }
        }

        eprintln!("Checked {} params. Max relative gradient error: {:.6} at param {}", params_to_check.len(), max_rel_error, worst_param);
        assert!(max_rel_error < 0.02, "Maximum relative error {:.6} exceeds 2%", max_rel_error);
    }
}
