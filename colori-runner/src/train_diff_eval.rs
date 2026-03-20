use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
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
    // Full game state snapshot
    state: GameState,
    // Outcome: index of winning player (or multiple for ties)
    winner_mask: Vec<bool>,
    // Round the game ended on (for urgency weighting)
    final_round: u32,
}

// ── Adam optimizer ──

struct Adam {
    lr: f64,
    beta1: f64,
    beta2: f64,
    epsilon: f64,
    m: Vec<f64>,
    v: Vec<f64>,
    t: u64,
}

impl Adam {
    fn new(lr: f64) -> Self {
        Adam {
            lr,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: vec![0.0; NUM_PARAMS],
            v: vec![0.0; NUM_PARAMS],
            t: 0,
        }
    }

    fn step(&mut self, params: &mut DiffEvalParams, grads: &DiffEvalGradients) {
        use colori_core::scoring::diff_eval::NUM_DIFF_PARAMS;
        self.t += 1;
        let bc1 = 1.0 - self.beta1.powi(self.t as i32);
        let bc2 = 1.0 - self.beta2.powi(self.t as i32);

        // Only update differentiable params; control params (threshold, lookahead, progressive bias) are left unchanged
        for i in 0..NUM_DIFF_PARAMS {
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads.grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads.grads[i] * grads.grads[i];

            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;

            params.weights[i] -= self.lr * m_hat / (v_hat.sqrt() + self.epsilon);
        }
    }
}

// ── Data generation ──

struct GameResult {
    samples: Vec<TrainingSample>,
    /// In vs-baseline mode: 1=diff-eval won, 0=draw, -1=diff-eval lost. None in self-play.
    diff_eval_outcome: Option<i8>,
    /// Final round number when the game ended.
    final_round: u32,
}

struct DataGenStats {
    samples: Vec<TrainingSample>,
}

fn generate_training_data(
    num_games: usize,
    eval_iterations: u32,
    baseline_iterations: u32,
    num_threads: usize,
    heuristic_params: &HeuristicParams,
    diff_eval_params: Option<&DiffEvalParams>,
    no_rollout: bool,
    epoch: usize,
) -> DataGenStats {
    let start = std::time::Instant::now();
    let all_results = std::sync::Mutex::new(Vec::new());
    let completed = std::sync::atomic::AtomicUsize::new(0);

    std::thread::scope(|s| {
        let games_per_thread = num_games / num_threads;
        let remainder = num_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let all_results = &all_results;
            let completed = &completed;
            let hp = heuristic_params.clone();
            let dep = diff_eval_params.cloned();
            let game_offset = t * games_per_thread + t.min(remainder);

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());
                let mut thread_results = Vec::new();

                for game_i in 0..count {
                    let result = play_game_and_collect(
                        &hp, dep.as_ref(), eval_iterations, baseline_iterations, no_rollout, game_offset + game_i, &mut rng,
                    );
                    thread_results.push(result);

                    let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if done % 100 == 0 {
                        eprintln!("  Epoch {}: generated {}/{} games", epoch + 1, done, num_games);
                    }
                }

                all_results.lock().unwrap().extend(thread_results);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    let results = all_results.into_inner().unwrap();
    let elapsed = start.elapsed();

    let mut samples = Vec::new();
    let mut wins = 0usize;
    let mut draws = 0usize;
    let mut losses = 0usize;
    let mut total_rounds = 0u64;
    let vs_baseline = diff_eval_params.is_some();

    for result in results {
        total_rounds += result.final_round as u64;
        samples.extend(result.samples);
        if let Some(outcome) = result.diff_eval_outcome {
            match outcome {
                1 => wins += 1,
                0 => draws += 1,
                _ => losses += 1,
            }
        }
    }

    let avg_rounds = total_rounds as f64 / num_games as f64;
    if vs_baseline {
        eprintln!("  Epoch {}: {} samples from {} games ({:.0}s) | avg {:.1} rounds | vs baseline: {}W/{}D/{}L ({:.1}%)",
            epoch + 1, samples.len(), num_games, elapsed.as_secs_f64(),
            avg_rounds, wins, draws, losses,
            (wins as f64 + 0.5 * draws as f64) / num_games as f64 * 100.0);
    } else {
        eprintln!("  Epoch {}: {} samples from {} games ({:.0}s) | avg {:.1} rounds",
            epoch + 1, samples.len(), num_games, elapsed.as_secs_f64(), avg_rounds);
    }

    DataGenStats { samples }
}

/// Play a game and collect training samples.
/// If `diff_eval_params` is Some, one player uses diff-eval and the other uses the baseline
/// heuristic (alternating sides based on `game_index`). If None, both players use the baseline.
fn play_game_and_collect(
    heuristic_params: &HeuristicParams,
    diff_eval_params: Option<&DiffEvalParams>,
    eval_iterations: u32,
    baseline_iterations: u32,
    no_rollout: bool,
    game_index: usize,
    rng: &mut WyRand,
) -> GameResult {
    let num_players = 2;
    let ai_players = vec![true; num_players];
    let expansions = Expansions { glass: false };
    let mut state = create_initial_game_state_with_expansions(num_players, &ai_players, expansions, rng);

    let baseline_config = MctsConfig {
        iterations: baseline_iterations,
        use_heuristic_eval: true,
        heuristic_params: heuristic_params.clone(),
        ..MctsConfig::default()
    };

    let diff_player: Option<usize> = diff_eval_params.as_ref().map(|_| {
        if game_index % 2 == 0 { 0 } else { 1 }
    });

    let configs: [MctsConfig; 2] = if let Some(dep) = diff_eval_params {
        let diff_config = MctsConfig {
            iterations: eval_iterations,
            use_heuristic_eval: true,
            diff_eval_params: Some(Box::new(dep.clone())),
            no_rollout,
            ..MctsConfig::default()
        };
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

    while !matches!(state.phase, GamePhase::GameOver) {
        // Snapshot before every choice point so the model learns to evaluate
        // any game state — not just draft-phase starts. This is essential for
        // --no-rollout where the eval is called on mid-phase states.
        for p in state.players.iter_mut() {
            p.cached_score = calculate_score(p);
        }
        snapshots.push(TrainingSample {
            state: state.clone(),
            winner_mask: vec![false; num_players],
            final_round: 0, // backfilled after game ends
        });

        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => { break; }
            GamePhase::GameOver => break,
        };

        let config = &configs[player_index];
        let max_rollout_round = std::cmp::max(8, state.round + 2);
        let result = ismcts(&state, player_index, config, &None, Some(max_rollout_round), None, rng);
        apply_choice_to_state(&mut state, &result.choice, rng);
    }

    // Determine winner
    let mut best_score = 0u32;
    for p in state.players.iter() {
        let s = calculate_score(p);
        if s > best_score { best_score = s; }
    }
    let winners: Vec<bool> = state.players.iter().map(|p| calculate_score(p) == best_score).collect();

    // Compute diff-eval outcome
    let diff_eval_outcome = diff_player.map(|dp| {
        let diff_won = winners[dp];
        let base_won = winners[1 - dp];
        if diff_won && !base_won { 1 }
        else if !diff_won && base_won { -1 }
        else { 0 }
    });

    let final_round = state.round;
    for sample in &mut snapshots {
        sample.winner_mask = winners.clone();
        sample.final_round = final_round;
    }

    GameResult { samples: snapshots, diff_eval_outcome, final_round: state.round }
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
        let n = sample.state.players.len();

        // Forward: compute logits (one per player, from each player's perspective)
        let mut logits = vec![0.0f64; n];
        for i in 0..n {
            logits[i] = diff_eval_score(&sample.state, i, params, table);
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

        // Target: uniform over winners, weighted by urgency.
        // Games won quickly are worth more -- encourages the model to
        // value positions that lead to fast wins.
        // Weight: 1.0 + (MAX_ROUNDS - final_round) / MAX_ROUNDS
        // e.g. win at round 5 -> weight 1.75, win at round 10 -> weight 1.5, round 20 -> weight 1.0
        const MAX_ROUNDS: f64 = 20.0;
        let urgency_weight = 1.0 + (MAX_ROUNDS - sample.final_round as f64).max(0.0) / MAX_ROUNDS;

        let num_winners = sample.winner_mask.iter().filter(|&&w| w).count() as f64;
        let mut targets = vec![0.0f64; n];
        for i in 0..n {
            targets[i] = if sample.winner_mask[i] { urgency_weight / num_winners } else { 0.0 };
        }
        // Normalize targets to sum to urgency_weight (not 1.0) -- this makes
        // samples from fast games contribute more to the loss and gradient.

        // Cross-entropy loss (weighted)
        for i in 0..n {
            if targets[i] > 0.0 {
                total_loss -= targets[i] * (probs[i].max(1e-10)).ln();
            }
        }

        // Gradient: d_logit_i = urgency_weight * probs[i] - targets[i]
        // The urgency weight scales the entire gradient for this sample.
        for i in 0..n {
            let grad_output = urgency_weight * probs[i] - targets[i];
            let grads = diff_eval_backward(&sample.state, i, params, table, grad_output);
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
    pub passes: usize,
    pub lr: f64,
    pub eval_iterations: u32,
    pub baseline_iterations: Option<u32>,
    pub vs_baseline: bool,
    pub no_rollout: bool,
    pub threads: usize,
    pub output: String,
    pub replay_buffer_epochs: usize,
}

pub fn run_training(args: &SimulationArgs, train: &TrainArgs) {
    eprintln!("=== Diff Eval Training ===");
    eprintln!("Games/epoch: {}, Epochs: {}, Batch size: {}, Passes: {}, LR: {}, Replay buffer: {} epochs",
        train.games, train.epochs, train.batch_size, train.passes, train.lr, train.replay_buffer_epochs);
    let baseline_iters_display = train.baseline_iterations.unwrap_or(train.eval_iterations);
    eprintln!("MCTS iterations: {} (baseline: {}), Threads: {}, Mode: {}, Rollout: {}",
        train.eval_iterations, baseline_iters_display, train.threads,
        if train.vs_baseline { "vs baseline" } else { "self-play" },
        if train.no_rollout { "none (direct eval)" } else { "standard" });

    let baseline_params = args.baseline_heuristic_params.clone().unwrap_or_default();
    if args.baseline_heuristic_params.is_some() {
        eprintln!("Baseline: custom heuristic params from --baseline-params");
    } else {
        eprintln!("Baseline: default heuristic params (use --baseline-params to override)");
    }

    // Create output directory
    std::fs::create_dir_all(&train.output).expect("Failed to create output directory");

    // Try to resume from latest checkpoint
    let (mut params, mut optimizer, start_epoch) = load_checkpoint(&train.output, train.lr);
    let table = DiffEvalTable::new(&params);

    if start_epoch > 0 {
        eprintln!("Resuming from epoch {}", start_epoch);
    }
    eprintln!();

    let mut prev_loss: Option<f64> = None;

    // Replay buffer: sliding window of recent epochs' samples
    use std::collections::VecDeque;
    let mut replay_buffer: VecDeque<Vec<TrainingSample>> = VecDeque::new();

    // Training loop
    for epoch in start_epoch..train.epochs {
        let epoch_start = std::time::Instant::now();

        let dep = if train.vs_baseline { Some(&params) } else { None };
        let baseline_iters = train.baseline_iterations.unwrap_or(train.eval_iterations);
        let data = generate_training_data(
            train.games, train.eval_iterations, baseline_iters, train.threads, &baseline_params, dep, train.no_rollout, epoch,
        );
        let new_samples = data.samples;
        if new_samples.is_empty() {
            eprintln!("No training samples generated for epoch {}!", epoch + 1);
            continue;
        }

        // Add new samples to replay buffer, evict oldest if over capacity
        replay_buffer.push_back(new_samples);
        while replay_buffer.len() > train.replay_buffer_epochs {
            replay_buffer.pop_front();
        }

        // Collect references to all samples in the buffer
        let total_samples: usize = replay_buffer.iter().map(|s| s.len()).sum();

        // Train on mini-batches with multiple passes over the buffered data
        let train_start = std::time::Instant::now();
        use rand::seq::SliceRandom;
        let mut rng = WyRand::from_rng(&mut rand::rng());
        let mut indices: Vec<usize> = (0..total_samples).collect();

        let mut epoch_loss = 0.0;
        let mut num_batches = 0;

        // Build a flat index → (buffer_slot, sample_index) mapping
        let flat_refs: Vec<(usize, usize)> = replay_buffer.iter().enumerate()
            .flat_map(|(slot, samples)| (0..samples.len()).map(move |i| (slot, i)))
            .collect();

        for _ in 0..train.passes {
            indices.shuffle(&mut rng);
            for chunk in indices.chunks(train.batch_size) {
                let batch: Vec<&TrainingSample> = chunk.iter()
                    .map(|&i| {
                        let (slot, idx) = flat_refs[i];
                        &replay_buffer[slot][idx]
                    })
                    .collect();
                let (loss, grads) = compute_loss_and_grads(&batch, &params, &table);
                optimizer.step(&mut params, &grads);
                epoch_loss += loss;
                num_batches += 1;
            }
        }

        let avg_loss = epoch_loss / num_batches as f64;
        let train_secs = train_start.elapsed().as_secs_f64();
        let total_secs = epoch_start.elapsed().as_secs_f64();

        // Format loss with trend indicator
        let trend = match prev_loss {
            Some(pl) if avg_loss < pl - 0.001 => " \u{2193}",  // ↓
            Some(pl) if avg_loss > pl + 0.001 => " \u{2191}",  // ↑
            Some(_) => " =",
            None => "",
        };
        prev_loss = Some(avg_loss);

        eprintln!("Epoch {}/{}: loss={:.4}{} ({} samples from {} epochs, {:.0}s train, {:.0}s total)",
            epoch + 1, train.epochs, avg_loss, trend, total_samples, replay_buffer.len(), train_secs, total_secs);

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
                    let len = cp.adam_m.len().min(NUM_PARAMS);
                    optimizer.m[..len].copy_from_slice(&cp.adam_m[..len]);
                    optimizer.v[..len].copy_from_slice(&cp.adam_v[..len]);
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
