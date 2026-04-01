use crate::cli::{TrainGaArgs, load_heuristic_params};
use crate::cmaes::CmaEsTarget;
use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state;
use colori_core::types::GamePhase;
use rand::RngExt;
use rand::SeedableRng;
use std::sync::atomic::Ordering;
use std::time::Instant;
use wyrand::WyRand;

fn sample_normal(rng: &mut WyRand, scale: f64) -> f64 {
    // Box-Muller transform
    let u1: f64 = rng.random::<f64>().max(1e-10);
    let u2: f64 = rng.random::<f64>();
    scale * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn tournament_select(
    fitness: &[(usize, f64)],
    tournament_size: usize,
    rng: &mut WyRand,
) -> usize {
    use rand::RngExt;
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

fn run_ga_game(
    params_a: &HeuristicParams,
    params_b: &HeuristicParams,
    eval_iterations: u32,
    rng: &mut WyRand,
) -> (f64, f64) {
    let num_players = 2;
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, rng);

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

    let max_steps = 5000;
    for _ in 0..max_steps {
        match &state.phase {
            GamePhase::GameOver => break,
            GamePhase::Draw => {
                execute_draw_phase(&mut state, rng);
                continue;
            }
            _ => {}
        }

        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            _ => continue,
        };

        let config = &configs[player_index];
        let result = ismcts(&state, player_index, config, None, None, rng);
        apply_choice_to_state(&mut state, &result.choice, rng);
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

fn generate_batch_id() -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = WyRand::from_rng(&mut rand::rng());
    (0..6)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

pub fn run_genetic_algorithm(args: &TrainGaArgs, threads: usize, output: &str) {
    let batch_id = generate_batch_id();
    let num_genes = <HeuristicParams as CmaEsTarget>::to_genes(&HeuristicParams::default()).len();

    eprintln!(
        "Genetic Algorithm: population={}, generations={}, games_per_eval={}, eval_iterations={}, \
         mutation_rate={}, mutation_scale={}, threads={}",
        args.population, args.generations, args.games_per_eval, args.eval_iterations,
        args.mutation_rate, args.mutation_scale, threads
    );

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let seed_params = args.seed_params.as_ref().map(|p| load_heuristic_params(p));
    let default_params = HeuristicParams::default();
    let seed = seed_params.as_ref().unwrap_or(&default_params);
    let seed_genes = seed.to_genes();

    if seed_params.is_some() {
        eprintln!("Seeding population from provided params file");
    }

    // Initialize population: first individual is seed, rest are perturbed
    let mut population: Vec<Vec<f64>> = Vec::with_capacity(args.population);
    population.push(seed_genes.clone());

    for _ in 1..args.population {
        let mut genes = seed_genes.clone();
        for g in genes.iter_mut() {
            use rand::RngExt;
            let factor = 0.5 + rng.random::<f64>() * 1.5; // [0.5, 2.0)
            *g *= factor;
            if *g < 0.0 {
                *g = 0.0;
            }
        }
        // Clamp integer genes
        for &idx in &HeuristicParams::integer_gene_indices() {
            genes[idx] = genes[idx].round().max(1.0);
        }
        population.push(genes);
    }

    let baseline_params = args.baseline_params.as_ref()
        .map(|p| load_heuristic_params(p))
        .unwrap_or_else(|| seed.clone());

    for gen in 0..args.generations {
        let gen_start = Instant::now();
        let pop_size = population.len();

        let population_params: Vec<HeuristicParams> = population
            .iter()
            .map(|g| HeuristicParams::from_genes(g))
            .collect();

        let baseline_ref = &baseline_params;
        let eval_iterations = args.eval_iterations;
        let games_per_eval = args.games_per_eval;
        let num_threads = threads;

        let wins: Vec<std::sync::atomic::AtomicU64> = (0..pop_size)
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();

        // Evaluate each individual against baseline
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
                            let (w, _) = run_ga_game(params, baseline_ref, eval_iterations, &mut rng);
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

        // Compute fitness
        let mut fitness: Vec<(usize, f64)> = (0..pop_size)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                (i, w / games_per_eval as f64)
            })
            .collect();

        fitness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_idx = fitness[0].0;
        let best_fitness = fitness[0].1;
        let best_params = HeuristicParams::from_genes(&population[best_idx]);

        // Save best individual
        let output_path = format!("{}/batch-{}-gen-{}.json", output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let elapsed = gen_start.elapsed();
        eprintln!(
            "Gen {}/{}: best_fitness={:.4}, worst_fitness={:.4}, elapsed={:.1}s, saved {}",
            gen + 1,
            args.generations,
            best_fitness,
            fitness.last().unwrap().1,
            elapsed.as_secs_f64(),
            output_path,
        );

        if gen + 1 >= args.generations {
            break;
        }

        // Selection, crossover, mutation
        let mut new_population: Vec<Vec<f64>> = Vec::with_capacity(args.population);

        // Elitism: top 2 survive
        new_population.push(population[fitness[0].0].clone());
        new_population.push(population[fitness[1].0].clone());

        while new_population.len() < args.population {
            use rand::RngExt;
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
            for g in child.iter_mut() {
                if rng.random::<f64>() < args.mutation_rate {
                    let perturbation = sample_normal(&mut rng, args.mutation_scale);
                    *g *= 1.0 + perturbation;
                    if *g < 0.0 {
                        *g = 0.0;
                    }
                }
            }

            // Clamp integer genes
            for &idx in &HeuristicParams::integer_gene_indices() {
                child[idx] = child[idx].round().max(1.0);
            }

            new_population.push(child);
        }

        population = new_population;
    }

    eprintln!("Genetic algorithm complete. Results in {}/", output);
}
