//! What every feature needs, worked out once per decision.
//!
//! A decision scores every legal choice. Each choice needs the same picture of
//! the player's position — which colours the sitting buyers still want, what a
//! unit of each is worth in points — so it is built once here and shared.

use crate::action_phase::can_afford_sell_card;
use crate::colors::PRIMARIES;
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
    /// Distinct colours the player is holding.
    pub distinct_colors: u32,
    /// Cards in the draft pool, i.e. the ones this turn could still trash.
    pub drafted: u32,
    /// Every card the player owns, wherever it sits.
    pub owned_cards: u32,
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

        // A buyer's points are spread evenly over the units it takes. That
        // understates the last unit, which is what actually completes the
        // sale, so completion has features of its own.
        for buyer in player.buyers.iter() {
            let stats = buyer.sell_card.stats();
            let point_share = stats.points as f64;
            let materials: u32 = stats.materials.iter().map(|&m| m as u32).sum();
            let units = stats.total_colors as u32 + materials + stats.boats;
            let per_unit = point_share / (units.max(1) as f64);

            pending_points += stats.points;
            if can_afford_sell_card(player, &buyer.sell_card) {
                affordable_points += stats.points;
            }

            for &(color, needed) in stats.colors() {
                color_demand[color.index()] += needed as u32;
                color_value[color.index()] += per_unit * needed as f64;
            }
            for (i, &needed) in stats.materials.iter().enumerate() {
                material_demand[i] += needed as u32;
                material_value[i] += per_unit * needed as f64;
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
