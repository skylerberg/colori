use colori_core::ismcts::MctsConfig;
use colori_core::scoring::{DiffEvalParams, HeuristicParams};

use serde::Deserialize;

const DEFAULT_EVAL_ITERATIONS: u32 = 4_000;

// ── CLI args ──

pub struct SimulationArgs {
    pub games: usize,
    pub threads: usize,
    pub output: String,
    pub note: Option<String>,
    pub variants: Vec<NamedVariant>,
    pub glass: bool,
    pub genetic: Option<CmaEsArgs>,
    pub tournament: bool,
    pub train_diff_eval: bool,
    pub train_games_per_epoch: usize,
    pub train_epochs: usize,
    pub train_batch_size: usize,
    pub train_passes: usize,
    pub train_lr: f64,
    pub train_vs_baseline: bool,
    pub train_no_rollout: bool,
    pub train_eval_iterations: u32,
    pub train_baseline_iterations: Option<u32>,
    pub baseline_heuristic_params: Option<HeuristicParams>,
}

pub struct CmaEsArgs {
    pub population: usize,
    pub generations: usize,
    pub games_per_eval: usize,
    pub initial_sigma: f64,
    pub eval_iterations: u32,
    pub seed_params: Option<HeuristicParams>,
    pub baseline_params: Option<HeuristicParams>,
}

#[derive(Clone)]
pub struct NamedVariant {
    pub name: Option<String>,
    pub ai: MctsConfig,
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
    progressive_bias_weight: Option<f64>,
    #[serde(default)]
    rave_constant: Option<f64>,
    #[serde(default)]
    rave_track_rollout: Option<bool>,
    #[serde(default)]
    rave_track_draft: Option<bool>,
    #[serde(default)]
    heuristic_params: Option<HeuristicParams>,
    #[serde(default)]
    heuristic_params_file: Option<String>,
    #[serde(default)]
    diff_eval_params_file: Option<String>,
    // Overrides for diff eval control params (applied on top of params file values)
    #[serde(default)]
    heuristic_round_threshold: Option<u32>,
    #[serde(default)]
    heuristic_lookahead: Option<u32>,
    #[serde(default)]
    no_rollout: Option<bool>,
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
        let diff_eval_params = self.diff_eval_params_file.as_ref().map(|path| {
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|_| panic!("Failed to read diff eval params file: {}", path));
            let mut params = serde_json::from_str::<DiffEvalParams>(&contents)
                .unwrap_or_else(|_| panic!("Failed to parse diff eval params file: {}", path));
            // Variant-level overrides take precedence over params file values
            if let Some(v) = self.progressive_bias_weight {
                params.set_progressive_bias_weight(v);
            }
            if let Some(v) = self.heuristic_round_threshold {
                params.set_heuristic_round_threshold(v);
            }
            if let Some(v) = self.heuristic_lookahead {
                params.set_heuristic_lookahead(v);
            }
            params
        });
        NamedVariant {
            name: self.name,
            ai: MctsConfig {
                iterations: self.iterations.unwrap_or(defaults.iterations),
                exploration_constant: self.exploration_constant.unwrap_or(defaults.exploration_constant),
                max_rollout_steps: self.max_rollout_steps.unwrap_or(defaults.max_rollout_steps),
                use_heuristic_eval: self.use_heuristic_eval.unwrap_or(defaults.use_heuristic_eval),
                progressive_bias_weight: self.progressive_bias_weight.unwrap_or(defaults.progressive_bias_weight),
                rave_constant: self.rave_constant.unwrap_or(defaults.rave_constant),
                rave_track_rollout: self.rave_track_rollout.unwrap_or(defaults.rave_track_rollout),
                rave_track_draft: self.rave_track_draft.unwrap_or(defaults.rave_track_draft),
                heuristic_params,
                diff_eval_params,
                no_rollout: self.no_rollout.unwrap_or(defaults.no_rollout),
            },
        }
    }
}

pub fn parse_args() -> SimulationArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut games = 10_000usize;
    let mut threads = 10usize;
    let mut output = "game-logs".to_string();
    let mut note: Option<String> = None;
    let mut variants: Option<Vec<NamedVariant>> = None;
    let mut variants_file = "variants.json".to_string();
    let mut glass = false;
    let mut tournament = false;

    let mut train_diff_eval = false;
    let mut train_vs_baseline = false;
    let mut train_no_rollout = false;
    let mut train_baseline_iterations: Option<u32> = None;
    let mut train_games_per_epoch = 500usize;
    let mut train_epochs = 100_000usize;
    let mut train_batch_size = 256usize;
    let mut train_passes = 1usize;
    let mut train_lr = 1e-3f64;

    let mut genetic = false;
    let mut population = 14usize;
    let mut generations = 50usize;
    let mut games_per_eval = 100usize;
    let mut initial_sigma = 0.3f64;
    let mut eval_iterations = DEFAULT_EVAL_ITERATIONS;
    let mut seed_params_file: Option<String> = None;
    let mut baseline_params_file: Option<String> = None;

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
            "--tournament" => {
                tournament = true;
                i += 1;
                continue;
            }
            "--genetic" => {
                genetic = true;
                i += 1;
                continue;
            }
            "--train-diff-eval" => {
                train_diff_eval = true;
                i += 1;
                continue;
            }
            "--train-vs-baseline" => {
                train_vs_baseline = true;
                i += 1;
                continue;
            }
            "--no-rollout" => {
                train_no_rollout = true;
                i += 1;
                continue;
            }
            "--baseline-iterations" => {
                i += 1;
                train_baseline_iterations = Some(args[i].parse().expect("Invalid --baseline-iterations value"));
            }
            "--games-per-epoch" => {
                i += 1;
                train_games_per_epoch = args[i].parse().expect("Invalid --games-per-epoch value");
            }
            "--train-epochs" => {
                i += 1;
                train_epochs = args[i].parse().expect("Invalid --train-epochs value");
            }
            "--train-batch-size" => {
                i += 1;
                train_batch_size = args[i].parse().expect("Invalid --train-batch-size value");
            }
            "--train-passes" => {
                i += 1;
                train_passes = args[i].parse().expect("Invalid --train-passes value");
            }
            "--train-lr" => {
                i += 1;
                train_lr = args[i].parse().expect("Invalid --train-lr value");
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
            "--eval-iterations" => {
                i += 1;
                eval_iterations = args[i].parse().expect("Invalid --eval-iterations value");
            }
            "--seed-params" => {
                i += 1;
                seed_params_file = Some(args[i].clone());
            }
            "--baseline-params" => {
                i += 1;
                baseline_params_file = Some(args[i].clone());
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let baseline_heuristic_params = baseline_params_file.map(|path| {
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read baseline params file: {}", path));
        serde_json::from_str::<HeuristicParams>(&contents)
            .unwrap_or_else(|_| panic!("Failed to parse baseline params file: {}", path))
    });

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
        Some(CmaEsArgs {
            population,
            generations,
            games_per_eval,
            initial_sigma,
            eval_iterations,
            seed_params,
            baseline_params: baseline_heuristic_params.clone(),
        })
    } else {
        None
    };

    let variants = variants.unwrap_or_else(|| {
        if genetic || train_diff_eval {
            // In genetic/training mode, variants file is not required
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

    if train_diff_eval && output == "game-logs" {
        output = "diff-eval-training".to_string();
    }

    SimulationArgs {
        games,
        threads,
        output,
        note,
        variants,
        glass,
        genetic: genetic_args,
        tournament,
        train_diff_eval,
        train_vs_baseline,
        train_no_rollout,
        train_games_per_epoch,
        train_epochs,
        train_batch_size,
        train_passes,
        train_lr,
        train_eval_iterations: eval_iterations,
        train_baseline_iterations,
        baseline_heuristic_params,
    }
}
