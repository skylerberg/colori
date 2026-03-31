pub mod diff_eval;
pub mod diff_eval_grad;
pub mod first_pick;
mod heuristic_params;
pub(crate) mod simd_ops;

pub use diff_eval::{DiffEvalParams, DiffEvalTable, diff_eval_score, compute_diff_eval_rewards};
pub use first_pick::FirstPickParams;
pub use heuristic_params::HeuristicParams;

use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;

const ALL_CARDS: [Card; 48] = [
    Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
    Card::Kermes, Card::Weld, Card::Woad,
    Card::Lac, Card::Brazilwood, Card::Pomegranate,
    Card::Sumac, Card::Elderberry, Card::Turnsole,
    Card::Madder, Card::Turmeric, Card::DyersGreenweed,
    Card::Verdigris, Card::Orchil, Card::Logwood,
    Card::VermilionDye, Card::Saffron, Card::PersianBerries,
    Card::Azurite, Card::IndigoDye, Card::Cochineal,
    Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
    Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
    Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
    Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
    Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    Card::Alum, Card::CreamOfTartar, Card::GumArabic,
    Card::Potash, Card::Vinegar, Card::Argol, Card::Chalk,
    Card::LinseedOil, Card::Lye,
];

pub struct CardHeuristicTable {
    quality: [f64; 48],
    primary_mask: [u8; 48],
    secondary_mask: [u8; 48],
    material_mask: [u8; 48],
}

impl CardHeuristicTable {
    pub fn new(params: &HeuristicParams) -> Self {
        let mut quality = [0.0f64; 48];
        let mut primary_mask = [0u8; 48];
        let mut secondary_mask = [0u8; 48];
        let mut material_mask = [0u8; 48];
        for &card in &ALL_CARDS {
            let idx = card as usize;
            quality[idx] = card_quality(card, params);
            for &color in card.colors() {
                for (i, &p) in PRIMARIES.iter().enumerate() {
                    if color == p {
                        primary_mask[idx] |= 1 << i;
                    }
                }
                for (i, &s) in SECONDARIES.iter().enumerate() {
                    if color == s {
                        secondary_mask[idx] |= 1 << i;
                    }
                }
            }
            for &mt in card.material_types() {
                material_mask[idx] |= 1 << (mt as usize);
            }
        }
        CardHeuristicTable { quality, primary_mask, secondary_mask, material_mask }
    }
}

pub fn calculate_score(player: &PlayerState) -> u32 {
    let sell_card_ducats: u32 = player.completed_sell_cards.iter().map(|bi| bi.sell_card.ducats()).sum();
    sell_card_ducats + player.ducats
}

/// Returns a comparable ranking tuple: (score, completed_sell_cards_count, color_wheel_total).
/// Rust tuples compare lexicographically, giving correct tiebreak order.
pub fn player_ranking(player: &PlayerState) -> (u32, usize, u32) {
    (
        calculate_score(player),
        player.completed_sell_cards.len(),
        player.color_wheel.counts.iter().sum(),
    )
}

/// Compute terminal rewards using tiebreakers. Uses cached_score for consistency with ISMCTS.
/// Each true-tied winner gets 1.0 / num_winners, losers get 0.0.
/// Solo mode: reward = min(score / 16, 1) to reflect progress toward the 16-ducat goal.
pub fn compute_terminal_rewards(players: &FixedVec<PlayerState, MAX_PLAYERS>) -> [f64; MAX_PLAYERS] {
    if players.len() == 1 {
        let mut result = [0.0; MAX_PLAYERS];
        result[0] = (players[0].cached_score as f64 / 16.0).min(1.0);
        return result;
    }
    let mut rankings = [(0u32, 0usize, 0u32); MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        rankings[i] = (
            p.cached_score,
            p.completed_sell_cards.len(),
            p.color_wheel.counts.iter().sum(),
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
    if matches!(card, Card::Chalk) {
        return params.chalk_quality;
    }
    match card.kind() {
        CardKind::Action => {
            match card {
                Card::Alum => params.alum_quality,
                Card::CreamOfTartar => params.cream_of_tartar_quality,
                Card::GumArabic => params.gum_arabic_quality,
                Card::Potash => params.potash_quality,
                Card::Vinegar => params.vinegar_quality,
                Card::Argol => params.argol_quality,
                Card::LinseedOil => params.linseed_oil_quality,
                Card::Lye => params.lye_quality,
                _ => None,
            }
            .unwrap_or(params.action_quality)
        }
        CardKind::Dye => {
            match card {
                Card::Kermes | Card::Weld | Card::Woad => params.pure_primary_dye_quality,
                Card::Lac | Card::Brazilwood | Card::Pomegranate
                | Card::Sumac | Card::Elderberry | Card::Turnsole => params.primary_dye_quality,
                Card::Madder | Card::Turmeric | Card::DyersGreenweed
                | Card::Verdigris | Card::Orchil | Card::Logwood => params.secondary_dye_quality,
                Card::VermilionDye | Card::Saffron | Card::PersianBerries
                | Card::Azurite | Card::IndigoDye | Card::Cochineal => params.tertiary_dye_quality,
                _ => None,
            }
            .unwrap_or(params.dye_quality)
        }
        CardKind::BasicDye => params.basic_dye_quality,
        CardKind::Material => {
            let colors = card.colors();
            let mat_types = card.material_types();
            if colors.is_empty() && mat_types.len() == 1 {
                params.starter_material_quality
            } else if !colors.is_empty() {
                params.draft_material_quality
            } else {
                params.dual_material_quality
            }
        }
    }
}

pub fn heuristic_score(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
    card_table: &CardHeuristicTable,
) -> f64 {
    let score = player.cached_score as f64;

    let mut color_score = 0.0;
    for &c in &PRIMARIES {
        color_score += params.primary_color_value * player.color_wheel.get(c) as f64;
    }
    for &c in &SECONDARIES {
        color_score += params.secondary_color_value * player.color_wheel.get(c) as f64;
    }
    for &c in &TERTIARIES {
        color_score += params.tertiary_color_value * player.color_wheel.get(c) as f64;
    }

    let material_score = params.stored_material_weight * player.materials.counts.iter().sum::<u32>() as f64;

    let mut total_quality = 0.0;
    let mut card_count = 0u32;
    let mut primary_seen = 0u8;
    let mut secondary_seen = 0u8;
    let mut material_seen = 0u8;
    for cards in [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards] {
        for id in cards.iter() {
            let idx = card_lookup[id as usize] as usize;
            total_quality += card_table.quality[idx];
            card_count += 1;
            primary_seen |= card_table.primary_mask[idx];
            secondary_seen |= card_table.secondary_mask[idx];
            material_seen |= card_table.material_mask[idx];
        }
    }
    let deck_quality = if card_count > 0 {
        total_quality / card_count as f64
    } else {
        0.0
    };

    let mut best_alignment = 0.0f64;
    for bi in sell_card_display.iter() {
        let sell_card = bi.sell_card;
        let ducats = sell_card.ducats() as f64;
        let mut alignment = 0.0;

        if player.materials.get(sell_card.required_material()) > 0 {
            alignment += params.sell_card_material_alignment * ducats;
        }

        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        for &color in cost {
            if player.color_wheel.get(color) > 0 {
                alignment += (params.sell_card_color_alignment / cost_len) * ducats;
            }
        }

        best_alignment = best_alignment.max(alignment);
    }

    let glass_score = params.glass_weight * player.completed_glass.len() as f64;

    let primary_coverage = primary_seen.count_ones() as f64 / 3.0;
    let secondary_coverage = secondary_seen.count_ones() as f64 / 3.0;
    let material_type_count = player.materials.counts.iter().filter(|&&c| c > 0).count() as f64;
    let material_coverage = material_seen.count_ones() as f64 / 3.0;

    score + color_score + material_score + deck_quality + best_alignment + glass_score
        + params.primary_color_coverage_weight * primary_coverage
        + params.secondary_color_coverage_weight * secondary_coverage
        + params.cards_in_deck_weight * card_count as f64
        + params.cards_in_deck_squared_weight * (card_count as f64) * (card_count as f64)
        + params.material_type_count_weight * material_type_count
        + params.material_coverage_weight * material_coverage
}

/// Compute heuristic rewards for truncated early-game rollouts.
/// Highest heuristic score gets 1.0, others get 0.0. Ties split evenly.
/// Solo mode: clamp(heuristic_score / 20, 0, 1) as absolute progress estimate.
pub fn compute_heuristic_rewards(
    players: &FixedVec<PlayerState, MAX_PLAYERS>,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
    card_table: &CardHeuristicTable,
) -> [f64; MAX_PLAYERS] {
    let mut scores = [0.0f64; MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        scores[i] = heuristic_score(p, sell_card_display, card_lookup, params, card_table);
    }

    if players.len() == 1 {
        let mut result = [0.0; MAX_PLAYERS];
        result[0] = (scores[0] / 20.0).clamp(0.0, 1.0);
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
            deck: UnorderedCards::new(),
            discard: UnorderedCards::new(),
            workshopped_cards: UnorderedCards::new(),
            workshop_cards: UnorderedCards::new(),
            drafted_cards: UnorderedCards::new(),
            color_wheel: ColorWheel { counts: color_counts },
            materials: Materials::new(),
            completed_sell_cards,
            completed_glass: SmallVec::new(),
            ducats,
            cached_score: 0,
        };
        p.cached_score = calculate_score(&p);
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
        // p1: 2 3-ducat sell cards = 6 ducats, 2 sell cards
        // p2: 1 3-ducat sell card + 3 ducats = 6 ducats, 1 sell card
        let p1 = make_player(0, &[SellCard::Ceramics3VermilionRed, SellCard::Ceramics3AmberRed], [0; 12]);
        let p2 = make_player(3, &[SellCard::Ceramics3VermilionRed], [0; 12]);
        assert_eq!(p1.cached_score, 6);
        assert_eq!(p2.cached_score, 6);
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
}
