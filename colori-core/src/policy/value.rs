//! Rules that value a **position** rather than a choice.
//!
//! The choice rules say what an action is worth. A lookahead needs the other
//! question: given the position a line of play leads to, how good is it? So
//! these are a value function over the board, in points, and they are what a
//! planned decision scores its leaves with.
//!
//! They are compiled and tuned exactly like the choice rules — same genome, same
//! weights, same search — but they never score a choice, so a build that does
//! not look ahead ignores them entirely.
//!
//! Every one is a linear function of the position, because a linear value
//! function is what the tuner can fit and what a trace can explain. What is
//! deliberately absent is anything that needs to know the future: no "how many
//! rounds until this card comes back" beyond what the board shows.

use crate::scoring::score_in_half_points;

use super::context::Ctx;
use super::features::storage_value;

/// A rule's body: the position, in points.
pub type ValueFn = fn(&Ctx) -> f64;

pub struct ValueDef {
    /// The name a genome uses. Shares a namespace with the choice rules.
    pub name: &'static str,
    /// What it measures, and which way a positive weight points.
    pub doc: &'static str,
    pub eval: ValueFn,
}

pub const VALUE_FEATURES: &[ValueDef] = &[
    ValueDef {
        name: "value_banked",
        doc: "Points already banked: completed sell cards, ducats, and the half points for \\
              wilds and boats still held.",
        eval: value_banked,
    },
    ValueDef {
        name: "value_pending",
        doc: "Points sitting in the buyers this seat holds. A plan is worth something, but \\
              not what the sale is worth.",
        eval: value_pending,
    },
    ValueDef {
        name: "value_affordable",
        doc: "Points among those that could be banked right now.",
        eval: value_affordable,
    },
    ValueDef {
        name: "value_stored_colors",
        doc: "Colours on the wheel, priced at what the sitting buyers pay for them.",
        eval: value_stored_colors,
    },
    ValueDef {
        name: "value_stored_materials",
        doc: "Stored materials, priced at what the sitting buyers pay for them.",
        eval: value_stored_materials,
    },
    ValueDef {
        name: "value_loose_colors",
        doc: "Colours no sitting buyer wants: stock, worth only what a later buyer might pay.",
        eval: value_loose_colors,
    },
    ValueDef {
        name: "value_loose_materials",
        doc: "Materials no sitting buyer wants.",
        eval: value_loose_materials,
    },
    ValueDef {
        name: "value_ducats",
        doc: "Ducats held, counted apart from `value_banked` so the option they carry can \\
              be priced separately from the point they score.",
        eval: value_ducats,
    },
    ValueDef {
        name: "value_wilds",
        doc: "Wild primaries held. Each is half a point unspent, and any primary it stands in for.",
        eval: value_wilds,
    },
    ValueDef {
        name: "value_boats",
        doc: "Boats held. Each is half a point unspent, and any material it stands in for.",
        eval: value_boats,
    },
    ValueDef {
        name: "value_engine",
        doc: "What the cards in the personal deck and workshop would store if they came \\
              round again: the deck's productive capacity, not just its size.",
        eval: value_engine,
    },
    ValueDef {
        name: "value_hand",
        doc: "Cards in the workshop, which are the options the next turn starts with.",
        eval: value_hand,
    },
    ValueDef {
        name: "value_shortfall",
        doc: "Units the buyers still want, negated. How far the plan is from paying.",
        eval: value_shortfall,
    },
    ValueDef {
        name: "value_options",
        doc: "Things the position could still do: mixes the wheel can supply and \
              purchases the ducats can buy.",
        eval: value_options,
    },
    ValueDef {
        name: "value_standing",
        doc: "Half-points above or below the best other seat. With seven rounds and no \
              interaction during the action phase, the race is the point.",
        eval: value_standing,
    },
    ValueDef {
        name: "value_engine_cycle",
        doc: "Negative of how many rounds the personal deck takes to come round again, \
              at five cards drawn a round: a long deck produces slowly.",
        eval: value_engine_cycle,
    },
    ValueDef {
        name: "value_slots_full",
        doc: "Buyer slots filled. Until a slot is filled it is a plan not yet started.",
        eval: value_slots_full,
    },
];

pub fn lookup_value(name: &str) -> Option<&'static ValueDef> {
    VALUE_FEATURES.iter().find(|def| def.name == name)
}

fn value_banked(ctx: &Ctx) -> f64 {
    score_in_half_points(ctx.player) as f64 / 2.0
}

fn value_pending(ctx: &Ctx) -> f64 {
    ctx.pending_points as f64
}

fn value_affordable(ctx: &Ctx) -> f64 {
    ctx.affordable_points as f64
}

fn value_stored_colors(ctx: &Ctx) -> f64 {
    (0..crate::types::NUM_COLORS)
        .map(|i| ctx.player.color_wheel.counts[i] as f64 * ctx.color_value[i])
        .sum()
}

fn value_stored_materials(ctx: &Ctx) -> f64 {
    (0..3)
        .map(|i| ctx.player.materials.counts[i] as f64 * ctx.material_value[i])
        .sum()
}

fn value_loose_colors(ctx: &Ctx) -> f64 {
    (0..crate::types::NUM_COLORS)
        .filter(|&i| ctx.color_demand[i] == 0)
        .map(|i| ctx.player.color_wheel.counts[i] as f64)
        .sum()
}

fn value_loose_materials(ctx: &Ctx) -> f64 {
    (0..3)
        .filter(|&i| ctx.material_demand[i] == 0)
        .map(|i| ctx.player.materials.counts[i] as f64)
        .sum()
}

fn value_ducats(ctx: &Ctx) -> f64 {
    ctx.player.ducats as f64
}

fn value_wilds(ctx: &Ctx) -> f64 {
    ctx.player.wild_primaries as f64
}

fn value_boats(ctx: &Ctx) -> f64 {
    ctx.player.boats as f64
}

fn value_engine(ctx: &Ctx) -> f64 {
    let player = ctx.player;
    let mut total = 0.0;
    for id in player.deck.cards().iter() {
        total += storage_value(ctx, ctx.card(id));
    }
    for cards in [player.workshop_cards, player.workshopped_cards, player.drafted_cards] {
        for id in cards.iter() {
            total += storage_value(ctx, ctx.card(id));
        }
    }
    total
}

fn value_hand(ctx: &Ctx) -> f64 {
    ctx.player.workshop_cards.len() as f64
}

fn value_shortfall(ctx: &Ctx) -> f64 {
    -((ctx.demanded_color_units() + ctx.demanded_material_units()) as f64)
}

fn value_options(ctx: &Ctx) -> f64 {
    ctx.mixable_pairs as f64 + ctx.purchasable as f64
}

fn value_standing(ctx: &Ctx) -> f64 {
    ctx.standing as f64 / 2.0
}

fn value_engine_cycle(ctx: &Ctx) -> f64 {
    // Five cards a round is the workshop size, so this is how many rounds the
    // deck takes to come round.
    -(ctx.deck_size as f64 / 5.0)
}

fn value_slots_full(ctx: &Ctx) -> f64 {
    ctx.player.buyers.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::test_support;
    use crate::scoring::{CardHeuristicTable, HeuristicParams};

    #[test]
    fn value_feature_names_are_unique_and_distinct_from_choice_rules() {
        let mut names: Vec<&str> = VALUE_FEATURES.iter().map(|def| def.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two value rules share a name");
        for name in names {
            assert!(
                crate::policy::features::lookup(name).is_none(),
                "{name} is both a choice rule and a value rule"
            );
        }
    }

    #[test]
    fn every_value_rule_is_finite_on_reachable_positions() {
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let mut checked = 0;
        for state in test_support::reachable_states() {
            for seat in 0..state.players.len() {
                let ctx = Ctx::new(&state, seat, &table);
                for def in VALUE_FEATURES {
                    let value = (def.eval)(&ctx);
                    assert!(value.is_finite(), "{} returned {value}", def.name);
                    checked += 1;
                }
            }
        }
        assert!(checked > 100);
    }

    #[test]
    fn banking_a_point_raises_the_banked_value_by_a_point() {
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let mut state = test_support::game_with_one_empty_player();
        let before = {
            let ctx = Ctx::new(&state, 0, &table);
            (value_banked(&ctx), value_ducats(&ctx))
        };
        state.players[0].ducats = 3;
        state.players[0].cached_whole_points = 3;
        let after = {
            let ctx = Ctx::new(&state, 0, &table);
            (value_banked(&ctx), value_ducats(&ctx))
        };
        assert_eq!(after.0 - before.0, 3.0);
        assert_eq!(after.1 - before.1, 3.0);
    }
}
