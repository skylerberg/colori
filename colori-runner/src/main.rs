use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;
use colori_core::unordered_cards::{set_buyer_registry, set_card_registry};

#[cfg(feature = "nn-ai")]
use colori_core::atomic::{apply_atomic_choice, AtomicChoice};
#[cfg(feature = "nn-ai")]
use colori_core::nn_mcts::{nn_mcts, NnMctsConfig, NnModel};
#[cfg(feature = "nn-ai")]
use smallvec::smallvec;

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
    glass: bool,
}

#[derive(Clone)]
enum AiConfig {
    Ismcts(MctsConfig),
    #[cfg(feature = "nn-ai")]
    NnMcts(NnMctsConfig),
}

#[derive(Clone)]
struct NamedVariant {
    name: Option<String>,
    ai: AiConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct VariantFileEntry {
    name: Option<String>,
    #[serde(default)]
    algorithm: Option<String>,
    #[serde(default)]
    iterations: Option<u32>,
    #[serde(default)]
    exploration_constant: Option<f64>,
    #[serde(default)]
    max_rollout_steps: Option<u32>,
    #[serde(default)]
    model_path: Option<String>,
    #[serde(default)]
    simulations: Option<u32>,
    #[serde(default)]
    c_puct: Option<f64>,
    #[serde(default)]
    determinize_draft_deck: Option<bool>,
}

impl VariantFileEntry {
    fn into_named_variant(self) -> NamedVariant {
        let algorithm = self.algorithm.as_deref().unwrap_or("ismcts");
        match algorithm {
            "nn-mcts" => {
                #[cfg(feature = "nn-ai")]
                {
                    let defaults = NnMctsConfig::default();
                    let model_path = self.model_path.expect("nn-mcts variant requires 'modelPath'");
                    NamedVariant {
                        name: self.name,
                        ai: AiConfig::NnMcts(NnMctsConfig {
                            simulations: self.simulations.unwrap_or(defaults.simulations),
                            c_puct: self.c_puct.map(|v| v as f32).unwrap_or(defaults.c_puct),
                            model_path,
                            determinize_draft_deck: self.determinize_draft_deck.unwrap_or(defaults.determinize_draft_deck),
                        }),
                    }
                }
                #[cfg(not(feature = "nn-ai"))]
                {
                    panic!("nn-mcts algorithm requires the 'nn-ai' feature. Build with: cargo build --features nn-ai");
                }
            }
            _ => {
                let defaults = MctsConfig::default();
                NamedVariant {
                    name: self.name,
                    ai: AiConfig::Ismcts(MctsConfig {
                        iterations: self.iterations.unwrap_or(defaults.iterations),
                        exploration_constant: self.exploration_constant.unwrap_or(defaults.exploration_constant),
                        max_rollout_steps: self.max_rollout_steps.unwrap_or(defaults.max_rollout_steps),
                        determinize_draft_deck: self.determinize_draft_deck.unwrap_or(defaults.determinize_draft_deck),
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
    let mut glass = false;

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
                                ai: AiConfig::Ismcts(MctsConfig { iterations: iters, ..MctsConfig::default() }),
                            }
                        })
                        .collect(),
                );
            }
            "--variants-file" => {
                i += 1;
                variants_file = args[i].clone();
            }
            "--glass" => {
                glass = true;
                i += 1;
                continue;
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
        glass,
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

fn format_variant_label(variant: &NamedVariant, differing: &DifferingFields) -> String {
    if let Some(name) = &variant.name {
        return name.clone();
    }
    match &variant.ai {
        AiConfig::Ismcts(config) => {
            let mut parts = Vec::new();
            if differing.iterations_differs {
                parts.push(format_iterations(config.iterations));
            }
            if differing.exploration_constant_differs {
                parts.push(format!("c={:.2}", config.exploration_constant));
            }
            if differing.max_rollout_steps_differs {
                parts.push(format!("rollout={}", config.max_rollout_steps));
            }
            if parts.is_empty() {
                parts.push(format_iterations(config.iterations));
            }
            parts.join(", ")
        }
        #[cfg(feature = "nn-ai")]
        AiConfig::NnMcts(config) => {
            let model_name = std::path::Path::new(&config.model_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| config.model_path.clone());
            format!("nn-mcts ({})", model_name)
        }
    }
}

struct DifferingFields {
    iterations_differs: bool,
    exploration_constant_differs: bool,
    max_rollout_steps_differs: bool,
}

fn compute_differing_fields(variants: &[NamedVariant]) -> DifferingFields {
    if variants.len() <= 1 {
        return DifferingFields {
            iterations_differs: false,
            exploration_constant_differs: false,
            max_rollout_steps_differs: false,
        };
    }
    // Only compare ISMCTS variants among themselves
    let ismcts_configs: Vec<&MctsConfig> = variants.iter().filter_map(|v| {
        match &v.ai {
            AiConfig::Ismcts(c) => Some(c),
            #[cfg(feature = "nn-ai")]
            AiConfig::NnMcts(_) => None,
        }
    }).collect();

    if ismcts_configs.len() <= 1 {
        return DifferingFields {
            iterations_differs: false,
            exploration_constant_differs: false,
            max_rollout_steps_differs: false,
        };
    }
    let first = ismcts_configs[0];
    DifferingFields {
        iterations_differs: ismcts_configs.iter().any(|c| c.iterations != first.iterations),
        exploration_constant_differs: ismcts_configs.iter().any(|c| c.exploration_constant != first.exploration_constant),
        max_rollout_steps_differs: ismcts_configs.iter().any(|c| c.max_rollout_steps != first.max_rollout_steps),
    }
}

fn has_any_difference(variants: &[NamedVariant]) -> bool {
    if variants.len() <= 1 {
        return false;
    }
    if variants.iter().any(|v| v.name.is_some()) {
        return true;
    }
    // Different algorithm types means there's a difference
    let has_ismcts = variants.iter().any(|v| matches!(&v.ai, AiConfig::Ismcts(_)));
    #[cfg(feature = "nn-ai")]
    let has_nn = variants.iter().any(|v| matches!(&v.ai, AiConfig::NnMcts(_)));
    #[cfg(not(feature = "nn-ai"))]
    let has_nn = false;
    if has_ismcts && has_nn {
        return true;
    }
    let diff = compute_differing_fields(variants);
    diff.iterations_differs || diff.exploration_constant_differs || diff.max_rollout_steps_differs
}

fn variant_to_player_variant(variant: &NamedVariant) -> PlayerVariant {
    let defaults = MctsConfig::default();
    match &variant.ai {
        AiConfig::Ismcts(config) => PlayerVariant {
            name: variant.name.clone(),
            algorithm: Some("ucb".to_string()),
            iterations: config.iterations,
            exploration_constant: if config.exploration_constant != defaults.exploration_constant {
                Some(config.exploration_constant)
            } else {
                None
            },
            max_rollout_steps: if config.max_rollout_steps != defaults.max_rollout_steps {
                Some(config.max_rollout_steps)
            } else {
                None
            },
            model_path: None,
            c_puct: None,
        },
        #[cfg(feature = "nn-ai")]
        AiConfig::NnMcts(config) => PlayerVariant {
            name: variant.name.clone(),
            algorithm: Some("nn-mcts".to_string()),
            iterations: config.simulations,
            exploration_constant: None,
            max_rollout_steps: None,
            model_path: Some(config.model_path.clone()),
            c_puct: Some(config.c_puct as f64),
        },
    }
}

// ── Game loop ──

fn run_game(
    _game_index: usize,
    player_variants: &[NamedVariant],
    note: Option<String>,
    glass: bool,
    rng: &mut WyRand,
    #[cfg(feature = "nn-ai")] nn_models: &mut std::collections::HashMap<String, NnModel>,
) -> GameRunOutput {
    let start = Instant::now();
    let num_players = player_variants.len();

    // Shuffle variant assignment to eliminate position bias
    let mut shuffled_variants = player_variants.to_vec();
    shuffled_variants.shuffle(rng);

    let has_variants = has_any_difference(&shuffled_variants);
    let differing = compute_differing_fields(&shuffled_variants);
    let names: Vec<String> = (1..=num_players)
        .map(|i| {
            if has_variants {
                format!("Player {} ({})", i, format_variant_label(&shuffled_variants[i - 1], &differing))
            } else {
                format!("Player {}", i)
            }
        })
        .collect();

    let ai_players = vec![true; num_players];
    let expansions = Expansions { glass };
    let mut state = create_initial_game_state_with_expansions(num_players, &ai_players, expansions, rng);
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

        match &shuffled_variants[player_index].ai {
            AiConfig::Ismcts(config) => {
                let max_rollout_round = std::cmp::max(8, state.round + 2);
                let choice = ismcts(&state, player_index, config, &None, Some(max_rollout_round), rng);

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
            #[cfg(feature = "nn-ai")]
            AiConfig::NnMcts(config) => {
                let model = nn_models.get_mut(&config.model_path)
                    .expect("NnModel not loaded for model_path");

                // NN-MCTS returns one atomic choice at a time.
                // Keep calling until the current player's turn ends.
                loop {
                    let phase_str = match &state.phase {
                        GamePhase::Draft { .. } => "draft",
                        GamePhase::Action { .. } => "action",
                        _ => "other",
                    };
                    let choice = nn_mcts(&state, player_index, model, config, &None, rng);

                    let log_choice: Option<Choice> = match &choice {
                        AtomicChoice::DraftPick(card) => Some(Choice::DraftPick { card: *card }),
                        AtomicChoice::DestroyDraftedCard(card) => Some(Choice::DestroyDraftedCard { card: *card }),
                        AtomicChoice::EndTurn => Some(Choice::EndTurn),
                        AtomicChoice::SelectBuyer(buyer) => Some(Choice::SelectBuyer { buyer: *buyer }),
                        AtomicChoice::MixPair(a, b) => Some(Choice::MixAll { mixes: smallvec![(*a, *b)] }),
                        AtomicChoice::WorkshopCard(card) => Some(Choice::Workshop { card_types: smallvec![*card] }),
                        AtomicChoice::SkipWorkshop => Some(Choice::SkipWorkshop),
                        AtomicChoice::DestroyTarget(card) => Some(Choice::DestroyAndDestroyCards { card: *card, target: None }),
                        AtomicChoice::GainSecondary(color) => Some(Choice::GainSecondary { color: *color }),
                        AtomicChoice::GainPrimary(color) => Some(Choice::GainPrimary { color: *color }),
                        AtomicChoice::SwapTertiary { lose, gain } => Some(Choice::SwapTertiary { lose: *lose, gain: *gain }),
                        AtomicChoice::SkipMix | AtomicChoice::SkipDestroy | AtomicChoice::SkipSwap => None,
                    };

                    if let Some(choice) = log_choice {
                        seq += 1;
                        entries.push(StructuredLogEntry {
                            seq,
                            timestamp: now_epoch_millis(),
                            round: state.round,
                            phase: phase_str.to_string(),
                            player_index,
                            choice,
                        });
                    }

                    apply_atomic_choice(&mut state, &choice, rng);

                    // Check if the current player's turn has ended
                    match &state.phase {
                        GamePhase::GameOver | GamePhase::Draw => break,
                        GamePhase::Draft { draft_state } => {
                            if draft_state.current_player_index != player_index {
                                break;
                            }
                        }
                        GamePhase::Action { action_state } => {
                            if action_state.current_player_index != player_index {
                                break;
                            }
                        }
                    }
                }
            }
        }
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
        match &shuffled_variants[0].ai {
            AiConfig::Ismcts(config) => (Some(config.iterations), None),
            #[cfg(feature = "nn-ai")]
            AiConfig::NnMcts(config) => (Some(config.simulations), None),
        }
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
            match &player_variants[0].ai {
                AiConfig::Ismcts(c) => c.iterations,
                #[cfg(feature = "nn-ai")]
                AiConfig::NnMcts(c) => c.simulations,
            },
            args.threads
        );
    }

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    // Collect unique NN model paths that need loading
    #[cfg(feature = "nn-ai")]
    let nn_model_paths: Vec<String> = {
        let mut paths = Vec::new();
        for v in player_variants {
            if let AiConfig::NnMcts(config) = &v.ai {
                if !paths.contains(&config.model_path) {
                    paths.push(config.model_path.clone());
                }
            }
        }
        paths
    };

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

            #[cfg(feature = "nn-ai")]
            let nn_model_paths = &nn_model_paths;

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                // Each thread loads its own NnModel instances (Session::run needs &mut self)
                #[cfg(feature = "nn-ai")]
                let mut nn_models: std::collections::HashMap<String, NnModel> = {
                    let mut map = std::collections::HashMap::new();
                    for path in nn_model_paths {
                        let model = NnModel::load(std::path::Path::new(path))
                            .unwrap_or_else(|e| panic!("Failed to load ONNX model {}: {}", path, e));
                        map.insert(path.clone(), model);
                    }
                    map
                };

                for _i in 0..count {
                    let log = run_game(
                        0,
                        player_variants,
                        note.clone(),
                        glass,
                        &mut rng,
                        #[cfg(feature = "nn-ai")]
                        &mut nn_models,
                    );
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
