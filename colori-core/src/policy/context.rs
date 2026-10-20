//! What every feature needs, worked out once per decision.
//!
//! A decision scores every legal choice. Each choice needs the same picture of
//! the player's position — which colours the sitting buyers still want, what a
//! unit of each is worth in points — so it is built once here and shared.

use crate::action_phase::{boats_to_sell, can_afford_sell_card, can_buy_with_ducat};
use crate::colors::{can_supply_mix, mix_result, wild_primaries_to_cover_sale, PRIMARIES, VALID_MIX_PAIRS};
use crate::types::ALL_DUCAT_PURCHASES;
use crate::scoring::CardHeuristicTable;
use crate::types::*;

/// Which phase a decision is being made in. The rules that matter differ
/// enough between them that features are documented and tested by group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseKind {
    Buyers,
    Draft,
    Action,
    Other,
}

pub struct Ctx<'a> {
    pub state: &'a GameState,
    pub player_index: usize,
    pub player: &'a PlayerState,
    pub card_table: &'a CardHeuristicTable,
    pub phase: PhaseKind,
    pub round: u32,
    pub max_rounds: u32,
    /// Rounds still to play, this one included. Never zero.
    pub rounds_left: u32,
    /// Colour units the player's own buyers still want, by colour.
    pub color_demand: [u32; NUM_COLORS],
    /// Material units the player's own buyers still want, by material.
    pub material_demand: [u32; 3],
    /// Boat units the player's own buyers want outright.
    pub boat_demand: u32,
    /// Points one still-wanted unit of each colour is worth: the buyer it
    /// would serve, its points spread evenly over the units it takes.
    pub color_value: [f64; NUM_COLORS],
    pub material_value: [f64; 3],
    pub boat_value: f64,
    /// What one wild primary is worth, taken as the mean primary value: a wild
    /// can be any primary its holder is short of.
    pub wild_value: f64,
    /// Points still sitting in the player's uncompleted buyers.
    pub pending_points: u32,
    /// Points among those the player could complete right now.
    pub affordable_points: u32,
    /// The points of the single best buyer that can be sold to right now. This
    /// is what a Sell ability is worth: an ability to bank points that are
    /// already on the table, not a fixed amount.
    pub best_affordable_points: u32,
    /// The highest-point buyer that is not affordable yet, if there is one.
    ///
    /// Worked out here rather than in the rules that need it: it depends on the
    /// position and not on the choice, and asking for it per choice meant
    /// walking the buyers and testing affordability dozens of times per
    /// decision.
    pub best_unaffordable: Option<SellCard>,
    /// How many of the player's buyers want each colour, and each material.
    /// Demand says how much is wanted in total; this says how many plans are
    /// competing for a unit, which is what makes one unit serve two buyers.
    pub color_buyers: [u8; NUM_COLORS],
    pub material_buyers: [u8; 3],
    /// Buyers the player could sell to right now.
    pub buyers_affordable: u32,
    /// Buyer slots this round, and how many are empty.
    pub slots: u32,
    pub empty_slots: u32,
    /// Units the highest-point buyer is still short of.
    pub best_buyer_points: u32,
    pub best_buyer_short_units: u32,
    /// Legal ducat purchases available, which is what gives a ducat its option
    /// value beyond the point it scores.
    pub purchasable: u32,
    /// Mixes the wheel could supply right now.
    pub mixable_pairs: u32,
    /// The best colour value a mix could produce from this wheel.
    pub best_mix_value: f64,
    /// The best storage value among the cards in the workshop, which is what a
    /// workshop pick is worth.
    pub workshop_storage: f64,
    /// Total colour and material units held, wanted or not.
    pub total_colors: u32,
    pub total_materials: u32,
    /// Distinct colours the player is holding.
    pub distinct_colors: u32,
    /// Cards in the draft pool, i.e. the ones this turn could still trash.
    pub drafted: u32,
    /// Every card the player owns, wherever it sits.
    pub owned_cards: u32,
}

/// How many units `sell_card` is still short of, boats and wild primaries
/// counted as the cover they are.
pub fn buyer_short_units(player: &PlayerState, sell_card: SellCard) -> u32 {
    let wilds = wild_primaries_to_cover_sale(&player.color_wheel, sell_card.stats()).unwrap_or(0);
    boats_to_sell(player, sell_card) + wilds
}

impl<'a> Ctx<'a> {
    pub fn new(state: &'a GameState, player_index: usize, card_table: &'a CardHeuristicTable) -> Self {
        let player = &state.players[player_index];
        let phase = match &state.phase {
            GamePhase::Buyers { .. } => PhaseKind::Buyers,
            GamePhase::Draft { .. } => PhaseKind::Draft,
            GamePhase::Action { .. } => PhaseKind::Action,
            GamePhase::Draw | GamePhase::GameOver => PhaseKind::Other,
        };

        let mut color_demand = [0u32; NUM_COLORS];
        let mut material_demand = [0u32; 3];
        let mut color_value = [0.0f64; NUM_COLORS];
        let mut material_value = [0.0f64; 3];
        let mut boat_value = 0.0;
        let mut boat_demand = 0u32;
        let mut pending_points = 0u32;
        let mut affordable_points = 0u32;
        let mut best_affordable_points = 0u32;
        let mut best_unaffordable: Option<SellCard> = None;
        let mut best_unaffordable_points = 0u32;
        let mut color_buyers = [0u8; NUM_COLORS];
        let mut material_buyers = [0u8; 3];
        let mut buyers_affordable = 0u32;
        let mut best_buyer_points = 0u32;
        let mut best_buyer_short_units = 0u32;

        // A buyer's points are spread evenly over the units it takes. That
        // understates the last unit, which is what actually completes the
        // sale, so completion has features of its own.
        //
        // Demand is counted as *unmet* demand. Counting what a buyer asks for
        // rather than what it still lacks prices a stored unit as though it
        // completed the sale, which makes hoarding look better than selling —
        // and it hides the fact that the fifth ceramic a two-ceramic buyer wants
        // is worth nothing at all.
        for buyer in player.buyers.iter() {
            let stats = buyer.sell_card.stats();
            let materials: u32 = stats.materials.iter().map(|&m| m as u32).sum();
            let units = stats.total_colors as u32 + materials + stats.boats;
            let per_unit = stats.points as f64 / (units.max(1) as f64);

            pending_points += stats.points;
            let short_units = buyer_short_units(player, buyer.sell_card);
            if stats.points > best_buyer_points {
                best_buyer_points = stats.points;
                best_buyer_short_units = short_units;
            }
            for &(color, _) in stats.colors() {
                color_buyers[color.index()] = color_buyers[color.index()].saturating_add(1);
            }
            for (i, &needed) in stats.materials.iter().enumerate() {
                if needed > 0 {
                    material_buyers[i] = material_buyers[i].saturating_add(1);
                }
            }
            if can_afford_sell_card(player, &buyer.sell_card) {
                buyers_affordable += 1;
                affordable_points += stats.points;
                best_affordable_points = best_affordable_points.max(stats.points);
            } else if stats.points > best_unaffordable_points {
                best_unaffordable_points = stats.points;
                best_unaffordable = Some(buyer.sell_card);
            }

            for &(color, needed) in stats.colors() {
                let short = (needed as u32).saturating_sub(player.color_wheel.get(color));
                color_demand[color.index()] += short;
                color_value[color.index()] += per_unit * short as f64;
            }
            for (i, &needed) in stats.materials.iter().enumerate() {
                let short = (needed as u32)
                    .saturating_sub(player.materials.get(ALL_MATERIAL_TYPES[i]));
                material_demand[i] += short;
                material_value[i] += per_unit * short as f64;
            }
            boat_demand += stats.boats;
            boat_value += per_unit * stats.boats as f64;
        }

        let wild_value = PRIMARIES
            .iter()
            .map(|&c| color_value[c.index()])
            .sum::<f64>()
            / PRIMARIES.len() as f64;

        let distinct_colors = player
            .color_wheel
            .counts
            .iter()
            .filter(|&&count| count > 0)
            .count() as u32;

        let deck_cards = player.deck.cards().len();
        let drafted = player.drafted_cards.len();

        // What the wheel could mix right now, and what the best of it is worth.
        let mut mixable_pairs = 0u32;
        let mut best_mix_value = 0.0f64;
        for &(a, b) in VALID_MIX_PAIRS.iter() {
            if can_supply_mix(&player.color_wheel, player.wild_primaries, a, b) {
                mixable_pairs += 1;
                best_mix_value = best_mix_value.max(color_value[mix_result(a, b).index()]);
            }
        }

        let workshop_storage = player
            .workshop_cards
            .iter()
            .map(|id| {
                let card = state.card_lookup[id as usize];
                card.material_types()
                    .iter()
                    .map(|&m| material_value[m as usize])
                    .sum::<f64>()
                    + card.colors().iter().map(|&c| color_value[c.index()]).sum::<f64>()
            })
            .fold(0.0f64, f64::max);

        let slots = (state.round.max(1) as usize).min(MAX_BUYERS) as u32;
        let purchasable = ALL_DUCAT_PURCHASES
            .iter()
            .filter(|&&purchase| can_buy_with_ducat(player, purchase))
            .count() as u32;

        Ctx {
            state,
            player_index,
            player,
            card_table,
            phase,
            round: state.round,
            max_rounds: state.max_rounds,
            rounds_left: state.max_rounds.saturating_sub(state.round) + 1,
            color_demand,
            material_demand,
            boat_demand,
            color_value,
            material_value,
            boat_value,
            wild_value,
            pending_points,
            affordable_points,
            best_affordable_points,
            best_unaffordable,
            color_buyers,
            material_buyers,
            buyers_affordable,
            slots,
            empty_slots: slots.saturating_sub(player.buyers.len() as u32),
            best_buyer_points,
            best_buyer_short_units,
            purchasable,
            mixable_pairs,
            best_mix_value,
            workshop_storage,
            total_colors: player.color_wheel.counts.iter().map(|&c| c as u32).sum(),
            total_materials: player.materials.counts.iter().sum(),
            distinct_colors,
            drafted,
            owned_cards: deck_cards
                + player.workshop_cards.len()
                + player.workshopped_cards.len()
                + drafted,
        }
    }

    /// Every colour unit the player's buyers still want.
    pub fn demanded_color_units(&self) -> u32 {
        self.color_demand.iter().sum()
    }

    /// Every material unit the player's buyers still want.
    pub fn demanded_material_units(&self) -> u32 {
        self.material_demand.iter().sum()
    }

    /// The card `id` names.
    #[inline]
    pub fn card(&self, id: u8) -> Card {
        self.state.card_lookup[id as usize]
    }

    /// How valuable one unit of `color` is to the player's sitting buyers.
    #[inline]
    pub fn value_of(&self, color: Color) -> f64 {
        self.color_value[color.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::test_support;
    use crate::scoring::HeuristicParams;

    #[test]
    fn an_empty_board_demands_nothing() {
        let state = test_support::game_with_one_empty_player();
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let ctx = Ctx::new(&state, 0, &table);
        assert_eq!(ctx.demanded_color_units(), 0);
        assert_eq!(ctx.demanded_material_units(), 0);
        assert_eq!(ctx.pending_points, 0);
        assert_eq!(ctx.distinct_colors, 0);
        assert_eq!(ctx.rounds_left, state.max_rounds);
    }

    #[test]
    fn a_buyer_prices_the_colours_it_wants() {
        let mut state = test_support::game_with_one_empty_player();
        let sell_card = ALL_SELL_CARDS[0];
        state.players[0].buyers.push(SellCardInstance {
            instance_id: 0,
            sell_card,
        });
        let table = CardHeuristicTable::new(&HeuristicParams::default());
        let ctx = Ctx::new(&state, 0, &table);
        assert_eq!(ctx.pending_points, sell_card.points());
        assert!(ctx.demanded_color_units() > 0);
        let wanted: f64 = ctx.color_value.iter().sum();
        assert!(wanted > 0.0, "a buyer's colours are worth points");
        let costs: u32 = sell_card.stats().colors().iter().map(|&(_, n)| n as u32).sum();
        assert_eq!(ctx.demanded_color_units(), costs);
    }
}
