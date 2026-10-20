//! Rules that compose two readings into one.
//!
//! A weighted sum can say "a card's storage is worth this much" and "a point is
//! worth this much", but not "a card's storage is worth this much *now*, and much
//! less in the last round". A single weight means a single setting of a rule,
//! and the central dial of this game is not set once.
//!
//! An interaction says it directly: take two readings — rules, or a scalar the
//! position supplies such as how late it is — and combine them with one operator.
//! The combination is what makes the vocabulary non-linear: the value of a rule
//! can depend on the clock, on how many buyers are affordable, on how much game
//! is left, without anyone writing that dependency by hand.
//!
//! The operator set is deliberately tiny. `Multiply` is the workhorse: a rule
//! scaled by another reading. `Max` and `Min` pick the binding constraint — the
//! smaller of what a card stores and what the buyer still needs is the part that
//! matters. `Add` is for two readings that reinforce each other.
//!
//! An interaction is a rule like any other: it has a weight, a shape, and a set
//! of choices it answers for (the intersection of its operands').

use serde::{Deserialize, Serialize};

use crate::types::Choice;

use super::context::Ctx;
use super::effects::Delta;
use super::{features, Shape};

/// A reading the position supplies on its own, with no reference to the choice.
///
/// These exist to be *multiplied by* something. Alone they would be the same for
/// every choice in a decision and cancel out of the comparison; as a factor they
/// are how a rule learns that the clock matters.
pub type ScalarFn = fn(&Ctx) -> f64;

pub const SCALARS: &[(&str, ScalarFn)] = &[
    ("lateness", lateness),
    ("earliness", earliness),
    ("rounds_left", rounds_left),
    ("empty_slots", empty_slots),
    ("purchasable", purchasable),
    ("affordable_buyers", affordable_buyers),
];

pub fn lookup_scalar(name: &str) -> Option<ScalarFn> {
    SCALARS
        .iter()
        .find(|(scalar, _)| *scalar == name)
        .map(|(_, eval)| *eval)
}

fn lateness(ctx: &Ctx) -> f64 {
    1.0 - (ctx.rounds_left.saturating_sub(1)) as f64 / ctx.max_rounds.max(1) as f64
}

fn earliness(ctx: &Ctx) -> f64 {
    1.0 - lateness(ctx)
}

fn rounds_left(ctx: &Ctx) -> f64 {
    ctx.rounds_left as f64
}

fn empty_slots(ctx: &Ctx) -> f64 {
    ctx.empty_slots as f64
}

fn purchasable(ctx: &Ctx) -> f64 {
    ctx.purchasable as f64
}

fn affordable_buyers(ctx: &Ctx) -> f64 {
    ctx.buyers_affordable as f64
}

// ── Operators ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Op {
    /// One reading scaling another.
    #[default]
    Multiply,
    /// The binding constraint: whichever reading is smaller.
    Min,
    /// The looser of the two: whichever is larger.
    Max,
    /// Two readings reinforcing each other.
    Add,
}

impl Op {
    pub const ALL: [Op; 4] = [Op::Multiply, Op::Min, Op::Max, Op::Add];

    pub fn name(self) -> &'static str {
        match self {
            Op::Multiply => "multiply",
            Op::Min => "min",
            Op::Max => "max",
            Op::Add => "add",
        }
    }

    pub fn from_name(name: &str) -> Option<Op> {
        Op::ALL.into_iter().find(|op| op.name() == name)
    }

    #[inline]
    pub fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Op::Multiply => a * b,
            Op::Min => a.min(b),
            Op::Max => a.max(b),
            Op::Add => a + b,
        }
    }
}

// ── The gene and the compiled rule ──

/// One composed reading, as a genome writes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionGene {
    pub op: Op,
    pub a: String,
    pub b: String,
    pub weight: f64,
    #[serde(default)]
    pub shape: Shape,
}

impl InteractionGene {
    pub fn new(op: Op, a: impl Into<String>, b: impl Into<String>, weight: f64) -> Self {
        InteractionGene {
            op,
            a: a.into(),
            b: b.into(),
            weight,
            shape: Shape::Linear,
        }
    }

    /// How this composed reading is labelled in a report or a trace.
    pub fn label(&self) -> String {
        format!("{}({}, {})", self.op.name(), self.a, self.b)
    }
}

#[derive(Clone, Copy)]
enum Operand {
    /// A choice rule: answers for the choices its group covers.
    Rule {
        eval: super::FeatureFn,
        mask: u32,
    },
    /// A scalar the position supplies, which answers for every choice.
    Scalar(ScalarFn),
}

impl Operand {
    #[inline]
    fn read(self, ctx: &Ctx, choice: &Choice, delta: &Delta) -> f64 {
        match self {
            Operand::Rule { eval, .. } => eval(ctx, choice, delta),
            Operand::Scalar(eval) => eval(ctx),
        }
    }

    #[inline]
    fn mask(self) -> u32 {
        match self {
            Operand::Rule { mask, .. } => mask,
            Operand::Scalar(_) => u32::MAX,
        }
    }
}

/// A composed rule, ready to score.
pub struct CompiledInteraction {
    pub label: String,
    pub weight: f64,
    pub shape: Shape,
    pub mask: u32,
    op: Op,
    left: Operand,
    right: Operand,
}

#[derive(Debug)]
pub enum InteractionError {
    UnknownOperand(String),
}

impl std::fmt::Display for InteractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionError::UnknownOperand(name) => write!(
                f,
                "{name:?} is neither a rule nor a scalar (scalars: {})",
                SCALARS
                    .iter()
                    .map(|(scalar, _)| *scalar)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl CompiledInteraction {
    pub fn compile(gene: &InteractionGene) -> Result<Self, InteractionError> {
        let left = operand(&gene.a)?;
        let right = operand(&gene.b)?;
        Ok(CompiledInteraction {
            label: gene.label(),
            weight: gene.weight,
            shape: gene.shape,
            mask: left.mask() & right.mask(),
            op: gene.op,
            left,
            right,
        })
    }

    /// What this composed reading contributes to the score of `choice`.
    #[inline]
    pub fn contribution(&self, ctx: &Ctx, choice: &Choice, delta: &Delta) -> f64 {
        let a = self.left.read(ctx, choice, delta);
        let b = self.right.read(ctx, choice, delta);
        self.shape.apply(self.op.apply(a, b)) * self.weight
    }
}

fn operand(name: &str) -> Result<Operand, InteractionError> {
    if let Some(def) = features::lookup(name) {
        return Ok(Operand::Rule {
            eval: def.eval,
            mask: def.group.mask(),
        });
    }
    if let Some(eval) = lookup_scalar(name) {
        return Ok(Operand::Scalar(eval));
    }
    Err(InteractionError::UnknownOperand(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operator_is_reachable_by_name() {
        for op in Op::ALL {
            assert_eq!(Op::from_name(op.name()), Some(op));
        }
        assert!(Op::from_name("divide").is_none());
    }

    #[test]
    fn scalars_do_not_shadow_rules() {
        for (name, _) in SCALARS {
            assert!(
                features::lookup(name).is_none(),
                "{name} is both a scalar and a rule"
            );
        }
    }

    #[test]
    fn an_unknown_operand_is_an_error_not_a_zero() {
        let gene = InteractionGene::new(Op::Multiply, "card_workshop_value", "nonsense", 1.0);
        assert!(CompiledInteraction::compile(&gene).is_err());
    }

    #[test]
    fn a_composed_rule_only_answers_where_both_operands_do() {
        // A draft rule times a scalar is still a draft rule.
        let gene = InteractionGene::new(Op::Multiply, "draft_fills_gap", "lateness", 1.0);
        let compiled = CompiledInteraction::compile(&gene).expect("both operands exist");
        let draft = 1u32 << 0; // ChoiceVariant::DraftPick
        assert_eq!(compiled.mask, draft);
    }

    #[test]
    fn the_operators_do_what_they_say() {
        assert_eq!(Op::Multiply.apply(3.0, 4.0), 12.0);
        assert_eq!(Op::Min.apply(3.0, 4.0), 3.0);
        assert_eq!(Op::Max.apply(3.0, 4.0), 4.0);
        assert_eq!(Op::Add.apply(3.0, 4.0), 7.0);
        // Signs survive, so a negative reading still reads as a cost.
        assert_eq!(Op::Multiply.apply(-2.0, 3.0), -6.0);
        assert_eq!(Op::Min.apply(-2.0, 3.0), -2.0);
    }
}
