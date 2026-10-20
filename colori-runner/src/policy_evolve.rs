//! Tuning the numbers in a rule system.
//!
//! The search here is deliberately small. It tunes **weights only** — the rules
//! themselves are fixed by the template genome, and adding or reshaping a rule
//! is a code change (a new feature) proposed from outside this loop. That split
//! is what keeps the loop honest: what it searches is a flat vector of real
//! numbers that a cross-entropy method handles well, and what it cannot do is
//! invent a rule it has no way to evaluate.
//!
//! # Why not a genetic algorithm
//!
//! Fifty continuous parameters is not a genetic algorithm's problem. Crossover
//! on weights mixes unrelated magnitudes and destroys the fine structure the
//! search has found; mutation alone makes it a slow random walk. The
//! cross-entropy method — sample a Gaussian, keep the elite, refit the Gaussian
//! to them — needs a fraction of the games for the same result, and its state (a
//! mean and a spread per gene) is exactly what a human wants to read to see
//! whether the search is still learning or has collapsed.
//!
//! # The discipline the report enforces
//!
//! - The generation's mean genome is evaluated alongside its samples, so two
//!   generations' numbers are comparable instead of being measured against
//!   moving targets.
//! - The ladder's own first rung is measured exactly like a candidate. Nothing
//!   below that is progress.
//! - Every gene's contribution is measured, so a rule that never fires shows up
//!   rather than quietly absorbing a weight.
//! - A holdout seed range, never tuned against, is re-measured each generation.
//!   A genome that gains on the training seeds while losing on the holdout is
//!   overfitting the seeds, and the report says so before a run wastes another
//!   hour.

use std::collections::BTreeMap;

use colori_core::policy::{
    features, rule_count, CompiledPolicy, Gate, InteractionGene, Op, PolicyGenome, Shape,
    VALUE_FEATURES,
};
use colori_core::scoring::HeuristicParams;
use rand::{RngExt, SeedableRng};
use serde::Serialize;
use wyrand::WyRand;

use crate::cli::PolicyEvolveArgs;
use crate::policy_play::{self, FeatureActivity, ScoreSummary};

/// A candidate's weights, in the order the template listed them.
type Weights = Vec<f64>;

struct Candidate {
    label: String,
    weights: Weights,
    summary: ScoreSummary,
    /// Per-game scores, in seed order. Two candidates measured on the same seeds
    /// can be compared game by game, which cancels the deal each game got.
    scores: Vec<f64>,
}

impl Candidate {
    fn fitness(&self) -> f64 {
        self.summary.mean
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WeightRow {
    feature: String,
    mean: f64,
    sigma: f64,
    /// What the search started from, so a drift is visible at a glance.
    started_at: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneRow {
    label: String,
    summary: ScoreSummary,
    weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationReport {
    generation: usize,
    games_per_candidate: usize,
    seed_base: u64,
    population: usize,
    /// The generation's mean genome: the centre of the search, and the starting
    /// point for the next generation's sampling.
    mean_genome: GeneRow,
    best_in_generation: GeneRow,
    best_ever: GeneRow,
    /// The ladder's first rung, measured on the candidate's seat against the
    /// rest of the ladder. Nothing below it is progress.
    baseline: ScoreSummary,
    /// The best-ever genome on seeds it was never tuned against.
    #[serde(skip_serializing_if = "Option::is_none")]
    holdout: Option<ScoreSummary>,
    /// The composed readings the search is currently working with.
    compositions: Vec<String>,
    /// What each structural attempt was, what it scored, and whether it was kept.
    structure_attempts: Vec<String>,
    candidates: Vec<GeneRow>,
    weights: Vec<WeightRow>,
    activity: Vec<FeatureActivity>,
    wall_clock_secs: f64,
}

/// One tunable number: a rule, optionally in one stretch of the game, or a
/// composed reading.
#[derive(Debug, Clone)]
struct Tunable {
    gate: Option<Gate>,
    name: &'static str,
    /// Set for a composed reading, which is addressed by its position in the
    /// genome's list along with the label it writes out.
    composed: Option<(usize, String)>,
}

impl Tunable {
    fn rule(name: &'static str) -> Self {
        Tunable { gate: None, name, composed: None }
    }

    fn regime(gate: Gate, name: &'static str) -> Self {
        Tunable { gate: Some(gate), name, composed: None }
    }

    fn composed(index: usize, label: String) -> Self {
        Tunable { gate: None, name: "", composed: Some((index, label)) }
    }

    /// How the number is labelled in a report.
    fn label(&self) -> String {
        if let Some((_, label)) = &self.composed {
            return label.clone();
        }
        match self.gate {
            Some(gate) => format!("{}/{}", gate.name(), self.name),
            None => self.name.to_string(),
        }
    }
}

/// The rules a tuning run may touch: the template's switched-on features, and
/// the regime weights it wrote for them.
struct Rules {
    tunables: Vec<Tunable>,
    shapes: Vec<Shape>,
    started_at: Weights,
}

pub fn run_policy_evolution(args: &PolicyEvolveArgs, threads: usize, output: &str) {
    let params = policy_play::load_params(args.heuristic_params_file.as_deref());
    let template = policy_play::load_genome(&args.template);
    // The incumbent: its weights are searched, and its list of composed readings
    // is what the structural step mutates.
    let mut current = template.clone();
    let ladder_policies = policy_play::compile_ladder(&args.anchors, &params);
    assert!(
        !ladder_policies.is_empty(),
        "policy-evolve needs at least one frozen opponent to measure against"
    );
    let ladder: Vec<&CompiledPolicy> = ladder_policies.iter().collect();
    let mut rules = rules_of(&current, args.tune);

    assert!(
        !rules.tunables.is_empty(),
        "the template genome switches nothing on, so there is nothing to tune"
    );

    let mut mean = rules.started_at.clone();
    let mut sigma: Weights = rules
        .started_at
        .iter()
        .map(|weight| args.sigma * weight.abs().max(0.1))
        .collect();
    let mut sampler = WyRand::seed_from_u64(args.sampler_seed);

    let train_seeds: Vec<u64> = (0..args.games).map(|i| args.seed_base + i as u64).collect();
    // Used only to fit a newly composed reading's weight, never for the verdict.
    let screen_seeds: Vec<u64> = (0..args.screen_games)
        .map(|i| args.seed_base + SCREEN_SEED_OFFSET + i as u64)
        .collect();
    let holdout_seeds: Vec<u64> = (0..args.holdout_games)
        .map(|i| args.holdout_seed_base + i as u64)
        .collect();

    eprintln!(
        "Tuning {} of {} rules for {} generations, {} games each, {} players, {} threads,
         planning {} ahead over {} choices",
        rules.tunables.len(),
        rule_count() + VALUE_FEATURES.len(),
        args.generations,
        args.games,
        args.players,
        threads,
        args.plan().depth,
        args.plan().width
    );
    eprintln!("Frozen opponents: {}", args.anchors.join(", "));
    eprintln!(
        "Rules under the search: {}",
        rules
            .tunables
            .iter()
            .map(|tunable| tunable.label())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // The ladder's own first rung, measured exactly like a candidate: one seat
    // against the rest of the ladder. Nothing below it is progress, and because
    // it goes through the same seating and the same seeds, the comparison is
    // like for like.
    let baseline_rung = &ladder_policies[0];
    let baseline = evaluate(&baseline_rung, &ladder, args, &train_seeds, threads, "baseline");
    eprintln!("Baseline (the ladder's first rung, measured like a candidate): {}", baseline.summary);

    std::fs::create_dir_all(output).expect("failed to create the output directory");

    let pool = operand_pool();
    let mut best_ever: Option<Candidate> = None;
    let mut compositions: Vec<String> = current.interactions.iter().map(|gene| gene.label()).collect();

    for generation in 0..args.generations {
        let started = std::time::Instant::now();

        let mut candidates: Vec<Candidate> = Vec::with_capacity(args.population);

        // Candidate zero is the mean itself.
        let mean_policy = build(&current, &rules, &mean, &params);
        let mut mean_candidate = evaluate(&mean_policy, &ladder, args, &train_seeds, threads, "mean");
        mean_candidate.weights = mean.clone();
        candidates.push(mean_candidate);

        for index in 1..args.population {
            let weights: Weights = mean
                .iter()
                .zip(&sigma)
                .map(|(centre, spread)| {
                    (centre + sample_normal(&mut sampler, *spread)).clamp(-args.max_weight, args.max_weight)
                })
                .collect();
            let policy = build(&current, &rules, &weights, &params);
            let mut candidate = evaluate(
                &policy,
                &ladder,
                args,
                &train_seeds,
                threads,
                &format!("sample {index}"),
            );
            candidate.weights = weights;
            candidates.push(candidate);
        }

        candidates.sort_by(|a, b| {
            b.fitness()
                .partial_cmp(&a.fitness())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let elites = ((args.population as f64 * args.elite_fraction).ceil() as usize)
            .max(2)
            .min(candidates.len());
        let (new_mean, new_sigma) = refit(&candidates[..elites], &rules, args);

        let mean_row = candidates
            .iter()
            .find(|candidate| candidate.label == "mean")
            .expect("the mean genome was evaluated");
        let generation_best = &candidates[0];
        if best_ever
            .as_ref()
            .map_or(true, |best| generation_best.fitness() > best.fitness())
        {
            best_ever = Some(Candidate {
                label: format!("generation {generation}"),
                weights: generation_best.weights.clone(),
                summary: generation_best.summary.clone(),
                scores: generation_best.scores.clone(),
            });
        }
        let best = best_ever.as_ref().expect("set just above");

        let best_policy = build(&current, &rules, &best.weights, &params);
        let activity = policy_play::activity_of(&best_policy, args.players, 4, args.seed_base, threads);
        let holdout = if holdout_seeds.is_empty() {
            None
        } else {
            Some(
                evaluate(&best_policy, &ladder, args, &holdout_seeds, threads, "holdout").summary,
            )
        };

        // Structure: mutate one composed reading and see whether the result
        // beats the incumbent's own numbers by more than its error bars.
        let mut adopted: Option<(PolicyGenome, Rules, Weights)> = None;
        let mut mutation_rows: Vec<String> = Vec::new();
        if args.mutations > 0 && !pool.is_empty() {
            let seed_genome = genome_with(&current, &rules, &new_mean);
            // A composition entering at a weight nobody chose is not that
            // composition at its best, so it is fitted first — on a separate
            // set of seeds, so the verdict that follows is not a best-of-seven
            // dressed up as a single test.
            let screen_baseline = evaluate(
                &build(&current, &rules, &new_mean, &params),
                &ladder,
                args,
                &screen_seeds,
                threads,
                "screen",
            );
            for attempt in 0..args.mutations {
                let mut mutant = seed_genome.clone();
                let (change, moved) = mutate_compositions(&mut mutant.interactions, &mut sampler, &pool);

                // Fit the weight of whatever just changed.
                if let Some(index) = moved {
                    let mut best_fit = mutant.interactions[index].weight;
                    let mut best_screen = f64::NEG_INFINITY;
                    for trial in WEIGHT_TRIALS {
                        let mut candidate = mutant.clone();
                        candidate.interactions[index].weight = trial;
                        let rules_for = rules_of(&candidate, args.tune);
                        let policy = build(&candidate, &rules_for, &rules_for.started_at, &params);
                        let scored = evaluate(
                            &policy,
                            &ladder,
                            args,
                            &screen_seeds,
                            threads,
                            "screen trial",
                        );
                        let (gain, _) = paired_gain(&screen_baseline.scores, &scored.scores);
                        if gain > best_screen {
                            best_screen = gain;
                            best_fit = trial;
                        }
                    }
                    mutant.interactions[index].weight = best_fit;
                }

                let mutant_rules = rules_of(&mutant, args.tune);
                let mutant_weights = mutant_rules.started_at.clone();
                let policy = build(&mutant, &mutant_rules, &mutant_weights, &params);
                let outcome = evaluate(
                    &policy,
                    &ladder,
                    args,
                    &train_seeds,
                    threads,
                    &format!("mutant {attempt}"),
                );
                // Paired against the mean genome, game by game: both were played
                // on the same seeds, so a game that dealt well to one dealt well
                // to the other and the difference is what the change did.
                //
                // The bar leans towards accepting, because the two mistakes are
                // not the same size. A composition that is no good enters with a
                // fitted weight near zero and gets driven to zero by the weight
                // search, which costs a little compute; a good composition that
                // is rejected is gone, and no later generation will propose it
                // again. `paired_gain` reports two standard errors, so this
                // divides by two and applies the configured bars.
                let (gain, two_errors) = paired_gain(&mean_row.scores, &outcome.scores);
                let bars = two_errors / 2.0 * args.mutant_bars;
                let keep = gain > bars;
                mutation_rows.push(format!(
                    "{change}: {gain:+.3} vs bars {bars:.3}{}",
                    if keep { " — adopted" } else { "" }
                ));
                if keep {
                    adopted = Some((mutant, mutant_rules, mutant_weights));
                    break;
                }
            }
        }
        if let Some((mutant, _, mutant_weights)) = &adopted {
            compositions = mutant.interactions.iter().map(|gene| gene.label()).collect();
            mean = mutant_weights.clone();
            current = mutant.clone();
            rules = rules_of(&current, args.tune);
            sigma = rules
                .started_at
                .iter()
                .map(|weight| args.sigma * weight.abs().max(0.1))
                .collect();
        } else {
            mean = new_mean;
            sigma = new_sigma;
        }

        let report = GenerationReport {
            generation,
            compositions: compositions.clone(),
            structure_attempts: mutation_rows.clone(),
            games_per_candidate: args.games,
            seed_base: args.seed_base,
            population: args.population,
            mean_genome: gene_row("mean", mean_row, &rules),
            best_in_generation: gene_row("best in generation", generation_best, &rules),
            best_ever: gene_row(&best.label, best, &rules),
            baseline: baseline.summary.clone(),
            holdout: holdout.clone(),
            candidates: candidates
                .iter()
                .map(|candidate| gene_row(&candidate.label, candidate, &rules))
                .collect(),
            weights: rules
                .tunables
                .iter()
                .zip(&rules.started_at)
                .enumerate()
                .map(|(index, (tunable, start))| WeightRow {
                    feature: tunable.label(),
                    mean: mean[index],
                    sigma: sigma[index],
                    started_at: *start,
                })
                .collect(),
            activity,
            wall_clock_secs: started.elapsed().as_secs_f64(),
        };

        let path = format!("{output}/generation-{generation:03}.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&report).expect("a generation report serializes"),
        )
        .expect("failed to write the generation report");

        eprintln!(
            "gen {generation:>3}  mean {:.2}±{:.2}  best {:.2}±{:.2}  ever {:.2}{}  {:.0}s",
            mean_row.summary.mean,
            mean_row.summary.std_error,
            generation_best.summary.mean,
            generation_best.summary.std_error,
            best.summary.mean,
            match &holdout {
                Some(holdout) => format!("  holdout {:.2}±{:.2}", holdout.mean, holdout.std_error),
                None => String::new(),
            },
            report.wall_clock_secs,
        );
        for row in &mutation_rows {
            eprintln!("         composed: {row}");
        }

    }

    let best = best_ever.expect("at least one generation ran");
    let genome = genome_with(&current, &rules, &best.weights);
    let path = format!("{output}/best-genome.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&genome).expect("a genome serializes"),
    )
    .expect("failed to write the best genome");

    eprintln!();
    eprintln!("Best: {} — {}", best.label, best.summary);
    eprintln!("Baseline was: {}", baseline.summary);
    eprintln!("Best genome written to {path}");
}

fn rules_of(template: &PolicyGenome, scope: crate::cli::TuneScope) -> Rules {
    let mut names: Vec<Tunable> = Vec::new();
    let mut shapes = Vec::new();
    let mut started_at = Vec::new();
    let values_only = scope == crate::cli::TuneScope::Values;
    // Walk the feature tables rather than the genome's map so that the search's
    // vector order is the same in every run of the same template. Choice rules
    // first, then the value rules a planned decision scores its leaves with.
    for def in features() {
        if values_only {
            break;
        }
        let Some(gene) = template.features.get(def.name) else {
            continue;
        };
        if gene.weight == 0.0 {
            continue;
        }
        names.push(Tunable::rule(def.name));
        shapes.push(gene.shape);
        started_at.push(gene.weight);
    }
    for def in VALUE_FEATURES {
        let Some(gene) = template.features.get(def.name) else {
            continue;
        };
        if gene.weight == 0.0 {
            continue;
        }
        names.push(Tunable::rule(def.name));
        shapes.push(gene.shape);
        started_at.push(gene.weight);
    }
    // Then each regime's own weights, in gate order then table order.
    for gate in Gate::ALL {
        if values_only {
            break;
        }
        let Some(rules) = template.regimes.get(gate.name()) else {
            continue;
        };
        for def in features() {
            let Some(gene) = rules.get(def.name) else {
                continue;
            };
            names.push(Tunable::regime(gate, def.name));
            shapes.push(gene.shape);
            started_at.push(gene.weight);
        }
    }
    for (index, gene) in template.interactions.iter().enumerate() {
        if values_only {
            break;
        }
        names.push(Tunable::composed(index, gene.label()));
        shapes.push(gene.shape);
        started_at.push(gene.weight);
    }
    for name in template.features.keys() {
        assert!(
            features().any(|def| def.name == name)
                || VALUE_FEATURES.iter().any(|def| def.name == name),
            "the template names {name:?}, which is not a feature"
        );
    }
    for (gate, rules) in &template.regimes {
        assert!(
            Gate::from_name(gate).is_some(),
            "the template names {gate:?}, which is not a regime"
        );
        for name in rules.keys() {
            assert!(
                features().any(|def| def.name == name)
                    || VALUE_FEATURES.iter().any(|def| def.name == name),
                "a regime names {name:?}, which is not a feature"
            );
        }
    }
    Rules {
        tunables: names,
        shapes,
        started_at,
    }
}

fn genome_with(template: &PolicyGenome, rules: &Rules, weights: &[f64]) -> PolicyGenome {
    let mut genome = template.clone();
    if !genome.description.ends_with("(tuned)") {
        genome.description = format!("{} (tuned)", template.description);
    }
    for ((tunable, shape), weight) in rules.tunables.iter().zip(&rules.shapes).zip(weights) {
        if let Some((index, _)) = &tunable.composed {
            genome.interactions[*index].weight = *weight;
            genome.interactions[*index].shape = *shape;
            continue;
        }
        match tunable.gate {
            Some(gate) => genome.set_regime_shaped(gate, tunable.name, *weight, *shape),
            None => genome.set_shaped(tunable.name, *weight, *shape),
        };
    }
    genome
}

fn build(
    template: &PolicyGenome,
    rules: &Rules,
    weights: &[f64],
    params: &HeuristicParams,
) -> CompiledPolicy {
    let genome = genome_with(template, rules, weights);
    CompiledPolicy::from_genome(&genome, params).unwrap_or_else(|error| {
        panic!(
            "a sampled genome will not compile: {error}. Weights were {}",
            weights
                .iter()
                .zip(&rules.tunables)
                .map(|(weight, tunable)| format!("{}={weight:.4}", tunable.label()))
                .collect::<Vec<_>>()
                .join(" ")
        )
    })
}

/// Play one seat against the frozen ladder and summarise the scores.
fn evaluate(
    policy: &CompiledPolicy,
    ladder: &[&CompiledPolicy],
    args: &PolicyEvolveArgs,
    seeds: &[u64],
    threads: usize,
    label: &str,
) -> Candidate {
    let outcomes = policy_play::play_batch(
        policy,
        Some(ladder),
        args.players,
        seeds,
        threads,
        policy_play::Gameplay {
            record: policy_play::Record::Nothing,
            plan: args.plan(),
        },
    );
    let batch = policy_play::Batch::of(&outcomes, args.players, false);
    let summary = ScoreSummary::summarize(&batch, &outcomes, args.players, false);
    Candidate {
        label: label.to_string(),
        weights: Vec::new(),
        summary,
        scores: batch.scores.clone(),
    }
}

/// Refit the sampling distribution to the elite: mean and spread per gene.
fn refit(elite: &[Candidate], rules: &Rules, args: &PolicyEvolveArgs) -> (Weights, Weights) {
    let count = elite.len().max(1) as f64;
    let mut mean = vec![0.0; rules.tunables.len()];
    for candidate in elite {
        for (slot, weight) in mean.iter_mut().zip(&candidate.weights) {
            *slot += weight / count;
        }
    }
    let mut sigma = vec![0.0; rules.tunables.len()];
    for candidate in elite {
        for (slot, (weight, centre)) in sigma.iter_mut().zip(candidate.weights.iter().zip(&mean)) {
            *slot += (weight - centre).powi(2) / count;
        }
    }
    for slot in sigma.iter_mut() {
        // A spread that collapses to nothing stops the search; the floor keeps
        // it probing.
        *slot = slot.sqrt().max(args.sigma_floor);
    }
    for weight in mean.iter_mut() {
        *weight = weight.clamp(-args.max_weight, args.max_weight);
    }
    (mean, sigma)
}

fn gene_row(label: &str, candidate: &Candidate, rules: &Rules) -> GeneRow {
    let weights = rules
        .tunables
        .iter()
        .zip(&candidate.weights)
        .map(|(tunable, weight)| (tunable.label(), *weight))
        .collect();
    GeneRow {
        label: label.to_string(),
        summary: candidate.summary.clone(),
        weights,
    }
}

/// The mean per-game gain of one set of scores over another, with the bar it has
/// to clear: twice the standard error of the paired difference.
fn paired_gain(baseline: &[f64], other: &[f64]) -> (f64, f64) {
    let n = baseline.len().min(other.len());
    if n < 2 {
        return (f64::NEG_INFINITY, f64::INFINITY);
    }
    let differences: Vec<f64> = (0..n).map(|game| other[game] - baseline[game]).collect();
    let mean = differences.iter().sum::<f64>() / n as f64;
    let variance = differences
        .iter()
        .map(|difference| (difference - mean).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    (mean, 2.0 * variance.sqrt() / (n as f64).sqrt())
}

/// Every name a composed reading may draw on: the choice rules and the scalars
/// the position supplies. Value rules are excluded — they read a position with no
/// choice in hand, so they cannot be one half of something about a choice.
fn operand_pool() -> Vec<&'static str> {
    let mut pool: Vec<&'static str> = features().map(|def| def.name).collect();
    pool.extend(
        colori_core::policy::interactions::SCALARS
            .iter()
            .map(|(name, _)| *name),
    );
    pool
}

/// The most composed readings the search will hold at once.
const MAX_COMPOSITIONS: usize = 14;

/// A separate seed range for fitting a new composition's weight, so that the
/// verdict that follows is not a best-of-N dressed up as a single test.
const SCREEN_SEED_OFFSET: u64 = 1_000_000;

/// The weights a newly composed reading is tried at before it is judged.
const WEIGHT_TRIALS: [f64; 5] = [-2.0, -0.5, 0.5, 1.0, 2.0];

/// Change one composed reading: its operator, one of its operands, or how many
/// there are. Returns a description for the report.
fn mutate_compositions(
    genes: &mut Vec<InteractionGene>,
    rng: &mut WyRand,
    pool: &[&'static str],
) -> (String, Option<usize>) {
    let draw = |rng: &mut WyRand| pool[rng.random_range(0..pool.len())];
    let draw_op = |rng: &mut WyRand| Op::ALL[rng.random_range(0..Op::ALL.len())];

    let roll: f64 = rng.random();
    if genes.is_empty() || (roll < 0.2 && genes.len() < MAX_COMPOSITIONS) {
        let gene = InteractionGene::new(draw_op(rng), draw(rng), draw(rng), 0.2);
        let label = gene.label();
        genes.push(gene);
        return (format!("added {label}"), Some(genes.len() - 1));
    }
    if genes.len() > 1 && roll < 0.32 {
        let index = rng.random_range(0..genes.len());
        let label = genes.remove(index).label();
        return (format!("dropped {label}"), None);
    }
    let index = rng.random_range(0..genes.len());
    let before = genes[index].label();
    // Whatever is changed has to actually change: a mutation that draws the same
    // operand spends an evaluation on nothing.
    for _ in 0..8 {
        match rng.random_range(0..3) {
            0 => genes[index].op = draw_op(rng),
            1 => genes[index].a = draw(rng).to_string(),
            _ => genes[index].b = draw(rng).to_string(),
        }
        if genes[index].label() != before {
            break;
        }
    }
    (format!("{before} -> {}", genes[index].label()), Some(index))
}

fn sample_normal(rng: &mut WyRand, scale: f64) -> f64 {
    // Box-Muller. `u1` stays off zero so the log remains finite.
    let u1: f64 = rng.random::<f64>().max(1e-12);
    let u2: f64 = rng.random::<f64>();
    scale * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}
