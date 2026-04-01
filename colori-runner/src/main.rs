mod cli;
mod genetic;
mod simulation;
mod tournament;

use clap::Parser;
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;

use cli::{Cli, Commands, SimulateArgs};

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

    match cli.command {
        Some(Commands::Simulate(args)) => {
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            simulation::run_simulation(&args, threads, &output);
        }
        Some(Commands::Tournament(args)) => {
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            tournament::run_tournament(&args, threads, &output);
        }
        Some(Commands::Train(args)) => {
            let output = cli.output.unwrap_or_else(|| "genetic-algorithm".to_string());
            genetic::run_genetic_algorithm(&args, threads, &output);
        }
        None => {
            // Default: simulate with default args
            let args = SimulateArgs {
                games: 10_000,
                note: None,
                variants: None,
                variants_file: "variants.json".to_string(),
                max_rounds: 5,
            };
            let output = cli.output.unwrap_or_else(|| "game-logs".to_string());
            simulation::run_simulation(&args, threads, &output);
        }
    }
}
