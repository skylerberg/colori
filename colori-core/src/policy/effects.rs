//! What a choice moves, read off the choice's payload.
//!
//! The policy scores a choice without applying it. Rather than cloning the
//! game state and diffing — which would pay a state clone per candidate choice
//! — this reads the effect straight out of the choice, which is always
//! self-describing: a workshop choice carries the cards it stores, a sale
//! carries the sell card it takes, a mix carries the pairs it mixes.
//!
//! The one effect that needs simulating is a mix sequence, because a mix both
//! spends its inputs and produces an output, and wild primaries can stand in
//! for a missing primary. [`Delta::mix`] replays the sequence against a copy of
//! the colour wheel and diffs it, which is exact and costs a twelve-byte copy.

use crate::action_phase::boats_to_sell;
use crate::colors::perform_mix_unchecked;
use crate::scoring::CardHeuristicTable;
use crate::types::*;

/// One slot per [`Ability`] variant, in declaration order.
pub const NUM_ABILITY_KINDS: usize = 8;

pub const ABILITY_WORKSHOP: usize = 0;
pub const ABILITY_MIX: usize = 1;
pub const ABILITY_MOVE_TO_DRAFT_POOL: usize = 2;
pub const ABILITY_SELL: usize = 3;
pub const ABILITY_GAIN_DUCATS: usize = 4;
pub const ABILITY_GAIN_SECONDARY: usize = 5;
pub const ABILITY_GAIN_WILD_PRIMARIES: usize = 6;
pub const ABILITY_GAIN_BOATS: usize = 7;

pub const ABILITY_NAMES: [&str; NUM_ABILITY_KINDS] = [
    "workshop",
    "mix",
    "moveToDraftPool",
    "sell",
    "gainDucats",
    "gainSecondary",
    "gainWildPrimaries",
    "gainBoats",
];

/// Which slot an ability occupies.
pub fn ability_kind(ability: Ability) -> usize {
    match ability {
        Ability::Workshop { .. } => ABILITY_WORKSHOP,
        Ability::MixColors { .. } => ABILITY_MIX,
        Ability::MoveToDraftPool => ABILITY_MOVE_TO_DRAFT_POOL,
        Ability::Sell => ABILITY_SELL,
        Ability::GainDucats { .. } => ABILITY_GAIN_DUCATS,
        Ability::GainSecondary => ABILITY_GAIN_SECONDARY,
        Ability::GainWildPrimaries { .. } => ABILITY_GAIN_WILD_PRIMARIES,
        Ability::GainBoats { .. } => ABILITY_GAIN_BOATS,
    }
}

/// How much of an ability one grant is worth: its count when it carries one,
/// and a single unit otherwise.
pub fn ability_amount(ability: Ability) -> u32 {
    match ability {
        Ability::Workshop { count }
        | Ability::MixColors { count }
        | Ability::GainDucats { count }
        | Ability::GainWildPrimaries { count }
        | Ability::GainBoats { count } => count,
        Ability::MoveToDraftPool | Ability::Sell | Ability::GainSecondary => 1,
    }
}

/// Everything a choice changes about the acting player that the features can
/// want to weigh, in the player's own units rather than in points.
///
/// Points are the one exception: a sale banks them outright, so it carries
/// them as they are.
#[derive(Debug, Clone, Copy)]
pub struct Delta {
    /// Points the choice banks immediately.
    pub points: f64,
    /// Ducats gained (positive) or spent (negative).
    pub ducats: i32,
    pub gained_colors: [u32; NUM_COLORS],
    pub spent_colors: [u32; NUM_COLORS],
    pub gained_materials: [u32; 3],
    pub spent_materials: [u32; 3],
    pub boats_gained: u32,
    pub boats_spent: u32,
    pub wilds_gained: u32,
    pub wilds_spent: u32,
    /// Cards the choice puts in the trash.
    pub cards_trashed: u32,
    /// Summed heuristic quality of the cards it puts in the trash.
    pub trash_quality: f64,
    /// Summed heuristic quality of the cards it takes or keeps.
    pub gained_card_quality: f64,
    /// Ability grants, by [`ability_kind`], summed by [`ability_amount`].
    pub abilities: [u32; NUM_ABILITY_KINDS],
}

impl Delta {
    pub fn new() -> Self {
        Delta {
            points: 0.0,
            ducats: 0,
            gained_colors: [0; NUM_COLORS],
            spent_colors: [0; NUM_COLORS],
            gained_materials: [0; 3],
            spent_materials: [0; 3],
            boats_gained: 0,
            boats_spent: 0,
            wilds_gained: 0,
            wilds_spent: 0,
            cards_trashed: 0,
            trash_quality: 0.0,
            gained_card_quality: 0.0,
            abilities: [0; NUM_ABILITY_KINDS],
        }
    }

    /// Total colour units the choice adds, over every colour.
    pub fn gained_color_units(&self) -> u32 {
        self.gained_colors.iter().sum()
    }

    /// Total colour units the choice consumes, over every colour.
    pub fn spent_color_units(&self) -> u32 {
        self.spent_colors.iter().sum()
    }

    pub fn gained_material_units(&self) -> u32 {
        self.gained_materials.iter().sum()
    }

    pub fn spent_material_units(&self) -> u32 {
        self.spent_materials.iter().sum()
    }
}

impl Default for Delta {
    fn default() -> Self {
        Self::new()
    }
}

/// The effect of `choice` on `player`.
///
/// `card_table` is only read for card quality, which is a heuristic parameter
/// rather than a rule of the game.
pub fn delta(player: &PlayerState, choice: &Choice, card_table: &CardHeuristicTable) -> Delta {
    let mut d = Delta::new();
    match choice {
        Choice::DraftPick { card } => {
            d.gained_card_quality += card_table.quality(*card);
        }
        Choice::TrashDraftedCard { card } => trash(*card, card_table, &mut d),
        Choice::EndTurn | Choice::SkipWorkshop | Choice::DrawBuyer => {}
        Choice::Workshop { card_types } => workshop(card_types, card_table, &mut d),
        Choice::MoveToDraftPool { card } => {
            // The card leaves the workshop area for the draft pool. It was not
            // stored before and is not stored by this, so the wheel does not
            // move; what changes is that the card is now the player's to trash
            // this turn and to keep next round.
            if let Some(card) = card {
                d.gained_card_quality += card_table.quality(*card);
            }
        }
        Choice::SelectSellCard { sell_card } => sale(player, *sell_card, &mut d),
        Choice::GainSecondary { color } => {
            d.gained_colors[color.index()] += 1;
        }
        Choice::MixAll { mixes } => mix(player, mixes, &mut d),
        Choice::TrashAndMix { card, mixes } => {
            trash(*card, card_table, &mut d);
            mix(player, mixes, &mut d);
        }
        Choice::TrashAndSell { card, sell_card } => {
            trash(*card, card_table, &mut d);
            sale(player, *sell_card, &mut d);
        }
        Choice::TrashAndWorkshop { card, workshop_cards } => {
            trash(*card, card_table, &mut d);
            workshop(workshop_cards, card_table, &mut d);
        }
        Choice::TrashAndMoveToDraftPool { card, target } => {
            trash(*card, card_table, &mut d);
            if let Some(target) = target {
                d.gained_card_quality += card_table.quality(*target);
            }
        }
        Choice::SpendDucat { purchase } => {
            d.ducats -= 1;
            let ability = purchase.ability();
            d.abilities[ability_kind(ability)] += ability_amount(ability);
        }
        // A buyer claimed in the buyers phase is not stored, spent or banked:
        // the trade is what a later sale is worth. The buyers-phase features
        // read the offer off the choice itself.
        Choice::TakeBuyer { .. } => {}
    }
    d
}

fn trash(card: Card, card_table: &CardHeuristicTable, d: &mut Delta) {
    d.cards_trashed += 1;
    d.trash_quality += card_table.quality(card);
    for &ability in card.trash_abilities() {
        d.abilities[ability_kind(ability)] += ability_amount(ability);
    }
}

fn workshop(cards: &[Card], card_table: &CardHeuristicTable, d: &mut Delta) {
    for &card in cards {
        d.gained_card_quality += card_table.quality(card);
        for &material in card.material_types() {
            d.gained_materials[material as usize] += 1;
        }
        for &color in card.colors() {
            d.gained_colors[color.index()] += 1;
        }
        for &ability in card.workshop_abilities() {
            d.abilities[ability_kind(ability)] += ability_amount(ability);
        }
    }
}

/// A sale pays the colours and materials it names and takes its boats — one
/// per material or boat it asks for that the player has not stored.
///
/// Only the stored part of a material counts as spent; the rest is already
/// accounted for as a boat, so the two do not double-count.
fn sale(player: &PlayerState, sell_card: SellCard, d: &mut Delta) {
    let stats = sell_card.stats();
    d.points += stats.points as f64;
    for &(color, needed) in stats.colors() {
        d.spent_colors[color.index()] += needed as u32;
    }
    for (i, &material) in ALL_MATERIAL_TYPES.iter().enumerate() {
        let needed = stats.materials[i] as u32;
        let stored = player.materials.get(material);
        d.spent_materials[i] += needed.min(stored);
    }
    d.boats_spent += boats_to_sell(player, sell_card);
}

/// Replay the sequence against a copy of the wheel and diff it.
///
/// Exact for colours and wild primaries, which is all a mix touches.
fn mix(player: &PlayerState, mixes: &MixSequence, d: &mut Delta) {
    let before = player.color_wheel.counts;
    let mut wheel = player.color_wheel.clone();
    let mut wilds = player.wild_primaries;
    for &(a, b) in mixes.iter() {
        perform_mix_unchecked(&mut wheel, &mut wilds, a, b);
    }
    for i in 0..NUM_COLORS {
        let after = wheel.counts[i];
        if after > before[i] {
            d.gained_colors[i] += (after - before[i]) as u32;
        } else if after < before[i] {
            d.spent_colors[i] += (before[i] - after) as u32;
        }
    }
    d.wilds_spent += player.wild_primaries - wilds;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::HeuristicParams;

    fn table() -> CardHeuristicTable {
        CardHeuristicTable::new(&HeuristicParams::default())
    }

    #[test]
    fn every_ability_has_its_own_kind() {
        let abilities = [
            Ability::Workshop { count: 1 },
            Ability::MixColors { count: 1 },
            Ability::MoveToDraftPool,
            Ability::Sell,
            Ability::GainDucats { count: 1 },
            Ability::GainSecondary,
            Ability::GainWildPrimaries { count: 1 },
            Ability::GainBoats { count: 1 },
        ];
        let mut seen = [false; NUM_ABILITY_KINDS];
        for ability in abilities {
            let kind = ability_kind(ability);
            assert!(!seen[kind], "{ability:?} shares a kind");
            seen[kind] = true;
            assert!(ability_amount(ability) >= 1);
        }
        assert!(seen.iter().all(|&s| s), "an ability kind is unused");
        assert_eq!(ABILITY_NAMES.len(), NUM_ABILITY_KINDS);
    }

    #[test]
    fn a_sale_spends_its_stored_materials_and_boats() {
        let table = table();
        // The card wanting the most of one material, whatever that turns out
        // to be, with more than enough stored to cover it.
        let card = ALL_SELL_CARDS
            .iter()
            .copied()
            .max_by_key(|c| c.stats().materials[0])
            .expect("there are sell cards");
        let wanted = card.stats().materials[0] as u32;

        let mut player = crate::policy::test_support::empty_player();
        player.materials.counts[0] = wanted + 1;
        player.boats = 3;

        let d = delta(&player, &Choice::SelectSellCard { sell_card: card }, &table);
        assert_eq!(d.points, card.points() as f64);
        assert_eq!(d.spent_materials[0], wanted);
        // Every material is stored, so only the card's own boats are taken.
        assert_eq!(d.boats_spent, card.required_boats());
        assert_eq!(d.spent_color_units(), card.stats().total_colors as u32);
    }

    #[test]
    fn a_sale_without_the_materials_pays_in_boats() {
        let table = table();
        let card = ALL_SELL_CARDS
            .iter()
            .copied()
            .max_by_key(|c| c.stats().materials[0])
            .expect("there are sell cards");
        let wanted = card.stats().materials[0] as u32;
        let player = crate::policy::test_support::empty_player();
        let d = delta(&player, &Choice::SelectSellCard { sell_card: card }, &table);
        assert_eq!(d.spent_material_units(), 0, "nothing was stored to spend");
        assert_eq!(d.boats_spent, card.required_boats() + wanted);
    }

    #[test]
    fn a_mix_reports_what_it_takes_and_makes() {
        let mut player = crate::policy::test_support::empty_player();
        player.color_wheel.set(Color::Red, 1);
        player.color_wheel.set(Color::Yellow, 1);
        let mut mixes = MixSequence::new();
        mixes.push((Color::Red, Color::Yellow));
        let d = delta(&player, &Choice::MixAll { mixes }, &table());
        assert_eq!(d.spent_colors[Color::Red.index()], 1);
        assert_eq!(d.spent_colors[Color::Yellow.index()], 1);
        assert_eq!(d.gained_colors[Color::Orange.index()], 1);
        assert_eq!(d.wilds_spent, 0);
    }

    #[test]
    fn a_workshop_stores_the_card_and_grants_its_abilities() {
        let player = crate::policy::test_support::empty_player();
        let table = table();
        let card = Card::StarterCeramics;
        let mut cards = WorkshopCards::new();
        cards.push(card);
        let d = delta(&player, &Choice::Workshop { card_types: cards }, &table);
        assert!(d.gained_material_units() >= 1);
        assert!(d.gained_card_quality > 0.0);
        let expected: u32 = card.workshop_abilities().iter().map(|&a| ability_amount(a)).sum();
        assert_eq!(d.abilities.iter().sum::<u32>(), expected);
    }
}
