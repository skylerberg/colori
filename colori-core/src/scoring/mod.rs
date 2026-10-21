mod heuristic_params;

pub use heuristic_params::HeuristicParams;

use crate::action_phase::boats_to_sell;
use crate::colors::{is_primary, PRIMARIES, SECONDARIES, TERTIARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;

const ALL_CARDS: [Card; NUM_CARDS] = [
    Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
    Card::Realgar, Card::Kermes, Card::Litharge,
    Card::Malachite, Card::Woad, Card::Logwood,
    Card::DragonsBlood, Card::Orpiment, Card::GreenEarth,
    Card::Verdigris, Card::Brazilwood, Card::Indigo,
    Card::Cinnabar, Card::Saffron, Card::CopperResinate,
    Card::Azurite, Card::LapisLazuli, Card::Cochineal,
    Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric,
    Card::CeramicsRed, Card::CeramicsYellow, Card::CeramicsBlue,
    Card::CanvasRed, Card::CanvasYellow, Card::CanvasBlue,
    Card::FabricOrange, Card::FabricGreen, Card::FabricPurple,
    Card::CanvasCeramics, Card::CeramicsFabric, Card::CanvasFabric,
    Card::Alum, Card::GumArabic, Card::LinseedOil,
    Card::Warehouse, Card::Gondola, Card::Chalk,
];

pub struct CardHeuristicTable {
    quality: [f64; NUM_CARDS],
}

impl CardHeuristicTable {
    pub fn new(params: &HeuristicParams) -> Self {
        let mut quality = [0.0f64; NUM_CARDS];
        for &card in &ALL_CARDS {
            let idx = card as usize;
            quality[idx] = card_quality(card, params);
        }
        CardHeuristicTable { quality }
    }
}

/// Points from completed sell cards plus ducats: the whole-point part of a score,
/// and what `PlayerState::cached_whole_points` caches.
pub fn calculate_whole_points(player: &PlayerState) -> u32 {
    let sell_card_points: u32 = player.completed_sell_cards.iter().map(|bi| bi.sell_card.points()).sum();
    sell_card_points + player.ducats
}

/// A player's score, counted in half points so it stays whole: every ducat is
/// two, and each unspent wild primary or boat is one.
pub fn score_in_half_points(player: &PlayerState) -> u32 {
    2 * calculate_whole_points(player) + player.wild_primaries + player.boats
}

/// `score_in_half_points` off the cache, for the search's hot paths.
#[inline]
fn cached_half_points(player: &PlayerState) -> u32 {
    2 * player.cached_whole_points + player.wild_primaries + player.boats
}

/// "12" or "12.5".
pub fn format_half_points(half_points: u32) -> String {
    if half_points % 2 == 0 {
        format!("{}", half_points / 2)
    } else {
        format!("{}.5", half_points / 2)
    }
}

/// Returns a comparable ranking tuple: (half points, completed_sell_cards_count, color_wheel_total).
/// Rust tuples compare lexicographically, giving correct tiebreak order.
pub fn player_ranking(player: &PlayerState) -> (u32, usize, u32) {
    (
        score_in_half_points(player),
        player.completed_sell_cards.len(),
        player.color_wheel.counts.iter().map(|&c| c as u32).sum(),
    )
}

/// Solo mode: 1.0 for reaching 16 points (win) + score/100 for gradient signal.
fn solo_reward(player: &PlayerState) -> f64 {
    let score = cached_half_points(player) as f64 / 2.0;
    let win_bonus = if score >= 16.0 { 1.0 } else { 0.0 };
    win_bonus + score / 100.0
}

/// Compute terminal rewards using tiebreakers. Uses cached_whole_points for consistency with ISMCTS.
/// Each true-tied winner gets 1.0 / num_winners, losers get 0.0.
pub fn compute_terminal_rewards(players: &FixedVec<PlayerState, MAX_PLAYERS>) -> [f64; MAX_PLAYERS] {
    if players.len() == 1 {
        let mut result = [0.0; MAX_PLAYERS];
        result[0] = solo_reward(&players[0]);
        return result;
    }
    let mut rankings = [(0u32, 0usize, 0u32); MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        rankings[i] = (
            cached_half_points(p),
            p.completed_sell_cards.len(),
            p.color_wheel.counts.iter().map(|&c| c as u32).sum(),
        );
    }
    let best = rankings[..players.len()].iter().copied().max().unwrap_or((0, 0, 0));
    let num_winners = rankings[..players.len()].iter().filter(|&&r| r == best).count() as f64;
    let mut result = [0.0; MAX_PLAYERS];
    for i in 0..players.len() {
        result[i] = if rankings[i] == best { 1.0 / num_winners } else { 0.0 };
    }
    result
}

fn card_quality(card: Card, params: &HeuristicParams) -> f64 {
    match card.kind() {
        CardKind::Action => match card {
            Card::Chalk => params.chalk_quality,
            Card::Alum => params.alum_quality,
            Card::GumArabic => params.gum_arabic_quality,
            Card::LinseedOil => params.linseed_oil_quality,
            Card::Warehouse => params.warehouse_quality,
            Card::Gondola => params.gondola_quality,
            _ => 0.0,
        },
        // A dye's tier shows in how many colours it carries.
        CardKind::Dye => match card.colors().len() {
            3 => params.primary_dye_quality,
            2 => params.secondary_dye_quality,
            _ => params.tertiary_dye_quality,
        },
        CardKind::BasicDye => params.basic_dye_quality,
        CardKind::Material => {
            let colors = card.colors();
            let mat_types = card.material_types();
            if colors.is_empty() && mat_types.len() == 1 {
                params.starter_material_quality
            } else if !colors.is_empty() {
                match card.material_types()[0] {
                    MaterialType::Ceramics => params.ceramics_material_quality,
                    MaterialType::Paintings => params.paintings_material_quality,
                    MaterialType::Textiles => params.textiles_material_quality,
                }
            } else {
                params.dual_material_quality
            }
        }
    }
}

/// How well a sell card suits what a player already has stored.
///
/// Shared by the evaluation function and by the rollout's buyers-phase policy,
/// so the cards a rollout claims are the ones the evaluation would reward.
pub fn buyer_alignment(player: &PlayerState, sell_card: SellCard, params: &HeuristicParams) -> f64 {
    let stats = sell_card.stats();
    let points = stats.points as f64;
    let mut alignment = 0.0;

    if boats_to_sell(player, sell_card) <= player.boats {
        alignment += params.sell_card_material_alignment * points;
    }

    let cost_len = stats.total_colors as f64;
    for &(color, needed) in stats.colors() {
        if player.color_wheel.get(color) > 0 || (is_primary(color) && player.wild_primaries > 0) {
            // One addition per unit, as a sum over the cost's entries would
            // make: a product rounds differently.
            for _ in 0..needed {
                alignment += (params.sell_card_color_alignment / cost_len) * points;
            }
        }
    }

    alignment
}

pub fn heuristic_score(
    player: &PlayerState,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
    card_table: &CardHeuristicTable,
) -> f64 {
    // A ducat still in hand scores its point plus a premium for the Workshop,
    // Mix or Sell it can still buy (`unspent_ducat_multiplier`). Ducats already
    // banked in completed sell cards get no premium: that choice is spent.
    let score = cached_half_points(player) as f64 / 2.0
        + (params.unspent_ducat_multiplier - 1.0) * player.ducats as f64;

    // Beyond their half point, wild primaries and boats are worth what they can stand
    // in for: any primary, and whichever material is weighted highest. The
    // multipliers price the flexibility of a wild over the one resource it is
    // being compared against.
    let mut color_score =
        params.primary_color_value * params.wild_primary_multiplier * player.wild_primaries as f64;
    for &c in &PRIMARIES {
        color_score += params.primary_color_value * player.color_wheel.get(c) as f64;
    }
    for &c in &SECONDARIES {
        color_score += params.secondary_color_value * player.color_wheel.get(c) as f64;
    }
    for &c in &TERTIARIES {
        color_score += params.tertiary_color_value * player.color_wheel.get(c) as f64;
    }

    let material_score = params.stored_ceramics_weight * player.materials.get(MaterialType::Ceramics) as f64
        + params.stored_paintings_weight * player.materials.get(MaterialType::Paintings) as f64
        + params.stored_textiles_weight * player.materials.get(MaterialType::Textiles) as f64
        + params
            .stored_ceramics_weight
            .max(params.stored_paintings_weight)
            .max(params.stored_textiles_weight)
            * params.boat_multiplier
            * player.boats as f64;

    let mut total_quality = 0.0;
    let mut card_count = 0u32;
    for cards in [player.deck.cards(), player.workshop_cards, player.workshopped_cards, player.drafted_cards] {
        for id in cards.iter() {
            let idx = card_lookup[id as usize] as usize;
            total_quality += card_table.quality[idx];
            card_count += 1;
        }
    }
    let deck_quality = if card_count > 0 {
        total_quality / card_count as f64
    } else {
        0.0
    };

    let deck_thinning_bonus = params.deck_thinning_value / (card_count as f64 + 1.0);

    // Only the player's own buyers count: the shared display cannot be sold to
    // until a buyers phase moves a card across, and the value of doing that
    // shows up in the resulting position rather than needing its own term.
    let mut best_alignment = 0.0f64;
    for bi in player.buyers.iter() {
        best_alignment = best_alignment.max(buyer_alignment(player, bi.sell_card, params));
    }

    score + color_score + material_score + deck_quality + deck_thinning_bonus + best_alignment
}

/// Compute heuristic rewards for truncated early-game rollouts.
/// Highest heuristic score gets 1.0, others get 0.0. Ties split evenly.
/// Solo mode: same formula as terminal (win bonus + score/100).
pub fn compute_heuristic_rewards(
    players: &FixedVec<PlayerState, MAX_PLAYERS>,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
    card_table: &CardHeuristicTable,
) -> [f64; MAX_PLAYERS] {
    let mut scores = [0.0f64; MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        scores[i] = heuristic_score(p, card_lookup, params, card_table);
    }

    if players.len() == 1 {
        let mut result = [0.0; MAX_PLAYERS];
        result[0] = solo_reward(&players[0]);
        return result;
    }

    let best = scores[..players.len()].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let num_winners = scores[..players.len()].iter().filter(|&&s| s == best).count() as f64;
    let mut result = [0.0; MAX_PLAYERS];
    for i in 0..players.len() {
        result[i] = if scores[i] == best { 1.0 / num_winners } else { 0.0 };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SellCard, SellCardInstance, ColorWheel, Materials, PlayerState};
    use crate::deck::Deck;
    use crate::unordered_cards::UnorderedCards;
    use smallvec::SmallVec;

    fn make_player(ducats: u32, sell_cards: &[SellCard], color_counts: [u32; 12]) -> PlayerState {
        let completed_sell_cards: SmallVec<[SellCardInstance; 12]> = sell_cards
            .iter()
            .enumerate()
            .map(|(i, &sell_card)| SellCardInstance {
                instance_id: i as u32,
                sell_card,
            })
            .collect();
        let mut p = PlayerState {
            deck: Deck::new(),
            buyers: FixedVec::new(),
            workshopped_cards: UnorderedCards::new(),
            workshop_cards: UnorderedCards::new(),
            drafted_cards: UnorderedCards::new(),
            kept_cards: UnorderedCards::new(),
            color_wheel: ColorWheel { counts: color_counts.map(|c| c as u8) },
            materials: Materials::new(),
            completed_sell_cards,
            ducats,
            wild_primaries: 0,
            boats: 0,
            cached_whole_points: 0,
        };
        p.cached_whole_points = calculate_whole_points(&p);
        p
    }

    #[test]
    fn test_clear_winner() {
        let p1 = make_player(5, &[], [0; 12]);
        let p2 = make_player(3, &[], [0; 12]);
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        let rewards = compute_terminal_rewards(&players);
        assert_eq!(rewards[0], 1.0);
        assert_eq!(rewards[1], 0.0);
    }

    #[test]
    fn test_tie_broken_by_sell_cards() {
        // p1: 2 3-point sell cards = 6 points, 2 sell cards
        // p2: 1 3-point sell card + 3 ducats = 6 points, 1 sell card
        let p1 = make_player(0, &[SellCard::PotterVermilion, SellCard::PotterAmber], [0; 12]);
        let p2 = make_player(3, &[SellCard::PotterVermilion], [0; 12]);
        assert_eq!(p1.cached_whole_points, 6);
        assert_eq!(p2.cached_whole_points, 6);
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        let rewards = compute_terminal_rewards(&players);
        assert_eq!(rewards[0], 1.0); // p1 wins on sell card count
        assert_eq!(rewards[1], 0.0);
    }

    #[test]
    fn test_tie_broken_by_colors() {
        // Same score, same sell card count, different color wheel totals
        let p1 = make_player(3, &[], [1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // total=3
        let p2 = make_player(3, &[], [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // total=1
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        let rewards = compute_terminal_rewards(&players);
        assert_eq!(rewards[0], 1.0); // p1 wins on color total
        assert_eq!(rewards[1], 0.0);
    }

    #[test]
    fn test_true_tie() {
        // Same score, same sell card count, same color wheel total
        let p1 = make_player(3, &[], [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let p2 = make_player(3, &[], [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        let rewards = compute_terminal_rewards(&players);
        assert_eq!(rewards[0], 0.5);
        assert_eq!(rewards[1], 0.5);
    }

    #[test]
    fn test_wild_primaries_and_boats_are_worth_half_a_point_each() {
        let mut p1 = make_player(3, &[], [0; 12]);
        p1.wild_primaries = 1;
        let mut p2 = make_player(3, &[], [0; 12]);
        assert_eq!(score_in_half_points(&p1), 7);
        assert_eq!(format_half_points(score_in_half_points(&p1)), "3.5");
        assert_eq!(format_half_points(score_in_half_points(&p2)), "3");

        let mut players = FixedVec::new();
        players.push(p1.clone());
        players.push(p2.clone());
        assert_eq!(compute_terminal_rewards(&players), [1.0, 0.0, 0.0, 0.0]);

        // A boat is worth exactly what a wild primary is.
        p2.boats = 1;
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        assert_eq!(compute_terminal_rewards(&players), [0.5, 0.5, 0.0, 0.0]);
    }

    /// The three resource multipliers default to 1.0 on a params file written
    /// before they existed, and scaling one moves only the term it names.
    #[test]
    fn test_wild_boat_and_ducat_multipliers_scale_their_own_term() {
        const BASELINE: &str =
            include_str!("../../../genetic-algorithm/batch-lki08w-gen-32.json");
        let mut params: HeuristicParams =
            serde_json::from_str(BASELINE).expect("baseline params parse");
        assert_eq!(params.wild_primary_multiplier, 1.0);
        assert_eq!(params.boat_multiplier, 1.0);
        assert_eq!(params.unspent_ducat_multiplier, 1.0);

        let lookup = [Card::BasicRed; 256];
        let mut player = make_player(3, &[], [0; 12]);
        player.wild_primaries = 1;
        player.boats = 1;
        let base = heuristic_score(&player, &lookup, &params, &CardHeuristicTable::new(&params));

        params.wild_primary_multiplier = 2.0;
        params.boat_multiplier = 2.0;
        params.unspent_ducat_multiplier = 2.0;
        let boosted = heuristic_score(&player, &lookup, &params, &CardHeuristicTable::new(&params));

        let best_material = params
            .stored_ceramics_weight
            .max(params.stored_paintings_weight)
            .max(params.stored_textiles_weight);
        let expected = base + params.primary_color_value + best_material + player.ducats as f64;
        assert!((boosted - expected).abs() < 1e-9, "{boosted} != {expected}");
    }
}
