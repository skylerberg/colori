//! The rule vocabulary: every feature a genome can weigh.
//!
//! A feature answers one question about a choice and answers it **in points** —
//! the points it expects the choice to be worth over the rest of the game. One
//! currency is what lets a genome compare choices of different kinds against
//! each other: trashing a card, spending a ducat and ending the turn all have
//! to land on the same number line, or the rules rot as soon as they touch.
//!
//! A feature takes no free parameters. Anything that should be tunable is a
//! separate feature with its own weight, which keeps every number in a genome
//! a weight and makes the policy readable as a linear model: in a trace, a
//! decision's score is the sum of its parts, so a contribution is also an
//! explanation.
//!
//! Features are pure: they read the state, the choice and the choice's effect,
//! and never mutate anything or consult randomness. That is what makes the
//! policy deterministic and a game replayable.

use crate::action_phase::{boats_to_sell, can_afford_sell_card, can_mix_any, can_sell_to_any_buyer};
use crate::colors::{is_primary, wild_primaries_to_cover_sale, PRIMARIES};
use crate::types::*;

use super::context::Ctx;
use super::effects::{
    Delta, ABILITY_GAIN_BOATS, ABILITY_GAIN_DUCATS, ABILITY_GAIN_SECONDARY,
    ABILITY_GAIN_WILD_PRIMARIES, ABILITY_MIX, ABILITY_MOVE_TO_DRAFT_POOL, ABILITY_SELL,
    ABILITY_WORKSHOP,
};

/// A rule's body.
pub type FeatureFn = fn(&Ctx, &Choice, &Delta) -> f64;

/// Which choices a feature speaks about. Used to document a feature and to
/// hold it to its own ground in the tests: a feature that fires outside its
/// group is a bug, not a discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Universal,
    Draft,
    Sell,
    Buyers,
    Action,
    Mix,
    Workshop,
    Trash,
    EndTurn,
}

impl Group {
    pub fn applies_to(self, choice: &Choice) -> bool {
        match self {
            Group::Universal => true,
            Group::Draft => matches!(choice, Choice::DraftPick { .. }),
            Group::Sell => matches!(
                choice,
                Choice::SelectSellCard { .. } | Choice::TrashAndSell { .. }
            ),
            Group::Buyers => matches!(choice, Choice::TakeBuyer { .. } | Choice::DrawBuyer),
            Group::Action => matches!(choice, Choice::SpendDucat { .. } | Choice::EndTurn),
            Group::Mix => matches!(choice, Choice::MixAll { .. } | Choice::TrashAndMix { .. }),
            Group::Workshop => matches!(
                choice,
                Choice::Workshop { .. } | Choice::TrashAndWorkshop { .. }
            ),
            Group::Trash => matches!(
                choice,
                Choice::TrashDraftedCard { .. }
                    | Choice::TrashAndMix { .. }
                    | Choice::TrashAndSell { .. }
                    | Choice::TrashAndWorkshop { .. }
                    | Choice::TrashAndMoveToDraftPool { .. }
            ),
            Group::EndTurn => matches!(choice, Choice::EndTurn),
        }
    }
}

pub struct FeatureDef {
    /// The name a genome uses. Stable: renaming one invalidates genomes.
    pub name: &'static str,
    pub group: Group,
    /// What the feature measures, and which way a positive weight points.
    pub doc: &'static str,
    pub eval: FeatureFn,
}

/// Every feature a genome may weigh, in evaluation order.
///
/// Order is fixed here rather than taken from the genome file so that summing
/// the terms is reproducible: floating point addition is not associative, and
/// a policy whose score depends on JSON key order is not deterministic.
pub const FEATURES: &[FeatureDef] = &[
    // ── Straight value ──
    FeatureDef {
        name: "points_immediate",
        group: Group::Universal,
        doc: "Points the choice banks at once, as a sale's card is worth.",
        eval: points_immediate,
    },
    FeatureDef {
        name: "ducats_gained",
        group: Group::Universal,
        doc: "Ducats gained. A ducat is a point, and also a buy.",
        eval: ducats_gained,
    },
    FeatureDef {
        name: "ducats_spent",
        group: Group::Universal,
        doc: "Ducats spent. A positive weight makes spending cost something.",
        eval: ducats_spent,
    },
    FeatureDef {
        name: "colors_demanded",
        group: Group::Universal,
        doc: "Points-worth of the colours gained that the sitting buyers still want.",
        eval: colors_demanded,
    },
    FeatureDef {
        name: "colors_undemanded",
        group: Group::Universal,
        doc: "Colour units gained in colours no sitting buyer wants.",
        eval: colors_undemanded,
    },
    FeatureDef {
        name: "colors_gained",
        group: Group::Universal,
        doc: "Colour units gained, wanted or not.",
        eval: colors_gained,
    },
    FeatureDef {
        name: "colors_spent_demanded",
        group: Group::Universal,
        doc: "Points-worth of the colours spent that the buyers still wanted.",
        eval: colors_spent_demanded,
    },
    FeatureDef {
        name: "colors_spent",
        group: Group::Universal,
        doc: "Colour units spent, wanted or not.",
        eval: colors_spent,
    },
    FeatureDef {
        name: "materials_demanded",
        group: Group::Universal,
        doc: "Points-worth of the materials gained that the buyers still want.",
        eval: materials_demanded,
    },
    FeatureDef {
        name: "materials_undemanded",
        group: Group::Universal,
        doc: "Material units gained that no sitting buyer wants.",
        eval: materials_undemanded,
    },
    FeatureDef {
        name: "materials_gained",
        group: Group::Universal,
        doc: "Material units gained, wanted or not.",
        eval: materials_gained,
    },
    FeatureDef {
        name: "materials_spent",
        group: Group::Universal,
        doc: "Points-worth of the stored materials a sale takes.",
        eval: materials_spent,
    },
    FeatureDef {
        name: "materials_spent_raw",
        group: Group::Universal,
        doc: "Material units a sale takes, wanted or not.",
        eval: materials_spent_raw,
    },
    FeatureDef {
        name: "boats_gained",
        group: Group::Universal,
        doc: "Boats gained. Unspent, a boat is worth half a point.",
        eval: boats_gained,
    },
    FeatureDef {
        name: "boats_spent",
        group: Group::Universal,
        doc: "Boats a sale takes, each giving up its half point and its cover.",
        eval: boats_spent,
    },
    FeatureDef {
        name: "wilds_gained",
        group: Group::Universal,
        doc: "Wild primaries gained. Unspent, each is worth half a point.",
        eval: wilds_gained,
    },
    FeatureDef {
        name: "wilds_spent",
        group: Group::Universal,
        doc: "Wild primaries a mix spends covering a primary.",
        eval: wilds_spent,
    },
    FeatureDef {
        name: "card_quality_gained",
        group: Group::Universal,
        doc: "Heuristic quality of the cards the choice takes or keeps.",
        eval: card_quality_gained,
    },
    FeatureDef {
        name: "trash_thinning",
        group: Group::Universal,
        doc: "Cards trashed: every one is a card the deck no longer draws.",
        eval: trash_thinning,
    },
    FeatureDef {
        name: "trash_quality",
        group: Group::Universal,
        doc: "Heuristic quality of the cards trashed. A positive weight prices losing them.",
        eval: trash_quality,
    },
    // ── Abilities ──
    FeatureDef {
        name: "ability_workshop",
        group: Group::Universal,
        doc: "Workshop picks the choice grants.",
        eval: ability_workshop,
    },
    FeatureDef {
        name: "ability_mix",
        group: Group::Universal,
        doc: "Mixes the choice grants.",
        eval: ability_mix,
    },
    FeatureDef {
        name: "ability_sell",
        group: Group::Universal,
        doc: "Sell actions the choice grants.",
        eval: ability_sell,
    },
    FeatureDef {
        name: "ability_ducats",
        group: Group::Universal,
        doc: "Ducats the choice's abilities pay.",
        eval: ability_ducats,
    },
    FeatureDef {
        name: "ability_secondary",
        group: Group::Universal,
        doc: "Gain-a-secondary actions the choice grants.",
        eval: ability_secondary,
    },
    FeatureDef {
        name: "ability_wilds",
        group: Group::Universal,
        doc: "Wild primaries the choice's abilities pay.",
        eval: ability_wilds,
    },
    FeatureDef {
        name: "ability_boats",
        group: Group::Universal,
        doc: "Boats the choice's abilities pay.",
        eval: ability_boats,
    },
    FeatureDef {
        name: "ability_move_to_pool",
        group: Group::Universal,
        doc: "Move-to-draft-pool actions the choice grants.",
        eval: ability_move_to_pool,
    },
    // ── The plan the player is on ──
    FeatureDef {
        name: "color_diversity_delta",
        group: Group::Universal,
        doc: "Change in how many distinct colours are held. Mixing costs one.",
        eval: color_diversity_delta,
    },
    FeatureDef {
        name: "buyer_progress_points",
        group: Group::Universal,
        doc: "Points-worth of the best unaffordable buyer's cost the choice supplies.",
        eval: buyer_progress_points,
    },
    FeatureDef {
        name: "buyer_completion_points",
        group: Group::Universal,
        doc: "Points of the best buyer the choice makes affordable where it was not before.",
        eval: buyer_completion_points,
    },
    FeatureDef {
        name: "pending_points_share",
        group: Group::Universal,
        doc: "Share of the points sitting in uncompleted buyers that the choice banks.",
        eval: pending_points_share,
    },
    // ── Draft ──
    FeatureDef {
        name: "draft_color_demand",
        group: Group::Draft,
        doc: "How many of the picked card's colours a sitting buyer still wants.",
        eval: draft_color_demand,
    },
    FeatureDef {
        name: "draft_material_demand",
        group: Group::Draft,
        doc: "How many of the picked card's materials a sitting buyer still wants.",
        eval: draft_material_demand,
    },
    FeatureDef {
        name: "draft_ability_count",
        group: Group::Draft,
        doc: "Abilities the picked card carries, trash and workshop together.",
        eval: draft_ability_count,
    },
    FeatureDef {
        name: "draft_redundancy",
        group: Group::Draft,
        doc: "Cards already owned that share an ability with the picked card.",
        eval: draft_redundancy,
    },
    // ── Sales ──
    FeatureDef {
        name: "sell_points",
        group: Group::Sell,
        doc: "Points the sell card is worth.",
        eval: sell_points,
    },
    FeatureDef {
        name: "sell_points_per_unit",
        group: Group::Sell,
        doc: "Points per colour and material unit the sale takes.",
        eval: sell_points_per_unit,
    },
    FeatureDef {
        name: "sell_boat_cost",
        group: Group::Sell,
        doc: "Boats the sale takes, including those standing in for missing materials.",
        eval: sell_boat_cost,
    },
    // ── The buyers phase ──
    FeatureDef {
        name: "offer_points",
        group: Group::Buyers,
        doc: "Points the claimed sell card is worth.",
        eval: offer_points,
    },
    FeatureDef {
        name: "offer_affordable",
        group: Group::Buyers,
        doc: "One when the offered card could be sold with what is already stored.",
        eval: offer_affordable,
    },
    FeatureDef {
        name: "offer_distance",
        group: Group::Buyers,
        doc: "Negative of what the offered card is still short of: boats plus wilds.",
        eval: offer_distance,
    },
    FeatureDef {
        name: "offer_deck_gamble",
        group: Group::Buyers,
        doc: "One when the choice takes the deck's top card unseen.",
        eval: offer_deck_gamble,
    },
    // ── Mixing ──
    FeatureDef {
        name: "mix_output_demanded",
        group: Group::Mix,
        doc: "Mix output units in colours a sitting buyer wants.",
        eval: mix_output_demanded,
    },
    FeatureDef {
        name: "mix_higher_tier",
        group: Group::Mix,
        doc: "Mix output units that are secondary or tertiary.",
        eval: mix_higher_tier,
    },
    FeatureDef {
        name: "mix_burns_primaries",
        group: Group::Mix,
        doc: "Negative of the primary units a mix sequence spends.",
        eval: mix_burns_primaries,
    },
    FeatureDef {
        name: "mix_count",
        group: Group::Mix,
        doc: "Mixes in the sequence.",
        eval: mix_count,
    },
    // ── Workshop ──
    FeatureDef {
        name: "workshop_set_size",
        group: Group::Workshop,
        doc: "Cards the workshop choice stores.",
        eval: workshop_set_size,
    },
    // ── Ending the turn ──
    FeatureDef {
        name: "end_turn_unspent_cards",
        group: Group::EndTurn,
        doc: "Draft-pool cards still in hand when the turn ends, each an activation not taken.",
        eval: end_turn_unspent_cards,
    },
    FeatureDef {
        name: "end_turn_open_actions",
        group: Group::EndTurn,
        doc: "Actions still available when the turn ends.",
        eval: end_turn_open_actions,
    },
    FeatureDef {
        name: "end_turn_late_game",
        group: Group::EndTurn,
        doc: "One in the last round, falling off towards the first.",
        eval: end_turn_late_game,
    },
];

pub fn lookup(name: &str) -> Option<&'static FeatureDef> {
    FEATURES.iter().find(|def| def.name == name)
}

// ── Helpers ──

fn sale_card_of(choice: &Choice) -> Option<SellCard> {
    match choice {
        Choice::SelectSellCard { sell_card } | Choice::TrashAndSell { sell_card, .. } => {
            Some(*sell_card)
        }
        _ => None,
    }
}

fn offered_card_of(choice: &Choice) -> Option<SellCard> {
    match choice {
        Choice::TakeBuyer { sell_card } => Some(*sell_card),
        _ => None,
    }
}

fn mix_sequence_of(choice: &Choice) -> Option<&MixSequence> {
    match choice {
        Choice::MixAll { mixes } | Choice::TrashAndMix { mixes, .. } => Some(mixes),
        _ => None,
    }
}

fn drafted_card_of(choice: &Choice) -> Option<Card> {
    match choice {
        Choice::DraftPick { card } => Some(*card),
        _ => None,
    }
}

/// Units a sale takes in all: colours, materials and boats.
fn sale_units(sell_card: SellCard) -> f64 {
    let stats = sell_card.stats();
    let materials: u32 = stats.materials.iter().map(|&m| m as u32).sum();
    (stats.total_colors as u32 + materials + stats.boats).max(1) as f64
}

/// Walk every card the player owns, wherever it sits.
fn for_each_owned_card(ctx: &Ctx, mut visit: impl FnMut(Card)) {
    for id in ctx.player.deck.cards().iter() {
        visit(ctx.card(id));
    }
    for cards in [ctx.player.workshop_cards, ctx.player.workshopped_cards, ctx.player.drafted_cards] {
        for id in cards.iter() {
            visit(ctx.card(id));
        }
    }
}

/// Whether the player holds a draft-pool card carrying an ability that
/// `wanted` accepts.
fn drafted_has(ctx: &Ctx, wanted: impl Fn(Ability) -> bool) -> bool {
    ctx.player.drafted_cards.iter().any(|id| {
        ctx.card(id)
            .trash_abilities()
            .iter()
            .any(|&ability| wanted(ability))
    })
}

/// What the player could still do this turn.
fn open_actions(ctx: &Ctx) -> u32 {
    let player = ctx.player;
    let mut open = 0;
    if !player.drafted_cards.is_empty() {
        open += 1;
    }
    if !player.workshop_cards.is_empty() {
        open += 1;
    }
    if player.ducats > 0 {
        open += 1;
    }
    if can_sell_to_any_buyer(player) && drafted_has(ctx, |a| matches!(a, Ability::Sell)) {
        open += 1;
    }
    if can_mix_any(player) && drafted_has(ctx, |a| matches!(a, Ability::MixColors { .. })) {
        open += 1;
    }
    open
}

/// Whether `sell_card` would be affordable once `d` has been applied.
fn affordable_after(ctx: &Ctx, d: &Delta, sell_card: SellCard) -> bool {
    let stats = sell_card.stats();
    let player = ctx.player;

    let mut boats_needed = stats.boats as i32;
    for (i, &needed) in stats.materials.iter().enumerate() {
        let stored = ctx_stored_materials(ctx, d, i);
        boats_needed += (needed as i32 - stored).max(0);
    }
    let boats_available = player.boats as i32 + d.boats_gained as i32 - d.boats_spent as i32;
    if boats_needed > boats_available {
        return false;
    }

    let mut wilds_needed = 0i32;
    for &(color, needed) in stats.colors() {
        let held = player.color_wheel.get(color) as i32 + d.gained_colors[color.index()] as i32
            - d.spent_colors[color.index()] as i32;
        let short = (needed as i32 - held).max(0);
        if short > 0 {
            if !is_primary(color) {
                return false;
            }
            wilds_needed += short;
        }
    }
    let wilds_available =
        player.wild_primaries as i32 + d.wilds_gained as i32 - d.wilds_spent as i32;
    wilds_needed <= wilds_available
}

fn ctx_stored_materials(ctx: &Ctx, d: &Delta, material_index: usize) -> i32 {
    ctx.player.materials.counts[material_index] as i32 + d.gained_materials[material_index] as i32
        - d.spent_materials[material_index] as i32
}

/// The highest-point buyer the player cannot afford yet, if there is one.
fn best_unaffordable_buyer(ctx: &Ctx) -> Option<SellCard> {
    let mut best: Option<SellCard> = None;
    let mut best_points = 0;
    for buyer in ctx.player.buyers.iter() {
        let sell_card = buyer.sell_card;
        if can_afford_sell_card(ctx.player, &sell_card) {
            continue;
        }
        if sell_card.points() > best_points {
            best_points = sell_card.points();
            best = Some(sell_card);
        }
    }
    best
}

// ── Values ──

fn points_immediate(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.points
}

fn ducats_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.ducats.max(0) as f64
}

fn ducats_spent(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (-d.ducats).max(0) as f64
}

fn colors_demanded(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..NUM_COLORS)
        .map(|i| d.gained_colors[i] as f64 * ctx.color_value[i])
        .sum()
}

fn colors_undemanded(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..NUM_COLORS)
        .filter(|&i| ctx.color_demand[i] == 0)
        .map(|i| d.gained_colors[i] as f64)
        .sum()
}

fn colors_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.gained_color_units() as f64
}

fn colors_spent_demanded(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(0..NUM_COLORS)
        .map(|i| d.spent_colors[i] as f64 * ctx.color_value[i])
        .sum::<f64>()
}

fn colors_spent(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(d.spent_color_units() as f64)
}

fn materials_demanded(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..3)
        .map(|i| d.gained_materials[i] as f64 * ctx.material_value[i])
        .sum()
}

fn materials_undemanded(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..3)
        .filter(|&i| ctx.material_demand[i] == 0)
        .map(|i| d.gained_materials[i] as f64)
        .sum()
}

fn materials_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.gained_material_units() as f64
}

fn materials_spent(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(0..3)
        .map(|i| d.spent_materials[i] as f64 * ctx.material_value[i])
        .sum::<f64>()
}

fn materials_spent_raw(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(d.spent_material_units() as f64)
}

fn boats_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    0.5 * d.boats_gained as f64
}

fn boats_spent(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(d.boats_spent as f64)
}

fn wilds_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    0.5 * d.wilds_gained as f64
}

fn wilds_spent(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(d.wilds_spent as f64)
}

fn card_quality_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.gained_card_quality
}

fn trash_thinning(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.cards_trashed as f64
}

fn trash_quality(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -d.trash_quality
}

fn ability_slot(d: &Delta, kind: usize) -> f64 {
    d.abilities[kind] as f64
}

fn ability_workshop(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_WORKSHOP)
}

fn ability_mix(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_MIX)
}

fn ability_sell(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_SELL)
}

fn ability_ducats(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_GAIN_DUCATS)
}

fn ability_secondary(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_GAIN_SECONDARY)
}

fn ability_wilds(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_GAIN_WILD_PRIMARIES)
}

fn ability_boats(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_GAIN_BOATS)
}

fn ability_move_to_pool(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    ability_slot(d, ABILITY_MOVE_TO_DRAFT_POOL)
}

// ── The plan the player is on ──

fn color_diversity_delta(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut before = 0i32;
    let mut after = 0i32;
    for i in 0..NUM_COLORS {
        let held = ctx.player.color_wheel.counts[i] as i32;
        if held > 0 {
            before += 1;
        }
        if held + d.gained_colors[i] as i32 - d.spent_colors[i] as i32 > 0 {
            after += 1;
        }
    }
    (after - before) as f64
}

fn buyer_progress_points(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let Some(sell_card) = best_unaffordable_buyer(ctx) else {
        return 0.0;
    };
    let stats = sell_card.stats();
    let per_unit = if stats.points == 0 {
        0.0
    } else {
        stats.points as f64 / sale_units(sell_card)
    };

    let mut supplied = 0.0;
    for &(color, needed) in stats.colors() {
        supplied += (d.gained_colors[color.index()] as u32).min(needed as u32) as f64;
    }
    for (i, &needed) in stats.materials.iter().enumerate() {
        supplied += (d.gained_materials[i]).min(needed as u32) as f64;
    }
    // A boat or a wild stands in for whatever is missing.
    supplied += d.boats_gained as f64 + d.wilds_gained as f64;

    supplied * per_unit
}

fn buyer_completion_points(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let Some(sell_card) = best_unaffordable_buyer(ctx) else {
        return 0.0;
    };
    if affordable_after(ctx, d, sell_card) {
        sell_card.points() as f64
    } else {
        0.0
    }
}

fn pending_points_share(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    if ctx.pending_points == 0 {
        0.0
    } else {
        d.points / ctx.pending_points as f64
    }
}

// ── Draft ──

fn draft_color_demand(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = drafted_card_of(choice) else {
        return 0.0;
    };
    card.colors()
        .iter()
        .filter(|&&color| ctx.color_demand[color.index()] > 0)
        .count() as f64
}

fn draft_material_demand(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = drafted_card_of(choice) else {
        return 0.0;
    };
    card.material_types()
        .iter()
        .filter(|&&material| ctx.material_demand[material as usize] > 0)
        .count() as f64
}

fn draft_ability_count(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = drafted_card_of(choice) else {
        return 0.0;
    };
    (card.trash_abilities().len() + card.workshop_abilities().len()) as f64
}

fn draft_redundancy(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = drafted_card_of(choice) else {
        return 0.0;
    };
    let mut shared = 0u32;
    for_each_owned_card(ctx, |other| {
        let already = card
            .trash_abilities()
            .iter()
            .chain(card.workshop_abilities())
            .any(|ability| {
                other.trash_abilities().contains(ability)
                    || other.workshop_abilities().contains(ability)
            });
        if already {
            shared += 1;
        }
    });
    shared as f64
}

// ── Sales ──

fn sell_points(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    sale_card_of(choice).map_or(0.0, |sell_card| sell_card.points() as f64)
}

fn sell_points_per_unit(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match sale_card_of(choice) {
        Some(sell_card) => sell_card.points() as f64 / sale_units(sell_card),
        None => 0.0,
    }
}

fn sell_boat_cost(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match sale_card_of(choice) {
        Some(sell_card) => -(boats_to_sell(ctx.player, sell_card) as f64),
        None => 0.0,
    }
}

// ── The buyers phase ──

fn offer_points(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    offered_card_of(choice).map_or(0.0, |sell_card| sell_card.points() as f64)
}

fn offer_affordable(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match offered_card_of(choice) {
        Some(sell_card) if can_afford_sell_card(ctx.player, &sell_card) => 1.0,
        _ => 0.0,
    }
}

fn offer_distance(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(sell_card) = offered_card_of(choice) else {
        return 0.0;
    };
    let boats = boats_to_sell(ctx.player, sell_card) as f64;
    let wilds = match wild_primaries_to_cover_sale(&ctx.player.color_wheel, sell_card.stats()) {
        Some(needed) => needed as f64,
        // A colour no wild primary can cover: as far away as it gets.
        None => 9.0,
    };
    -(boats + wilds)
}

fn offer_deck_gamble(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if matches!(choice, Choice::DrawBuyer) {
        1.0
    } else {
        0.0
    }
}

// ── Mixing ──

fn mix_output_demanded(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if mix_sequence_of(choice).is_none() {
        return 0.0;
    }
    (0..NUM_COLORS)
        .filter(|&i| ctx.color_demand[i] > 0)
        .map(|i| d.gained_colors[i] as f64)
        .sum()
}

fn mix_higher_tier(_ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if mix_sequence_of(choice).is_none() {
        return 0.0;
    }
    crate::colors::SECONDARIES
        .iter()
        .chain(crate::colors::TERTIARIES.iter())
        .map(|color| d.gained_colors[color.index()] as f64)
        .sum()
}

fn mix_burns_primaries(_ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if mix_sequence_of(choice).is_none() {
        return 0.0;
    }
    -(PRIMARIES
        .iter()
        .map(|color| d.spent_colors[color.index()] as f64)
        .sum::<f64>())
}

fn mix_count(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    mix_sequence_of(choice).map_or(0.0, |mixes| mixes.len() as f64)
}

// ── Workshop ──

fn workshop_set_size(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match choice {
        Choice::Workshop { card_types } | Choice::TrashAndWorkshop { workshop_cards: card_types, .. } => {
            card_types.len() as f64
        }
        _ => 0.0,
    }
}

// ── Ending the turn ──

fn end_turn_unspent_cards(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if matches!(choice, Choice::EndTurn) {
        ctx.drafted as f64
    } else {
        0.0
    }
}

fn end_turn_open_actions(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if matches!(choice, Choice::EndTurn) {
        open_actions(ctx) as f64
    } else {
        0.0
    }
}

fn end_turn_late_game(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if !matches!(choice, Choice::EndTurn) {
        return 0.0;
    }
    let max_rounds = ctx.max_rounds.max(1);
    1.0 - (ctx.rounds_left - 1) as f64 / max_rounds as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::context::PhaseKind;
    use crate::policy::effects::{delta, NUM_ABILITY_KINDS};
    use crate::policy::test_support;
    use crate::scoring::{CardHeuristicTable, HeuristicParams};

    #[test]
    fn feature_names_are_unique() {
        let mut names: Vec<&str> = FEATURES.iter().map(|def| def.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two features share a name");
    }

    #[test]
    fn every_ability_slot_has_a_weight() {
        // One feature per ability kind, so no ability is silently free.
        const ABILITY_FEATURES: [&str; NUM_ABILITY_KINDS] = [
            "ability_workshop",
            "ability_mix",
            "ability_move_to_pool",
            "ability_sell",
            "ability_ducats",
            "ability_secondary",
            "ability_wilds",
            "ability_boats",
        ];
        for name in ABILITY_FEATURES {
            let def = lookup(name).unwrap_or_else(|| panic!("{name} is not a feature"));
            assert_eq!(def.group, Group::Universal);
        }
    }

    #[test]
    fn no_feature_fires_outside_its_group() {
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let mut checked = 0;
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                for choice in &choices {
                    let ctx = Ctx::new(&state, player_index, &table);
                    let d = delta(ctx.player, choice, &table);
                    for def in FEATURES {
                        if def.group.applies_to(choice) {
                            continue;
                        }
                        let value = (def.eval)(&ctx, choice, &d);
                        assert_eq!(
                            value, 0.0,
                            "{} fired on {choice:?} outside {:?}",
                            def.name, def.group
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 0, "no choices were reachable to check");
    }

    #[test]
    fn no_feature_returns_a_non_finite_number() {
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        for state in test_support::reachable_states() {
            for player_index in 0..state.players.len() {
                let choices = crate::choices::enumerate_choices(&state, player_index);
                for choice in &choices {
                    let ctx = Ctx::new(&state, player_index, &table);
                    let d = delta(ctx.player, choice, &table);
                    for def in FEATURES {
                        let value = (def.eval)(&ctx, choice, &d);
                        assert!(
                            value.is_finite(),
                            "{} returned {value} on {choice:?}",
                            def.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_buyers_phase_groups_read_their_own_choice() {
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let state = test_support::game_at_buyers_phase();
        let choices = crate::choices::enumerate_choices(&state, 0);
        let ctx = Ctx::new(&state, 0, &table);
        assert_eq!(ctx.phase, PhaseKind::Buyers);
        let offered = choices
            .iter()
            .find_map(|choice| offered_card_of(choice))
            .expect("the buyers phase offers a face-up card");
        assert!(offered.points() > 0);
    }
}
