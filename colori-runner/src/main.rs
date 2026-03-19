mod cli;
mod cmaes;
mod simulation;
mod tournament;
mod train_diff_eval;

use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use cli::parse_args;
use cmaes::run_genetic_algorithm;
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
            vs_baseline: args.train_vs_baseline,
            no_rollout: args.train_no_rollout,
            threads: args.threads,
            output: args.output.clone(),
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
                        // Back-compute budget and used from savings ratio
                        // savings = 1 - used/budget, so used = budget * (1 - savings)
                        // We use a fixed scale to accumulate without floating point drift
                        let scale = 10000u64;
                        let used_frac = ((1.0 - savings) * scale as f64).round() as u64;
                        agg_iterations_budget.fetch_add(scale, Ordering::Relaxed);
                        agg_iterations_used.fetch_add(used_frac, Ordering::Relaxed);
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
    eprintln!("All {} games written to {}/", total_games, args.output);
}
