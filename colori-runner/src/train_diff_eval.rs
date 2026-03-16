use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::fixed_vec::FixedVec;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::diff_eval::{
    diff_eval_score, DiffEvalParams, DiffEvalTable, NUM_PARAMS,
};
use colori_core::scoring::diff_eval_grad::{diff_eval_backward, DiffEvalGradients};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;

use rand::SeedableRng;
use wyrand::WyRand;

use crate::cli::SimulationArgs;

// ── Training sample ──

struct TrainingSample {
    // Per-player snapshots (we store the raw player state + context)
    players: Vec<PlayerState>,
    sell_card_display: Vec<SellCardInstance>,
    card_lookup: [Card; 256],
    round: u32,
    // Outcome: index of winning player (or multiple for ties)
    winner_mask: Vec<bool>,
}

// ── Adam optimizer ──

struct Adam {
    lr: f64,
    beta1: f64,
    beta2: f64,
    epsilon: f64,
    m: [f64; NUM_PARAMS],
    v: [f64; NUM_PARAMS],
    t: u64,
}

impl Adam {
    fn new(lr: f64) -> Self {
        Adam {
            lr,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: [0.0; NUM_PARAMS],
            v: [0.0; NUM_PARAMS],
            t: 0,
        }
    }

    fn step(&mut self, params: &mut DiffEvalParams, grads: &DiffEvalGradients) {
        self.t += 1;
        let bc1 = 1.0 - self.beta1.powi(self.t as i32);
        let bc2 = 1.0 - self.beta2.powi(self.t as i32);

        for i in 0..NUM_PARAMS {
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads.grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads.grads[i] * grads.grads[i];

            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;

            params.weights[i] -= self.lr * m_hat / (v_hat.sqrt() + self.epsilon);
        }
    }
}

// ── Data generation ──

fn generate_training_data(
    num_games: usize,
    eval_iterations: u32,
    num_threads: usize,
    heuristic_params: &HeuristicParams,
    diff_eval_params: Option<&DiffEvalParams>,
    epoch: usize,
) -> Vec<TrainingSample> {
    let samples = std::sync::Mutex::new(Vec::new());
    let completed = std::sync::atomic::AtomicUsize::new(0);

    std::thread::scope(|s| {
        let games_per_thread = num_games / num_threads;
        let remainder = num_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let samples = &samples;
            let completed = &completed;
            let hp = heuristic_params.clone();
            let dep = diff_eval_params.cloned();
            let game_offset = t * games_per_thread + t.min(remainder);

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());
                let mut thread_samples = Vec::new();

                for game_i in 0..count {
                    let game_samples = play_game_and_collect(
                        &hp, dep.as_ref(), eval_iterations, game_offset + game_i, &mut rng,
                    );
                    thread_samples.extend(game_samples);

                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if done % 100 == 0 {
                        eprintln!("  Epoch {}: generated {}/{} games", epoch + 1, done, num_games);
                    }
                }

                samples.lock().unwrap().extend(thread_samples);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    let samples = samples.into_inner().unwrap();
    eprintln!("  Epoch {}: {} samples from {} games", epoch + 1, samples.len(), num_games);
    samples
}

/// Play a game and collect training samples.
/// If `diff_eval_params` is Some, one player uses diff-eval and the other uses the baseline
/// heuristic (alternating sides based on `game_index`). If None, both players use the baseline.
fn play_game_and_collect(
    heuristic_params: &HeuristicParams,
    diff_eval_params: Option<&DiffEvalParams>,
    eval_iterations: u32,
    game_index: usize,
    rng: &mut WyRand,
) -> Vec<TrainingSample> {
    let num_players = 2;
    let ai_players = vec![true; num_players];
    let expansions = Expansions { glass: false };
    let mut state = create_initial_game_state_with_expansions(num_players, &ai_players, expansions, rng);

    let baseline_config = MctsConfig {
        iterations: eval_iterations,
        use_heuristic_eval: true,
        heuristic_params: heuristic_params.clone(),
        ..MctsConfig::default()
    };

    let configs: [MctsConfig; 2] = if let Some(dep) = diff_eval_params {
        let diff_config = MctsConfig {
            iterations: eval_iterations,
            use_heuristic_eval: true,
            diff_eval_params: Some(dep.clone()),
            ..MctsConfig::default()
        };
        // Alternate sides based on game index
        if game_index % 2 == 0 {
            [diff_config, baseline_config]
        } else {
            [baseline_config.clone(), diff_config]
        }
    } else {
        [baseline_config.clone(), baseline_config]
    };

    execute_draw_phase(&mut state, rng);

    let mut snapshots: Vec<TrainingSample> = Vec::new();
    let mut last_snapshot_round = u32::MAX;

    while !matches!(state.phase, GamePhase::GameOver) {
        // Snapshot at the start of each draft phase (once per round)
        if matches!(state.phase, GamePhase::Draft { .. }) && state.round != last_snapshot_round {
            last_snapshot_round = state.round;
            // Update cached scores before snapshot
            for p in state.players.iter_mut() {
                p.cached_score = calculate_score(p);
            }
            snapshots.push(TrainingSample {
                players: state.players.iter().cloned().collect(),
                sell_card_display: state.sell_card_display.iter().cloned().collect(),
                card_lookup: state.card_lookup,
                round: state.round,
                winner_mask: vec![false; num_players], // filled in later
            });
        }

        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => { break; }
            GamePhase::GameOver => break,
        };

        let config = &configs[player_index];
        let max_rollout_round = std::cmp::max(8, state.round + 2);
        let choice = ismcts(&state, player_index, config, &None, Some(max_rollout_round), rng);
        apply_choice_to_state(&mut state, &choice, rng);
    }

    // Determine winner
    let mut best_score = 0u32;
    for p in state.players.iter() {
        let s = calculate_score(p);
        if s > best_score { best_score = s; }
    }
    let winners: Vec<bool> = state.players.iter().map(|p| calculate_score(p) == best_score).collect();

    // Backfill outcomes
    for sample in &mut snapshots {
        sample.winner_mask = winners.clone();
    }

    snapshots
}

// ── Training loop ──

fn compute_loss_and_grads(
    batch: &[&TrainingSample],
    params: &DiffEvalParams,
    table: &DiffEvalTable,
) -> (f64, DiffEvalGradients) {
    let mut total_loss = 0.0;
    let mut total_grads = DiffEvalGradients::zeros();

    for sample in batch {
        let n = sample.players.len();
        let mut display = FixedVec::new();
        for sc in &sample.sell_card_display {
            display.push(*sc);
        }

        // Forward: compute logits
        let mut logits = vec![0.0f64; n];
        for (i, p) in sample.players.iter().enumerate() {
            logits[i] = diff_eval_score(p, &display, &sample.card_lookup, sample.round, params, table);
        }

        // Softmax
        let max_logit = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut probs = vec![0.0f64; n];
        let mut exp_sum = 0.0;
        for i in 0..n {
            probs[i] = (logits[i] - max_logit).exp();
            exp_sum += probs[i];
        }
        for i in 0..n {
            probs[i] /= exp_sum;
        }

        // Target: uniform over winners
        let num_winners = sample.winner_mask.iter().filter(|&&w| w).count() as f64;
        let mut targets = vec![0.0f64; n];
        for i in 0..n {
            targets[i] = if sample.winner_mask[i] { 1.0 / num_winners } else { 0.0 };
        }

        // Cross-entropy loss
        for i in 0..n {
            if targets[i] > 0.0 {
                total_loss -= targets[i] * (probs[i].max(1e-10)).ln();
            }
        }

        // Gradient: d_logit_i = probs[i] - targets[i]
        for (i, p) in sample.players.iter().enumerate() {
            let grad_output = probs[i] - targets[i];
            let grads = diff_eval_backward(p, &display, &sample.card_lookup, sample.round, params, table, grad_output);
            total_grads.accumulate(&grads);
        }
    }

    // Average over batch
    let batch_size = batch.len() as f64;
    total_loss /= batch_size;
    total_grads.scale(1.0 / batch_size);

    (total_loss, total_grads)
}

// ── Public entry point ──

pub struct TrainArgs {
    pub games: usize,
    pub epochs: usize,
    pub batch_size: usize,
    pub lr: f64,
    pub eval_iterations: u32,
    pub vs_baseline: bool,
    pub threads: usize,
    pub output: String,
}

pub fn run_training(args: &SimulationArgs, train: &TrainArgs) {
    eprintln!("=== Diff Eval Training ===");
    eprintln!("Games/epoch: {}, Epochs: {}, Batch size: {}, LR: {}",
        train.games, train.epochs, train.batch_size, train.lr);
    eprintln!("MCTS iterations: {}, Threads: {}, Mode: {}",
        train.eval_iterations, train.threads,
        if train.vs_baseline { "vs baseline" } else { "self-play" });

    // Use baseline heuristic params from first variant (or default)
    let baseline_params = if !args.variants.is_empty() {
        args.variants[0].ai.heuristic_params.clone()
    } else {
        HeuristicParams::default()
    };

    // Create output directory
    std::fs::create_dir_all(&train.output).expect("Failed to create output directory");

    // Try to resume from latest checkpoint
    let (mut params, mut optimizer, start_epoch) = load_checkpoint(&train.output, train.lr);
    let table = DiffEvalTable::new();

    if start_epoch > 0 {
        eprintln!("Resuming from epoch {}", start_epoch);
    }

    // Training loop: fresh data each epoch
    for epoch in start_epoch..train.epochs {
        let dep = if train.vs_baseline { Some(&params) } else { None };
        let samples = generate_training_data(
            train.games, train.eval_iterations, train.threads, &baseline_params, dep, epoch,
        );
        if samples.is_empty() {
            eprintln!("No training samples generated for epoch {}!", epoch + 1);
            continue;
        }

        // Shuffle and train on mini-batches
        use rand::seq::SliceRandom;
        let mut rng = WyRand::from_rng(&mut rand::rng());
        let mut indices: Vec<usize> = (0..samples.len()).collect();
        indices.shuffle(&mut rng);

        let mut epoch_loss = 0.0;
        let mut num_batches = 0;

        for chunk in indices.chunks(train.batch_size) {
            let batch: Vec<&TrainingSample> = chunk.iter().map(|&i| &samples[i]).collect();
            let (loss, grads) = compute_loss_and_grads(&batch, &params, &table);
            optimizer.step(&mut params, &grads);
            epoch_loss += loss;
            num_batches += 1;
        }

        let avg_loss = epoch_loss / num_batches as f64;
        eprintln!("Epoch {}/{}: loss={:.4}", epoch + 1, train.epochs, avg_loss);

        // Save checkpoint + params after every epoch
        save_checkpoint(&train.output, &params, &optimizer, epoch + 1);
        let path = format!("{}/diff-eval-epoch-{}.json", train.output, epoch + 1);
        let json = serde_json::to_string_pretty(&params).unwrap();
        std::fs::write(&path, json).unwrap();
    }

    // Save final params
    let path = format!("{}/latest-diff-eval.json", train.output);
    let json = serde_json::to_string_pretty(&params).unwrap();
    std::fs::write(&path, json).unwrap();
    eprintln!("\nTraining complete. Final params saved to {}", path);
}

// ── Checkpoint save/load ──

use serde::{Serialize as SerdeSerialize, Deserialize as SerdeDeserialize};

#[derive(SerdeSerialize, SerdeDeserialize)]
struct Checkpoint {
    params: DiffEvalParams,
    adam_m: Vec<f64>,
    adam_v: Vec<f64>,
    adam_t: u64,
    epoch: usize,
}

fn save_checkpoint(output_dir: &str, params: &DiffEvalParams, optimizer: &Adam, epoch: usize) {
    let checkpoint = Checkpoint {
        params: params.clone(),
        adam_m: optimizer.m.to_vec(),
        adam_v: optimizer.v.to_vec(),
        adam_t: optimizer.t,
        epoch,
    };
    let path = format!("{}/checkpoint.json", output_dir);
    let json = serde_json::to_string(&checkpoint).unwrap();
    std::fs::write(&path, json).unwrap();
}

fn load_checkpoint(output_dir: &str, lr: f64) -> (DiffEvalParams, Adam, usize) {
    let path = format!("{}/checkpoint.json", output_dir);
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            match serde_json::from_str::<Checkpoint>(&json) {
                Ok(cp) => {
                    let mut optimizer = Adam::new(lr);
                    optimizer.m.copy_from_slice(&cp.adam_m);
                    optimizer.v.copy_from_slice(&cp.adam_v);
                    optimizer.t = cp.adam_t;
                    (cp.params, optimizer, cp.epoch)
                }
                Err(e) => {
                    eprintln!("Warning: failed to parse checkpoint: {}", e);
                    (DiffEvalParams::default(), Adam::new(lr), 0)
                }
            }
        }
        Err(_) => (DiffEvalParams::default(), Adam::new(lr), 0),
    }
}
