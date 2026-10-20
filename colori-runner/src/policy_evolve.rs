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
    features, rule_count, CompiledPolicy, Gate, PolicyGenome, Shape, VALUE_FEATURES,
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
    let ladder_policies = policy_play::compile_ladder(&args.anchors, &params);
    assert!(
        !ladder_policies.is_empty(),
        "policy-evolve needs at least one frozen opponent to measure against"
    );
    let ladder: Vec<&CompiledPolicy> = ladder_policies.iter().collect();
    let rules = rules_of(&template);

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

    let mut best_ever: Option<Candidate> = None;

    for generation in 0..args.generations {
        let started = std::time::Instant::now();

        let mut candidates: Vec<Candidate> = Vec::with_capacity(args.population);

        // Candidate zero is the mean itself.
        let mean_policy = build(&template, &rules, &mean, &params);
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
            let policy = build(&template, &rules, &weights, &params);
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
            });
        }
        let best = best_ever.as_ref().expect("set just above");

        let best_policy = build(&template, &rules, &best.weights, &params);
        let activity = policy_play::activity_of(&best_policy, args.players, 4, args.seed_base, threads);
        let holdout = if holdout_seeds.is_empty() {
            None
        } else {
            Some(
                evaluate(&best_policy, &ladder, args, &holdout_seeds, threads, "holdout").summary,
            )
        };

        let report = GenerationReport {
            generation,
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
                    mean: new_mean[index],
                    sigma: new_sigma[index],
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

        mean = new_mean;
        sigma = new_sigma;
    }

    let best = best_ever.expect("at least one generation ran");
    let genome = genome_with(&template, &rules, &best.weights);
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

fn rules_of(template: &PolicyGenome) -> Rules {
    let mut names: Vec<Tunable> = Vec::new();
    let mut shapes = Vec::new();
    let mut started_at = Vec::new();
    // Walk the feature tables rather than the genome's map so that the search's
    // vector order is the same in every run of the same template. Choice rules
    // first, then the value rules a planned decision scores its leaves with.
    for def in features() {
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

fn sample_normal(rng: &mut WyRand, scale: f64) -> f64 {
    // Box-Muller. `u1` stays off zero so the log remains finite.
    let u1: f64 = rng.random::<f64>().max(1e-12);
    let u2: f64 = rng.random::<f64>();
    scale * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}
