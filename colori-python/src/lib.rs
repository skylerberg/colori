use colori_core::colori_game::{apply_choice_to_state, enumerate_choices};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::nn_mcts::{nn_ismcts, onnx_evaluator::OnnxEvaluator, NnMctsConfig};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::state_encoding::{encode_action, encode_state, ACTION_ENCODING_SIZE, STATE_ENCODING_SIZE};
use colori_core::types::*;

use numpy::{PyArray1, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::path::Path;
use std::sync::Arc;

struct TrainingSample {
    state: Vec<f32>,
    action_features: Vec<Vec<f32>>,
    policy: Vec<f32>,
    value: f32,
}

fn run_single_self_play_game(
    evaluator: &dyn colori_core::nn_mcts::NnEvaluator,
    config: &NnMctsConfig,
    rng: &mut SmallRng,
) -> Vec<TrainingSample> {
    let num_players = 3;
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, rng);
    execute_draw_phase(&mut state, rng);

    let mut pending_samples: Vec<(usize, Vec<f32>, Vec<Vec<f32>>, Vec<f32>)> = Vec::new();

    let mut move_count = 0u32;

    loop {
        if matches!(state.phase, GamePhase::GameOver) {
            break;
        }

        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => {
                if draft_state.waiting_for_pass {
                    break;
                }
                draft_state.current_player_index
            }
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Cleanup { cleanup_state } => cleanup_state.current_player_index,
            GamePhase::Draw => break,
            GamePhase::GameOver => break,
        };

        let max_round = std::cmp::max(8, state.round + 2);
        let (best_choice, visit_dist) =
            nn_ismcts(&state, player_index, config, evaluator, &None, Some(max_round), rng);

        // Record training sample
        let state_enc = encode_state(&state, player_index);
        let legal_actions = enumerate_choices(&state);
        let action_encs: Vec<Vec<f32>> =
            legal_actions.iter().map(|c| encode_action(c, &state)).collect();

        // Map visit distribution to policy vector aligned with legal_actions
        let mut policy = vec![0.0f32; legal_actions.len()];
        for (choice, visits) in &visit_dist {
            if let Some(idx) = legal_actions.iter().position(|c| c == choice) {
                policy[idx] = *visits;
            }
        }

        pending_samples.push((player_index, state_enc, action_encs, policy));

        // Sample action using temperature
        move_count += 1;
        let temperature = if move_count <= 30 { 1.0f32 } else { 0.1f32 };

        let chosen_action = if temperature >= 1.0 {
            // Sample proportional to visit counts
            let total: f32 = visit_dist.iter().map(|(_, v)| v).sum();
            if total > 0.0 {
                let r: f32 = rng.random::<f32>() * total;
                let mut cumulative = 0.0f32;
                let mut selected = &best_choice;
                for (choice, visits) in &visit_dist {
                    cumulative += visits;
                    if r <= cumulative {
                        selected = choice;
                        break;
                    }
                }
                selected.clone()
            } else {
                best_choice.clone()
            }
        } else {
            // Low temperature: sharpen distribution, sample from sharpened
            let visit_counts: Vec<(ColoriChoice, f32)> = visit_dist
                .iter()
                .map(|(c, v)| (c.clone(), v.powf(1.0 / temperature)))
                .collect();
            let total: f32 = visit_counts.iter().map(|(_, v)| v).sum();
            if total > 0.0 {
                let r: f32 = rng.random::<f32>() * total;
                let mut cumulative = 0.0f32;
                let mut selected = &best_choice;
                for (choice, v) in &visit_counts {
                    cumulative += v;
                    if r <= cumulative {
                        selected = choice;
                        break;
                    }
                }
                selected.clone()
            } else {
                best_choice.clone()
            }
        };

        apply_choice_to_state(&mut state, &chosen_action, rng);
    }

    // Backfill values based on game outcome
    let scores: Vec<u32> = state.players.iter().map(|p| calculate_score(p)).collect();
    let max_score = *scores.iter().max().unwrap_or(&0);
    let num_winners = scores.iter().filter(|&&s| s == max_score).count();

    pending_samples
        .into_iter()
        .map(|(player_id, state_enc, action_features, policy)| {
            let value = if scores[player_id] == max_score {
                1.0 / num_winners as f32
            } else {
                0.0
            };
            TrainingSample {
                state: state_enc,
                action_features,
                policy,
                value,
            }
        })
        .collect()
}

fn samples_to_numpy(py: Python<'_>, samples: &[TrainingSample]) -> PyResult<PyObject> {
    let n = samples.len();
    let max_actions = samples
        .iter()
        .map(|s| s.action_features.len())
        .max()
        .unwrap_or(1);

    let mut states = Vec::with_capacity(n * STATE_ENCODING_SIZE);
    let mut action_features = vec![0.0f32; n * max_actions * ACTION_ENCODING_SIZE];
    let mut action_masks = vec![false; n * max_actions];
    let mut policies = vec![0.0f32; n * max_actions];
    let mut values = Vec::with_capacity(n);

    for (i, sample) in samples.iter().enumerate() {
        states.extend_from_slice(&sample.state);
        values.push(sample.value);

        for (j, af) in sample.action_features.iter().enumerate() {
            let offset = (i * max_actions + j) * ACTION_ENCODING_SIZE;
            action_features[offset..offset + ACTION_ENCODING_SIZE].copy_from_slice(af);
            action_masks[i * max_actions + j] = true;
            if j < sample.policy.len() {
                policies[i * max_actions + j] = sample.policy[j];
            }
        }
    }

    let dict = PyDict::new(py);

    let states_arr = PyArray1::from_vec(py, states)
        .reshape([n, STATE_ENCODING_SIZE])
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("reshape error: {}", e)))?;
    let af_arr = PyArray1::from_vec(py, action_features)
        .reshape([n, max_actions, ACTION_ENCODING_SIZE])
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("reshape error: {}", e)))?;
    let mask_arr = PyArray1::from_vec(py, action_masks)
        .reshape([n, max_actions])
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("reshape error: {}", e)))?;
    let policy_arr = PyArray1::from_vec(py, policies)
        .reshape([n, max_actions])
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("reshape error: {}", e)))?;
    let values_arr = PyArray1::from_vec(py, values);

    dict.set_item("states", states_arr)?;
    dict.set_item("action_features", af_arr)?;
    dict.set_item("action_masks", mask_arr)?;
    dict.set_item("policies", policy_arr)?;
    dict.set_item("values", values_arr)?;

    Ok(dict.into())
}

#[pyfunction]
#[pyo3(signature = (num_games, model_path, mcts_iterations=200, c_puct=1.5, num_threads=8))]
fn run_self_play_games(
    py: Python<'_>,
    num_games: usize,
    model_path: String,
    mcts_iterations: u32,
    c_puct: f32,
    num_threads: usize,
) -> PyResult<PyObject> {
    let evaluator = Arc::new(
        OnnxEvaluator::new(Path::new(&model_path)).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to load model: {}", e))
        })?,
    );

    let config = NnMctsConfig {
        iterations: mcts_iterations,
        c_puct,
    };

    let all_samples: Vec<Vec<TrainingSample>> = std::thread::scope(|s| {
        let games_per_thread = num_games / num_threads;
        let remainder = num_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let eval = Arc::clone(&evaluator);
            let cfg = config.clone();

            handles.push(s.spawn(move || {
                let mut rng = SmallRng::from_os_rng();
                let mut thread_samples = Vec::new();

                for _ in 0..count {
                    let samples = run_single_self_play_game(eval.as_ref(), &cfg, &mut rng);
                    thread_samples.push(samples);
                }
                thread_samples
            }));
        }

        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().unwrap());
        }
        all
    });

    let samples: Vec<TrainingSample> = all_samples.into_iter().flatten().collect();

    samples_to_numpy(py, &samples)
}

#[pyfunction]
fn encode_game_state(py: Python<'_>, state_json: &str, perspective_player: usize) -> PyResult<PyObject> {
    let state: GameState = serde_json::from_str(state_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e)))?;
    let encoding = encode_state(&state, perspective_player);
    Ok(PyArray1::from_vec(py, encoding).into())
}

#[pyfunction]
fn get_legal_actions(state_json: &str) -> PyResult<String> {
    let state: GameState = serde_json::from_str(state_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e)))?;
    let choices = enumerate_choices(&state);
    let json = serde_json::to_string(&choices)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Serialization error: {}", e)))?;
    Ok(json)
}

#[pymodule]
fn colori_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_self_play_games, m)?)?;
    m.add_function(wrap_pyfunction!(encode_game_state, m)?)?;
    m.add_function(wrap_pyfunction!(get_legal_actions, m)?)?;
    Ok(())
}
