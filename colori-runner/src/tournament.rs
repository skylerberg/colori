use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use colori_core::unordered_cards::{set_card_registry, set_sell_card_registry};
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;

use crate::cli::{NamedVariant, SimulationArgs};
use crate::generate_batch_id;
use crate::simulation::{now_epoch_millis, run_game};

struct TournamentStats {
    labels: Vec<String>,
    wins: Vec<AtomicU64>,
    draws: Vec<AtomicU64>,
    games: Vec<AtomicU64>,
    time_ms: Vec<AtomicU64>,
    iterations: Vec<AtomicU64>,
}

impl TournamentStats {
    fn new(labels: Vec<String>) -> Self {
        let n = labels.len();
        TournamentStats {
            labels,
            wins: (0..n).map(|_| AtomicU64::new(0)).collect(),
            draws: (0..n).map(|_| AtomicU64::new(0)).collect(),
            games: (0..n).map(|_| AtomicU64::new(0)).collect(),
            time_ms: (0..n).map(|_| AtomicU64::new(0)).collect(),
            iterations: (0..n).map(|_| AtomicU64::new(0)).collect(),
        }
    }
}

pub fn run_tournament(args: &SimulationArgs) {
    let num_variants = args.variants.len();
    if num_variants < 2 {
        eprintln!("Tournament mode requires at least 2 variants");
        std::process::exit(1);
    }

    // Build unique labels for all variants
    let mut labels: Vec<String> = args
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| v.name.clone().unwrap_or_else(|| format!("variant_{}", i)))
        .collect();

    // Ensure uniqueness by appending index if there are duplicates
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut has_dupes = false;
    for label in &labels {
        let count = seen.entry(label.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            has_dupes = true;
        }
    }
    if has_dupes {
        labels = labels
            .iter()
            .enumerate()
            .map(|(i, name)| format!("{} [{}]", name, i))
            .collect();
    }

    // Create variants with guaranteed-unique names
    let variants: Vec<NamedVariant> = args
        .variants
        .iter()
        .zip(labels.iter())
        .map(|(v, label)| NamedVariant {
            name: Some(label.clone()),
            ai: v.ai.clone(),
        })
        .collect();

    // Build name -> index map
    let name_to_index: HashMap<String, usize> = labels
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();

    eprintln!(
        "Tournament: {} games, {} variants, {} threads",
        args.games, num_variants, args.threads
    );
    for (i, label) in labels.iter().enumerate() {
        eprintln!("  [{}] {}", i, label);
    }

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let batch_id = generate_batch_id();
    let stats = TournamentStats::new(labels.clone());
    let total_games = args.games;
    let num_threads = args.threads;
    let output_dir = &args.output;
    let batch_id_str = batch_id.as_str();
    let note = &args.note;
    let glass = args.glass;
    let variants = variants.as_slice();
    let stats = &stats;
    let name_to_index = &name_to_index;
    let completed = AtomicU64::new(0);
    let completed = &completed;

    std::thread::scope(|s| {
        let games_per_thread = total_games / num_threads;
        let remainder = total_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                for _ in 0..count {
                    // Pick 2 distinct random variant indices
                    let i = rng.random_range(0..num_variants);
                    let mut j = rng.random_range(0..num_variants - 1);
                    if j >= i {
                        j += 1;
                    }

                    let pair = vec![variants[i].clone(), variants[j].clone()];
                    let log = run_game(0, &pair, note.clone(), glass, &mut rng);

                    set_card_registry(&log.initial_state.card_lookup);
                    set_sell_card_registry(&log.initial_state.sell_card_lookup);

                    // Determine winner from final_scores
                    let scores = log.final_scores.as_ref().unwrap();
                    let ranking_0 = (
                        scores[0].score,
                        scores[0].completed_sell_cards,
                        scores[0].color_wheel_total,
                    );
                    let ranking_1 = (
                        scores[1].score,
                        scores[1].completed_sell_cards,
                        scores[1].color_wheel_total,
                    );

                    // Map player positions back to original variant indices
                    let pv = log.player_variants.as_ref().unwrap();
                    let idx_0 = name_to_index[pv[0].name.as_ref().unwrap()];
                    let idx_1 = name_to_index[pv[1].name.as_ref().unwrap()];

                    if ranking_0 > ranking_1 {
                        stats.wins[idx_0].fetch_add(1, Ordering::Relaxed);
                    } else if ranking_1 > ranking_0 {
                        stats.wins[idx_1].fetch_add(1, Ordering::Relaxed);
                    } else {
                        stats.draws[idx_0].fetch_add(1, Ordering::Relaxed);
                        stats.draws[idx_1].fetch_add(1, Ordering::Relaxed);
                    }
                    stats.games[idx_0].fetch_add(1, Ordering::Relaxed);
                    stats.games[idx_1].fetch_add(1, Ordering::Relaxed);

                    // Accumulate per-variant MCTS time and iterations
                    for (player_pos, player_variant) in pv.iter().enumerate() {
                        let vi = name_to_index[player_variant.name.as_ref().unwrap()];
                        stats.time_ms[vi].fetch_add(log.player_time_ms[player_pos], Ordering::Relaxed);
                        stats.iterations[vi].fetch_add(log.player_iterations[player_pos], Ordering::Relaxed);
                    }

                    // Save game log
                    let epoch_millis = now_epoch_millis();
                    let game_id: String = {
                        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                        (0..4)
                            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
                            .collect()
                    };
                    let path = format!(
                        "{}/game-{}-{}-{}.json",
                        output_dir, epoch_millis, batch_id_str, game_id
                    );
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
    print_summary(stats);
}

fn print_summary(stats: &TournamentStats) {
    let n = stats.labels.len();

    struct VariantResult {
        label: String,
        games: u64,
        wins: u64,
        draws: u64,
        losses: u64,
        win_rate: f64,
        avg_time_secs: f64,
        avg_iterations: f64,
    }

    let mut results: Vec<VariantResult> = (0..n)
        .map(|i| {
            let games = stats.games[i].load(Ordering::Relaxed);
            let wins = stats.wins[i].load(Ordering::Relaxed);
            let draws = stats.draws[i].load(Ordering::Relaxed);
            let time_ms = stats.time_ms[i].load(Ordering::Relaxed);
            let iters = stats.iterations[i].load(Ordering::Relaxed);
            let losses = games.saturating_sub(wins + draws);
            let win_rate = if games > 0 {
                (wins as f64 + 0.5 * draws as f64) / games as f64
            } else {
                0.0
            };
            let avg_time_secs = if games > 0 {
                time_ms as f64 / games as f64 / 1000.0
            } else {
                0.0
            };
            let avg_iterations = if games > 0 {
                iters as f64 / games as f64
            } else {
                0.0
            };
            VariantResult {
                label: stats.labels[i].clone(),
                games,
                wins,
                draws,
                losses,
                win_rate,
                avg_time_secs,
                avg_iterations,
            }
        })
        .collect();

    results.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());

    let max_label = results
        .iter()
        .map(|r| r.label.len())
        .max()
        .unwrap_or(7)
        .max(7);

    eprintln!();
    eprintln!("=== Tournament Results ===");
    eprintln!();
    eprintln!(
        "{:<width$}  {:>5}  {:>5}  {:>5}  {:>6}  {:>8}  {:>8}  {:>10}",
        "Variant",
        "Games",
        "Wins",
        "Draws",
        "Losses",
        "Win Rate",
        "Avg Time",
        "Avg Iters",
        width = max_label,
    );
    eprintln!("{}", "-".repeat(max_label + 58));
    for r in &results {
        eprintln!(
            "{:<width$}  {:>5}  {:>5}  {:>5}  {:>6}  {:>7.1}%  {:>7.1}s  {:>10.0}",
            r.label,
            r.games,
            r.wins,
            r.draws,
            r.losses,
            r.win_rate * 100.0,
            r.avg_time_secs,
            r.avg_iterations,
            width = max_label,
        );
    }
}
