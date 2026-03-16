mod cli;
mod cmaes;
mod simulation;
mod tournament;
mod train_diff_eval;

use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::{AtomicUsize, Ordering};

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
            lr: args.train_lr,
            eval_iterations: if !args.variants.is_empty() { args.variants[0].ai.iterations } else { 100 },
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

    let batch_id = generate_batch_id();
    let completed = AtomicUsize::new(0);
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

    eprintln!("All {} games written to {}/", total_games, args.output);
}
