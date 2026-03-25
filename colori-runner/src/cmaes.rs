use colori_core::colori_game::{apply_choice_to_state, enumerate_choices};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, FirstPickParams, HeuristicParams};
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;

use nalgebra::{DMatrix, DVector};
use nalgebra::linalg::SymmetricEigen;
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::cli::{TrainHeuristicEvalArgs, TrainFirstPickArgs, load_heuristic_params};
use crate::generate_batch_id;

pub trait CmaEsTarget: Clone {
    fn to_genes(&self) -> Vec<f64>;
    fn from_genes(genes: &[f64]) -> Self;
    /// Gene indices that should be rounded to integers and clamped >= 1
    fn integer_gene_indices() -> Vec<usize>;
}

/// Box-Muller transform: generate a sample from N(0, std_dev)
fn sample_normal(rng: &mut WyRand, std_dev: f64) -> f64 {
    let u1: f64 = rng.random::<f64>();
    let u2: f64 = rng.random::<f64>();
    let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    z * std_dev
}

/// Gene indices for CMA-ES optimization. Each variant maps a gene vector
/// position to a HeuristicParams field.
#[repr(usize)]
#[derive(Clone, Copy)]
pub enum Gene {
    PrimaryColorValue = 0,
    SecondaryColorValue = 1,
    TertiaryColorValue = 2,
    StoredMaterialWeight = 3,
    ChalkQuality = 4,
    AlumQuality = 5,
    CreamOfTartarQuality = 6,
    GumArabicQuality = 7,
    PotashQuality = 8,
    VinegarQuality = 9,
    ArgolQuality = 10,
    PurePrimaryDyeQuality = 11,
    PrimaryDyeQuality = 12,
    SecondaryDyeQuality = 13,
    TertiaryDyeQuality = 14,
    BasicDyeQuality = 15,
    StarterMaterialQuality = 16,
    DraftMaterialQuality = 17,
    DualMaterialQuality = 18,
    SellCardMaterialAlignment = 19,
    SellCardColorAlignment = 20,
    GlassWeight = 21,
    PrimaryColorCoverage = 22,
    SecondaryColorCoverage = 23,
    CardsInDeck = 24,
    CardsInDeckSquared = 25,
    MaterialTypeCount = 26,
    MaterialCoverage = 27,
    HeuristicScoreThreshold = 28,
    HeuristicLookahead = 29,
}

pub const NUM_GENES: usize = 30;

impl CmaEsTarget for HeuristicParams {
    fn to_genes(&self) -> Vec<f64> {
        use Gene::*;
        let mut v = vec![0.0; NUM_GENES];
        v[PrimaryColorValue as usize] = self.primary_color_value;
        v[SecondaryColorValue as usize] = self.secondary_color_value;
        v[TertiaryColorValue as usize] = self.tertiary_color_value;
        v[StoredMaterialWeight as usize] = self.stored_material_weight;
        v[ChalkQuality as usize] = self.chalk_quality;
        v[AlumQuality as usize] = self.alum_quality.unwrap_or(self.action_quality);
        v[CreamOfTartarQuality as usize] = self.cream_of_tartar_quality.unwrap_or(self.action_quality);
        v[GumArabicQuality as usize] = self.gum_arabic_quality.unwrap_or(self.action_quality);
        v[PotashQuality as usize] = self.potash_quality.unwrap_or(self.action_quality);
        v[VinegarQuality as usize] = self.vinegar_quality.unwrap_or(self.action_quality);
        v[ArgolQuality as usize] = self.argol_quality.unwrap_or(self.action_quality);
        v[PurePrimaryDyeQuality as usize] = self.pure_primary_dye_quality.unwrap_or(self.dye_quality);
        v[PrimaryDyeQuality as usize] = self.primary_dye_quality.unwrap_or(self.dye_quality);
        v[SecondaryDyeQuality as usize] = self.secondary_dye_quality.unwrap_or(self.dye_quality);
        v[TertiaryDyeQuality as usize] = self.tertiary_dye_quality.unwrap_or(self.dye_quality);
        v[BasicDyeQuality as usize] = self.basic_dye_quality;
        v[StarterMaterialQuality as usize] = self.starter_material_quality;
        v[DraftMaterialQuality as usize] = self.draft_material_quality;
        v[DualMaterialQuality as usize] = self.dual_material_quality;
        v[SellCardMaterialAlignment as usize] = self.sell_card_material_alignment;
        v[SellCardColorAlignment as usize] = self.sell_card_color_alignment;
        v[GlassWeight as usize] = self.glass_weight;
        v[PrimaryColorCoverage as usize] = self.primary_color_coverage_weight;
        v[SecondaryColorCoverage as usize] = self.secondary_color_coverage_weight;
        v[CardsInDeck as usize] = self.cards_in_deck_weight;
        v[CardsInDeckSquared as usize] = self.cards_in_deck_squared_weight;
        v[MaterialTypeCount as usize] = self.material_type_count_weight;
        v[MaterialCoverage as usize] = self.material_coverage_weight;
        v[HeuristicScoreThreshold as usize] = self.heuristic_score_threshold.unwrap_or(10.0);
        v[HeuristicLookahead as usize] = self.heuristic_lookahead as f64;
        v
    }

    fn from_genes(v: &[f64]) -> Self {
        use Gene::*;
        let defaults = HeuristicParams::default();
        HeuristicParams {
            primary_color_value: v[PrimaryColorValue as usize],
            secondary_color_value: v[SecondaryColorValue as usize],
            tertiary_color_value: v[TertiaryColorValue as usize],
            stored_material_weight: v[StoredMaterialWeight as usize],
            chalk_quality: v[ChalkQuality as usize],
            action_quality: defaults.action_quality,
            dye_quality: defaults.dye_quality,
            basic_dye_quality: v[BasicDyeQuality as usize],
            starter_material_quality: v[StarterMaterialQuality as usize],
            draft_material_quality: v[DraftMaterialQuality as usize],
            dual_material_quality: v[DualMaterialQuality as usize],
            sell_card_material_alignment: v[SellCardMaterialAlignment as usize],
            sell_card_color_alignment: v[SellCardColorAlignment as usize],
            glass_weight: v[GlassWeight as usize],
            heuristic_round_threshold: defaults.heuristic_round_threshold,
            heuristic_lookahead: (v[HeuristicLookahead as usize].round() as u32).max(1),
            alum_quality: Some(v[AlumQuality as usize]),
            cream_of_tartar_quality: Some(v[CreamOfTartarQuality as usize]),
            gum_arabic_quality: Some(v[GumArabicQuality as usize]),
            potash_quality: Some(v[PotashQuality as usize]),
            vinegar_quality: Some(v[VinegarQuality as usize]),
            argol_quality: Some(v[ArgolQuality as usize]),
            pure_primary_dye_quality: Some(v[PurePrimaryDyeQuality as usize]),
            primary_dye_quality: Some(v[PrimaryDyeQuality as usize]),
            secondary_dye_quality: Some(v[SecondaryDyeQuality as usize]),
            tertiary_dye_quality: Some(v[TertiaryDyeQuality as usize]),
            primary_color_coverage_weight: v[PrimaryColorCoverage as usize],
            secondary_color_coverage_weight: v[SecondaryColorCoverage as usize],
            cards_in_deck_weight: v[CardsInDeck as usize],
            cards_in_deck_squared_weight: v[CardsInDeckSquared as usize],
            material_type_count_weight: v[MaterialTypeCount as usize],
            material_coverage_weight: v[MaterialCoverage as usize],
            heuristic_score_threshold: Some(v[HeuristicScoreThreshold as usize]),
        }
    }

    fn integer_gene_indices() -> Vec<usize> {
        vec![Gene::HeuristicLookahead as usize]
    }
}

fn run_eval_game(
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
        let result = ismcts(&state, player_index, config, Some(max_rollout_round), None, rng);
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
    integer_genes: Vec<usize>,
}

impl CmaEsState {
    fn new(seed_genes: &[f64], lambda: usize, initial_sigma: f64, frozen_genes: Vec<usize>, integer_genes: Vec<usize>) -> Self {
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
            integer_genes,
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

            for &idx in &self.integer_genes {
                genes[idx] = genes[idx].round().max(1.0);
            }

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

pub fn run_genetic_algorithm(args: &TrainHeuristicEvalArgs, threads: usize, output: &str, glass: bool) {
    let batch_id = generate_batch_id();

    eprintln!(
        "CMA-ES: lambda={}, generations={}, games_per_eval={}, eval_iterations={}, initial_sigma={}, threads={}",
        args.population, args.generations, args.games_per_eval, args.eval_iterations, args.initial_sigma, threads
    );

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let seed_params = args.seed_params.as_ref().map(|p| load_heuristic_params(p));
    let default_params = HeuristicParams::default();
    let seed = seed_params.as_ref().unwrap_or(&default_params);
    let seed_genes = seed.to_genes();

    if seed_params.is_some() {
        eprintln!("Seeding CMA-ES from provided params file");
    }

    let baseline_heuristic_params = args.baseline_params.as_ref().map(|p| load_heuristic_params(p));
    if baseline_heuristic_params.is_some() {
        eprintln!("Using provided baseline params file");
    }

    // Always freeze vinegar/argol (not in draft deck); freeze glass_weight when glass expansion is disabled
    // Freezing deck size genes for our current test
    let mut frozen_genes: Vec<usize> = vec![
        Gene::VinegarQuality as usize,
        Gene::ArgolQuality as usize,
        Gene::CardsInDeck as usize,
        Gene::CardsInDeckSquared as usize,
    ];
    if !glass {
        frozen_genes.push(Gene::GlassWeight as usize);
    }

    let mut cma = CmaEsState::new(&seed_genes, args.population, args.initial_sigma, frozen_genes, HeuristicParams::integer_gene_indices());
    let baseline_params = baseline_heuristic_params.as_ref().unwrap_or(seed).clone();

    for gen in 0..args.generations {
        let gen_start = Instant::now();

        // Sample lambda offspring from CMA-ES distribution
        let offspring = cma.sample_offspring(&mut rng);
        let pop_size = offspring.len();

        // Evaluate all individuals against baseline
        let eval_params: Vec<HeuristicParams> = offspring
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

        for i in 0..pop_size {
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

                        for game_idx in 0..count {
                            if game_idx % 2 == 0 {
                                let (w, _) = run_eval_game(params, baseline_ref, eval_iterations, glass, &mut rng);
                                thread_wins += w;
                            } else {
                                let (_, w) = run_eval_game(baseline_ref, params, eval_iterations, glass, &mut rng);
                                thread_wins += w;
                            }
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

        // Compute fitness for all individuals
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
        let best_params = HeuristicParams::from_genes(&offspring[best_idx]);

        // Save best individual
        let output_path = format!("{}/batch-{}-gen-{}.json", output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let avg_fitness = fitness.iter().map(|(_, wr)| wr).sum::<f64>() / pop_size as f64;

        // Compute average pairwise gene distance as a diversity measure
        let mut total_dist = 0.0;
        let mut pairs = 0u64;
        for a in 0..pop_size {
            for b in (a + 1)..pop_size {
                let dist: f64 = offspring[a]
                    .iter()
                    .zip(offspring[b].iter())
                    .map(|(gene_a, gene_b)| {
                        let scale = gene_a.abs().max(gene_b.abs()).max(0.1);
                        ((gene_a - gene_b) / scale).powi(2)
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
            args.generations,
            best_fitness,
            avg_fitness,
            fitness.last().unwrap().1,
            avg_diversity,
            cma.sigma,
            elapsed.as_secs_f64(),
            output_path,
        );

        // Update CMA-ES distribution
        let offspring_fitnesses: Vec<f64> = (0..args.population)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                w / games_per_eval as f64
            })
            .collect();
        cma.update(&offspring, &offspring_fitnesses, gen);
    }

    eprintln!("CMA-ES optimization complete. Results in {}/", output);
}

// ── First Pick CMA-ES ──

impl CmaEsTarget for FirstPickParams {
    fn to_genes(&self) -> Vec<f64> {
        vec![
            self.is_tertiary_dye,
            self.is_secondary_dye,
            self.is_primary_dye,
            self.is_pure_primary_dye,
            self.is_alum,
            self.is_gum_arabic,
            self.is_cream_of_tartar,
            self.is_potash,
            self.is_dual_material,
            self.is_material_plus_color,
            self.matching_tertiary_colors,
            self.matching_secondary_colors,
            self.matching_primary_colors,
            self.matching_materials,
        ]
    }

    fn from_genes(v: &[f64]) -> Self {
        FirstPickParams {
            is_tertiary_dye: v[0],
            is_secondary_dye: v[1],
            is_primary_dye: v[2],
            is_pure_primary_dye: v[3],
            is_alum: v[4],
            is_gum_arabic: v[5],
            is_cream_of_tartar: v[6],
            is_potash: v[7],
            is_dual_material: v[8],
            is_material_plus_color: v[9],
            matching_tertiary_colors: v[10],
            matching_secondary_colors: v[11],
            matching_primary_colors: v[12],
            matching_materials: v[13],
        }
    }

    fn integer_gene_indices() -> Vec<usize> {
        vec![]
    }
}

fn run_first_pick_eval_game(
    first_pick: &FirstPickParams,
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
            first_pick_params: Some(Box::new(first_pick.clone())),
            ..MctsConfig::default()
        },
        MctsConfig {
            iterations: eval_iterations,
            ..MctsConfig::default()
        },
    ];

    execute_draw_phase(&mut state, rng);

    while !matches!(state.phase, GamePhase::GameOver) {
        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => break,
            GamePhase::GameOver => break,
        };

        let config = &configs[player_index];

        // Check for first pick heuristic
        let is_first_pick = state.round == 1
            && matches!(&state.phase, GamePhase::Draft { draft_state } if draft_state.pick_number == 0);

        if is_first_pick && config.first_pick_params.is_some() {
            let fpp = config.first_pick_params.as_ref().unwrap();
            let choices = enumerate_choices(&state);
            let best = choices.iter()
                .max_by(|a, b| {
                    let sa = match a { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    let sb = match b { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone();
            apply_choice_to_state(&mut state, &best, rng);
        } else {
            let max_rollout_round = std::cmp::max(8, state.round + 2);
            let result = ismcts(&state, player_index, config, Some(max_rollout_round), None, rng);
            apply_choice_to_state(&mut state, &result.choice, rng);
        }
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

pub fn run_first_pick_cmaes(args: &TrainFirstPickArgs, threads: usize, output: &str, glass: bool) {
    let batch_id = generate_batch_id();

    eprintln!(
        "First Pick CMA-ES: lambda={}, generations={}, games_per_eval={}, eval_iterations={}, initial_sigma={}, threads={}",
        args.population, args.generations, args.games_per_eval, args.eval_iterations, args.initial_sigma, threads
    );

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let seed = FirstPickParams::default();
    let seed_genes = seed.to_genes();

    let mut cma = CmaEsState::new(&seed_genes, args.population, args.initial_sigma, vec![], FirstPickParams::integer_gene_indices());

    for gen in 0..args.generations {
        let gen_start = Instant::now();

        let offspring = cma.sample_offspring(&mut rng);
        let pop_size = offspring.len();

        let eval_params: Vec<FirstPickParams> = offspring
            .iter()
            .map(|g| FirstPickParams::from_genes(g))
            .collect();

        let eval_iterations = args.eval_iterations;
        let games_per_eval = args.games_per_eval;
        let num_threads = threads;

        let wins: Vec<std::sync::atomic::AtomicU64> = (0..pop_size)
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();

        for i in 0..pop_size {
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

                        for game_idx in 0..count {
                            if game_idx % 2 == 0 {
                                let (w, _) = run_first_pick_eval_game(params, eval_iterations, glass, &mut rng);
                                thread_wins += w;
                            } else {
                                // Swap positions: baseline is player 0, candidate is player 1
                                let (_, w) = run_first_pick_eval_game_swapped(params, eval_iterations, glass, &mut rng);
                                thread_wins += w;
                            }
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
        let best_params = FirstPickParams::from_genes(&offspring[best_idx]);

        let output_path = format!("{}/batch-{}-gen-{}.json", output, batch_id, gen);
        let json = serde_json::to_string_pretty(&best_params).unwrap();
        std::fs::write(&output_path, json).unwrap();

        let avg_fitness = fitness.iter().map(|(_, wr)| wr).sum::<f64>() / pop_size as f64;

        let elapsed = gen_start.elapsed();
        eprintln!(
            "Gen {}/{}: best={:.4}, avg={:.4}, worst={:.4}, sigma={:.6}, elapsed={:.1}s, saved {}",
            gen + 1,
            args.generations,
            best_fitness,
            avg_fitness,
            fitness.last().unwrap().1,
            cma.sigma,
            elapsed.as_secs_f64(),
            output_path,
        );

        let offspring_fitnesses: Vec<f64> = (0..args.population)
            .map(|i| {
                let w = wins[i].load(Ordering::Relaxed) as f64 / 1000.0;
                w / games_per_eval as f64
            })
            .collect();
        cma.update(&offspring, &offspring_fitnesses, gen);
    }

    eprintln!("First Pick CMA-ES complete. Results in {}/", output);
}

fn run_first_pick_eval_game_swapped(
    first_pick: &FirstPickParams,
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
            ..MctsConfig::default()
        },
        MctsConfig {
            iterations: eval_iterations,
            first_pick_params: Some(Box::new(first_pick.clone())),
            ..MctsConfig::default()
        },
    ];

    execute_draw_phase(&mut state, rng);

    while !matches!(state.phase, GamePhase::GameOver) {
        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => break,
            GamePhase::GameOver => break,
        };

        let config = &configs[player_index];

        let is_first_pick = state.round == 1
            && matches!(&state.phase, GamePhase::Draft { draft_state } if draft_state.pick_number == 0);

        if is_first_pick && config.first_pick_params.is_some() {
            let fpp = config.first_pick_params.as_ref().unwrap();
            let choices = enumerate_choices(&state);
            let best = choices.iter()
                .max_by(|a, b| {
                    let sa = match a { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    let sb = match b { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone();
            apply_choice_to_state(&mut state, &best, rng);
        } else {
            let max_rollout_round = std::cmp::max(8, state.round + 2);
            let result = ismcts(&state, player_index, config, Some(max_rollout_round), None, rng);
            apply_choice_to_state(&mut state, &result.choice, rng);
        }
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
