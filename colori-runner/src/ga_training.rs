use colori_core::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ga::{evolve_generation, random_individual, GAConfig, Individual};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::rollout_policy::{RolloutPolicy, TOTAL_WEIGHTS};
use colori_core::scoring::compute_terminal_rewards;
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;

use rand::SeedableRng;
use wyrand::WyRand;

pub struct GATrainingConfig {
    pub population_size: usize,
    pub generations: usize,
    pub games_per_eval: usize,
    pub tournament_size: usize,
    pub crossover_alpha: f64,
    pub mutation_rate: f64,
    pub mutation_sigma: f64,
    pub elitism_count: usize,
    pub mcts_iterations: u32,
    pub num_players: usize,
    pub temperature: f64,
    pub num_threads: usize,
    pub opponent_mode: OpponentMode,
}

#[derive(Clone)]
pub enum OpponentMode {
    Random,
    Best,
}

impl Default for GATrainingConfig {
    fn default() -> Self {
        GATrainingConfig {
            population_size: 50,
            generations: 50,
            games_per_eval: 10,
            tournament_size: 3,
            crossover_alpha: 0.5,
            mutation_rate: 0.3,
            mutation_sigma: 0.5,
            elitism_count: 2,
            mcts_iterations: 20,
            num_players: 2,
            temperature: 1.0,
            num_threads: 4,
            opponent_mode: OpponentMode::Random,
        }
    }
}

fn make_policy(weights: &[f64; TOTAL_WEIGHTS], temperature: f64) -> RolloutPolicy {
    RolloutPolicy {
        weights: *weights,
        temperature,
    }
}

fn evaluate_individual(
    weights: &[f64; TOTAL_WEIGHTS],
    opponent_weights: Option<&[f64; TOTAL_WEIGHTS]>,
    config: &GATrainingConfig,
    rng: &mut WyRand,
) -> f64 {
    let mut total_reward = 0.0f64;

    let policy = make_policy(weights, config.temperature);
    let evolved_config = MctsConfig {
        iterations: config.mcts_iterations,
        rollout_policy: Some(policy),
        ..MctsConfig::default()
    };

    let opponent_config = if let Some(opp_w) = opponent_weights {
        let opp_policy = make_policy(opp_w, config.temperature);
        MctsConfig {
            iterations: config.mcts_iterations,
            rollout_policy: Some(opp_policy),
            ..MctsConfig::default()
        }
    } else {
        MctsConfig {
            iterations: config.mcts_iterations,
            rollout_policy: None,
            ..MctsConfig::default()
        }
    };

    for _ in 0..config.games_per_eval {
        // Evolved player is always index 0
        let ai_players = vec![true; config.num_players];
        let mut state = create_initial_game_state(config.num_players, &ai_players, rng);
        execute_draw_phase(&mut state, rng);

        let mut steps = 0u32;
        while !matches!(state.phase, GamePhase::GameOver) && steps < 5000 {
            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => break,
            };

            let player_config = if player_index == 0 {
                &evolved_config
            } else {
                &opponent_config
            };

            let max_rollout_round = std::cmp::max(8, state.round + 2);
            let choice = ismcts(
                &state,
                player_index,
                player_config,
                &None,
                Some(max_rollout_round),
                rng,
            );

            apply_choice_to_state(&mut state, &choice, rng);
            steps += 1;
        }

        let rewards = compute_terminal_rewards(&state.players);
        if !rewards.is_empty() {
            total_reward += rewards[0];
        }
    }

    total_reward / config.games_per_eval as f64
}

fn evaluate_population(
    population: &mut [Individual],
    opponent_weights: Option<&[f64; TOTAL_WEIGHTS]>,
    config: &GATrainingConfig,
) {
    let num_threads = config.num_threads.min(population.len());

    // Collect results in a thread-safe way
    let results: Vec<(usize, f64)> = std::thread::scope(|s| {
        let chunks: Vec<(usize, &[Individual])> = {
            let chunk_size = (population.len() + num_threads - 1) / num_threads;
            population
                .chunks(chunk_size)
                .enumerate()
                .map(|(i, chunk)| (i * chunk_size, chunk))
                .collect()
        };

        let handles: Vec<_> = chunks
            .into_iter()
            .map(|(start_idx, chunk)| {
                let opp_w = opponent_weights.cloned();
                s.spawn(move || {
                    let mut rng = WyRand::from_rng(&mut rand::rng());
                    let mut local_results = Vec::new();
                    for (i, ind) in chunk.iter().enumerate() {
                        let fitness = evaluate_individual(
                            &ind.weights,
                            opp_w.as_ref(),
                            config,
                            &mut rng,
                        );
                        local_results.push((start_idx + i, fitness));
                    }
                    local_results
                })
            })
            .collect();

        let mut all_results = Vec::new();
        for handle in handles {
            all_results.extend(handle.join().unwrap());
        }
        all_results
    });

    for (idx, fitness) in results {
        population[idx].fitness = fitness;
    }
}

pub fn run_ga_training(config: GATrainingConfig, output_path: &str) {
    let mut rng = WyRand::from_rng(&mut rand::rng());

    eprintln!(
        "GA Training: pop={}, gen={}, games/eval={}, iters={}, players={}, threads={}, opponent={:?}",
        config.population_size,
        config.generations,
        config.games_per_eval,
        config.mcts_iterations,
        config.num_players,
        config.num_threads,
        match config.opponent_mode {
            OpponentMode::Random => "random",
            OpponentMode::Best => "best",
        }
    );

    // Initialize population
    let mut population: Vec<Individual> = (0..config.population_size)
        .map(|_| random_individual(&mut rng))
        .collect();

    let mut best_weights: Option<[f64; TOTAL_WEIGHTS]> = None;

    let ga_config = GAConfig {
        tournament_size: config.tournament_size,
        crossover_alpha: config.crossover_alpha,
        mutation_rate: config.mutation_rate,
        mutation_sigma: config.mutation_sigma,
        elitism_count: config.elitism_count,
    };

    for gen in 0..config.generations {
        // Determine opponent weights
        let opponent_weights = match config.opponent_mode {
            OpponentMode::Random => None,
            OpponentMode::Best => best_weights,
        };

        // Evaluate
        evaluate_population(&mut population, opponent_weights.as_ref(), &config);

        // Find best
        let best_idx = population
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.fitness.partial_cmp(&b.fitness).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let best = &population[best_idx];
        let avg_fitness: f64 =
            population.iter().map(|i| i.fitness).sum::<f64>() / population.len() as f64;

        eprintln!(
            "Gen {}/{}: best={:.4}, avg={:.4}",
            gen + 1,
            config.generations,
            best.fitness,
            avg_fitness,
        );

        best_weights = Some(best.weights);

        // Evolve next generation (unless last)
        if gen + 1 < config.generations {
            population = evolve_generation(&population, &ga_config, &mut rng);
        }
    }

    // Output best individual
    let best_idx = population
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.fitness.partial_cmp(&b.fitness).unwrap())
        .map(|(i, _)| i)
        .unwrap();

    let best = &population[best_idx];
    let policy = make_policy(&best.weights, config.temperature);

    let json = serde_json::to_string_pretty(&policy).unwrap();
    std::fs::write(output_path, &json).unwrap();
    eprintln!("Best weights written to {}", output_path);
    eprintln!("Best fitness: {:.4}", best.fitness);
    eprintln!("Weights: {:?}", best.weights);
}
