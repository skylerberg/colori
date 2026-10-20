//! A deterministic, rules-based player: a linear policy over named features.
//!
//! # The model
//!
//! A decision offers a set of legal [`Choice`]s. The policy scores each one and
//! takes the highest score; nothing else — no search, no sampling, no
//! randomness. A game played by these policies is reproducible from its seeds
//! alone.
//!
//! The score of a choice is a weighted sum of [`features`], each of which
//! answers one question about the choice **in points**. One currency is what
//! lets the policy compare a trash against a ducat spend against ending the
//! turn; without it, per-choice-type constants drift apart and the rules stop
//! being comparable as soon as they interact.
//!
//! Writing the policy as a weighted sum rather than as nested conditionals buys
//! three things:
//!
//! - A genome is a flat table of numbers, so a tuner can search it without
//!   understanding it.
//! - A decision's score is the sum of its parts, so a trace is also an
//!   explanation: the largest contributions say what the policy was thinking.
//! - A new rule is a new [`features::FeatureDef`] and nothing else. It cannot
//!   change how existing rules behave, so it is safe to propose one from
//!   outside the loop and measure it.
//!
//! # Determinism
//!
//! Terms are evaluated in [`features::FEATURES`] order, not in the order a
//! genome file happens to list them: floating point addition is not
//! associative, so a genome that scored differently after a re-save would not
//! be deterministic. Ties go to the first choice in enumeration order, which is
//! itself deterministic.
//!
//! # Tuning
//!
//! Weights are the tunable part; a [`Shape`] applies a saturating or
//! amplifying transform to a feature before its weight, which covers most of
//! the structural variation worth having in a linear model. New *kinds* of rule
//! come from adding features, which is a code change — deliberately, so the
//! loop cannot invent a rule that outruns what the evaluator measures.

pub mod context;
pub mod effects;
pub mod features;
pub mod value;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scoring::{CardHeuristicTable, HeuristicParams};
use crate::types::{Choice, GamePhase, GameState};

pub use context::{Ctx, PhaseKind};
pub use effects::Delta;
pub use features::{features, rule_count, ChoiceVariant, FeatureDef, FeatureFn, Group, ALL_CHOICE_VARIANTS};
pub use value::{ValueDef, ValueFn, VALUE_FEATURES};

// ── Shapes ──

/// What a feature's raw value becomes before its weight is applied.
///
/// Every shape is odd (a negative raw value stays negative) except `Unit`,
/// which reports only the sign. All are zero-preserving: a feature that does
/// not apply contributes nothing whatever its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    #[default]
    Linear,
    /// `sqrt`, damped: many small gains beat one large one.
    Sqrt,
    /// Squared, amplified: the large value dominates.
    Square,
    /// `ln(1 + |x|)`, strongly damped.
    Log1p,
    /// The sign alone: a pure indicator.
    Unit,
}

impl Shape {
    #[inline]
    pub fn apply(self, value: f64) -> f64 {
        if value == 0.0 || !value.is_finite() {
            return 0.0;
        }
        let sign = if value > 0.0 { 1.0 } else { -1.0 };
        match self {
            Shape::Linear => value,
            Shape::Sqrt => sign * value.abs().sqrt(),
            Shape::Square => sign * value.abs() * value.abs(),
            Shape::Log1p => sign * value.abs().ln_1p(),
            Shape::Unit => sign,
        }
    }
}

// ── Genome ──

/// One tunable number, plus how its feature is shaped before it counts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureGene {
    pub weight: f64,
    #[serde(default)]
    pub shape: Shape,
}

/// A complete rule system: a weight for every feature that is switched on.
///
/// A feature absent from `features` is switched off, which is the same as a
/// weight of zero but keeps a genome readable as the rules it actually uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyGenome {
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub features: BTreeMap<String, FeatureGene>,
}

impl PolicyGenome {
    pub const VERSION: u32 = 1;

    pub fn new(description: impl Into<String>) -> Self {
        PolicyGenome {
            version: Self::VERSION,
            description: description.into(),
            features: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, weight: f64) -> &mut Self {
        self.set_shaped(name, weight, Shape::Linear)
    }

    pub fn set_shaped(&mut self, name: &str, weight: f64, shape: Shape) -> &mut Self {
        self.features.insert(name.to_string(), FeatureGene { weight, shape });
        self
    }

    pub fn weight(&self, name: &str) -> f64 {
        self.features.get(name).map_or(0.0, |gene| gene.weight)
    }

    /// The genome a tuning run starts from: one coherent set of rules, hand-set
    /// rather than trained, and only a starting point for the search.
    ///
    /// Every weight is a guess at what a unit of its feature is worth in points.
    /// A point is a point, so the point-valued features start at 1; storage of
    /// something no buyer wants yet starts low; completing a sale starts above
    /// the sum of its parts, because the last unit is what pays.
    pub fn starter() -> Self {
        let mut genome = PolicyGenome::new("starter: hand-set rules, untuned");
        for (name, weight) in STARTER_WEIGHTS {
            genome.set(name, *weight);
        }
        genome
    }
}

/// The starter rules, as (feature, weight).
///
/// Kept as a table rather than as a JSON file so that a typo in a feature name
/// fails the test that reads it instead of a tuning run hours later.
const STARTER_WEIGHTS: &[(&str, f64)] = &[
    // Value in points.
    ("points_immediate", 1.0),
    ("pending_points_share", 1.0),
    ("ducats_spent", 1.0),
    // Colour and material, priced by what the sitting buyers want.
    ("colors_demanded", 1.0),
    ("colors_undemanded", 0.15),
    ("colors_spent_demanded", 1.0),
    ("colors_spent", 0.2),
    ("materials_demanded", 1.0),
    ("materials_undemanded", 0.15),
    ("materials_spent", 1.0),
    ("materials_spent_raw", 0.2),
    ("boats_spent", 1.0),
    ("wilds_spent", 1.0),
    // Cards.
    ("card_quality_gained", 0.5),
    // A card's quality is another bot's hand-strength estimate, not points, so
    // it is a weak stand-in for what trashing really gives up.
    ("trash_quality", 0.15),
    // Abilities.
    ("ability_workshop", 0.5),
    // A mix ability on its own can be one the wheel cannot feed, so it is
    // worth less than a mix actually made.
    ("ability_mix", 0.4),
    ("ability_sell", 0.9),
    // An ability is worth what it can bank, so a Sell is priced by the buyer it
    // would sell to rather than by a constant.
    ("ability_sell_priced", 1.0),
    ("ability_ducats", 1.0),
    // Taking a card is what a Gain Secondary turns into once trashed.
    ("ability_secondary", 0.8),
    ("ability_wilds", 1.0),
    ("ability_boats", 1.0),
    ("ability_move_to_pool", 0.3),
    // The plan.
    ("color_diversity_delta", 0.4),
    ("buyer_progress_points", 0.6),
    ("buyer_completion_points", 1.5),
    // Draft: take what the buyers want, without piling up the same ability.
    // The two halves of a card: what it stores if kept, what it does if
    // trashed. The action half is priced by the ability rules.
    ("card_workshop_value", 1.0),
    ("card_workshop_value_long", 1.0),
    ("draft_color_demand", 0.5),
    ("draft_material_demand", 0.8),
    // Abilities are priced by kind through the ability rules; a flat count on
    // top of them would argue with them.
    ("draft_ability_count", 0.0),
    ("draft_redundancy", -0.3),
    // Selling.
    ("sell_points", 1.0),
    ("sell_points_per_unit", 0.5),
    ("sell_boat_cost", 0.6),
    // The buyers phase: prefer what can be sold soon, but do not fear the deck
    // outright.
    ("offer_points", 1.0),
    ("offer_affordable", 0.8),
    ("offer_distance", 0.5),
    ("offer_deck_gamble", -0.2),
    // Mixing. A mix's inputs are already priced by `colors_spent_demanded`, so
    // the extras here are small: burning a primary costs a little flexibility,
    // and using a granted mix at all is worth more than the grant was.
    ("mix_output_demanded", 0.9),
    ("mix_higher_tier", 0.3),
    ("mix_burns_primaries", 0.1),
    ("mix_count", 0.6),
    ("mix_grant_unused", 0.8),
    // Workshop.
    ("workshop_set_size", 0.1),
    // Giving up a card's workshop ability, which fires every round the card
    // comes back round: this is what makes keeping an action card worth more
    // than its cheap trash ability.
    ("forgone_ducats", 1.5),
    ("forgone_wilds", 0.8),
    ("forgone_boats", 0.5),
    ("forgone_secondary", 0.6),
    ("forgone_mix", 0.4),
    // Ending the turn. These two are negative on purpose: ending the turn
    // *while* cards and actions are still open is what should cost, so the
    // weight prices leaving work undone rather than rewarding it.
    ("end_turn_unspent_cards", -0.6),
    ("end_turn_open_actions", -0.5),
    ("end_turn_kept_value", 1.0),
    ("end_turn_late_game", 1.5),
    // ── The second vocabulary ──
    // Hand-set priors for rules that read the position more sharply. They are
    // guesses at magnitudes, not at direction: the search is free to invert any
    // of them, and several were inverted after the first run.
    ("colours_completing", 1.0),
    ("colours_shared_demand", 0.5),
    ("colours_tertiary_gained", 0.8),
    ("colours_secondary_gained", 0.5),
    ("colours_primary_spent", 0.3),
    ("colours_duplicate_gained", -0.1),
    ("colours_breadth_gained", 0.2),
    ("wheel_mixing_options_delta", 0.3),
    ("materials_completing", 1.0),
    ("materials_shared_demand", 0.5),
    ("materials_surplus_gained", -0.2),
    ("materials_diversity_gained", 0.2),
    ("materials_boat_relief", 0.6),
    ("ducats_options", 0.1),
    ("buyer_short_one_filled", 2.0),
    ("buyer_affordable_delta", 1.0),
    ("buyer_completion_efficiency", 1.0),
    ("buyer_slot_freed", -0.3),
    ("buyer_stacked_demand", 0.5),
    ("sell_net_opportunity_cost", 1.0),
    ("sell_is_the_best_affordable", 0.5),
    ("offer_material_alignment", 0.5),
    ("offer_color_alignment", 0.4),
    ("offer_blind_cost", 0.3),
    ("spare_mixer", 0.2),
    ("draft_fills_gap", 0.6),
    ("draft_redundant_supply", 0.3),
    ("draft_completes_an_order", 0.8),
    ("draft_ability_novelty", 0.3),
    ("draft_no_demand", 0.4),
    ("card_serves_best_buyer", 0.5),
    ("picks_that_store", 0.5),
    ("ability_workshop_priced", 0.3),
    ("ability_mix_priced", 0.3),
    ("ability_move_to_pool_priced", 0.2),
    ("ability_secondary_priced", 0.5),
    ("ability_wilds_priced", 0.5),
    ("ability_boats_priced", 0.5),
    ("ability_sell_ready", 0.5),
    ("points_immediate_late", 0.5),
    ("colours_demanded_late", 0.5),
    ("materials_demanded_late", 0.5),
    ("keep_value_early", 0.5),
    // ── Reading something specific rather than something total ──
    ("textiles_gained", 0.6),
    ("ceramics_gained", 0.6),
    ("paintings_gained", 0.6),
    ("textiles_spent", 0.6),
    ("ceramics_spent", 0.6),
    ("paintings_spent", 0.6),
    ("textiles_completing", 0.8),
    ("ceramics_completing", 0.8),
    ("paintings_completing", 0.8),
    ("primary_demanded", 0.4),
    ("secondary_demanded", 0.4),
    ("tertiary_demanded", 0.6),
    ("sell_dear_buyer", 0.5),
    ("sell_filler_buyer", 0.2),
    ("dear_buyers_shortfall", 0.4),
    ("dear_buyer_points_unlocked", 0.5),
    ("offer_dear_buyer", 0.4),
    ("mix_completes_a_buyer", 0.6),
    ("mix_reaches_tertiary", 0.4),
    ("offer_cheap_to_finish", 0.4),
    ("offer_boat_demand", 0.3),
    ("end_turn_lost_storage", 0.5),
    // The value function a planned decision scores its leaves with. A position
    // is worth the points it has banked, plus a fraction of the plan it is on
    // and the resources it is holding to get there.
    // Scaled so that a whole position is worth a few points — the order of a
    // choice's own score — rather than the twenty-odd a game totals. The search
    // moves them from here.
    ("value_banked", 0.10),
    ("value_pending", 0.03),
    ("value_affordable", 0.05),
    ("value_stored_colors", 0.06),
    ("value_stored_materials", 0.06),
    ("value_loose_colors", 0.01),
    ("value_loose_materials", 0.01),
    ("value_wilds", 0.05),
    ("value_boats", 0.05),
    ("value_engine", 0.05),
    ("value_hand", 0.0),
    ("value_drafted", 0.0),
];

// ── Errors ──

#[derive(Debug)]
pub enum GenomeError {
    /// A weight named a feature this build does not have. Usually a genome from
    /// a newer build, or a typo.
    UnknownFeature(String),
    /// A weight that is not a number: a tuner that drifted out of range.
    NonFiniteWeight { feature: String, weight: f64 },
    /// A genome that switches nothing on would score every choice zero and take
    /// the first one offered. Refusing is friendlier than playing like that.
    NothingEnabled,
}

impl std::fmt::Display for GenomeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenomeError::UnknownFeature(name) => {
                write!(f, "no feature named {name:?} (known: {})", known_feature_names())
            }
            GenomeError::NonFiniteWeight { feature, weight } => {
                write!(f, "weight for {feature:?} is {weight}, which is not a number")
            }
            GenomeError::NothingEnabled => write!(f, "the genome enables no features"),
        }
    }
}

impl std::error::Error for GenomeError {}

fn known_feature_names() -> String {
    features()
        .map(|def| def.name)
        .chain(VALUE_FEATURES.iter().map(|def| def.name))
        .collect::<Vec<_>>()
        .join(", ")
}

// ── Compiled policy ──

/// One feature switched on, with the weight and shape a genome gave it.
#[derive(Debug, Clone, Copy)]
pub struct Term {
    pub name: &'static str,
    pub group: Group,
    pub weight: f64,
    pub shape: Shape,
    /// The choice variants this term answers for, one bit each. Every rule is
    /// asked about every choice otherwise, and most of a decision's cost is
    /// rules that could only ever answer zero.
    pub mask: u32,
    eval: FeatureFn,
}

impl Term {
    /// What this term contributes to the score of `choice`.
    #[inline]
    pub fn contribution(&self, ctx: &Ctx, choice: &Choice, delta: &Delta) -> f64 {
        self.shape.apply((self.eval)(ctx, choice, delta)) * self.weight
    }
}

/// One value rule switched on.
#[derive(Debug, Clone, Copy)]
pub struct ValueTerm {
    pub name: &'static str,
    pub weight: f64,
    pub shape: Shape,
    eval: ValueFn,
}

/// How far a planned decision looks ahead over the current turn.
///
/// Planning is not a search of the whole game — it cannot be, and it does not
/// need to be. Almost every sale and every storage play is *two or three
/// decisions long* inside one turn, and a one-ply policy cannot see past the
/// first of them. Following the best few choices a couple of decisions deeper,
/// and scoring the position they leave, is where that structure gets looked at.
///
/// `width` is only spent at the root; past it the plan follows the single best
/// choice, so cost grows with `width * depth` rather than with its product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// How many of the best-scoring choices to follow.
    pub width: usize,
    /// Decisions deep to follow them. Zero is the plain one-ply policy.
    pub depth: usize,
}

impl Plan {
    pub const NONE: Plan = Plan { width: 0, depth: 0 };

    pub const fn new(width: usize, depth: usize) -> Self {
        Plan { width, depth }
    }

    #[inline]
    pub fn is_on(self) -> bool {
        self.width > 0 && self.depth > 0
    }
}

/// A genome checked against this build, ready to play.
///
/// Holds no interior mutability and no randomness, so one policy can be shared
/// across threads or seats and is safe to reuse for a whole run.
pub struct CompiledPolicy {
    terms: Vec<Term>,
    values: Vec<ValueTerm>,
    card_table: CardHeuristicTable,
}

impl CompiledPolicy {
    pub fn from_genome(genome: &PolicyGenome, params: &HeuristicParams) -> Result<Self, GenomeError> {
        // Every named feature must exist, even the ones with zero weight: a
        // genome that mentions a rule this build lacks is a mismatch worth
        // reporting, not something to ignore.
        for (name, gene) in &genome.features {
            if features::lookup(name).is_none() && value::lookup_value(name).is_none() {
                return Err(GenomeError::UnknownFeature(name.clone()));
            }
            if !gene.weight.is_finite() {
                return Err(GenomeError::NonFiniteWeight {
                    feature: name.clone(),
                    weight: gene.weight,
                });
            }
        }

        let mut terms = Vec::with_capacity(genome.features.len());
        for def in features() {
            let Some(gene) = genome.features.get(def.name) else {
                continue;
            };
            if gene.weight == 0.0 {
                continue;
            }
            terms.push(Term {
                name: def.name,
                group: def.group,
                weight: gene.weight,
                shape: gene.shape,
                mask: def.group.mask(),
                eval: def.eval,
            });
        }

        // Value rules are optional: without planning they are never read, and a
        // policy that only values positions cannot choose anything.
        let mut values = Vec::new();
        for def in VALUE_FEATURES {
            let Some(gene) = genome.features.get(def.name) else {
                continue;
            };
            if gene.weight == 0.0 {
                continue;
            }
            values.push(ValueTerm {
                name: def.name,
                weight: gene.weight,
                shape: gene.shape,
                eval: def.eval,
            });
        }

        if terms.is_empty() {
            return Err(GenomeError::NothingEnabled);
        }

        Ok(CompiledPolicy {
            terms,
            values,
            card_table: CardHeuristicTable::new(params),
        })
    }

    /// The starter rules, compiled. Panics only if the built-in table is wrong,
    /// which [`starter_genome_is_valid`] checks in tests.
    pub fn starter(params: &HeuristicParams) -> Self {
        Self::from_genome(&PolicyGenome::starter(), params)
            .expect("the starter genome names only features this build has")
    }

    /// The choice rules this policy weighs, in evaluation order.
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// The value rules this policy weighs. Empty when it does not plan.
    pub fn value_terms(&self) -> &[ValueTerm] {
        &self.values
    }

    /// What this position is worth to `player_index`, in points.
    ///
    /// The value function a planned decision scores its leaves with.
    pub fn value(&self, state: &GameState, player_index: usize) -> f64 {
        let ctx = Ctx::new(state, player_index, &self.card_table);
        self.values
            .iter()
            .map(|term| term.shape.apply((term.eval)(&ctx)) * term.weight)
            .sum()
    }

    /// Every choice's score, in enumeration order. Allocation-free.
    fn score_all(&self, state: &GameState, player_index: usize, choices: &[Choice], out: &mut [f64]) {
        let ctx = Ctx::new(state, player_index, &self.card_table);
        for (index, choice) in choices.iter().enumerate() {
            let delta = effects::delta(ctx.player, choice, &self.card_table);
            let mut score = 0.0;
            for term in self.applicable(choice) {
                score += term.contribution(&ctx, choice, &delta);
            }
            out[index] = score;
        }
    }

    /// The terms that answer for this choice.
    #[inline]
    fn applicable<'t>(&'t self, choice: &Choice) -> impl Iterator<Item = &'t Term> {
        let bit = ChoiceVariant::of(choice).bit();
        self.terms.iter().filter(move |term| term.mask & bit != 0)
    }

    /// The card table the policy reads card quality through.
    pub fn card_table(&self) -> &CardHeuristicTable {
        &self.card_table
    }

    /// What `choice` scores, and how each feature contributed.
    ///
    /// A trace is only built for a decision worth explaining; [`decide`] is the
    /// same loop without the bookkeeping. [`CompiledPolicy::decide`] never
    /// allocates beyond the choice list it is handed.
    pub fn explain(&self, state: &GameState, player_index: usize, choice: &Choice) -> Trace {
        let ctx = Ctx::new(state, player_index, &self.card_table);
        let delta = effects::delta(ctx.player, choice, &self.card_table);
        let mut contributions = Vec::with_capacity(self.terms.len());
        let mut score = 0.0;
        for term in self.applicable(choice) {
            let raw = (term.eval)(&ctx, choice, &delta);
            let weighted = term.shape.apply(raw) * term.weight;
            score += weighted;
            contributions.push(Contribution {
                feature: term.name,
                raw,
                weighted,
            });
        }
        Trace {
            choice: *choice,
            score,
            contributions,
        }
    }

    /// Score every choice and take the best.
    ///
    /// Ties go to the earliest choice in enumeration order, which is
    /// deterministic. Returns `None` only when there is nothing to choose.
    pub fn decide(
        &self,
        state: &GameState,
        player_index: usize,
        choices: &[Choice],
    ) -> Option<Choice> {
        let (best, _score) = self.best_scored(state, player_index, choices)?;
        Some(choices[best])
    }

    /// Like [`CompiledPolicy::decide_scored`], but able to follow the best few
    /// choices a couple of decisions into the turn and score the position they
    /// leave.
    ///
    /// Falls back to the plain one-ply choice when the plan is off, when the
    /// genome weighs no value rules, or when nothing is being decided.
    pub fn decide_planned(
        &self,
        state: &GameState,
        player_index: usize,
        choices: &[Choice],
        plan: Plan,
    ) -> Option<Decision> {
        if choices.is_empty() {
            return None;
        }
        if !plan.is_on() || self.values.is_empty() {
            return self.decide_scored(state, player_index, choices);
        }

        let mut scores = vec![0.0; choices.len()];
        self.score_all(state, player_index, choices, &mut scores);
        let order = best_first(&scores);

        // What a line is worth is the choice's own score *plus* what the
        // position it leads to is worth. The first part is the tuned one-ply
        // policy and the second is the value function, so a genome whose value
        // weights are all near zero plans nothing and the policy degrades
        // exactly to its one-ply self. Planning can therefore lose strength
        // through bad tuning, but never through being switched on.
        let mut best_choice = order[0];
        let mut best_value = f64::NEG_INFINITY;
        for &index in order.iter().take(plan.width) {
            let value = scores[index] + self.follow(state, player_index, &choices[index], plan.depth);
            // Strictly greater, so on a tie the better-scoring choice keeps it.
            if value > best_value {
                best_value = value;
                best_choice = index;
            }
        }

        Some(Decision {
            choice: choices[best_choice],
            score: scores[best_choice],
            runner_up: None,
            runner_up_score: None,
            contributions: Vec::new(),
            runner_up_contributions: Vec::new(),
        })
    }

    /// Take `choice`, then follow the single best choice `depth - 1` more times,
    /// and score the position that leaves.
    ///
    /// Nothing inside a turn is random, so probing a line is exact rather than a
    /// sample — the only liberty is that a line reaching the end of the round
    /// draws its cards from a seeded generator rather than the game's.
    fn follow(&self, state: &GameState, player_index: usize, choice: &Choice, depth: usize) -> f64 {
        let mut next = state.clone();
        probe(&mut next, player_index, choice);
        if depth <= 1 || !turn_is_open(&next, player_index) {
            return self.value(&next, player_index);
        }
        let options = crate::colori_game::enumerate_choices(&next, player_index);
        if options.is_empty() {
            return self.value(&next, player_index);
        }
        let mut scores = vec![0.0; options.len()];
        self.score_all(&next, player_index, &options, &mut scores);
        // Only the best child is followed, so depth costs branchings, not
        // branchings to the power of depth.
        let best = best_first(&scores)[0];
        self.follow(&next, player_index, &options[best], depth - 1)
    }

    /// Like [`CompiledPolicy::decide`], with the score the winner took but
    /// without the per-term breakdown.
    ///
    /// This is what a game loop should call: it allocates nothing per choice.
    pub fn decide_scored(
        &self,
        state: &GameState,
        player_index: usize,
        choices: &[Choice],
    ) -> Option<Decision> {
        let (best, score) = self.best_scored(state, player_index, choices)?;
        Some(Decision {
            choice: choices[best],
            score,
            runner_up: None,
            runner_up_score: None,
            contributions: Vec::new(),
            runner_up_contributions: Vec::new(),
        })
    }

    /// Like [`CompiledPolicy::decide`], and also says what the runner-up was,
    /// how far behind it finished, and what the rules saw in each of them.
    ///
    /// Contributions are reported in full, unsorted: a caller that wants the
    /// headline terms trims with [`Trace::trim`].
    pub fn decide_with_trace(
        &self,
        state: &GameState,
        player_index: usize,
        choices: &[Choice],
    ) -> Option<Decision> {
        if choices.is_empty() {
            return None;
        }
        let ranked = self.rank(state, player_index, choices);

        // Best, then best of the rest. Strictly greater keeps the earliest
        // choice on a tie, matching `decide`.
        let mut winner = 0;
        for index in 1..ranked.len() {
            if ranked[index].score > ranked[winner].score {
                winner = index;
            }
        }
        let mut runner_up: Option<usize> = None;
        for index in 0..ranked.len() {
            if index == winner {
                continue;
            }
            match runner_up {
                Some(held) if ranked[held].score >= ranked[index].score => {}
                _ => runner_up = Some(index),
            }
        }

        let winning_trace = ranked[winner].clone();
        let runner_up = runner_up.map(|index| ranked[index].clone());

        Some(Decision {
            choice: winning_trace.choice,
            score: winning_trace.score,
            contributions: winning_trace.contributions,
            runner_up: runner_up.as_ref().map(|trace| trace.choice),
            runner_up_score: runner_up.as_ref().map(|trace| trace.score),
            runner_up_contributions: runner_up.map_or_else(Vec::new, |trace| trace.contributions),
        })
    }

    /// Every choice scored, in enumeration order.
    ///
    /// Order is preserved rather than sorted so that ties resolve the same way
    /// for callers as for [`CompiledPolicy::decide`].
    pub fn rank(&self, state: &GameState, player_index: usize, choices: &[Choice]) -> Vec<Trace> {
        let ctx = Ctx::new(state, player_index, &self.card_table);
        let mut traces = Vec::with_capacity(choices.len());
        for choice in choices {
            let delta = effects::delta(ctx.player, choice, &self.card_table);
            let mut score = 0.0;
            let mut contributions = Vec::with_capacity(self.terms.len());
            for term in self.applicable(choice) {
                let raw = (term.eval)(&ctx, choice, &delta);
                let weighted = term.shape.apply(raw) * term.weight;
                score += weighted;
                contributions.push(Contribution {
                    feature: term.name,
                    raw,
                    weighted,
                });
            }
            traces.push(Trace {
                choice: *choice,
                score,
                contributions,
            });
        }
        traces
    }

    /// The winning choice's index and score. Allocation-free, and the loop
    /// every other entry point is built on.
    fn best_scored(
        &self,
        state: &GameState,
        player_index: usize,
        choices: &[Choice],
    ) -> Option<(usize, f64)> {
        if choices.is_empty() {
            return None;
        }
        let ctx = Ctx::new(state, player_index, &self.card_table);
        let mut best_index = 0;
        let mut best_score = f64::NEG_INFINITY;
        for (index, choice) in choices.iter().enumerate() {
            let delta = effects::delta(ctx.player, choice, &self.card_table);
            let mut score = 0.0;
            for term in self.applicable(choice) {
                score += term.contribution(&ctx, choice, &delta);
            }
            // Strictly greater, so an earlier choice keeps a tie.
            if score > best_score {
                best_score = score;
                best_index = index;
            }
        }
        Some((best_index, best_score))
    }
}

/// Choices ordered best first, keeping enumeration order on a tie.
fn best_first(scores: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).collect();
    // Stable, so an equal score keeps the earlier choice.
    order.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    order
}

/// Apply one choice to a position being probed.
fn probe(state: &mut GameState, player_index: usize, choice: &Choice) {
    // Nothing inside a turn is random; the generator decides nothing.
    use rand::SeedableRng;
    let mut rng = wyrand::WyRand::seed_from_u64(0);
    crate::colori_game::apply_choice_to_state(state, player_index, choice, &mut rng);
}

/// Whether `player_index` still has decisions left in this turn.
fn turn_is_open(state: &GameState, player_index: usize) -> bool {
    matches!(
        &state.phase,
        GamePhase::Action { action_state } if !action_state.turns[player_index].ended
    )
}

/// What one feature contributed to a choice's score.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contribution {
    pub feature: &'static str,
    /// The feature's value before shaping.
    pub raw: f64,
    /// The shaped value times the weight: what it added to the score.
    pub weighted: f64,
}

/// A scored choice, with the terms that made it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    pub choice: Choice,
    pub score: f64,
    pub contributions: Vec<Contribution>,
}

impl Trace {
    /// Keep only the `keep` terms that moved the score most, largest first.
    pub fn trim(&mut self, keep: usize) {
        self.contributions
            .sort_by(|a, b| b.weighted.abs().partial_cmp(&a.weighted.abs()).unwrap_or(std::cmp::Ordering::Equal));
        self.contributions.truncate(keep);
    }
}

/// What the policy decided, and what it nearly decided instead.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub choice: Choice,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_up: Option<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_up_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributions: Vec<Contribution>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runner_up_contributions: Vec<Contribution>,
}

impl Decision {
    /// How far clear of the second choice the winner finished, or `None` when
    /// there was nothing to choose between.
    ///
    /// A small margin marks a decision worth reading in a trace: it is one the
    /// rules nearly got wrong, and the runner-up's contributions say why.
    pub fn margin(&self) -> Option<f64> {
        self.runner_up_score.map(|score| self.score - score)
    }
}

// ── Tests ──

#[cfg(test)]
pub(crate) mod test_support {
    use crate::choices::enumerate_choices;
    use crate::colori_game::apply_choice_to_state;
    use crate::deck::Deck;
    use crate::draw_phase::execute_draw_phase;
    use crate::fixed_vec::FixedVec;
    use crate::setup::create_initial_game_state;
    use crate::types::*;
    use crate::unordered_cards::UnorderedCards;
    use rand::{RngExt, SeedableRng};
    use smallvec::SmallVec;
    use wyrand::WyRand;

    /// A player holding nothing at all.
    pub fn empty_player() -> PlayerState {
        PlayerState {
            deck: Deck::new(),
            workshopped_cards: UnorderedCards::new(),
            workshop_cards: UnorderedCards::new(),
            drafted_cards: UnorderedCards::new(),
            color_wheel: ColorWheel::new(),
            materials: Materials::new(),
            buyers: FixedVec::new(),
            completed_sell_cards: SmallVec::new(),
            ducats: 0,
            wild_primaries: 0,
            boats: 0,
            cached_whole_points: 0,
        }
    }

    /// A two-player game just as the first buyers phase begins.
    pub fn game_at_buyers_phase() -> GameState {
        let mut rng = WyRand::seed_from_u64(11);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        state
    }

    /// A game where no seat is holding or owed anything: no buyers, no
    /// colours, no materials, no ducats.
    pub fn game_with_one_empty_player() -> GameState {
        let mut state = game_at_buyers_phase();
        for player in state.players.iter_mut() {
            player.buyers = FixedVec::new();
            player.completed_sell_cards = SmallVec::new();
            player.color_wheel = ColorWheel::new();
            player.materials = Materials::new();
            player.ducats = 0;
            player.wild_primaries = 0;
            player.boats = 0;
            player.cached_whole_points = 0;
        }
        state
    }

    /// The seat that owes the game a decision.
    pub fn waiting_player(state: &GameState) -> usize {
        match &state.phase {
            GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state
                .first_unended()
                .expect("the round ends as the last turn does"),
            GamePhase::Draw | GamePhase::GameOver => 0,
        }
    }

    /// States reachable by playing a few seeds out with no plan at all, so that
    /// features are checked against real hands, wheels and buyers rather than
    /// against a fixture someone had to imagine.
    pub fn reachable_states() -> Vec<GameState> {
        let mut states = Vec::new();
        for seed in 0..3u64 {
            states.extend(walk(seed));
        }
        states
    }

    /// A wider walk, for the checks that need to see rarer positions.
    pub fn reachable_states_from(first_seed: u64, steps: usize) -> Vec<GameState> {
        let mut states = Vec::new();
        for seed in first_seed..first_seed + 6 {
            states.extend(walk_from(seed, steps));
        }
        states
    }

    fn walk(seed: u64) -> Vec<GameState> {
        walk_from(seed, 200)
    }

    fn walk_from(seed: u64, steps: usize) -> Vec<GameState> {
        let mut states = Vec::new();
        {
            let mut rng = WyRand::seed_from_u64(seed * 2_000 + 1);
            let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
            execute_draw_phase(&mut state, &mut rng);
            states.push(state.clone());
            for _ in 0..steps {
                if matches!(state.phase, GamePhase::GameOver) {
                    break;
                }
                let player_index = waiting_player(&state);
                let choices = enumerate_choices(&state, player_index);
                if choices.is_empty() {
                    break;
                }
                let pick = rng.random_range(0..choices.len());
                apply_choice_to_state(&mut state, player_index, &choices[pick], &mut rng);
                states.push(state.clone());
            }
        }
        states
    }

}

#[cfg(test)]
mod tests {
    use super::test_support;
    use super::*;
    use crate::scoring::HeuristicParams;

    #[test]
    fn the_starter_genome_names_only_real_features() {
        for (name, _) in STARTER_WEIGHTS {
            assert!(
                features::lookup(name).is_some() || value::lookup_value(name).is_some(),
                "the starter rules name {name:?}, which is not a feature"
            );
        }
        let params = HeuristicParams::default();
        let policy = CompiledPolicy::starter(&params);
        assert!(policy.terms().len() > 20, "the starter rules are barely a policy");
    }

    #[test]
    fn an_unknown_feature_is_an_error_not_a_warning() {
        let mut genome = PolicyGenome::new("typo");
        genome.set("points_immediate", 1.0);
        genome.set("points_immediat", 1.0);
        let params = HeuristicParams::default();
        let error = match CompiledPolicy::from_genome(&genome, &params) {
            Ok(_) => panic!("a misspelled feature was accepted"),
            Err(error) => error,
        };
        match error {
            GenomeError::UnknownFeature(name) => assert_eq!(name, "points_immediat"),
            other => panic!("expected an unknown-feature error, got {other}"),
        }
    }

    #[test]
    fn an_empty_genome_is_refused() {
        let genome = PolicyGenome::new("nothing");
        let params = HeuristicParams::default();
        assert!(matches!(
            CompiledPolicy::from_genome(&genome, &params),
            Err(GenomeError::NothingEnabled)
        ));
    }

    #[test]
    fn a_non_finite_weight_is_refused() {
        let mut genome = PolicyGenome::new("infinite");
        genome.set("points_immediate", f64::NAN);
        let params = HeuristicParams::default();
        assert!(matches!(
            CompiledPolicy::from_genome(&genome, &params),
            Err(GenomeError::NonFiniteWeight { .. })
        ));
    }

    #[test]
    fn shapes_are_odd_zero_preserving_and_finite() {
        for shape in [Shape::Linear, Shape::Sqrt, Shape::Square, Shape::Log1p, Shape::Unit] {
            assert_eq!(shape.apply(0.0), 0.0, "{shape:?} does not preserve zero");
            assert!(shape.apply(f64::NAN) == 0.0, "{shape:?} passes NaN through");
            assert_eq!(shape.apply(f64::INFINITY), 0.0, "{shape:?} passes infinity through");
            for value in [0.001, 0.5, 1.0, 4.0, 97.0] {
                assert!(shape.apply(value) > 0.0, "{shape:?} is not odd");
                assert!(shape.apply(-value) < 0.0, "{shape:?} is not odd");
                assert!((shape.apply(value) + shape.apply(-value)).abs() < 1e-9);
            }
        }
        // The damped shapes damp and the amplifying one amplifies.
        assert!(Shape::Sqrt.apply(4.0) < Shape::Linear.apply(4.0));
        assert!(Shape::Log1p.apply(4.0) < Shape::Sqrt.apply(4.0));
        assert!(Shape::Square.apply(4.0) > Shape::Linear.apply(4.0));
    }

    #[test]
    fn terms_are_evaluated_in_a_fixed_order_whatever_the_genome_lists() {
        let params = HeuristicParams::default();
        let built_in_order: Vec<&str> = CompiledPolicy::starter(&params)
            .terms()
            .iter()
            .map(|term| term.name)
            .collect();
        let mut reversed = PolicyGenome::new("same rules, other key order");
        for (name, weight) in STARTER_WEIGHTS.iter().rev() {
            reversed.set(name, *weight);
        }
        let reversed_order: Vec<&str> = CompiledPolicy::from_genome(&reversed, &params)
            .expect("the rules are valid")
            .terms()
            .iter()
            .map(|term| term.name)
            .collect();
        assert_eq!(built_in_order, reversed_order);
    }

    #[test]
    fn a_policy_always_takes_a_legal_choice() {
        let params = HeuristicParams::default();
        let policy = CompiledPolicy::starter(&params);
        let mut decisions = 0;
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                if choices.is_empty() {
                    continue;
                }
                let choice = policy
                    .decide(&state, player_index, &choices)
                    .expect("there were choices to take");
                assert!(
                    choices.contains(&choice),
                    "the policy invented a choice that was not offered"
                );
                decisions += 1;
            }
        }
        assert!(decisions > 100, "hardly any decisions were checked");
    }

    #[test]
    fn the_same_position_always_gets_the_same_choice() {
        let params = HeuristicParams::default();
        let policy = CompiledPolicy::starter(&params);
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                if choices.is_empty() {
                    continue;
                }
                let first = policy.decide(&state, player_index, &choices);
                for _ in 0..3 {
                    assert_eq!(first, policy.decide(&state, player_index, &choices));
                }
            }
        }
    }

    #[test]
    fn a_trace_sums_to_the_score_it_reports() {
        let params = HeuristicParams::default();
        let policy = CompiledPolicy::starter(&params);
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                for choice in choices.iter().take(8) {
                    let trace = policy.explain(&state, player_index, choice);
                    let summed: f64 = trace.contributions.iter().map(|c| c.weighted).sum();
                    assert!(
                        (summed - trace.score).abs() < 1e-6,
                        "{} terms sum to {summed} but the score is {}",
                        trace.contributions.len(),
                        trace.score
                    );
                    assert!(trace.score.is_finite());
                }
            }
        }
    }

    #[test]
    fn a_traced_decision_agrees_with_an_untraced_one() {
        let params = HeuristicParams::default();
        let policy = CompiledPolicy::starter(&params);
        let mut compared = 0;
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                if choices.is_empty() {
                    continue;
                }
                let plain = policy.decide(&state, player_index, &choices);
                let traced = policy.decide_with_trace(&state, player_index, &choices);
                assert_eq!(plain, traced.as_ref().map(|d| d.choice));
                if let Some(decision) = traced {
                    assert!(decision.margin().map_or(true, |margin| margin >= 0.0));
                    assert!(
                        decision.runner_up.is_none() == decision.margin().is_none(),
                        "a runner-up and its score must agree"
                    );
                    assert!(!decision.contributions.is_empty(), "a traced decision says nothing");
                    assert!(
                        decision.contributions.len() <= policy.terms().len(),
                        "a traced decision cannot report more terms than the policy has"
                    );
                    // Every term reported answers for the choice that took it.
                    let choice = decision.choice;
                    for contribution in &decision.contributions {
                        let term = policy
                            .terms()
                            .iter()
                            .find(|term| term.name == contribution.feature)
                            .expect("a reported term is one of the policy's");
                        assert!(
                            term.mask & ChoiceVariant::of(&choice).bit() != 0,
                            "{} reported for a choice outside its group",
                            contribution.feature
                        );
                    }
                }
                compared += 1;
            }
        }
        assert!(compared > 100);
    }

    #[test]
    fn a_zero_weight_switches_a_rule_off_without_disturbing_the_others() {
        let params = HeuristicParams::default();
        let mut genome = PolicyGenome::starter();
        genome.set("points_immediate", 0.0);
        let policy = CompiledPolicy::from_genome(&genome, &params).expect("valid");
        assert!(policy.terms().iter().all(|term| term.name != "points_immediate"));
        assert_eq!(policy.terms().len(), CompiledPolicy::starter(&params).terms().len() - 1);
    }
}
