use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;
use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use nalgebra::{DMatrix, DVector};
use nalgebra::linalg::SymmetricEigen;
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
    initial_sigma: f64,
    elitism: usize,
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
    let mut population = 14usize;
    let mut generations = 50usize;
    let mut games_per_eval = 100usize;
    let mut initial_sigma = 0.3f64;
    let mut elitism = 2usize;
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
            "--initial-sigma" => {
                i += 1;
                initial_sigma = args[i].parse().expect("Invalid --initial-sigma value");
            }
            "--elitism" => {
                i += 1;
                elitism = args[i].parse().expect("Invalid --elitism value");
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
            initial_sigma,
            elitism,
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

// ── CMA-ES ──

struct CmaEsState {
    n: usize,
    lambda: usize,
    mu: usize,
    weights: Vec<f64>,
    mu_eff: f64,
    c_c: f64,
    c_sigma: f64,
    c_1: f64,
    c_mu: f64,
    d_sigma: f64,
    chi_n: f64,
    mean: DVector<f64>,
    sigma: f64,
    c_mat: DMatrix<f64>,
    p_c: DVector<f64>,
    p_sigma: DVector<f64>,
    b_mat: DMatrix<f64>,
    d_vec: DVector<f64>,
    inv_sqrt_c: DMatrix<f64>,
    frozen_genes: Vec<usize>,
}

impl CmaEsState {
    fn new(seed_genes: &[f64], lambda: usize, initial_sigma: f64, frozen_genes: Vec<usize>) -> Self {
        let n = seed_genes.len();
        let mu = lambda / 2;

        // Compute recombination weights (log-linear)
        let mut weights: Vec<f64> = (0..mu)
            .map(|i| (mu as f64 + 0.5).ln() - ((i + 1) as f64).ln())
            .collect();
        let w_sum: f64 = weights.iter().sum();
        for w in weights.iter_mut() {
            *w /= w_sum;
        }

        let mu_eff: f64 = 1.0 / weights.iter().map(|w| w * w).sum::<f64>();

        // Adaptation rates (standard Hansen formulas)
        let c_c = (4.0 + mu_eff / n as f64) / (n as f64 + 4.0 + 2.0 * mu_eff / n as f64);
        let c_sigma = (mu_eff + 2.0) / (n as f64 + mu_eff + 5.0);
        let c_1 = 2.0 / ((n as f64 + 1.3).powi(2) + mu_eff);
        let c_mu_raw = 2.0 * (mu_eff - 2.0 + 1.0 / mu_eff)
            / ((n as f64 + 2.0).powi(2) + mu_eff);
        let c_mu = c_mu_raw.min(1.0 - c_1);
        let d_sigma = 1.0 + 2.0 * (0.0f64).max(((mu_eff - 1.0) / (n as f64 + 1.0)).sqrt() - 1.0) + c_sigma;
        let chi_n = (n as f64).sqrt() * (1.0 - 1.0 / (4.0 * n as f64) + 1.0 / (21.0 * (n as f64).powi(2)));

        let mean = DVector::from_column_slice(seed_genes);

        // Initial C is diagonal with C_ii = max(|seed_i|, 0.1)^2
        let diag: Vec<f64> = seed_genes
            .iter()
            .map(|&v| {
                let s = v.abs().max(0.1);
                s * s
            })
            .collect();
        let c_mat = DMatrix::from_diagonal(&DVector::from_column_slice(&diag));

        let p_c = DVector::zeros(n);
        let p_sigma = DVector::zeros(n);

        // Initial eigendecomposition
        let eigen = SymmetricEigen::new(c_mat.clone());
        let b_mat = eigen.eigenvectors.clone();
        let d_vec = eigen.eigenvalues.map(|v| v.max(1e-20).sqrt());
        let inv_d = d_vec.map(|v| 1.0 / v);
        let inv_sqrt_c = &b_mat * DMatrix::from_diagonal(&inv_d) * b_mat.transpose();

        CmaEsState {
            n,
            lambda,
            mu,
            weights,
            mu_eff,
            c_c,
            c_sigma,
            c_1,
            c_mu,
            d_sigma,
            chi_n,
            mean,
            sigma: initial_sigma,
            c_mat,
            p_c,
            p_sigma,
            b_mat,
            d_vec,
            inv_sqrt_c,
            frozen_genes,
        }
    }

    fn sample_offspring(&self, rng: &mut WyRand) -> Vec<Vec<f64>> {
        let mut offspring = Vec::with_capacity(self.lambda);
        for _ in 0..self.lambda {
            // z ~ N(0, I)
            let z: DVector<f64> = DVector::from_fn(self.n, |_, _| sample_normal(rng, 1.0));
            // x = mean + sigma * B * D * z
            let scaled = &self.b_mat * DMatrix::from_diagonal(&self.d_vec) * &z;
            let x = &self.mean + self.sigma * scaled;
            let mut genes: Vec<f64> = x.as_slice().to_vec();

            // Freeze frozen genes
            for &idx in &self.frozen_genes {
                genes[idx] = self.mean[idx];
            }

            // Clamp heuristic_lookahead >= 1
            genes[29] = genes[29].round().max(1.0);

            offspring.push(genes);
        }
        offspring
    }

    fn update(&mut self, offspring: &[Vec<f64>], fitnesses: &[f64], generation: usize) {
        assert_eq!(offspring.len(), self.lambda);
        assert_eq!(fitnesses.len(), self.lambda);

        // Sort offspring by fitness (descending — higher is better)
        let mut indices: Vec<usize> = (0..self.lambda).collect();
        indices.sort_by(|&a, &b| fitnesses[b].partial_cmp(&fitnesses[a]).unwrap());

        // Compute weighted mean of top mu offspring
        let old_mean = self.mean.clone();
        let mut new_mean = DVector::zeros(self.n);
        for i in 0..self.mu {
            let x = DVector::from_column_slice(&offspring[indices[i]]);
            new_mean += self.weights[i] * &x;
        }
        self.mean = new_mean;

        // Update sigma evolution path p_sigma
        let mean_diff = (&self.mean - &old_mean) / self.sigma;
        let invsqrt_times_diff = &self.inv_sqrt_c * &mean_diff;
        let csn = (self.c_sigma * (2.0 - self.c_sigma) * self.mu_eff).sqrt();
        self.p_sigma = (1.0 - self.c_sigma) * &self.p_sigma + csn * &invsqrt_times_diff;

        // Heaviside function h_sigma
        let ps_norm = self.p_sigma.norm();
        let threshold = (1.0 - (1.0 - self.c_sigma).powi(2 * (generation as i32 + 1))).sqrt()
            * (1.4 + 2.0 / (self.n as f64 + 1.0))
            * self.chi_n;
        let h_sigma: f64 = if ps_norm < threshold { 1.0 } else { 0.0 };

        // Update C evolution path p_c
        let ccn = (self.c_c * (2.0 - self.c_c) * self.mu_eff).sqrt();
        self.p_c = (1.0 - self.c_c) * &self.p_c + h_sigma * ccn * &mean_diff;

        // Rank-1 + rank-mu update of C
        let delta_h = (1.0 - h_sigma) * self.c_c * (2.0 - self.c_c);
        let old_c = (1.0 - self.c_1 - self.c_mu + self.c_1 * delta_h) * &self.c_mat;
        let rank1 = self.c_1 * (&self.p_c * self.p_c.transpose());
        let mut rank_mu = DMatrix::zeros(self.n, self.n);
        for i in 0..self.mu {
            let yi = (DVector::from_column_slice(&offspring[indices[i]]) - &old_mean) / self.sigma;
            rank_mu += self.weights[i] * (&yi * yi.transpose());
        }
        self.c_mat = old_c + rank1 + self.c_mu * rank_mu;

        // Enforce symmetry
        self.c_mat = (&self.c_mat + self.c_mat.transpose()) * 0.5;

        // Update sigma via CSA
        self.sigma *= ((self.c_sigma / self.d_sigma) * (ps_norm / self.chi_n - 1.0)).exp();
        self.sigma = self.sigma.max(1e-20);

        // Eigendecompose C and cache B, D, C^{-1/2}
        let eigen = SymmetricEigen::new(self.c_mat.clone());
        self.b_mat = eigen.eigenvectors.clone();
        self.d_vec = eigen.eigenvalues.map(|v| v.max(1e-20).sqrt());
        let inv_d = self.d_vec.map(|v| 1.0 / v);
        self.inv_sqrt_c = &self.b_mat * DMatrix::from_diagonal(&inv_d) * self.b_mat.transpose();
    }
}

fn run_genetic_algorithm(args: &Args, ga: &GeneticArgs) {
    let batch_id = generate_batch_id();

    eprintln!(
        "CMA-ES: lambda={}, generations={}, games_per_eval={}, eval_iterations={}, initial_sigma={}, elitism={}, threads={}",
        ga.population, ga.generations, ga.games_per_eval, ga.eval_iterations, ga.initial_sigma, ga.elitism, args.threads
    );

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let default_params = HeuristicParams::default();
    let seed = ga.seed_params.as_ref().unwrap_or(&default_params);
    let seed_genes = heuristic_params_to_vec(seed);

    if ga.seed_params.is_some() {
        eprintln!("Seeding CMA-ES from provided params file");
    }

    // glass_weight index — freeze when glass expansion is disabled
    const GLASS_WEIGHT_IDX: usize = 21;
    let frozen_genes: Vec<usize> = if !args.glass {
        vec![GLASS_WEIGHT_IDX]
    } else {
        vec![]
    };

    let mut cma = CmaEsState::new(&seed_genes, ga.population, ga.initial_sigma, frozen_genes);
    let baseline_params = seed.clone();

    let mut elites: Vec<(Vec<f64>, f64)> = Vec::new(); // (genes, fitness) — not used for CMA-ES updates

    for gen in 0..ga.generations {
        let gen_start = Instant::now();

        // Sample lambda offspring from CMA-ES distribution
        let offspring = cma.sample_offspring(&mut rng);

        // Build eval pool: offspring + elites
        let mut eval_pool: Vec<Vec<f64>> = offspring.clone();
        for (elite_genes, _) in &elites {
            eval_pool.push(elite_genes.clone());
        }
        let pool_size = eval_pool.len();

        // Evaluate all individuals against baseline
        let eval_params: Vec<HeuristicParams> = eval_pool
            .iter()
            .map(|g| vec_to_heuristic_params(g))
            .collect();

        let baseline_ref = &baseline_params;
        let eval_iterations = ga.eval_iterations;
        let games_per_eval = ga.games_per_eval;
        let glass = args.glass;
        let num_threads = args.threads;

        let wins: Vec<std::sync::atomic::AtomicU64> = (0..pool_size)
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();

        for i in 0..pool_size {
            let params = &eval_params[i];
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
            let label = if i < ga.population { "offspring" } else { "elite" };
            eprintln!(
                "  Gen {} [{}/{}] {} {}: win_rate={:.4}",
                gen + 1, i + 1, pool_size, label, i, wr
            );
        }

        // Compute fitness for all individuals
        let mut fitness: Vec<(usize, f64)> = (0..pool_size)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                let wr = w / games_per_eval as f64;
                (i, wr)
            })
            .collect();
        fitness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_idx = fitness[0].0;
        let best_fitness = fitness[0].1;
        let best_params = vec_to_heuristic_params(&eval_pool[best_idx]);

        // Save best individual
        let output_path = format!("{}/batch-{}-gen-{}.json", args.output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let avg_fitness = fitness.iter().map(|(_, wr)| wr).sum::<f64>() / pool_size as f64;

        // Compute average pairwise gene distance as a diversity measure
        let mut total_dist = 0.0;
        let mut pairs = 0u64;
        for a in 0..pool_size {
            for b in (a + 1)..pool_size {
                let dist: f64 = eval_pool[a]
                    .iter()
                    .zip(eval_pool[b].iter())
                    .map(|(ga, gb)| {
                        let scale = ga.abs().max(gb.abs()).max(0.1);
                        ((ga - gb) / scale).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                total_dist += dist;
                pairs += 1;
            }
        }
        let avg_diversity = if pairs > 0 { total_dist / pairs as f64 } else { 0.0 };

        let elapsed = gen_start.elapsed();
        eprintln!(
            "Gen {}/{}: best={:.4}, avg={:.4}, worst={:.4}, diversity={:.3}, sigma={:.6}, elapsed={:.1}s, saved {}",
            gen + 1,
            ga.generations,
            best_fitness,
            avg_fitness,
            fitness.last().unwrap().1,
            avg_diversity,
            cma.sigma,
            elapsed.as_secs_f64(),
            output_path,
        );

        // Update elites: top `elitism` from full pool
        elites.clear();
        for i in 0..ga.elitism.min(fitness.len()) {
            let idx = fitness[i].0;
            elites.push((eval_pool[idx].clone(), fitness[i].1));
        }

        // Update CMA-ES using only offspring fitnesses (not elites)
        let offspring_fitnesses: Vec<f64> = (0..ga.population)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                w / games_per_eval as f64
            })
            .collect();
        cma.update(&offspring, &offspring_fitnesses, gen);
    }

    eprintln!("CMA-ES optimization complete. Results in {}/", args.output);
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
