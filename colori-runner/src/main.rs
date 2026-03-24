mod cli;
mod cmaes;
pub(crate) mod legacy_eval;
mod simulation;
mod tournament;
mod train_diff_eval;

use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use cli::parse_args;
use cmaes::{run_genetic_algorithm, run_first_pick_cmaes};
use simulation::{
    compute_differing_fields, format_variant_label, has_any_difference, now_epoch_millis,
    run_game,
};

pub(crate) fn generate_batch_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = WyRand::from_rng(&mut rand::rng());
    (0..6)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

fn main() {
    let args = parse_args();

    if args.train_first_pick {
        let ga = args.genetic.as_ref().cloned().unwrap_or(cli::CmaEsArgs {
            population: 14,
            generations: 50,
            games_per_eval: 100,
            initial_sigma: 0.3,
            eval_iterations: 4000,
            seed_params: None,
            baseline_params: None,
        });
        run_first_pick_cmaes(&args, &ga);
        return;
    }

    if let Some(ref ga) = args.genetic {
        run_genetic_algorithm(&args, ga);
        return;
    }

    if args.tournament {
        tournament::run_tournament(&args);
        return;
    }

    if args.train_diff_eval {
        let train_args = train_diff_eval::TrainArgs {
            games: args.train_games_per_epoch,
            epochs: args.train_epochs,
            batch_size: args.train_batch_size,
            passes: args.train_passes,
            lr: args.train_lr,
            eval_iterations: args.train_eval_iterations,
            baseline_iterations: args.train_baseline_iterations,
            self_play: args.train_self_play,
            vs_baseline: args.train_vs_baseline,
            no_rollout: args.train_no_rollout,
            threads: args.threads,
            output: args.output.clone(),
            replay_buffer_epochs: args.train_replay_buffer_epochs,
            distill_from: args.distill_from.clone(),
        };
        train_diff_eval::run_training(&args, &train_args);
        return;
    }

    let player_variants = &args.variants;
    let num_players = player_variants.len();

    if has_any_difference(player_variants) {
        let differing = compute_differing_fields(player_variants);
        let labels: Vec<String> = player_variants.iter().map(|v| format_variant_label(v, &differing)).collect();
        eprintln!(
            "Running {} games with variants: {}, {} threads",
            args.games,
            labels.join(", "),
            args.threads
        );
    } else if let Some(tl) = player_variants[0].ai.time_limit_ms {
        eprintln!(
            "Running {} games with {} players, {}ms MCTS time limit, {} threads",
            args.games, num_players, tl, args.threads
        );
    } else {
        eprintln!(
            "Running {} games with {} players, {} ISMCTS iterations, {} threads",
            args.games, num_players,
            player_variants[0].ai.iterations,
            args.threads
        );
    }

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let any_early_termination = player_variants.iter().any(|v| v.ai.early_termination);
    let batch_id = generate_batch_id();
    let completed = AtomicUsize::new(0);
    let agg_iterations_budget = AtomicU64::new(0);
    let agg_iterations_used = AtomicU64::new(0);
    let agg_reuse_budget = AtomicU64::new(0);
    let agg_reuse_saved = AtomicU64::new(0);
    let variant_time_ms: Vec<AtomicU64> = (0..num_players).map(|_| AtomicU64::new(0)).collect();
    let variant_iterations: Vec<AtomicU64> = (0..num_players).map(|_| AtomicU64::new(0)).collect();
    let total_games = args.games;
    let num_threads = args.threads;
    let output_dir = &args.output;
    let batch_id = batch_id.as_str();
    let note = &args.note;
    let glass = args.glass;
    let player_variants = player_variants.as_slice();

    std::thread::scope(|s| {
        let games_per_thread = total_games / num_threads;
        let remainder = total_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let completed = &completed;
            let agg_iterations_budget = &agg_iterations_budget;
            let agg_iterations_used = &agg_iterations_used;
            let agg_reuse_budget = &agg_reuse_budget;
            let agg_reuse_saved = &agg_reuse_saved;
            let variant_time_ms = &variant_time_ms;
            let variant_iterations = &variant_iterations;

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                for _i in 0..count {
                    let log = run_game(
                        0,
                        player_variants,
                        note.clone(),
                        glass,
                        &mut rng,
                    );
                    if let Some(savings) = log.early_termination_savings {
                        let scale = 10000u64;
                        let used_frac = ((1.0 - savings) * scale as f64).round() as u64;
                        agg_iterations_budget.fetch_add(scale, Ordering::Relaxed);
                        agg_iterations_used.fetch_add(used_frac, Ordering::Relaxed);
                    }
                    if let Some(savings) = log.subtree_reuse_savings {
                        let scale = 10000u64;
                        let saved_frac = (savings * scale as f64).round() as u64;
                        agg_reuse_budget.fetch_add(scale, Ordering::Relaxed);
                        agg_reuse_saved.fetch_add(saved_frac, Ordering::Relaxed);
                    }
                    for (player_pos, &orig_idx) in log.variant_order.iter().enumerate() {
                        variant_time_ms[orig_idx].fetch_add(log.player_time_ms[player_pos], Ordering::Relaxed);
                        variant_iterations[orig_idx].fetch_add(log.player_iterations[player_pos], Ordering::Relaxed);
                    }
                    set_card_registry(&log.initial_state.card_lookup);
                    set_sell_card_registry(&log.initial_state.sell_card_lookup);
                    let epoch_millis = now_epoch_millis();
                    let game_id: String = {
                        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                        (0..4)
                            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
                            .collect()
                    };
                    let path = format!("{}/game-{}-{}-{}.json", output_dir, epoch_millis, batch_id, game_id);
                    let json = serde_json::to_string_pretty(&log).unwrap();
                    std::fs::write(&path, json).unwrap();
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    eprintln!("Game {}/{} complete", done, total_games);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    if any_early_termination {
        let budget = agg_iterations_budget.load(Ordering::Relaxed);
        let used = agg_iterations_used.load(Ordering::Relaxed);
        if budget > 0 {
            let savings = 1.0 - (used as f64 / budget as f64);
            eprintln!("Early termination saved {:.1}% of iterations across all games", savings * 100.0);
        }
    }
    {
        let budget = agg_reuse_budget.load(Ordering::Relaxed);
        let saved = agg_reuse_saved.load(Ordering::Relaxed);
        if budget > 0 {
            let savings = saved as f64 / budget as f64;
            eprintln!("Subtree reuse saved {:.1}% of iterations across all games", savings * 100.0);
        }
    }
    if has_any_difference(player_variants) {
        let differing = compute_differing_fields(player_variants);
        for (i, v) in player_variants.iter().enumerate() {
            let total_ms = variant_time_ms[i].load(Ordering::Relaxed);
            let total_iters = variant_iterations[i].load(Ordering::Relaxed);
            let avg_secs = total_ms as f64 / total_games as f64 / 1000.0;
            let avg_iters = total_iters as f64 / total_games as f64;
            eprintln!("{}: {:.1}s avg, {:.0} avg iters per game", format_variant_label(v, &differing), avg_secs, avg_iters);
        }
    } else {
        let total_ms: u64 = variant_time_ms.iter().map(|a| a.load(Ordering::Relaxed)).sum();
        let total_iters: u64 = variant_iterations.iter().map(|a| a.load(Ordering::Relaxed)).sum();
        let avg_secs = total_ms as f64 / (total_games as f64 * num_players as f64) / 1000.0;
        let avg_iters = total_iters as f64 / (total_games as f64 * num_players as f64);
        eprintln!("Average per player per game: {:.1}s, {:.0} iters", avg_secs, avg_iters);
    }
    eprintln!("All {} games written to {}/", total_games, args.output);
}
