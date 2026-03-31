use colori_core::ismcts::MctsConfig;
use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

use crate::cli::{SoloArgs, NamedVariant, load_variants_from_file};
use crate::generate_batch_id;
use crate::simulation::{run_game, now_epoch_millis};

pub fn run_solo(args: &SoloArgs, threads: usize, output: &str, glass: bool) {
    let config = if let Some(ref path) = args.variant_file {
        let variants = load_variants_from_file(path);
        variants.into_iter().next().expect("Variant file is empty").ai
    } else {
        MctsConfig {
            iterations: args.iterations,
            ..MctsConfig::default()
        }
    };

    let player_variants = vec![NamedVariant {
        name: None,
        ai: config,
    }];

    eprintln!(
        "Running {} solo games ({} rounds, {} MCTS iterations, {} threads)",
        args.games, args.max_rounds, player_variants[0].ai.iterations, threads
    );

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let batch_id = generate_batch_id();
    let wins = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let total_ducats = AtomicU64::new(0);
    let total_games = args.games;
    let max_rounds = args.max_rounds;
    let note_text = format!("solo-{}r", max_rounds);
    let batch_id = batch_id.as_str();
    let player_variants = player_variants.as_slice();
    let note_text = note_text.as_str();

    std::thread::scope(|s| {
        let games_per_thread = total_games / threads;
        let remainder = total_games % threads;
        let mut handles = Vec::new();

        for t in 0..threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let wins = &wins;
            let completed = &completed;
            let total_ducats = &total_ducats;

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                for _ in 0..count {
                    let log = run_game(
                        0,
                        player_variants,
                        Some(note_text.to_string()),
                        glass,
                        Some(max_rounds),
                        &mut rng,
                    );

                    let score = log.final_scores.as_ref()
                        .and_then(|fs| fs.first())
                        .map(|fs| fs.score)
                        .unwrap_or(0);
                    if score >= 16 {
                        wins.fetch_add(1, Ordering::Relaxed);
                    }
                    total_ducats.fetch_add(score as u64, Ordering::Relaxed);

                    set_card_registry(&log.initial_state.card_lookup);
                    set_sell_card_registry(&log.initial_state.sell_card_lookup);
                    let epoch_millis = now_epoch_millis();
                    let game_id: String = {
                        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                        (0..4)
                            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
                            .collect()
                    };
                    let path = format!("{}/game-{}-{}-{}.json", output, epoch_millis, batch_id, game_id);
                    let json = serde_json::to_string_pretty(&log).unwrap();
                    std::fs::write(&path, json).unwrap();

                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if done % 100 == 0 || done == total_games {
                        let w = wins.load(Ordering::Relaxed);
                        eprintln!(
                            "Game {}/{} — win rate: {:.1}%",
                            done,
                            total_games,
                            w as f64 / done as f64 * 100.0
                        );
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    let total_wins = wins.load(Ordering::Relaxed);
    let total_d = total_ducats.load(Ordering::Relaxed);
    let avg_ducats = total_d as f64 / total_games as f64;
    let win_rate = total_wins as f64 / total_games as f64 * 100.0;

    eprintln!();
    eprintln!("=== Solo Results ({} rounds) ===", max_rounds);
    eprintln!("Games:      {}", total_games);
    eprintln!("Wins:       {} ({:.1}%)", total_wins, win_rate);
    eprintln!("Avg ducats: {:.1}", avg_ducats);
    eprintln!("Logs written to {}/", output);
}
