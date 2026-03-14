use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;
use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

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
    genetic: Option<GeneticArgs>,
}

struct GeneticArgs {
    population: usize,
    generations: usize,
    games_per_eval: usize,
    mutation_rate: f64,
    mutation_scale: f64,
    eval_iterations: u32,
    seed_params: Option<HeuristicParams>,
}

#[derive(Clone)]
struct NamedVariant {
    name: Option<String>,
    ai: MctsConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariantFileEntry {
    name: Option<String>,
    #[serde(default)]
    iterations: Option<u32>,
    #[serde(default)]
    exploration_constant: Option<f64>,
    #[serde(default)]
    max_rollout_steps: Option<u32>,
    #[serde(default)]
    use_heuristic_eval: Option<bool>,
    #[serde(default)]
    heuristic_params: Option<HeuristicParams>,
    #[serde(default)]
    heuristic_params_file: Option<String>,
}

impl VariantFileEntry {
    fn into_named_variant(self) -> NamedVariant {
        let defaults = MctsConfig::default();
        let heuristic_params = if let Some(params) = self.heuristic_params {
            params
        } else if let Some(path) = &self.heuristic_params_file {
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|_| panic!("Failed to read heuristic params file: {}", path));
            serde_json::from_str(&contents)
                .unwrap_or_else(|_| panic!("Failed to parse heuristic params file: {}", path))
        } else {
            HeuristicParams::default()
        };
        NamedVariant {
            name: self.name,
            ai: MctsConfig {
                iterations: self.iterations.unwrap_or(defaults.iterations),
                exploration_constant: self.exploration_constant.unwrap_or(defaults.exploration_constant),
                max_rollout_steps: self.max_rollout_steps.unwrap_or(defaults.max_rollout_steps),
                use_heuristic_eval: self.use_heuristic_eval.unwrap_or(defaults.use_heuristic_eval),
                heuristic_params,
            },
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

    let mut genetic = false;
    let mut population = 20usize;
    let mut generations = 50usize;
    let mut games_per_eval = 100usize;
    let mut mutation_rate = 0.15f64;
    let mut mutation_scale = 0.25f64;
    let mut eval_iterations = 10_000u32;
    let mut seed_params_file: Option<String> = None;

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
                                ai: MctsConfig { iterations: iters, ..MctsConfig::default() },
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
            "--genetic" => {
                genetic = true;
                i += 1;
                continue;
            }
            "--population" => {
                i += 1;
                population = args[i].parse().expect("Invalid --population value");
            }
            "--generations" => {
                i += 1;
                generations = args[i].parse().expect("Invalid --generations value");
            }
            "--games-per-eval" => {
                i += 1;
                games_per_eval = args[i].parse().expect("Invalid --games-per-eval value");
            }
            "--mutation-rate" => {
                i += 1;
                mutation_rate = args[i].parse().expect("Invalid --mutation-rate value");
            }
            "--mutation-scale" => {
                i += 1;
                mutation_scale = args[i].parse().expect("Invalid --mutation-scale value");
            }
            "--eval-iterations" => {
                i += 1;
                eval_iterations = args[i].parse().expect("Invalid --eval-iterations value");
            }
            "--seed-params" => {
                i += 1;
                seed_params_file = Some(args[i].clone());
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let genetic_args = if genetic {
        // In genetic mode, default output to "genetic-algorithm"
        if output == "game-logs" {
            output = "genetic-algorithm".to_string();
        }
        let seed_params = seed_params_file.map(|path| {
            let contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read seed params file: {}", path));
            serde_json::from_str::<HeuristicParams>(&contents)
                .unwrap_or_else(|_| panic!("Failed to parse seed params file: {}", path))
        });
        Some(GeneticArgs {
            population,
            generations,
            games_per_eval,
            mutation_rate,
            mutation_scale,
            eval_iterations,
            seed_params,
        })
    } else {
        None
    };

    let variants = variants.unwrap_or_else(|| {
        if genetic {
            // In genetic mode, variants file is not required
            vec![NamedVariant { name: None, ai: MctsConfig::default() }; 2]
        } else {
            let contents = std::fs::read_to_string(&variants_file)
                .unwrap_or_else(|_| panic!("Failed to read variants file: {}", variants_file));
            let entries: Vec<VariantFileEntry> = serde_json::from_str(&contents)
                .unwrap_or_else(|_| panic!("Failed to parse variants file: {}", variants_file));
            entries
                .into_iter()
                .map(|e| e.into_named_variant())
                .collect()
        }
    });

    Args {
        games,
        threads,
        output,
        note,
        variants,
        glass,
        genetic: genetic_args,
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
    let config = &variant.ai;
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
    let first = &variants[0].ai;
    DifferingFields {
        iterations_differs: variants.iter().any(|v| v.ai.iterations != first.iterations),
        exploration_constant_differs: variants.iter().any(|v| v.ai.exploration_constant != first.exploration_constant),
        max_rollout_steps_differs: variants.iter().any(|v| v.ai.max_rollout_steps != first.max_rollout_steps),
    }
}

fn has_any_difference(variants: &[NamedVariant]) -> bool {
    if variants.len() <= 1 {
        return false;
    }
    if variants.iter().any(|v| v.name.is_some()) {
        return true;
    }
    let diff = compute_differing_fields(variants);
    diff.iterations_differs || diff.exploration_constant_differs || diff.max_rollout_steps_differs
}

fn is_default_heuristic_params(params: &HeuristicParams) -> bool {
    let d = HeuristicParams::default();
    let json_params = serde_json::to_string(params).unwrap_or_default();
    let json_default = serde_json::to_string(&d).unwrap_or_default();
    json_params == json_default
}

fn variant_to_player_variant(variant: &NamedVariant) -> PlayerVariant {
    let defaults = MctsConfig::default();
    let config = &variant.ai;
    PlayerVariant {
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
        heuristic_params: if !is_default_heuristic_params(&config.heuristic_params) {
            Some(config.heuristic_params.clone())
        } else {
            None
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

        let config = &shuffled_variants[player_index].ai;
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
                completed_sell_cards: p.completed_sell_cards.len() as u32,
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
                completed_sell_cards: p.completed_sell_cards.to_vec(),
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
        (Some(shuffled_variants[0].ai.iterations), None)
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

// ── Genetic Algorithm ──

/// Box-Muller transform: generate a sample from N(0, std_dev)
fn sample_normal(rng: &mut WyRand, std_dev: f64) -> f64 {
    let u1: f64 = rng.random::<f64>();
    let u2: f64 = rng.random::<f64>();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    z * std_dev
}

fn heuristic_params_to_vec(params: &HeuristicParams) -> Vec<f64> {
    vec![
        params.primary_pip_weight,                                          // 0
        params.secondary_pip_weight,                                        // 1
        params.tertiary_pip_weight,                                         // 2
        params.stored_material_weight,                                      // 3
        params.chalk_quality,                                               // 4
        params.alum_quality.unwrap_or(params.action_quality),               // 5
        params.cream_of_tartar_quality.unwrap_or(params.action_quality),    // 6
        params.gum_arabic_quality.unwrap_or(params.action_quality),         // 7
        params.potash_quality.unwrap_or(params.action_quality),             // 8
        params.vinegar_quality.unwrap_or(params.action_quality),            // 9
        params.argol_quality.unwrap_or(params.action_quality),              // 10
        params.pure_primary_dye_quality.unwrap_or(params.dye_quality),      // 11
        params.primary_dye_quality.unwrap_or(params.dye_quality),           // 12
        params.secondary_dye_quality.unwrap_or(params.dye_quality),         // 13
        params.tertiary_dye_quality.unwrap_or(params.dye_quality),          // 14
        params.basic_dye_quality,                                           // 15
        params.starter_material_quality,                                    // 16
        params.draft_material_quality,                                      // 17
        params.dual_material_quality,                                       // 18
        params.buyer_material_weight,                                       // 19
        params.buyer_color_weight,                                          // 20
        params.glass_weight,                                                // 21
        params.primary_color_coverage_weight,                               // 22
        params.secondary_color_coverage_weight,                             // 23
        params.cards_in_deck_weight,                                        // 24
        params.cards_in_deck_squared_weight,                                // 25
        params.material_type_count_weight,                                  // 26
        params.material_coverage_weight,                                    // 27
        params.heuristic_score_threshold.unwrap_or(
            params.heuristic_round_threshold as f64),                       // 28
        params.heuristic_lookahead as f64,                                  // 29
    ]
}

fn vec_to_heuristic_params(v: &[f64]) -> HeuristicParams {
    let defaults = HeuristicParams::default();
    HeuristicParams {
        primary_pip_weight: v[0],
        secondary_pip_weight: v[1],
        tertiary_pip_weight: v[2],
        stored_material_weight: v[3],
        chalk_quality: v[4],
        action_quality: defaults.action_quality,
        dye_quality: defaults.dye_quality,
        basic_dye_quality: v[15],
        starter_material_quality: v[16],
        draft_material_quality: v[17],
        dual_material_quality: v[18],
        buyer_material_weight: v[19],
        buyer_color_weight: v[20],
        glass_weight: v[21],
        heuristic_round_threshold: defaults.heuristic_round_threshold,
        heuristic_lookahead: (v[29].round() as u32).max(1),
        alum_quality: Some(v[5]),
        cream_of_tartar_quality: Some(v[6]),
        gum_arabic_quality: Some(v[7]),
        potash_quality: Some(v[8]),
        vinegar_quality: Some(v[9]),
        argol_quality: Some(v[10]),
        pure_primary_dye_quality: Some(v[11]),
        primary_dye_quality: Some(v[12]),
        secondary_dye_quality: Some(v[13]),
        tertiary_dye_quality: Some(v[14]),
        primary_color_coverage_weight: v[22],
        secondary_color_coverage_weight: v[23],
        cards_in_deck_weight: v[24],
        cards_in_deck_squared_weight: v[25],
        material_type_count_weight: v[26],
        material_coverage_weight: v[27],
        heuristic_score_threshold: Some(v[28]),
    }
}

fn run_ga_game(
    params_a: &HeuristicParams,
    params_b: &HeuristicParams,
    eval_iterations: u32,
    glass: bool,
    rng: &mut WyRand,
) -> (f64, f64) {
    let num_players = 2;
    let ai_players = vec![true; num_players];
    let expansions = Expansions { glass };
    let mut state = create_initial_game_state_with_expansions(num_players, &ai_players, expansions, rng);

    let configs = [
        MctsConfig {
            iterations: eval_iterations,
            use_heuristic_eval: true,
            heuristic_params: params_a.clone(),
            ..MctsConfig::default()
        },
        MctsConfig {
            iterations: eval_iterations,
            use_heuristic_eval: true,
            heuristic_params: params_b.clone(),
            ..MctsConfig::default()
        },
    ];

    execute_draw_phase(&mut state, rng);

    while !matches!(state.phase, GamePhase::GameOver) {
        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => {
                break;
            }
            GamePhase::GameOver => break,
        };

        let config = &configs[player_index];
        let max_rollout_round = std::cmp::max(8, state.round + 2);
        let choice = ismcts(&state, player_index, config, &None, Some(max_rollout_round), rng);
        apply_choice_to_state(&mut state, &choice, rng);
    }

    let score_a = calculate_score(&state.players[0]);
    let score_b = calculate_score(&state.players[1]);
    if score_a > score_b {
        (1.0, 0.0)
    } else if score_b > score_a {
        (0.0, 1.0)
    } else {
        (0.5, 0.5)
    }
}

fn run_genetic_algorithm(args: &Args, ga: &GeneticArgs) {
    let batch_id = generate_batch_id();
    let num_genes = 30;

    eprintln!(
        "Genetic Algorithm: population={}, generations={}, games_per_eval={}, eval_iterations={}, threads={}",
        ga.population, ga.generations, ga.games_per_eval, ga.eval_iterations, args.threads
    );

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    // Initialize population
    let mut population: Vec<Vec<f64>> = Vec::with_capacity(ga.population);
    let default_params = HeuristicParams::default();
    let seed = ga.seed_params.as_ref().unwrap_or(&default_params);
    let seed_genes = heuristic_params_to_vec(seed);

    if ga.seed_params.is_some() {
        eprintln!("Seeding population from provided params file");
    }

    // First individual is the seed (or default)
    population.push(seed_genes.clone());

    // glass_weight index — skip when glass expansion is disabled
    const GLASS_WEIGHT_IDX: usize = 21;
    let glass = args.glass;

    // Rest are randomly perturbed from seed
    for _ in 1..ga.population {
        let mut genes = seed_genes.clone();
        for (i, g) in genes.iter_mut().enumerate() {
            if i == GLASS_WEIGHT_IDX && !glass {
                continue;
            }
            let factor = 0.5 + rng.random::<f64>() * 1.5; // [0.5, 2.0)
            *g *= factor;
        }
        // Clamp u32 fields
        genes[29] = (genes[29].round()).max(1.0);
        population.push(genes);
    }

    let baseline_params = HeuristicParams::default();

    for gen in 0..ga.generations {
        let gen_start = Instant::now();
        let pop_size = population.len();

        // Evaluate each individual against the baseline (default params)
        let wins: Vec<std::sync::atomic::AtomicU64> = (0..pop_size)
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();

        let population_params: Vec<HeuristicParams> = population
            .iter()
            .map(|g| vec_to_heuristic_params(g))
            .collect();

        let baseline_ref = &baseline_params;
        let eval_iterations = ga.eval_iterations;
        let games_per_eval = ga.games_per_eval;
        let glass = args.glass;

        // Evaluate one individual at a time, all threads cooperating on its games
        let num_threads = args.threads;

        for i in 0..pop_size {
            let params = &population_params[i];
            let wins_for_individual = std::sync::atomic::AtomicU64::new(0);
            let wins_ind_ref = &wins_for_individual;

            std::thread::scope(|s| {
                let games_per_thread = games_per_eval / num_threads;
                let remainder = games_per_eval % num_threads;
                let mut handles = Vec::new();

                for t in 0..num_threads {
                    let count = games_per_thread + if t < remainder { 1 } else { 0 };

                    handles.push(s.spawn(move || {
                        let mut rng = WyRand::from_rng(&mut rand::rng());
                        let mut thread_wins = 0.0f64;

                        for _ in 0..count {
                            let (w, _) = run_ga_game(params, baseline_ref, eval_iterations, glass, &mut rng);
                            thread_wins += w;
                        }

                        wins_ind_ref.fetch_add((thread_wins * 1000.0) as u64, Ordering::Relaxed);
                    }));
                }

                for h in handles {
                    h.join().unwrap();
                }
            });

            let total_wins = wins_for_individual.load(Ordering::Relaxed) as f64 / 1000.0;
            wins[i].store((total_wins * 1000.0) as u64, Ordering::Relaxed);
            let wr = total_wins / games_per_eval as f64;
            eprintln!(
                "  Gen {} [{}/{}] individual {}: win_rate={:.4}",
                gen + 1, i + 1, pop_size, i, wr
            );
        }

        // Compute fitness (win rate vs baseline)
        let mut fitness: Vec<(usize, f64)> = (0..pop_size)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                let wr = w / games_per_eval as f64;
                (i, wr)
            })
            .collect();

        fitness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_idx = fitness[0].0;
        let best_fitness = fitness[0].1;
        let best_params = vec_to_heuristic_params(&population[best_idx]);

        // Save best individual
        let output_path = format!("{}/batch-{}-gen-{}.json", args.output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let elapsed = gen_start.elapsed();
        eprintln!(
            "Gen {}/{}: best_fitness={:.4}, worst_fitness={:.4}, elapsed={:.1}s, saved {}",
            gen + 1,
            ga.generations,
            best_fitness,
            fitness.last().unwrap().1,
            elapsed.as_secs_f64(),
            output_path,
        );

        // If this is the last generation, we're done
        if gen + 1 >= ga.generations {
            break;
        }

        // Selection, crossover, mutation to create next generation
        let mut new_population: Vec<Vec<f64>> = Vec::with_capacity(ga.population);

        // Elitism: top 2 survive
        new_population.push(population[fitness[0].0].clone());
        new_population.push(population[fitness[1].0].clone());

        while new_population.len() < ga.population {
            // Tournament selection (size 3) for two parents
            let parent_a = tournament_select(&fitness, 3, &mut rng);
            let parent_b = tournament_select(&fitness, 3, &mut rng);

            // Uniform crossover
            let mut child = Vec::with_capacity(num_genes);
            for g in 0..num_genes {
                if rng.random_bool(0.5) {
                    child.push(population[parent_a][g]);
                } else {
                    child.push(population[parent_b][g]);
                }
            }

            // Mutation
            for (i, g) in child.iter_mut().enumerate() {
                if i == GLASS_WEIGHT_IDX && !glass {
                    continue;
                }
                if rng.random::<f64>() < ga.mutation_rate {
                    let perturbation = sample_normal(&mut rng, ga.mutation_scale);
                    *g *= 1.0 + perturbation;
                }
            }

            // Clamp u32 fields
            child[29] = (child[29].round()).max(1.0);

            new_population.push(child);
        }

        population = new_population;
    }

    eprintln!("Genetic algorithm complete. Results in {}/", args.output);
}

fn tournament_select(
    fitness: &[(usize, f64)],
    tournament_size: usize,
    rng: &mut WyRand,
) -> usize {
    let mut best_idx = rng.random_range(0..fitness.len());
    let mut best_fitness = fitness[best_idx].1;

    for _ in 1..tournament_size {
        let idx = rng.random_range(0..fitness.len());
        if fitness[idx].1 > best_fitness {
            best_fitness = fitness[idx].1;
            best_idx = idx;
        }
    }

    fitness[best_idx].0
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

    if let Some(ref ga) = args.genetic {
        run_genetic_algorithm(&args, ga);
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
