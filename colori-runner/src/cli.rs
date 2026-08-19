use clap::{Parser, Subcommand};
use colori_core::ismcts::MctsConfig;
use colori_core::scoring::HeuristicParams;
use serde::Deserialize;

// ── Top-level CLI ──

#[derive(Parser)]
#[command(name = "colori-runner", about = "Colori game simulation and AI training tool")]
pub struct Cli {
    /// Number of threads to use
    #[arg(long, default_value_t = 10, global = true)]
    pub threads: usize,

    /// Output directory
    #[arg(long, global = true)]
    pub output: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run batch game simulations
    Simulate(SimulateArgs),
    /// Run a round-robin tournament between variants
    Tournament(TournamentArgs),
    /// Train heuristic params using genetic algorithm
    Train(TrainArgs),
}

// ── Subcommand args ──

#[derive(Parser)]
pub struct SimulateArgs {
    /// Number of games to simulate
    #[arg(long, default_value_t = 10_000)]
    pub games: usize,

    /// Optional note to include in game logs
    #[arg(long)]
    pub note: Option<String>,

    /// Comma-separated MCTS iteration counts for quick variant comparison
    #[arg(long)]
    pub variants: Option<String>,

    /// Path to heuristic params JSON file (required when using --variants)
    #[arg(long)]
    pub heuristic_params_file: Option<String>,

    /// Path to variants JSON file
    #[arg(long, default_value = "variants.json")]
    pub variants_file: String,

    /// Max rounds for solo mode (1 variant = solo; ignored for multiplayer)
    #[arg(long, default_value_t = 5)]
    pub max_rounds: u32,
}

#[derive(Parser)]
pub struct TournamentArgs {
    /// Number of games to simulate
    #[arg(long, default_value_t = 10_000)]
    pub games: usize,

    /// Optional note to include in game logs
    #[arg(long)]
    pub note: Option<String>,

    /// Path to variants JSON file
    #[arg(long, default_value = "variants.json")]
    pub variants_file: String,
}

#[derive(Parser)]
pub struct TrainArgs {
    /// Population size
    #[arg(long, default_value_t = 20)]
    pub population: usize,

    /// Number of generations
    #[arg(long, default_value_t = 50)]
    pub generations: usize,

    /// Games per fitness evaluation
    #[arg(long, default_value_t = 400)]
    pub games_per_eval: usize,

    /// Mutation rate (probability of mutating each gene)
    #[arg(long, default_value_t = 0.15)]
    pub mutation_rate: f64,

    /// Mutation scale (std dev of Gaussian perturbation)
    #[arg(long, default_value_t = 0.25)]
    pub mutation_scale: f64,

    /// MCTS iterations for evaluation games
    #[arg(long, default_value_t = 4000)]
    pub eval_iterations: u32,

    /// Path to seed heuristic params JSON file
    #[arg(long)]
    pub seed_params: Option<String>,

    /// Path to baseline heuristic params JSON file
    #[arg(long)]
    pub baseline_params: Option<String>,
}

// ── Variant types ──

#[derive(Clone)]
pub struct NamedVariant {
    pub name: Option<String>,
    pub ai: MctsConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    progressive_bias_weight: Option<f64>,
    #[serde(default)]
    heuristic_params: Option<HeuristicParams>,
    #[serde(default)]
    heuristic_params_file: Option<String>,
    #[serde(default)]
    no_rollout: Option<bool>,
    #[serde(default)]
    heuristic_rollout: Option<bool>,
    #[serde(default)]
    heuristic_draft: Option<bool>,
    #[serde(default)]
    early_termination: Option<bool>,
    #[serde(default)]
    time_limit_ms: Option<u64>,
    #[serde(default)]
    random_first_pick: Option<bool>,
    #[serde(default)]
    force_max_workshop: Option<bool>,
}

impl VariantFileEntry {
    fn into_named_variant(self) -> NamedVariant {
        let heuristic_params = if let Some(params) = self.heuristic_params {
            params
        } else if let Some(path) = &self.heuristic_params_file {
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|_| panic!("Failed to read heuristic params file: {}", path));
            serde_json::from_str(&contents)
                .unwrap_or_else(|e| panic!("Failed to parse heuristic params file: {}: {}", path, e))
        } else {
            panic!("Variant must specify heuristicParams or heuristicParamsFile");
        };
        let base = MctsConfig::new(heuristic_params);
        NamedVariant {
            name: self.name,
            ai: MctsConfig {
                iterations: self.iterations.unwrap_or(base.iterations),
                exploration_constant: self.exploration_constant.unwrap_or(base.exploration_constant),
                max_rollout_steps: self.max_rollout_steps.unwrap_or(base.max_rollout_steps),
                use_heuristic_eval: self.use_heuristic_eval.unwrap_or(base.use_heuristic_eval),
                progressive_bias_weight: self.progressive_bias_weight.unwrap_or(base.progressive_bias_weight),
                heuristic_params: base.heuristic_params,
                no_rollout: self.no_rollout.unwrap_or(base.no_rollout),
                heuristic_rollout: self.heuristic_rollout.unwrap_or(base.heuristic_rollout),
                heuristic_draft: self.heuristic_draft.unwrap_or(base.heuristic_draft),
                early_termination: self.early_termination.unwrap_or(base.early_termination),
                time_limit_ms: self.time_limit_ms,
                random_first_pick: self.random_first_pick.unwrap_or(base.random_first_pick),
                force_max_workshop: self.force_max_workshop.unwrap_or(base.force_max_workshop),
            },
        }
    }
}

// ── Variant loading helpers ──

pub fn load_variants_from_file(variants_file: &str) -> Vec<NamedVariant> {
    let contents = std::fs::read_to_string(variants_file)
        .unwrap_or_else(|_| panic!("Failed to read variants file: {}", variants_file));
    let entries: Vec<VariantFileEntry> = serde_json::from_str(&contents)
        .unwrap_or_else(|_| panic!("Failed to parse variants file: {}", variants_file));
    entries
        .into_iter()
        .map(|e| e.into_named_variant())
        .collect()
}

pub fn parse_inline_variants(variants_str: &str, heuristic_params: HeuristicParams) -> Vec<NamedVariant> {
    variants_str
        .split(',')
        .map(|s| {
            let iters: u32 = s.trim().parse().expect("Invalid --variants value");
            NamedVariant {
                name: None,
                ai: MctsConfig { iterations: iters, ..MctsConfig::new(heuristic_params.clone()) },
            }
        })
        .collect()
}

pub fn load_heuristic_params(path: &str) -> HeuristicParams {
    let contents = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read heuristic params file: {}", path));
    serde_json::from_str::<HeuristicParams>(&contents)
        .unwrap_or_else(|_| panic!("Failed to parse heuristic params file: {}", path))
}
