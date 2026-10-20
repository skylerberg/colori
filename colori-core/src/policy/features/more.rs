//! A second, larger vocabulary of rules.
//!
//! The first table answers "what does this choice move, and what is that worth".
//! These answer harder questions: which plan does it serve, how close does it
//! get to finished, what does it open up, what does it close off, and what does
//! it give up.
//!
//! Each rule is one small, general reading of the position, priced in points,
//! and knows nothing about any other rule. Nothing here says what the bot should
//! do; the vocabulary says what the bot *can notice*, and the weights say how
//! much each thing matters. Combinations are supposed to come out of layering:
//! a rule that prices a card's materials at what the sitting buyers pay, next to
//! a rule that says how much a buyer still wants, next to a rule that says
//! whether the deck already supplies it, is what lets the bot keep the card that
//! gives it a ceramics and a red because something on the table takes two
//! ceramics.
//!
//! Two disciplines hold across the table:
//!
//! - **Every rule must vary with the choice.** A quantity that is the same for
//!   every choice in a decision cancels out of the comparison and is wasted work;
//!   that kind of reading belongs in [`super::super::value`] instead.
//! - **Everything is in points.** A count of wanted colours and a colour's point
//!   value are different units, and mixing them is what made the first draft of
//!   the draft rules useless.

use crate::colors::{is_primary, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;

use super::{sale_card_of, storage_value, FeatureDef, Group};
use super::super::context::Ctx;
use super::super::effects::{
    ability_kind, Delta, ABILITY_GAIN_BOATS, ABILITY_GAIN_SECONDARY, ABILITY_GAIN_WILD_PRIMARIES,
    ABILITY_MIX, ABILITY_MOVE_TO_DRAFT_POOL, ABILITY_SELL, ABILITY_WORKSHOP,
};

// ── Shared readings ──

/// The wheel as it would be with this choice's effect applied.
fn wheel_after(ctx: &Ctx, d: &Delta) -> [i32; NUM_COLORS] {
    let mut wheel = [0i32; NUM_COLORS];
    for i in 0..NUM_COLORS {
        wheel[i] = ctx.player.color_wheel.counts[i] as i32 + d.gained_colors[i] as i32
            - d.spent_colors[i] as i32;
    }
    wheel
}

fn colour_held(ctx: &Ctx, d: &Delta, color: Color) -> i32 {
    ctx.player.color_wheel.counts[color.index()] as i32 + d.gained_colors[color.index()] as i32
        - d.spent_colors[color.index()] as i32
}

fn material_held(ctx: &Ctx, d: &Delta, index: usize) -> i32 {
    ctx.player.materials.counts[index] as i32 + d.gained_materials[index] as i32
        - d.spent_materials[index] as i32
}

/// Whether `sell_card` would be affordable once `d` is applied.
fn affordable_after(ctx: &Ctx, d: &Delta, sell_card: SellCard) -> bool {
    let stats = sell_card.stats();
    let mut boats = stats.boats as i32;
    for (index, &needed) in stats.materials.iter().enumerate() {
        boats += (needed as i32 - material_held(ctx, d, index)).max(0);
    }
    if boats > ctx.player.boats as i32 + d.boats_gained as i32 - d.boats_spent as i32 {
        return false;
    }
    let mut wilds = 0i32;
    for &(color, needed) in stats.colors() {
        let short = (needed as i32 - colour_held(ctx, d, color)).max(0);
        if short > 0 {
            if !is_primary(color) {
                return false;
            }
            wilds += short;
        }
    }
    wilds <= ctx.player.wild_primaries as i32 + d.wilds_gained as i32 - d.wilds_spent as i32
}

/// The colours a card would put on the wheel, and its materials, if kept.
fn card_storage(ctx: &Ctx, card: Card) -> f64 {
    storage_value(ctx, card)
}

// ── The wheel ──

fn colours_completing(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut completing = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let stats = buyer.sell_card.stats();
        if stats.colors().is_empty() {
            continue;
        }
        let covered = stats.colors().iter().all(|&(color, needed)| {
            colour_held(ctx, d, color) >= needed as i32
        });
        if covered {
            completing += 1.0;
        }
    }
    completing
}

fn colours_shared_demand(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..NUM_COLORS)
        .map(|i| {
            let plans = ctx.color_buyers[i] as f64;
            if plans > 1.0 {
                d.gained_colors[i] as f64 * (plans - 1.0)
            } else {
                0.0
            }
        })
        .sum()
}

fn colours_tertiary_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    TERTIARIES
        .iter()
        .map(|color| d.gained_colors[color.index()] as f64)
        .sum()
}

fn colours_secondary_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    SECONDARIES
        .iter()
        .map(|color| d.gained_colors[color.index()] as f64)
        .sum()
}

/// Primaries are the flexible currency — everything else can be mixed from them
/// and sales of any shape want them — so spending one costs more than the unit.
fn colours_primary_spent(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    -(PRIMARIES
        .iter()
        .map(|color| d.spent_colors[color.index()] as f64)
        .sum::<f64>())
}

fn colours_duplicate_gained(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..NUM_COLORS)
        .filter(|&i| ctx.player.color_wheel.counts[i] >= 2)
        .map(|i| d.gained_colors[i] as f64)
        .sum()
}

fn colours_breadth_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.gained_colors.iter().filter(|&&count| count > 0).count() as f64
}

/// How many more mixes the wheel could supply afterwards.
///
/// A wheel that can mix keeps its options open, and mixing is the only way to
/// reach a secondary or tertiary from primaries.
fn wheel_mixing_options_delta(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    if d.gained_color_units() == 0 && d.spent_color_units() == 0 {
        return 0.0;
    }
    let wheel = wheel_after(ctx, d);
    let wilds = (ctx.player.wild_primaries as i32 + d.wilds_gained as i32 - d.wilds_spent as i32)
        .max(0) as u32;
    let mut after = 0.0;
    for &(a, b) in VALID_MIX_PAIRS.iter() {
        if supplies_mix(&wheel, wilds, a, b) {
            after += 1.0;
        }
    }
    after - ctx.mixable_pairs as f64
}

fn supplies_mix(wheel: &[i32; NUM_COLORS], wilds: u32, a: Color, b: Color) -> bool {
    let mut needed_wilds = 0u32;
    for color in [a, b] {
        if wheel[color.index()] > 0 {
            continue;
        }
        if !is_primary(color) {
            return false;
        }
        needed_wilds += 1;
    }
    needed_wilds <= wilds
}

// ── Materials ──

fn materials_completing(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut completing = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let stats = buyer.sell_card.stats();
        let wanted: u32 = stats.materials.iter().map(|&m| m as u32).sum();
        if wanted == 0 {
            continue;
        }
        let mut covered = 0u32;
        for (index, &needed) in stats.materials.iter().enumerate() {
            covered += (material_held(ctx, d, index).max(0) as u32).min(needed as u32);
        }
        if covered >= wanted {
            completing += 1.0;
        }
    }
    completing
}

fn materials_shared_demand(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..3)
        .map(|i| {
            let plans = ctx.material_buyers[i] as f64;
            if plans > 1.0 {
                d.gained_materials[i] as f64 * (plans - 1.0)
            } else {
                0.0
            }
        })
        .sum()
}

fn materials_surplus_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    (0..3)
        .map(|i| {
            let wanted = _ctx.material_demand[i] as i32;
            let held = _ctx.player.materials.counts[i] as i32 + d.gained_materials[i] as i32;
            (held - wanted).max(0) - (_ctx.player.materials.counts[i] as i32 - wanted).max(0)
        })
        .sum::<i32>()
        .max(0) as f64
}

fn materials_diversity_gained(_ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    d.gained_materials.iter().filter(|&&count| count > 0).count() as f64
}

/// Materials that save a boat: a unit the wheel is short of, where a boat is
/// what would otherwise have covered it. A boat is half a point and a wild
/// material both, so this is dearer than a unit nobody needed.
fn materials_boat_relief(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut relief = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let stats = buyer.sell_card.stats();
        for (index, &needed) in stats.materials.iter().enumerate() {
            let missing = (needed as i32 - ctx.player.materials.counts[index] as i32).max(0);
            if missing > 0 {
                relief += (d.gained_materials[index]).min(missing as u32) as f64;
            }
        }
    }
    relief
}

// ── Options: wilds, boats, ducats ──

/// What a ducat still in hand is worth: its point, and the purchases it could
/// still make.
fn ducats_options(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let held = ctx.player.ducats as i32 + d.ducats;
    if held <= 0 {
        return 0.0;
    }
    held as f64 * ctx.purchasable as f64
}

// ── The plan ──

fn buyer_short_one_filled(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut filled = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let sell_card = buyer.sell_card;
        let before = super::super::context::buyer_short_units(ctx.player, sell_card);
        if before != 1 {
            continue;
        }
        if affordable_after(ctx, d, sell_card) {
            filled += 1.0;
        }
    }
    filled
}

fn buyer_affordable_delta(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let after = ctx
        .player
        .buyers
        .iter()
        .filter(|buyer| affordable_after(ctx, d, buyer.sell_card))
        .count() as f64;
    after - ctx.buyers_affordable as f64
}

/// The points got per unit spent, summed over the buyers this choice completes.
fn buyer_completion_efficiency(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let mut total = 0.0;
    for buyer in ctx.player.buyers.iter() {
        let sell_card = buyer.sell_card;
        if !affordable_after(ctx, d, sell_card) {
            continue;
        }
        let before = super::super::context::buyer_short_units(ctx.player, sell_card);
        if before == 0 {
            continue;
        }
        let stats = sell_card.stats();
        let units = stats.total_colors as u32
            + stats.materials.iter().map(|&m| m as u32).sum::<u32>()
            + stats.boats;
        total += stats.points as f64 / units.max(1) as f64;
    }
    total
}

/// Selling empties the buyer slot, and the slot stays empty until the next
/// buyers phase refills it — possibly with something worse.
fn buyer_slot_freed(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match choice {
        Choice::SelectSellCard { .. } | Choice::TrashAndSell { .. } if ctx.empty_slots < ctx.slots => {
            1.0
        }
        _ => 0.0,
    }
}

fn buyer_stacked_demand(ctx: &Ctx, _choice: &Choice, d: &Delta) -> f64 {
    let colors: f64 = (0..NUM_COLORS)
        .filter(|&i| ctx.color_buyers[i] > 1)
        .map(|i| d.gained_colors[i] as f64)
        .sum();
    let materials: f64 = (0..3)
        .filter(|&i| ctx.material_buyers[i] > 1)
        .map(|i| d.gained_materials[i] as f64)
        .sum();
    colors + materials
}

/// What a sale spends, priced at what the *other* buyers would have paid.
///
/// The sold buyer's own share is not a cost: those units were earmarked for
/// exactly this sale, and the sale's points are the return on them.
fn sell_net_opportunity_cost(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    let Some(sell_card) = sale_card_of(choice) else {
        return 0.0;
    };
    let stats = sell_card.stats();
    let units = stats.total_colors as u32
        + stats.materials.iter().map(|&m| m as u32).sum::<u32>()
        + stats.boats;
    let per_unit = stats.points as f64 / units.max(1) as f64;

    let mut cost = 0.0;
    for &(color, needed) in stats.colors() {
        let spent = d.spent_colors[color.index()] as f64;
        let own = per_unit * (d.spent_colors[color.index()] as u32).min(needed as u32) as f64;
        cost += spent * ctx.color_value[color.index()] - own;
    }
    for (index, &needed) in stats.materials.iter().enumerate() {
        let spent = d.spent_materials[index] as f64;
        let own = per_unit * (d.spent_materials[index]).min(needed as u32) as f64;
        cost += spent * ctx.material_value[index] - own;
    }
    -cost.max(0.0)
}

fn sell_is_the_best_affordable(_ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match sale_card_of(choice) {
        Some(sell_card) if sell_card.points() == _ctx.best_affordable_points => 1.0,
        _ => 0.0,
    }
}

fn offer_material_alignment(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(sell_card) = super::offered_card_of(choice) else {
        return 0.0;
    };
    let stats = sell_card.stats();
    let wanted: u32 = stats.materials.iter().map(|&m| m as u32).sum();
    if wanted == 0 {
        return 0.0;
    }
    let mut held = 0u32;
    for (index, &needed) in stats.materials.iter().enumerate() {
        held += (ctx.player.materials.counts[index]).min(needed as u32);
    }
    held as f64 / wanted as f64
}

fn offer_color_alignment(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(sell_card) = super::offered_card_of(choice) else {
        return 0.0;
    };
    let stats = sell_card.stats();
    let wanted = stats.total_colors as u32;
    if wanted == 0 {
        return 0.0;
    }
    let mut held = 0u32;
    for &(color, needed) in stats.colors() {
        held += ctx.player.color_wheel.get(color).min(needed as u32);
    }
    held as f64 / wanted as f64
}

/// Drawing blind gives up a card that was on display and known.
fn offer_blind_cost(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    if !matches!(choice, Choice::DrawBuyer) {
        return 0.0;
    }
    let best = ctx
        .state
        .sell_card_display
        .iter()
        .map(|card| card.sell_card.points())
        .max()
        .unwrap_or(0);
    -(best as f64)
}

// ── The cards ──

fn draft_fills_gap(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = super::drafted_card_of(choice) else {
        return 0.0;
    };
    let mut have_colors = [0u32; NUM_COLORS];
    let mut have_materials = [0u32; 3];
    super::for_each_owned_card(ctx, |other| {
        for &color in other.colors() {
            have_colors[color.index()] += 1;
        }
        for &material in other.material_types() {
            have_materials[material as usize] += 1;
        }
    });
    let mut gap = 0.0;
    for &color in card.colors() {
        if have_colors[color.index()] == 0 && ctx.color_buyers[color.index()] > 0 {
            gap += 1.0;
        }
    }
    for &material in card.material_types() {
        if have_materials[material as usize] == 0 && ctx.material_buyers[material as usize] > 0 {
            gap += 1.0;
        }
    }
    gap
}

fn draft_redundant_supply(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = super::drafted_card_of(choice) else {
        return 0.0;
    };
    let mut have_colors = [0u32; NUM_COLORS];
    let mut have_materials = [0u32; 3];
    super::for_each_owned_card(ctx, |other| {
        for &color in other.colors() {
            have_colors[color.index()] += 1;
        }
        for &material in other.material_types() {
            have_materials[material as usize] += 1;
        }
    });
    let colors = card
        .colors()
        .iter()
        .filter(|&&color| have_colors[color.index()] >= 2)
        .count() as f64;
    let materials = card
        .material_types()
        .iter()
        .filter(|&&material| have_materials[material as usize] >= 2)
        .count() as f64;
    -(colors + materials)
}

/// The card supplies something the player already partly holds for a buyer who
/// wants more than one of it: the case where a buyer takes two ceramics and the
/// card is the ceramics the shelf is short of.
fn draft_completes_an_order(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = super::drafted_card_of(choice) else {
        return 0.0;
    };
    let mut fits = 0.0;
    for &material in card.material_types() {
        let index = material as usize;
        if ctx.material_buyers[index] > 0 && ctx.player.materials.counts[index] > 0 {
            fits += 1.0;
        }
    }
    for &color in card.colors() {
        if ctx.color_buyers[color.index()] > 0 && ctx.player.color_wheel.get(color) > 0 {
            fits += 1.0;
        }
    }
    fits
}

fn draft_ability_novelty(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = super::drafted_card_of(choice) else {
        return 0.0;
    };
    let mut held = [false; super::super::effects::NUM_ABILITY_KINDS];
    super::for_each_owned_card(ctx, |other| {
        for &ability in other.trash_abilities().iter().chain(other.workshop_abilities()) {
            held[ability_kind(ability)] = true;
        }
    });
    card.trash_abilities()
        .iter()
        .chain(card.workshop_abilities())
        .filter(|ability| !held[ability_kind(**ability)])
        .count() as f64
}

fn draft_no_demand(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let Some(card) = super::drafted_card_of(choice) else {
        return 0.0;
    };
    let wanted = card
        .colors()
        .iter()
        .any(|&color| ctx.color_buyers[color.index()] > 0)
        || card
            .material_types()
            .iter()
            .any(|&material| ctx.material_buyers[material as usize] > 0);
    if wanted {
        0.0
    } else {
        -1.0
    }
}

/// The card's colours and materials, weighted towards the highest-point buyer.
fn card_serves_best_buyer(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    let card = match choice {
        Choice::DraftPick { card } => *card,
        Choice::TrashDraftedCard { card }
        | Choice::TrashAndMix { card, .. }
        | Choice::TrashAndSell { card, .. }
        | Choice::TrashAndWorkshop { card, .. }
        | Choice::TrashAndMoveToDraftPool { card, .. } => *card,
        _ => return 0.0,
    };
    let Some(best) = ctx
        .player
        .buyers
        .iter()
        .max_by_key(|buyer| buyer.sell_card.points())
    else {
        return 0.0;
    };
    let stats = best.sell_card.stats();
    let wanted_colors = stats.colors().iter().map(|&(color, needed)| (color, needed)).collect::<Vec<_>>();
    let mut fits = 0.0;
    for &color in card.colors() {
        if wanted_colors.iter().any(|&(wanted, _)| wanted == color) {
            fits += 1.0;
        }
    }
    for &material in card.material_types() {
        if stats.materials[material as usize] > 0 {
            fits += 1.0;
        }
    }
    fits
}

fn picks_that_store(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    match choice {
        Choice::Workshop { card_types } | Choice::TrashAndWorkshop { workshop_cards: card_types, .. } => {
            card_types
                .iter()
                .map(|&card| card_storage(ctx, card))
                .sum()
        }
        _ => 0.0,
    }
}

// ── Abilities, priced by what they can do here ──

fn abilities(d: &Delta, kind: usize) -> f64 {
    d.abilities[kind] as f64
}

/// These price an activation against what it can do *right now*, which is only a
/// question about a choice made in the turn. A drafted card's abilities are
/// priced by the flat rules instead.
fn activation(choice: &Choice) -> bool {
    Group::Action.applies_to(choice)
}

fn ability_workshop_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    abilities(d, ABILITY_WORKSHOP) * ctx.workshop_storage
}

fn ability_mix_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    abilities(d, ABILITY_MIX) * ctx.best_mix_value
}

/// Moving a card to the draft pool buys the chance to use it — trash it for its
/// ability, or leave it to settle into the deck — so it is worth the best card
/// in the workshop can offer.
fn ability_move_to_pool_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    abilities(d, ABILITY_MOVE_TO_DRAFT_POOL) * ctx.workshop_storage
}

fn ability_secondary_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    let best = SECONDARIES
        .iter()
        .map(|color| ctx.value_of(*color))
        .fold(0.0f64, f64::max);
    abilities(d, ABILITY_GAIN_SECONDARY) * best
}

fn ability_wilds_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    abilities(d, ABILITY_GAIN_WILD_PRIMARIES) * ctx.wild_value
}

fn ability_boats_priced(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    abilities(d, ABILITY_GAIN_BOATS) * ctx.boat_value
}

fn ability_sell_ready(ctx: &Ctx, choice: &Choice, d: &Delta) -> f64 {
    if !activation(choice) {
        return 0.0;
    }
    if ctx.buyers_affordable == 0 {
        return 0.0;
    }
    abilities(d, ABILITY_SELL) * ctx.best_affordable_points as f64
}

// ── Time ──

/// Storage value weighted towards the early game: a card kept in round two
/// comes round five more times, one kept in round six once.
fn spare_mixer(ctx: &Ctx, choice: &Choice, _d: &Delta) -> f64 {
    // A mix that spends a colour the wheel holds several of costs less than one
    // that spends the last of it.
    let Some(mixes) = super::mix_sequence_of(choice) else {
        return 0.0;
    };
    let mut cost = 0.0;
    for &(a, b) in mixes.iter() {
        for color in [a, b] {
            let held = ctx.player.color_wheel.get(color);
            cost += if held > 1 { 0.0 } else { 1.0 };
        }
    }
    -cost
}

// ── The table ──

pub const MORE_RULES: &[FeatureDef] = &[
    FeatureDef {
        name: "colours_completing",
        group: Group::Universal,
        doc: "Buyers whose colour cost the choice satisfies in full.",
        eval: colours_completing,
    },
    FeatureDef {
        name: "colours_shared_demand",
        group: Group::Universal,
        doc: "Colour units gained that more than one sitting buyer wants.",
        eval: colours_shared_demand,
    },
    FeatureDef {
        name: "colours_tertiary_gained",
        group: Group::Universal,
        doc: "Tertiary colour units gained. Tertiaries are what the dear sell cards ask for.",
        eval: colours_tertiary_gained,
    },
    FeatureDef {
        name: "colours_secondary_gained",
        group: Group::Universal,
        doc: "Secondary colour units gained.",
        eval: colours_secondary_gained,
    },
    FeatureDef {
        name: "colours_primary_spent",
        group: Group::Universal,
        doc: "Primary units spent, negated. A primary is the flexible currency: mixes and \\
              sales of every shape are built from them.",
        eval: colours_primary_spent,
    },
    FeatureDef {
        name: "colours_duplicate_gained",
        group: Group::Universal,
        doc: "Units gained in colours already held twice over.",
        eval: colours_duplicate_gained,
    },
    FeatureDef {
        name: "colours_breadth_gained",
        group: Group::Universal,
        doc: "Distinct colours the choice adds, however many units of each.",
        eval: colours_breadth_gained,
    },
    FeatureDef {
        name: "wheel_mixing_options_delta",
        group: Group::Universal,
        doc: "Change in how many mixes the wheel could supply. Mixing is the only way to \\
              reach a secondary or tertiary from primaries, so options here are worth keeping.",
        eval: wheel_mixing_options_delta,
    },
    FeatureDef {
        name: "materials_completing",
        group: Group::Universal,
        doc: "Buyers whose material cost the choice satisfies in full.",
        eval: materials_completing,
    },
    FeatureDef {
        name: "materials_shared_demand",
        group: Group::Universal,
        doc: "Material units gained that more than one sitting buyer wants.",
        eval: materials_shared_demand,
    },
    FeatureDef {
        name: "materials_surplus_gained",
        group: Group::Universal,
        doc: "Material units gained beyond everything the buyers would take.",
        eval: materials_surplus_gained,
    },
    FeatureDef {
        name: "materials_diversity_gained",
        group: Group::Universal,
        doc: "Distinct material types the choice adds.",
        eval: materials_diversity_gained,
    },
    FeatureDef {
        name: "materials_boat_relief",
        group: Group::Universal,
        doc: "Material units that fill a gap a boat would otherwise have covered.",
        eval: materials_boat_relief,
    },
    FeatureDef {
        name: "ducats_options",
        group: Group::Universal,
        doc: "Ducats held afterwards, weighed by how many purchases they could still buy.",
        eval: ducats_options,
    },
    FeatureDef {
        name: "buyer_short_one_filled",
        group: Group::Universal,
        doc: "Buyers that were exactly one unit short and no longer are.",
        eval: buyer_short_one_filled,
    },
    FeatureDef {
        name: "buyer_affordable_delta",
        group: Group::Universal,
        doc: "Change in how many sitting buyers could be sold to right now.",
        eval: buyer_affordable_delta,
    },
    FeatureDef {
        name: "buyer_completion_efficiency",
        group: Group::Universal,
        doc: "Points per unit of the buyers this choice completes: a cheap sale is a good one.",
        eval: buyer_completion_efficiency,
    },
    FeatureDef {
        name: "buyer_slot_freed",
        group: Group::Universal,
        doc: "One when a sale empties a buyer slot that the next buyers phase must refill.",
        eval: buyer_slot_freed,
    },
    FeatureDef {
        name: "buyer_stacked_demand",
        group: Group::Universal,
        doc: "Units gained that two or more sitting buyers both want.",
        eval: buyer_stacked_demand,
    },
    FeatureDef {
        name: "sell_net_opportunity_cost",
        group: Group::Sell,
        doc: "What a sale spends, priced at what the *other* buyers would have paid. The \\
              sold buyer's own share is not a cost: those units were earmarked for it.",
        eval: sell_net_opportunity_cost,
    },
    FeatureDef {
        name: "sell_is_the_best_affordable",
        group: Group::Sell,
        doc: "One when this is the highest-point buyer that could be sold to.",
        eval: sell_is_the_best_affordable,
    },
    FeatureDef {
        name: "offer_material_alignment",
        group: Group::Buyers,
        doc: "Share of the offered buyer's material cost already stored.",
        eval: offer_material_alignment,
    },
    FeatureDef {
        name: "offer_color_alignment",
        group: Group::Buyers,
        doc: "Share of the offered buyer's colour cost already on the wheel.",
        eval: offer_color_alignment,
    },
    FeatureDef {
        name: "offer_blind_cost",
        group: Group::Buyers,
        doc: "Negative of the best card on display, when the choice draws blind instead.",
        eval: offer_blind_cost,
    },
    FeatureDef {
        name: "spare_mixer",
        group: Group::Mix,
        doc: "Mixes that spend a colour the wheel holds only one of, negated.",
        eval: spare_mixer,
    },
    FeatureDef {
        name: "draft_fills_gap",
        group: Group::Draft,
        doc: "Colours and materials the picked card supplies that no card the player owns \\
              supplies, where a sitting buyer wants them.",
        eval: draft_fills_gap,
    },
    FeatureDef {
        name: "draft_redundant_supply",
        group: Group::Draft,
        doc: "What the picked card supplies that two or more owned cards already supply, \\
              negated.",
        eval: draft_redundant_supply,
    },
    FeatureDef {
        name: "draft_completes_an_order",
        group: Group::Draft,
        doc: "Units the picked card supplies that a buyer wants and the player already \\
              holds some of: the card that finishes an order rather than starting one.",
        eval: draft_completes_an_order,
    },
    FeatureDef {
        name: "draft_ability_novelty",
        group: Group::Draft,
        doc: "Ability kinds on the picked card that nothing the player owns can produce.",
        eval: draft_ability_novelty,
    },
    FeatureDef {
        name: "draft_no_demand",
        group: Group::Draft,
        doc: "Negative one when the picked card supplies nothing any sitting buyer wants.",
        eval: draft_no_demand,
    },
    FeatureDef {
        name: "card_serves_best_buyer",
        group: Group::Universal,
        doc: "Units the card gives up or takes that the highest-point sitting buyer wants.",
        eval: card_serves_best_buyer,
    },
    FeatureDef {
        name: "picks_that_store",
        group: Group::Workshop,
        doc: "Storage value of the cards a workshop pick takes, in points.",
        eval: picks_that_store,
    },
    FeatureDef {
        name: "ability_workshop_priced",
        group: Group::Action,
        doc: "Workshop picks granted, each priced at the best storage the workshop offers.",
        eval: ability_workshop_priced,
    },
    FeatureDef {
        name: "ability_mix_priced",
        group: Group::Action,
        doc: "Mixes granted, each priced at the best colour the wheel could mix to.",
        eval: ability_mix_priced,
    },
    FeatureDef {
        name: "ability_move_to_pool_priced",
        group: Group::Action,
        doc: "Move-to-draft-pool grants, each priced at the best card in the workshop: \\
              moving a card buys either its ability now or its storage later.",
        eval: ability_move_to_pool_priced,
    },
    FeatureDef {
        name: "ability_secondary_priced",
        group: Group::Action,
        doc: "Gain-a-secondary grants, priced at the dearest secondary a buyer wants.",
        eval: ability_secondary_priced,
    },
    FeatureDef {
        name: "ability_wilds_priced",
        group: Group::Action,
        doc: "Wild-primary grants, priced at what a wild is worth against the wheel.",
        eval: ability_wilds_priced,
    },
    FeatureDef {
        name: "ability_boats_priced",
        group: Group::Action,
        doc: "Boat grants, priced at what a boat is worth against the buyers.",
        eval: ability_boats_priced,
    },
    FeatureDef {
        name: "ability_sell_ready",
        group: Group::Action,
        doc: "Sell grants, priced at the best buyer that could actually be sold to.",
        eval: ability_sell_ready,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_here_has_a_name_and_a_doc() {
        for def in MORE_RULES {
            assert!(!def.name.is_empty());
            assert!(def.name.len() < 40, "{} is a sentence", def.name);
            assert!(def.doc.len() > 20, "{} says nothing", def.name);
            assert!(
                def.name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{} is not snake_case",
                def.name
            );
        }
    }

}
