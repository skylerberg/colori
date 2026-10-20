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
    /// Play games with a deterministic rules policy
    PolicyPlay(PolicyPlayArgs),
    /// Tune a rules policy's weights with a cross-entropy method
    PolicyEvolve(PolicyEvolveArgs),
    /// Measure what each gene is worth, so the genome can be cut down
    PolicyPrune(PolicyPruneArgs),
    /// Dump decisions and position quantities, to see whether a tree could carry it
    PolicyDistill(PolicyDistillArgs),
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
    #[serde(default)]
    transpositions: Option<bool>,
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
                transpositions: self.transpositions.unwrap_or(base.transpositions),
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

// ── Deterministic policy ──

#[derive(Parser)]
pub struct PolicyPlayArgs {
    /// Genome to play: `starter` for the built-in rules, or a path to a JSON file
    #[arg(long, default_value = "starter")]
    pub genome: String,

    /// Frozen opponents for the other seats, comma separated: a path to a
    /// genome file, or `none` for a mirror match every seat the candidate.
    ///
    /// With one rung the candidate plays a straight match. With several the
    /// table varies while the ladder itself never moves, so a candidate's
    /// score still means the same thing across runs.
    #[arg(long, value_delimiter = ',', default_value = "starter")]
    pub anchors: Vec<String>,

    /// Heuristic params file the card-quality rules read. Defaults to the
    /// baked-in batch-alobyo-gen-41.
    #[arg(long)]
    pub heuristic_params_file: Option<String>,

    /// Number of games
    #[arg(long, default_value_t = 200)]
    pub games: usize,

    /// First seed. Game i uses seed_base + i, so two genomes measured on the
    /// same range see the same opening hands.
    #[arg(long, default_value_t = 0)]
    pub seed_base: u64,

    /// Players at the table
    #[arg(long, default_value_t = 4)]
    pub players: usize,

    /// Write one game's full decision trace and print the decisions the rules
    /// came closest to getting wrong
    #[arg(long)]
    pub trace_seed: Option<u64>,

    /// Decisions deep to look ahead inside a turn. 0 is the plain one-ply
    /// policy; 2 follows a choice, then the best reply, and scores the position
    /// that leaves.
    #[arg(long, default_value_t = 0)]
    pub lookahead: usize,

    /// How many of the best-scoring choices to follow when looking ahead
    #[arg(long, default_value_t = 4)]
    pub lookahead_width: usize,
}

#[derive(Parser)]
pub struct PolicyPruneArgs {
    /// Genome to measure: `starter` or a path to a JSON file
    #[arg(long, default_value = "starter")]
    pub genome: String,

    /// Heuristic params file the card-quality rules read
    #[arg(long)]
    pub heuristic_params_file: Option<String>,

    /// Games per gene measured, all on the same seeds
    #[arg(long, default_value_t = 300)]
    pub games: usize,

    #[arg(long, default_value_t = 0)]
    pub seed_base: u64,

    #[arg(long, default_value_t = 4)]
    pub players: usize,

    /// A gene whose removal costs less than this many points is free to cut
    #[arg(long, default_value_t = 0.05)]
    pub threshold: f64,

    /// Decisions deep to look ahead while measuring
    #[arg(long, default_value_t = 0)]
    pub lookahead: usize,

    /// How many of the best-scoring choices to follow when looking ahead
    #[arg(long, default_value_t = 4)]
    pub lookahead_width: usize,

    /// Write the genome with every free gene removed
    #[arg(long)]
    pub write: Option<String>,
}

impl PolicyPruneArgs {
    pub fn plan(&self) -> colori_core::policy::Plan {
        colori_core::policy::Plan::new(self.lookahead_width, self.lookahead)
    }
}

#[derive(Parser)]
pub struct PolicyDistillArgs {
    /// Genome whose decisions are dumped
    #[arg(long, default_value = "starter")]
    pub genome: String,

    #[arg(long)]
    pub heuristic_params_file: Option<String>,

    #[arg(long, default_value_t = 120)]
    pub games: usize,

    #[arg(long, default_value_t = 0)]
    pub seed_base: u64,

    #[arg(long, default_value_t = 4)]
    pub players: usize,

    /// Where to write the CSV
    #[arg(long, default_value = "policy-runs/decisions.csv")]
    pub write: String,

    #[arg(long, default_value_t = 0)]
    pub lookahead: usize,

    #[arg(long, default_value_t = 4)]
    pub lookahead_width: usize,
}

/// Which numbers a tuning run may move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum TuneScope {
    /// The choice rules, their regime weights, and the composed readings.
    All,
    /// Only the value function. Useful with `--lookahead`, where the leaves are
    /// what the search is actually judging.
    Values,
}

#[derive(Parser)]
pub struct PolicyEvolveArgs {
    /// Genome whose rules are tuned: `starter` or a path to a JSON file
    #[arg(long, default_value = "starter")]
    pub template: String,

    /// Frozen opponents for the seats the candidate does not take, comma
    /// separated. One rung is a straight match; several make the table vary
    /// while keeping the target stationary.
    #[arg(long, value_delimiter = ',', default_value = "starter")]
    pub anchors: Vec<String>,

    /// Heuristic params file the card-quality rules read. Defaults to the
    /// baked-in batch-alobyo-gen-41.
    #[arg(long)]
    pub heuristic_params_file: Option<String>,

    /// Genomes evaluated per generation
    #[arg(long, default_value_t = 24)]
    pub population: usize,

    #[arg(long, default_value_t = 40)]
    pub generations: usize,

    /// Games per candidate per generation
    #[arg(long, default_value_t = 64)]
    pub games: usize,

    /// First training seed; every candidate is measured on the same range
    #[arg(long, default_value_t = 0)]
    pub seed_base: u64,

    /// Games on the held-out seed range, measured every generation and never
    /// tuned against. Zero leaves the holdout out.
    #[arg(long, default_value_t = 0)]
    pub holdout_games: usize,

    #[arg(long, default_value_t = 1_000_000)]
    pub holdout_seed_base: u64,

    /// Initial spread, relative to each weight's magnitude
    #[arg(long, default_value_t = 0.25)]
    pub sigma: f64,

    /// Smallest spread the search keeps, so it never stops probing
    #[arg(long, default_value_t = 0.01)]
    pub sigma_floor: f64,

    /// Fraction of the population the sampling distribution is refitted to
    #[arg(long, default_value_t = 0.2)]
    pub elite_fraction: f64,

    /// Hard clamp on any weight
    #[arg(long, default_value_t = 20.0)]
    pub max_weight: f64,

    /// Seed for the sampler, so the sampling can be repeated exactly
    #[arg(long, default_value_t = 7)]
    pub sampler_seed: u64,

    /// Players at the table
    #[arg(long, default_value_t = 4)]
    pub players: usize,

    /// Composed readings to mutate per generation. 0 tunes weights only.
    #[arg(long, default_value_t = 4)]
    pub mutations: usize,

    /// Games used to fit a new composition's weight before judging it
    #[arg(long, default_value_t = 64)]
    pub screen_games: usize,

    /// Standard errors a new composition must clear. Below two on purpose: a
    /// worthless composition costs compute, a rejected good one is lost.
    #[arg(long, default_value_t = 1.0)]
    pub mutant_bars: f64,

    /// What to search: everything, or only the value function a planned
    /// decision scores its leaves with.
    #[arg(long, value_enum, default_value_t = TuneScope::All)]
    pub tune: TuneScope,

    /// Decisions deep to look ahead inside a turn
    #[arg(long, default_value_t = 0)]
    pub lookahead: usize,

    /// How many of the best-scoring choices to follow when looking ahead
    #[arg(long, default_value_t = 4)]
    pub lookahead_width: usize,
}

impl PolicyPlayArgs {
    pub fn plan(&self) -> colori_core::policy::Plan {
        colori_core::policy::Plan::new(self.lookahead_width, self.lookahead)
    }
}

impl PolicyEvolveArgs {
    pub fn plan(&self) -> colori_core::policy::Plan {
        colori_core::policy::Plan::new(self.lookahead_width, self.lookahead)
    }
}
