use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::rhea::{rhea, RheaConfig};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;
use colori_core::unordered_cards::{set_buyer_registry, set_card_registry};

use rand::seq::SliceRandom;
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ── CLI args ──

struct Args {
    games: usize,
    threads: usize,
    output: String,
    note: Option<String>,
    variants: Vec<NamedVariant>,
}

#[derive(Clone)]
enum AiConfig {
    Mcts(MctsConfig),
    Rhea(RheaConfig),
}

#[derive(Clone)]
struct NamedVariant {
    name: Option<String>,
    ai: AiConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariantFileEntry {
    name: Option<String>,
    #[serde(default)]
    algorithm: Option<String>,
    // MCTS fields
    #[serde(default)]
    iterations: Option<u32>,
    #[serde(default)]
    exploration_constant: Option<f64>,
    #[serde(default)]
    max_rollout_steps: Option<u32>,
    // RHEA fields
    #[serde(default)]
    generations: Option<u32>,
    #[serde(default)]
    population_size: Option<u32>,
    #[serde(default)]
    horizon_length: Option<u32>,
    #[serde(default)]
    mutation_rate: Option<f64>,
    #[serde(default)]
    elitism_count: Option<u32>,
    #[serde(default)]
    tournament_size: Option<u32>,
}

impl VariantFileEntry {
    fn into_named_variant(self) -> NamedVariant {
        let algorithm = self.algorithm.as_deref().unwrap_or("mcts");
        match algorithm {
            "rhea" => {
                let defaults = RheaConfig::default();
                NamedVariant {
                    name: self.name,
                    ai: AiConfig::Rhea(RheaConfig {
                        generations: self.generations.unwrap_or(defaults.generations),
                        population_size: self.population_size.unwrap_or(defaults.population_size),
                        horizon_length: self.horizon_length.unwrap_or(defaults.horizon_length),
                        mutation_rate: self.mutation_rate.unwrap_or(defaults.mutation_rate),
                        max_rollout_steps: self.max_rollout_steps.unwrap_or(defaults.max_rollout_steps),
                        elitism_count: self.elitism_count.unwrap_or(defaults.elitism_count),
                        tournament_size: self.tournament_size.unwrap_or(defaults.tournament_size),
                    }),
                }
            }
            _ => {
                let defaults = MctsConfig::default();
                NamedVariant {
                    name: self.name,
                    ai: AiConfig::Mcts(MctsConfig {
                        iterations: self.iterations.unwrap_or(defaults.iterations),
                        exploration_constant: self.exploration_constant.unwrap_or(defaults.exploration_constant),
                        max_rollout_steps: self.max_rollout_steps.unwrap_or(defaults.max_rollout_steps),
                    }),
                }
            }
        }
    }
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut games = 10_000usize;
    let mut threads = 10usize;
    let mut output = "game-logs".to_string();
    let mut note: Option<String> = None;
    let mut variants: Option<Vec<NamedVariant>> = None;
    let mut variants_file = "variants.json".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" => {
                i += 1;
                games = args[i].parse().expect("Invalid --games value");
            }
            "--threads" => {
                i += 1;
                threads = args[i].parse().expect("Invalid --threads value");
            }
            "--output" => {
                i += 1;
                output = args[i].clone();
            }
            "--note" => {
                i += 1;
                note = Some(args[i].clone());
            }
            "--variants" => {
                i += 1;
                variants = Some(
                    args[i]
                        .split(',')
                        .map(|s| {
                            let iters: u32 = s.trim().parse().expect("Invalid --variants value");
                            NamedVariant {
                                name: None,
                                ai: AiConfig::Mcts(MctsConfig { iterations: iters, ..MctsConfig::default() }),
                            }
                        })
                        .collect(),
                );
            }
            "--variants-file" => {
                i += 1;
                variants_file = args[i].clone();
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let variants = variants.unwrap_or_else(|| {
        let contents = std::fs::read_to_string(&variants_file)
            .unwrap_or_else(|_| panic!("Failed to read variants file: {}", variants_file));
        let entries: Vec<VariantFileEntry> = serde_json::from_str(&contents)
            .unwrap_or_else(|_| panic!("Failed to parse variants file: {}", variants_file));
        entries
            .into_iter()
            .map(|e| e.into_named_variant())
            .collect()
    });

    Args {
        games,
        threads,
        output,
        note,
        variants,
    }
}

// ── Serialization types ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameRunOutput {
    version: u32,
    game_started_at: String,
    game_ended_at: Option<String>,
    player_names: Vec<String>,
    ai_players: Vec<bool>,
    initial_state: GameState,
    final_scores: Option<Vec<FinalScore>>,
    final_player_stats: Option<Vec<FinalPlayerStats>>,
    entries: Vec<StructuredLogEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    player_variants: Option<Vec<PlayerVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StructuredLogEntry {
    seq: u32,
    timestamp: u64,
    round: u32,
    phase: String,
    player_index: usize,
    choice: Choice,
}

// ── Helpers ──

fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn now_epoch_secs_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", secs)
}

fn format_iterations(iters: u32) -> String {
    if iters >= 1000 && iters % 1000 == 0 {
        format!("{}k", iters / 1000)
    } else {
        format!("{}", iters)
    }
}

fn format_variant_label(variant: &NamedVariant) -> String {
    if let Some(name) = &variant.name {
        return name.clone();
    }
    match &variant.ai {
        AiConfig::Mcts(config) => format!("MCTS-{}", format_iterations(config.iterations)),
        AiConfig::Rhea(config) => format!("RHEA-{}", format_iterations(config.generations)),
    }
}

fn has_any_difference(variants: &[NamedVariant]) -> bool {
    if variants.len() <= 1 {
        return false;
    }
    if variants.iter().any(|v| v.name.is_some()) {
        return true;
    }
    // Check if algorithms differ
    let algorithms_differ = variants.windows(2).any(|w| {
        std::mem::discriminant(&w[0].ai) != std::mem::discriminant(&w[1].ai)
    });
    if algorithms_differ {
        return true;
    }
    // Check config differences within same algorithm type
    match &variants[0].ai {
        AiConfig::Mcts(first) => variants.iter().any(|v| {
            if let AiConfig::Mcts(c) = &v.ai {
                c.iterations != first.iterations
                    || c.exploration_constant != first.exploration_constant
                    || c.max_rollout_steps != first.max_rollout_steps
            } else {
                true
            }
        }),
        AiConfig::Rhea(first) => variants.iter().any(|v| {
            if let AiConfig::Rhea(c) = &v.ai {
                c.generations != first.generations
                    || c.population_size != first.population_size
                    || c.horizon_length != first.horizon_length
                    || c.mutation_rate != first.mutation_rate
            } else {
                true
            }
        }),
    }
}

fn variant_to_player_variant(v: &NamedVariant) -> PlayerVariant {
    let mcts_defaults = MctsConfig::default();
    match &v.ai {
        AiConfig::Mcts(config) => PlayerVariant {
            name: v.name.clone(),
            algorithm: Some("mcts".to_string()),
            iterations: config.iterations,
            exploration_constant: if config.exploration_constant != mcts_defaults.exploration_constant {
                Some(config.exploration_constant)
            } else {
                None
            },
            max_rollout_steps: if config.max_rollout_steps != mcts_defaults.max_rollout_steps {
                Some(config.max_rollout_steps)
            } else {
                None
            },
        },
        AiConfig::Rhea(config) => PlayerVariant {
            name: v.name.clone(),
            algorithm: Some("rhea".to_string()),
            iterations: config.generations,
            exploration_constant: None,
            max_rollout_steps: if config.max_rollout_steps != mcts_defaults.max_rollout_steps {
                Some(config.max_rollout_steps)
            } else {
                None
            },
        },
    }
}

// ── Game loop ──

fn run_game(
    _game_index: usize,
    player_variants: &[NamedVariant],
    note: Option<String>,
    rng: &mut WyRand,
) -> GameRunOutput {
    let start = Instant::now();
    let num_players = player_variants.len();

    // Shuffle variant assignment to eliminate position bias
    let mut shuffled_variants = player_variants.to_vec();
    shuffled_variants.shuffle(rng);

    let has_variants = has_any_difference(&shuffled_variants);
    let names: Vec<String> = (1..=num_players)
        .map(|i| {
            if has_variants {
                format!("Player {} ({})", i, format_variant_label(&shuffled_variants[i - 1]))
            } else {
                format!("Player {}", i)
            }
        })
        .collect();

    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, rng);
    let initial_state = state.clone();

    let game_started_at = now_epoch_secs_string();

    // Start first round (draw phase -> draft phase)
    execute_draw_phase(&mut state, rng);

    let mut entries: Vec<StructuredLogEntry> = Vec::new();
    let mut seq: u32 = 0;

    // Main game loop
    while !matches!(state.phase, GamePhase::GameOver) {
        let (player_index, phase_str) = match &state.phase {
            GamePhase::Draft { draft_state } => {
                (draft_state.current_player_index, "draft")
            }
            GamePhase::Action { action_state } => {
                (action_state.current_player_index, "action")
            }
            GamePhase::Draw => {
                break;
            }
            GamePhase::GameOver => break,
        };

        let max_rollout_round = std::cmp::max(8, state.round + 2);
        let choice = match &shuffled_variants[player_index].ai {
            AiConfig::Mcts(config) => ismcts(&state, player_index, config, &None, Some(max_rollout_round), rng),
            AiConfig::Rhea(config) => rhea(&state, player_index, config, &None, Some(max_rollout_round), rng),
        };

        seq += 1;
        entries.push(StructuredLogEntry {
            seq,
            timestamp: now_epoch_millis(),
            round: state.round,
            phase: phase_str.to_string(),
            player_index,
            choice: choice.clone(),
        });

        apply_choice_to_state(&mut state, &choice, rng);
    }

    let game_ended_at = Some(now_epoch_secs_string());

    // Compute final scores
    let final_scores: Option<Vec<FinalScore>> = Some(
        state
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| FinalScore {
                name: names[i].clone(),
                score: calculate_score(p),
                completed_buyers: p.completed_buyers.len() as u32,
                color_wheel_total: p.color_wheel.counts.iter().sum(),
            })
            .collect(),
    );

    // Compute final player stats
    let final_player_stats: Option<Vec<FinalPlayerStats>> = Some(
        state
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| FinalPlayerStats {
                name: names[i].clone(),
                deck_size: (p.deck.len() + p.discard.len() + p.workshop_cards.len() + p.workshopped_cards.len()) as usize,
                completed_buyers: p.completed_buyers.to_vec(),
                ducats: p.ducats,
                color_wheel: p.color_wheel.clone(),
                materials: p.materials.clone(),
            })
            .collect(),
    );

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    let (log_iterations, log_player_variants) = if has_variants {
        (
            None,
            Some(
                shuffled_variants
                    .iter()
                    .map(|v| variant_to_player_variant(v))
                    .collect(),
            ),
        )
    } else {
        let iters = match &shuffled_variants[0].ai {
            AiConfig::Mcts(c) => c.iterations,
            AiConfig::Rhea(c) => c.generations,
        };
        (Some(iters), None)
    };

    GameRunOutput {
        version: 1,
        game_started_at,
        game_ended_at,
        player_names: names,
        ai_players,
        initial_state,
        final_scores,
        final_player_stats,
        entries,
        duration_ms,
        iterations: log_iterations,
        player_variants: log_player_variants,
        note,
    }
}

// ── Main ──

fn generate_batch_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = WyRand::from_rng(&mut rand::rng());
    (0..6)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

fn main() {
    let args = parse_args();

    let player_variants = &args.variants;
    let num_players = player_variants.len();

    if has_any_difference(player_variants) {
        let labels: Vec<String> = player_variants.iter().map(|v| format_variant_label(v)).collect();
        eprintln!(
            "Running {} games with variants: {}, {} threads",
            args.games,
            labels.join(", "),
            args.threads
        );
    } else {
        let label = format_variant_label(&player_variants[0]);
        eprintln!(
            "Running {} games with {} players, {}, {} threads",
            args.games, num_players, label, args.threads
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
                    let log = run_game(0, player_variants, note.clone(), &mut rng);
                    set_card_registry(&log.initial_state.card_lookup);
                    set_buyer_registry(&log.initial_state.buyer_lookup);
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
