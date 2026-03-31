mod cli;
mod cmaes;
pub(crate) mod legacy_eval;
mod simulation;
mod solo;
mod tournament;
mod train_diff_eval;

use clap::Parser;
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;

use cli::{Cli, Commands, SimulateArgs};
use cmaes::{run_genetic_algorithm, run_first_pick_cmaes};

pub(crate) fn generate_batch_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = WyRand::from_rng(&mut rand::rng());
    (0..6)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

fn main() {
    let cli = Cli::parse();
    let threads = cli.threads;
    let glass = cli.glass;

    match cli.command {
        Some(Commands::Simulate(args)) => {
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            simulation::run_simulation(&args, threads, &output, glass);
        }
        Some(Commands::Tournament(args)) => {
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            tournament::run_tournament(&args, threads, &output, glass);
        }
        Some(Commands::TrainHeuristicEval(args)) => {
            let output = cli.output.unwrap_or_else(|| "genetic-algorithm".to_string());
            run_genetic_algorithm(&args, threads, &output, glass);
        }
        Some(Commands::TrainFirstPick(args)) => {
            let output = cli.output.unwrap_or_else(|| "first-pick-training".to_string());
            run_first_pick_cmaes(&args, threads, &output, glass);
        }
        Some(Commands::TrainDiffEval(args)) => {
            let output = cli.output.unwrap_or_else(|| "diff-eval-training".to_string());
            train_diff_eval::run_training(&args, threads, &output);
        }
        Some(Commands::Solo(args)) => {
            solo::run_solo(&args, threads, glass);
        }
        None => {
            // Default: simulate with default args
            let args = SimulateArgs {
                games: 10_000,
                note: None,
                variants: None,
                variants_file: "variants.json".to_string(),
            };
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            simulation::run_simulation(&args, threads, &output, glass);
        }
    }
}
