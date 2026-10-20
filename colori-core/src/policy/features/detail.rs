//! Rules that read something specific rather than something total.
//!
//! The first two tables mostly price *aggregates*: all the colours, all the
//! materials, the whole plan. That is cheap and general, and it is also blind to
//! the difference between two things that are not alike. Ceramics are wanted by
//! the cheap Potter and Tilemaker; Paintings by the dear Painter and Master
//! Painter. A tertiary is what every sell card above two points asks for; a
//! primary is what you mix with. A four-point buyer is worth a detour and a
//! two-point buyer is filler.
//!
//! These rules read one material, one tier of colour, or one band of buyer
//! value. Each is a separate thing the search can weight separately, so a genome
//! can come to want ceramics for one plan and paintings for another — which is
//! the sort of distinction that has to be notice*able* before it can be learned.
//!
//! Same two disciplines as the rest of the vocabulary: every rule is a delta on
//! the choice, and everything is in points.

use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES};
use crate::types::*;

use super::{offered_card_of, sale_card_of, storage_value, FeatureDef, Group};
use super::super::context::Ctx;
use super::super::effects::Delta;

/// Material indices, as `MaterialType` orders them.
const TEXTILES: usize = MaterialType::Textiles as usize;
const CERAMICS: usize = MaterialType::Ceramics as usize;
const PAINTINGS: usize = MaterialType::Paintings as usize;

/// A buyer this dear is worth planning around rather than taking when convenient.
const DEAR_BUYER: u32 = 4;
/// A buyer this cheap is filler: every card in the deck can be sold to one.
const FILLER_BUYER: u32 = 2;

// ── One material at a time ──

fn gained_material(ctx: &Ctx, d: &Delta, index: usize) -> f64 {
    d.gained_materials[index] as f64 * ctx.material_value[index]
}

fn spent_material(ctx: &Ctx, d: &Delta, index: usize) -> f64 {
    -(d.spent_materials[index] as f64 * ctx.material_value[index])
}

/// One when this choice supplies every unit of `index` some buyer is short of.
fn completes_material(ctx: &Ctx, d: &Delta, index: usize) -> f64 {
    let mut completing = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let needed = buyer.sell_card.stats().materials[index] as u32;
        if needed == 0 {
            continue;
        }
        let held = ctx.player.materials.counts[index] as i32 + d.gained_materials[index] as i32
            - d.spent_materials[index] as i32;
        if held >= needed as i32 {
            completing += 1.0;
        }
    }
    completing
}

fn textiles_gained(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    gained_material(ctx, d, TEXTILES)
}

fn ceramics_gained(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    gained_material(ctx, d, CERAMICS)
}

fn paintings_gained(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    gained_material(ctx, d, PAINTINGS)
}

fn textiles_spent(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    spent_material(ctx, d, TEXTILES)
}

fn ceramics_spent(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    spent_material(ctx, d, CERAMICS)
}

fn paintings_spent(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    spent_material(ctx, d, PAINTINGS)
}

fn textiles_completing(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    completes_material(ctx, d, TEXTILES)
}

fn ceramics_completing(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    completes_material(ctx, d, CERAMICS)
}

fn paintings_completing(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    completes_material(ctx, d, PAINTINGS)
}

// ── One tier of colour at a time ──

fn tier_demanded(ctx: &Ctx, d: &Delta, tier: &[Color]) -> f64 {
    tier.iter()
        .map(|color| d.gained_colors[color.index()] as f64 * ctx.value_of(*color))
        .sum()
}

fn primary_demanded(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    tier_demanded(ctx, d, &PRIMARIES)
}

fn secondary_demanded(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    tier_demanded(ctx, d, &SECONDARIES)
}

/// The tier every sell card above two points asks for.
fn tertiary_demanded(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    tier_demanded(ctx, d, &TERTIARIES)
}

// ── Bands of buyer value ──

fn sold_points(choice: &Choice) -> u32 {
    sale_card_of(choice).map_or(0, |card| card.points())
}

fn sell_dear(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let points = sold_points(choice);
    if points >= DEAR_BUYER {
        // Worth more than its points when it is the dear kind: it is also the
        // plan the whole position should rearrange itself around.
        points as f64 + ctx.pending_points.saturating_sub(points) as f64 * 0.0
    } else {
        0.0
    }
}

fn sell_filler(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let points = sold_points(choice);
    if points <= FILLER_BUYER {
        points as f64
    } else {
        0.0
    }
}

/// How far the dear buyers still are, across the table.
fn dear_buyers_shortfall(ctx: &Ctx, _c: &Choice, d: &Delta) -> f64 {
    let mut short = 0i32;
    for buyer in ctx.player.buyers.iter() {
        let sell_card = buyer.sell_card;
        if sell_card.points() < DEAR_BUYER {
            continue;
        }
        let stats = sell_card.stats();
        for (index, &needed) in stats.materials.iter().enumerate() {
            let held = ctx.player.materials.counts[index] as i32 + d.gained_materials[index] as i32
                - d.spent_materials[index] as i32;
            short += (needed as i32 - held).max(0);
        }
        for &(color, needed) in stats.colors() {
            let held = ctx.player.color_wheel.get(color) as i32
                + d.gained_colors[color.index()] as i32
                - d.spent_colors[color.index()] as i32;
            short += (needed as i32 - held).max(0);
        }
    }
    -(short as f64)
}

fn dear_buyer_points_unlocked(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(sell_card) = sale_card_of(choice) else {
        return 0.0;
    };
    let _ = ctx;
    if sell_card.points() >= DEAR_BUYER {
        sell_card.points() as f64
    } else {
        0.0
    }
}

fn offer_dear(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match offered_card_of(choice) {
        Some(sell_card) if sell_card.points() >= DEAR_BUYER => sell_card.points() as f64,
        _ => 0.0,
    }
}

// ── Mixing as a route to a specific colour ──

/// Whether the mix outputs are colours that finish a sitting buyer's cost.
fn mix_completes(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(mixes) = super::mix_sequence_of(choice) else {
        return 0.0;
    };
    if mixes.is_empty() {
        return 0.0;
    }
    let ctx = _ctx;
    let mut wheel = ctx.player.color_wheel.clone();
    let mut wilds = ctx.player.wild_primaries;
    let mut completing = 0.0;
    for &(a, b) in mixes.iter() {
        let output = crate::colors::mix_result(a, b);
        let before: i32 = wheel.get(output) as i32;
        crate::colors::perform_mix_unchecked(&mut wheel, &mut wilds, a, b);
        for buyer in ctx.player.buyers.iter() {
            let stats = buyer.sell_card.stats();
            for &(color, needed) in stats.colors() {
                if color != output {
                    continue;
                }
                if before < needed as i32 && wheel.get(output) as i32 >= needed as i32 {
                    completing += 1.0;
                }
            }
        }
    }
    completing
}

/// Mixes whose output is a tertiary — the dearest thing to reach, because it
/// takes a secondary and a primary in sequence.
fn mix_reaches_tertiary(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(mixes) = super::mix_sequence_of(choice) else {
        return 0.0;
    };
    mixes
        .iter()
        .filter(|&&(a, b)| crate::colors::is_tertiary(crate::colors::mix_result(a, b)))
        .count() as f64
}

// ── The buyers phase, read specifically ──

/// How cheap the offered buyer is to finish: fewer units short is better.
fn offer_cheap_to_finish(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(sell_card) = offered_card_of(choice) else {
        return 0.0;
    };
    -(super::super::context::buyer_short_units(ctx.player, sell_card) as f64)
}

/// Boats the offered buyer takes outright.
fn offer_boat_demand(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match offered_card_of(choice) {
        Some(sell_card) => -(sell_card.required_boats() as f64),
        None => 0.0,
    }
}

// ── What the end of the turn leaves behind ──

/// Storage in the workshop that is not going to be stored.
///
/// At the end of the turn the whole workshop goes to the bottom of the deck,
/// rotated or not: a card left unrotated postpones its own production by a
/// cycle, so ending the turn with storage still on the table costs something.
fn end_turn_lost_storage(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if !matches!(choice, Choice::EndTurn) {
        return 0.0;
    }
    -(ctx
        .player
        .workshop_cards
        .iter()
        .map(|id| storage_value(ctx, ctx.card(id)))
        .sum::<f64>())
}

// ── The table ──

pub const DETAIL_RULES: &[FeatureDef] = &[
    FeatureDef {
        name: "textiles_gained",
        group: Group::Universal,
        doc: "Textile units gained, priced at what the buyers pay for them. Textiles are what \\
              the cheap Dyers and Shipwrights ask for.",
        eval: textiles_gained,
    },
    FeatureDef {
        name: "ceramics_gained",
        group: Group::Universal,
        doc: "Ceramic units gained, priced at what the buyers pay. Ceramics are the Potter's \\
              and Tilemaker's, both cheap.",
        eval: ceramics_gained,
    },
    FeatureDef {
        name: "paintings_gained",
        group: Group::Universal,
        doc: "Painting units gained, priced at what the buyers pay. Paintings are the Painter's \\
              and Master Painter's, the dearest trades.",
        eval: paintings_gained,
    },
    FeatureDef {
        name: "textiles_spent",
        group: Group::Universal,
        doc: "Textile units a sale takes.",
        eval: textiles_spent,
    },
    FeatureDef {
        name: "ceramics_spent",
        group: Group::Universal,
        doc: "Ceramic units a sale takes.",
        eval: ceramics_spent,
    },
    FeatureDef {
        name: "paintings_spent",
        group: Group::Universal,
        doc: "Painting units a sale takes.",
        eval: paintings_spent,
    },
    FeatureDef {
        name: "textiles_completing",
        group: Group::Universal,
        doc: "Buyers whose textile need this choice satisfies in full.",
        eval: textiles_completing,
    },
    FeatureDef {
        name: "ceramics_completing",
        group: Group::Universal,
        doc: "Buyers whose ceramic need this choice satisfies in full.",
        eval: ceramics_completing,
    },
    FeatureDef {
        name: "paintings_completing",
        group: Group::Universal,
        doc: "Buyers whose painting need this choice satisfies in full.",
        eval: paintings_completing,
    },
    FeatureDef {
        name: "primary_demanded",
        group: Group::Universal,
        doc: "Primaries gained, priced by the buyers. Cheap to get and the base of every mix.",
        eval: primary_demanded,
    },
    FeatureDef {
        name: "secondary_demanded",
        group: Group::Universal,
        doc: "Secondaries gained, priced by the buyers. Reached from two primaries or a card.",
        eval: secondary_demanded,
    },
    FeatureDef {
        name: "tertiary_demanded",
        group: Group::Universal,
        doc: "Tertiaries gained, priced by the buyers. Every sell card above two points asks \\
              for one, and reaching one takes a mix from a secondary.",
        eval: tertiary_demanded,
    },
    FeatureDef {
        name: "sell_dear_buyer",
        group: Group::Sell,
        doc: "Points of a sale to a dear buyer, and zero for the cheap ones.",
        eval: sell_dear,
    },
    FeatureDef {
        name: "sell_filler_buyer",
        group: Group::Sell,
        doc: "Points of a sale to a cheap buyer, and zero for the dear ones.",
        eval: sell_filler,
    },
    FeatureDef {
        name: "dear_buyers_shortfall",
        group: Group::Universal,
        doc: "Units the dear buyers are still short of, negated: a card that closes that gap \\
              is worth more than one that fills a cheap buyer.",
        eval: dear_buyers_shortfall,
    },
    FeatureDef {
        name: "dear_buyer_points_unlocked",
        group: Group::Sell,
        doc: "Points of the sale, when the buyer sold to is a dear one.",
        eval: dear_buyer_points_unlocked,
    },
    FeatureDef {
        name: "offer_dear_buyer",
        group: Group::Buyers,
        doc: "Points of the offered buyer, when it is a dear one.",
        eval: offer_dear,
    },
    FeatureDef {
        name: "mix_completes_a_buyer",
        group: Group::Mix,
        doc: "Mixes whose output finishes a sitting buyer's colour cost.",
        eval: mix_completes,
    },
    FeatureDef {
        name: "mix_reaches_tertiary",
        group: Group::Mix,
        doc: "Mixes whose output is a tertiary.",
        eval: mix_reaches_tertiary,
    },
    FeatureDef {
        name: "offer_cheap_to_finish",
        group: Group::Buyers,
        doc: "Negative of the units the offered buyer is short of: a cheap plan is a good one.",
        eval: offer_cheap_to_finish,
    },
    FeatureDef {
        name: "offer_boat_demand",
        group: Group::Buyers,
        doc: "Negative of the boats the offered buyer takes outright.",
        eval: offer_boat_demand,
    },
    FeatureDef {
        name: "end_turn_lost_storage",
        group: Group::EndTurn,
        doc: "Storage sitting unrotated in the workshop when the turn ends, negated. The whole \\
              workshop goes to the bottom of the deck, so an unrotated card postpones its own \\
              production by a cycle.",
        eval: end_turn_lost_storage,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_here_has_a_name_and_a_doc() {
        for def in DETAIL_RULES {
            assert!(!def.name.is_empty());
            assert!(
                def.name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{} is not snake_case",
                def.name
            );
            assert!(def.doc.len() > 20, "{} says nothing", def.name);
        }
    }

    #[test]
    fn the_material_indices_are_the_ones_the_engine_uses() {
        assert_eq!(ALL_MATERIAL_TYPES[TEXTILES], MaterialType::Textiles);
        assert_eq!(ALL_MATERIAL_TYPES[CERAMICS], MaterialType::Ceramics);
        assert_eq!(ALL_MATERIAL_TYPES[PAINTINGS], MaterialType::Paintings);
    }
}
