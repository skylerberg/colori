use crate::cli::{TrainArgs, load_heuristic_params};
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

// ── Gene conversion ──

/// Gene indices for optimization. Each variant maps a gene vector
/// position to a HeuristicParams field.
#[repr(usize)]
#[derive(Clone, Copy)]
enum Gene {
    PrimaryColorValue = 0,
    SecondaryColorValue = 1,
    TertiaryColorValue = 2,
    StoredCeramicsWeight = 3,
    StoredPaintingsWeight = 4,
    StoredTextilesWeight = 5,
    DeckThinningValue = 6,
    ChalkQuality = 7,
    AlumQuality = 8,
    CreamOfTartarQuality = 9,
    GumArabicQuality = 10,
    PotashQuality = 11,
    VinegarQuality = 12,
    PrimaryDyeQuality = 13,
    SecondaryDyeQuality = 14,
    TertiaryDyeQuality = 15,
    BasicDyeQuality = 16,
    StarterMaterialQuality = 17,
    CeramicsMaterialQuality = 18,
    PaintingsMaterialQuality = 19,
    TextilesMaterialQuality = 20,
    DualMaterialQuality = 21,
    SellCardMaterialAlignment = 22,
    SellCardColorAlignment = 23,
    HeuristicLookahead = 24,
    RolloutEpsilon = 25,
    RolloutSellAffordableMultiplier = 26,
    RolloutSellBase = 27,
    RolloutMixBase = 28,
    RolloutMixPairWeight = 29,
    RolloutMixCountWeight = 30,
    RolloutMixNoPairs = 31,
    RolloutWorkshopBase = 32,
    RolloutWorkshopCountWeight = 33,
    RolloutWorkshopEmpty = 34,
    RolloutDestroyWithTargets = 35,
    RolloutDestroyNoTargets = 36,
    RolloutDrawBase = 37,
    RolloutDrawCountWeight = 38,
    RolloutOtherPriority = 39,
    RolloutEndTurnThreshold = 40,
    RolloutEndTurnProbabilityEarly = 41,
    RolloutEndTurnProbabilityLate = 42,
    RolloutEndTurnMaxRound = 43,
    RolloutWsMaterialBaseMultiplier = 44,
    RolloutWsMaterialColorsMetMultiplier = 45,
    RolloutWsActionGainDucatsValue = 46,
    RolloutWsActionDrawValue = 47,
    RolloutWsActionWorkshopPerCard = 48,
    RolloutWsActionColorDemandMultiplier = 49,
    LinseedOilQuality = 50,
    ExplorationConstant = 51,
}

const NUM_GENES: usize = 52;

trait GeneTarget: Clone {
    fn to_genes(&self) -> Vec<f64>;
    fn from_genes(genes: &[f64], base: &Self) -> Self;
    fn integer_gene_indices() -> Vec<usize>;
    fn probability_gene_indices() -> Vec<usize>;
}

impl GeneTarget for HeuristicParams {
    fn to_genes(&self) -> Vec<f64> {
        use Gene::*;
        let mut v = vec![0.0; NUM_GENES];
        v[PrimaryColorValue as usize] = self.primary_color_value;
        v[SecondaryColorValue as usize] = self.secondary_color_value;
        v[TertiaryColorValue as usize] = self.tertiary_color_value;
        v[StoredCeramicsWeight as usize] = self.stored_ceramics_weight;
        v[StoredPaintingsWeight as usize] = self.stored_paintings_weight;
        v[StoredTextilesWeight as usize] = self.stored_textiles_weight;
        v[DeckThinningValue as usize] = self.deck_thinning_value;
        v[ChalkQuality as usize] = self.chalk_quality;
        v[AlumQuality as usize] = self.alum_quality;
        v[CreamOfTartarQuality as usize] = self.cream_of_tartar_quality;
        v[GumArabicQuality as usize] = self.gum_arabic_quality;
        v[PotashQuality as usize] = self.potash_quality;
        v[VinegarQuality as usize] = self.vinegar_quality;
        v[LinseedOilQuality as usize] = self.linseed_oil_quality;
        v[PrimaryDyeQuality as usize] = self.primary_dye_quality;
        v[SecondaryDyeQuality as usize] = self.secondary_dye_quality;
        v[TertiaryDyeQuality as usize] = self.tertiary_dye_quality;
        v[BasicDyeQuality as usize] = self.basic_dye_quality;
        v[StarterMaterialQuality as usize] = self.starter_material_quality;
        v[CeramicsMaterialQuality as usize] = self.ceramics_material_quality;
        v[PaintingsMaterialQuality as usize] = self.paintings_material_quality;
        v[TextilesMaterialQuality as usize] = self.textiles_material_quality;
        v[DualMaterialQuality as usize] = self.dual_material_quality;
        v[SellCardMaterialAlignment as usize] = self.sell_card_material_alignment;
        v[SellCardColorAlignment as usize] = self.sell_card_color_alignment;
        v[HeuristicLookahead as usize] = self.heuristic_lookahead as f64;
        v[RolloutEpsilon as usize] = self.rollout_epsilon;
        v[RolloutSellAffordableMultiplier as usize] = self.rollout_sell_affordable_multiplier as f64;
        v[RolloutSellBase as usize] = self.rollout_sell_base as f64;
        v[RolloutMixBase as usize] = self.rollout_mix_base as f64;
        v[RolloutMixPairWeight as usize] = self.rollout_mix_pair_weight as f64;
        v[RolloutMixCountWeight as usize] = self.rollout_mix_count_weight as f64;
        v[RolloutMixNoPairs as usize] = self.rollout_mix_no_pairs as f64;
        v[RolloutWorkshopBase as usize] = self.rollout_workshop_base as f64;
        v[RolloutWorkshopCountWeight as usize] = self.rollout_workshop_count_weight as f64;
        v[RolloutWorkshopEmpty as usize] = self.rollout_workshop_empty as f64;
        v[RolloutDestroyWithTargets as usize] = self.rollout_destroy_with_targets as f64;
        v[RolloutDestroyNoTargets as usize] = self.rollout_destroy_no_targets as f64;
        v[RolloutDrawBase as usize] = self.rollout_draw_base as f64;
        v[RolloutDrawCountWeight as usize] = self.rollout_draw_count_weight as f64;
        v[RolloutOtherPriority as usize] = self.rollout_other_priority as f64;
        v[RolloutEndTurnThreshold as usize] = self.rollout_end_turn_threshold as f64;
        v[RolloutEndTurnProbabilityEarly as usize] = self.rollout_end_turn_probability_early;
        v[RolloutEndTurnProbabilityLate as usize] = self.rollout_end_turn_probability_late;
        v[RolloutEndTurnMaxRound as usize] = self.rollout_end_turn_max_round as f64;
        v[RolloutWsMaterialBaseMultiplier as usize] = self.rollout_ws_material_base_multiplier as f64;
        v[RolloutWsMaterialColorsMetMultiplier as usize] = self.rollout_ws_material_colors_met_multiplier as f64;
        v[RolloutWsActionGainDucatsValue as usize] = self.rollout_ws_action_gain_ducats_value as f64;
        v[RolloutWsActionDrawValue as usize] = self.rollout_ws_action_draw_value as f64;
        v[RolloutWsActionWorkshopPerCard as usize] = self.rollout_ws_action_workshop_per_card as f64;
        v[RolloutWsActionColorDemandMultiplier as usize] = self.rollout_ws_action_color_demand_multiplier as f64;
        v[ExplorationConstant as usize] = self.exploration_constant;
        v
    }

    fn from_genes(v: &[f64], base: &Self) -> Self {
        use Gene::*;
        HeuristicParams {
            primary_color_value: v[PrimaryColorValue as usize],
            secondary_color_value: v[SecondaryColorValue as usize],
            tertiary_color_value: v[TertiaryColorValue as usize],
            stored_ceramics_weight: v[StoredCeramicsWeight as usize],
            stored_paintings_weight: v[StoredPaintingsWeight as usize],
            stored_textiles_weight: v[StoredTextilesWeight as usize],
            deck_thinning_value: v[DeckThinningValue as usize],
            chalk_quality: v[ChalkQuality as usize],
            basic_dye_quality: v[BasicDyeQuality as usize],
            starter_material_quality: v[StarterMaterialQuality as usize],
            ceramics_material_quality: v[CeramicsMaterialQuality as usize],
            paintings_material_quality: v[PaintingsMaterialQuality as usize],
            textiles_material_quality: v[TextilesMaterialQuality as usize],
            dual_material_quality: v[DualMaterialQuality as usize],
            sell_card_material_alignment: v[SellCardMaterialAlignment as usize],
            sell_card_color_alignment: v[SellCardColorAlignment as usize],
            heuristic_round_threshold: base.heuristic_round_threshold,
            heuristic_lookahead: (v[HeuristicLookahead as usize].round() as u32).max(1),
            alum_quality: v[AlumQuality as usize],
            cream_of_tartar_quality: v[CreamOfTartarQuality as usize],
            gum_arabic_quality: v[GumArabicQuality as usize],
            potash_quality: v[PotashQuality as usize],
            vinegar_quality: v[VinegarQuality as usize],
            linseed_oil_quality: v[LinseedOilQuality as usize],
            primary_dye_quality: v[PrimaryDyeQuality as usize],
            secondary_dye_quality: v[SecondaryDyeQuality as usize],
            tertiary_dye_quality: v[TertiaryDyeQuality as usize],
            rollout_epsilon: v[RolloutEpsilon as usize].clamp(0.0, 1.0),
            rollout_sell_affordable_multiplier: v[RolloutSellAffordableMultiplier as usize].round().max(0.0) as u32,
            rollout_sell_base: v[RolloutSellBase as usize].round().max(0.0) as u32,
            rollout_mix_base: v[RolloutMixBase as usize].round().max(0.0) as u32,
            rollout_mix_pair_weight: v[RolloutMixPairWeight as usize].round().max(0.0) as u32,
            rollout_mix_count_weight: v[RolloutMixCountWeight as usize].round().max(0.0) as u32,
            rollout_mix_no_pairs: v[RolloutMixNoPairs as usize].round().max(0.0) as u32,
            rollout_workshop_base: v[RolloutWorkshopBase as usize].round().max(0.0) as u32,
            rollout_workshop_count_weight: v[RolloutWorkshopCountWeight as usize].round().max(0.0) as u32,
            rollout_workshop_empty: v[RolloutWorkshopEmpty as usize].round().max(0.0) as u32,
            rollout_destroy_with_targets: v[RolloutDestroyWithTargets as usize].round().max(0.0) as u32,
            rollout_destroy_no_targets: v[RolloutDestroyNoTargets as usize].round().max(0.0) as u32,
            rollout_draw_base: v[RolloutDrawBase as usize].round().max(0.0) as u32,
            rollout_draw_count_weight: v[RolloutDrawCountWeight as usize].round().max(0.0) as u32,
            rollout_other_priority: v[RolloutOtherPriority as usize].round().max(0.0) as u32,
            rollout_end_turn_threshold: v[RolloutEndTurnThreshold as usize].round().max(0.0) as u32,
            rollout_end_turn_probability_early: v[RolloutEndTurnProbabilityEarly as usize].clamp(0.0, 1.0),
            rollout_end_turn_probability_late: v[RolloutEndTurnProbabilityLate as usize].clamp(0.0, 1.0),
            rollout_end_turn_max_round: v[RolloutEndTurnMaxRound as usize].round().max(2.0) as u32,
            rollout_ws_material_base_multiplier: v[RolloutWsMaterialBaseMultiplier as usize].round().max(0.0) as u32,
            rollout_ws_material_colors_met_multiplier: v[RolloutWsMaterialColorsMetMultiplier as usize].round().max(0.0) as u32,
            rollout_ws_action_gain_ducats_value: v[RolloutWsActionGainDucatsValue as usize].round().max(0.0) as u32,
            rollout_ws_action_draw_value: v[RolloutWsActionDrawValue as usize].round().max(0.0) as u32,
            rollout_ws_action_workshop_per_card: v[RolloutWsActionWorkshopPerCard as usize].round().max(0.0) as u32,
            rollout_ws_action_color_demand_multiplier: v[RolloutWsActionColorDemandMultiplier as usize].round().max(0.0) as u32,
            exploration_constant: v[ExplorationConstant as usize].max(0.01),
            rollout_destroy_worst: base.rollout_destroy_worst,
        }
    }

    fn integer_gene_indices() -> Vec<usize> {
        vec![
            Gene::HeuristicLookahead as usize,
            Gene::RolloutSellAffordableMultiplier as usize,
            Gene::RolloutSellBase as usize,
            Gene::RolloutMixBase as usize,
            Gene::RolloutMixPairWeight as usize,
            Gene::RolloutMixCountWeight as usize,
            Gene::RolloutMixNoPairs as usize,
            Gene::RolloutWorkshopBase as usize,
            Gene::RolloutWorkshopCountWeight as usize,
            Gene::RolloutWorkshopEmpty as usize,
            Gene::RolloutDestroyWithTargets as usize,
            Gene::RolloutDestroyNoTargets as usize,
            Gene::RolloutDrawBase as usize,
            Gene::RolloutDrawCountWeight as usize,
            Gene::RolloutOtherPriority as usize,
            Gene::RolloutEndTurnThreshold as usize,
            Gene::RolloutEndTurnMaxRound as usize,
            Gene::RolloutWsMaterialBaseMultiplier as usize,
            Gene::RolloutWsMaterialColorsMetMultiplier as usize,
            Gene::RolloutWsActionGainDucatsValue as usize,
            Gene::RolloutWsActionDrawValue as usize,
            Gene::RolloutWsActionWorkshopPerCard as usize,
            Gene::RolloutWsActionColorDemandMultiplier as usize,
        ]
    }

    fn probability_gene_indices() -> Vec<usize> {
        vec![
            Gene::RolloutEpsilon as usize,
            Gene::RolloutEndTurnProbabilityEarly as usize,
            Gene::RolloutEndTurnProbabilityLate as usize,
        ]
    }
}

// ── Genetic algorithm ──

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
            ..MctsConfig::new(params_a.clone())
        },
        MctsConfig {
            iterations: eval_iterations,
            use_heuristic_eval: true,
            ..MctsConfig::new(params_b.clone())
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

pub fn run_genetic_algorithm(args: &TrainArgs, threads: usize, output: &str) {
    let batch_id = crate::generate_batch_id();

    eprintln!(
        "Genetic Algorithm: population={}, generations={}, games_per_eval={}, eval_iterations={}, \
         mutation_rate={}, mutation_scale={}, threads={}",
        args.population, args.generations, args.games_per_eval, args.eval_iterations,
        args.mutation_rate, args.mutation_scale, threads
    );

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let seed_params = load_heuristic_params(
        args.seed_params.as_ref().expect("--seed-params is required for training")
    );
    let seed = &seed_params;
    let seed_genes = seed.to_genes();

    eprintln!("Seeding population from provided params file");

    let num_genes = seed.to_genes().len();

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
        // Clamp probability genes
        for &idx in &HeuristicParams::probability_gene_indices() {
            genes[idx] = genes[idx].clamp(0.0, 1.0);
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
            .map(|g| HeuristicParams::from_genes(g, seed))
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
        let best_params = HeuristicParams::from_genes(&population[best_idx], seed);

        // Save best individual
        let output_path = format!("{}/batch-{}-gen-{}.json", output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let avg_fitness: f64 = fitness.iter().map(|(_, f)| f).sum::<f64>() / fitness.len() as f64;
        let elapsed = gen_start.elapsed();
        eprintln!(
            "Gen {}/{}: best={:.4}, avg={:.4}, worst={:.4}, elapsed={:.1}s, saved {}",
            gen + 1,
            args.generations,
            best_fitness,
            avg_fitness,
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
            // Clamp probability genes
            for &idx in &HeuristicParams::probability_gene_indices() {
                child[idx] = child[idx].clamp(0.0, 1.0);
            }

            new_population.push(child);
        }

        population = new_population;
    }

    eprintln!("Genetic algorithm complete. Results in {}/", output);
}
