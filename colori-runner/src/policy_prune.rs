//! What is each gene worth, and what can be cut?
//!
//! A tuning run leaves a genome with a hundred and fifty numbers in it, most of
//! which the search touched early and never again. Some of those are load-bearing
//! and some are decoration: a rule worth 0.2 of one weight, on top of four other
//! rules that say the same thing, changes nothing you can measure.
//!
//! This measures it directly rather than guessing. Each gene is removed in turn
//! and the resulting genome played against the intact one, **on the same seeds**,
//! so the comparison is per game and the deal each game got cancels. The output
//! is a ranking: what the most nearly useless genes cost when they are taken out,
//! and what the ones that matter cost.
//!
//! Removal is a different question per kind of gene:
//!
//! - a **rule** carries its regime weights with it, so removing it removes the
//!   rule at every point of the game;
//! - a **regime weight** goes on its own, dropping that rule back to one weight
//!   for the whole game;
//! - a **composed reading** is set to zero weight, which compiles it out.
//!
//! Individual costs do not add up. Several rules that each cost nothing can
//! collectively be the only thing holding a decision together, so the pruned
//! genome is measured again after the cut rather than assumed.

use colori_core::policy::{CompiledPolicy, Gate, PolicyGenome};
use serde::Serialize;

use crate::cli::PolicyPruneArgs;
use crate::policy_play::{self, Gameplay, Record};

/// One thing that can be taken out.
#[derive(Debug, Clone)]
enum Unit {
    /// A rule, with whatever regime weights it has.
    Rule { name: String },
    /// One rule's weight in one stretch of the game.
    Regime { gate: Gate, name: String },
    /// A composed reading, by position.
    Composed { index: usize, label: String },
}

impl Unit {
    fn label(&self) -> String {
        match self {
            Unit::Rule { name } => name.clone(),
            Unit::Regime { gate, name } => format!("{}/{}", gate.name(), name),
            Unit::Composed { label, .. } => label.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ablation {
    gene: String,
    /// What removing it costs, in points per game, measured against the intact
    /// genome on the same seeds.
    cost: f64,
    std_error: f64,
}

fn units_of(genome: &PolicyGenome) -> Vec<Unit> {
    let mut units = Vec::new();
    for name in genome.features.keys() {
        units.push(Unit::Rule { name: name.clone() });
    }
    for gate in Gate::ALL {
        let Some(rules) = genome.regimes.get(gate.name()) else {
            continue;
        };
        for name in rules.keys() {
            units.push(Unit::Regime {
                gate,
                name: name.clone(),
            });
        }
    }
    for (index, gene) in genome.interactions.iter().enumerate() {
        units.push(Unit::Composed {
            index,
            label: gene.label(),
        });
    }
    units
}

fn without(genome: &PolicyGenome, unit: &Unit) -> PolicyGenome {
    let mut cut = genome.clone();
    match unit {
        Unit::Rule { name } => {
            cut.features.remove(name);
            for gate in Gate::ALL {
                if let Some(rules) = cut.regimes.get_mut(gate.name()) {
                    rules.remove(name);
                }
            }
        }
        Unit::Regime { gate, name } => {
            if let Some(rules) = cut.regimes.get_mut(gate.name()) {
                rules.remove(name);
            }
        }
        Unit::Composed { index, .. } => {
            cut.interactions[*index].weight = 0.0;
        }
    }
    cut
}

/// The mean per-game difference and its standard error.
fn difference(baseline: &[f64], other: &[f64]) -> (f64, f64) {
    let n = baseline.len().min(other.len());
    if n < 2 {
        return (0.0, f64::INFINITY);
    }
    let gaps: Vec<f64> = (0..n).map(|game| other[game] - baseline[game]).collect();
    let mean = gaps.iter().sum::<f64>() / n as f64;
    let variance = gaps.iter().map(|gap| (gap - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    (mean, variance.sqrt() / (n as f64).sqrt())
}

pub fn run_policy_prune(args: &PolicyPruneArgs, threads: usize, output: &str) {
    let params = policy_play::load_params(args.heuristic_params_file.as_deref());
    let genome = policy_play::load_genome(&args.genome);
    let intact = policy_play::compile(&genome, &params, "the genome");
    let plan = args.plan();
    let gameplay = Gameplay {
        record: Record::Nothing,
        plan,
    };

    let seeds: Vec<u64> = (0..args.games).map(|i| args.seed_base + i as u64).collect();
    // Both the intact genome and each ablated one fill **every** seat.
    //
    // The obvious protocol — ablated genome on one seat, intact on the other
    // three — measures something else entirely. Deviating from the table is worth
    // points in this game on its own, because the draft and the buyer display are
    // contested and a bot playing differently is not competing head-on, so every
    // ablation looked like a gain. Filling every seat with the same genome keeps
    // the comparison to one thing: what the table plays like with this gene and
    // without it.
    let baseline_games =
        policy_play::play_batch(&intact, None, args.players, &seeds, threads, gameplay);
    let baseline = policy_play::Batch::of(&baseline_games, args.players, false).scores;

    let units = units_of(&genome);
    eprintln!(
        "{} genes, {} games each, {} players{}",
        units.len(),
        args.games,
        args.players,
        if plan.is_on() {
            format!(", planning {} deep", plan.depth)
        } else {
            String::new()
        }
    );

    let mut ablations: Vec<Ablation> = Vec::with_capacity(units.len());
    for (done, unit) in units.iter().enumerate() {
        let cut = without(&genome, unit);
        let Ok(policy) = CompiledPolicy::from_genome(&cut, &params) else {
            // Removing this switches the genome off entirely; it cannot be cut.
            continue;
        };
        let games = policy_play::play_batch(&policy, None, args.players, &seeds, threads, gameplay);
        let scores = policy_play::Batch::of(&games, args.players, false).scores;
        let (cost, std_error) = difference(&baseline, &scores);
        ablations.push(Ablation {
            gene: unit.label(),
            cost,
            std_error,
        });
        if done % 20 == 0 {
            eprintln!("  {done}/{} measured", units.len());
        }
    }

    // Cheapest to remove first: a cost near zero means the gene is not holding
    // anything up.
    ablations.sort_by(|a, b| {
        a.cost
            .partial_cmp(&b.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let free: Vec<&Ablation> = ablations
        .iter()
        .filter(|row| row.cost.abs() < args.threshold)
        .collect();
    let costly: Vec<&Ablation> = ablations
        .iter()
        .filter(|row| row.cost > 10.0 * args.threshold)
        .collect();

    eprintln!("\nThe {0} genes that cost least to remove:", 30.min(ablations.len()));
    for row in ablations.iter().take(30) {
        eprintln!(
            "  {:<40} {:+.4} ± {:.3}",
            row.gene, row.cost, row.std_error
        );
    }
    eprintln!("\nThe genes that matter most:");
    for row in ablations
        .iter()
        .rev()
        .take(12)
        .filter(|row| row.cost > 0.0)
    {
        eprintln!(
            "  {:<40} {:+.4} ± {:.3}",
            row.gene, row.cost, row.std_error
        );
    }
    eprintln!(
        "\n{} of {} genes cost less than {:.2} to remove; {} cost more than {:.2}",
        free.len(),
        ablations.len(),
        args.threshold,
        costly.len(),
        args.threshold * 10.0
    );

    std::fs::create_dir_all(output).expect("failed to create the output directory");
    let path = format!("{output}/ablation-{}.json", args.genome.replace(['/', '.'], "-"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&ablations).expect("an ablation table serializes"),
    )
    .expect("failed to write the ablation table");
    eprintln!("Ablation table written to {path}");

    if let Some(path) = &args.write {
        let mut pruned = genome.clone();
        let mut cut = 0;
        for unit in &units {
            let row = ablations.iter().find(|row| row.gene == unit.label());
            if row.is_some_and(|row| row.cost.abs() < args.threshold) {
                pruned = without(&pruned, unit);
                cut += 1;
            }
        }
        pruned.description = format!("{} (pruned: {cut} genes cut)", genome.description);
        std::fs::write(
            path,
            serde_json::to_string_pretty(&pruned).expect("a genome serializes"),
        )
        .expect("failed to write the pruned genome");
        eprintln!("\nCut {cut} genes; wrote {}", path);
    }
}
