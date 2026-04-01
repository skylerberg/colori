pub mod first_pick;
mod heuristic_params;

pub use first_pick::FirstPickParams;
pub use heuristic_params::HeuristicParams;

use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;

const ALL_CARDS: [Card; 46] = [
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
    Card::Potash, Card::Vinegar, Card::Chalk,
    Card::LinseedOil,
];

pub struct CardHeuristicTable {
    quality: [f64; 46],
}

impl CardHeuristicTable {
    pub fn new(params: &HeuristicParams) -> Self {
        let mut quality = [0.0f64; 46];
        for &card in &ALL_CARDS {
            let idx = card as usize;
            quality[idx] = card_quality(card, params);
        }
        CardHeuristicTable { quality }
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
/// Solo mode: 1.0 for reaching 16 ducats (win) + score/100 for gradient signal.
pub fn compute_terminal_rewards(players: &FixedVec<PlayerState, MAX_PLAYERS>) -> [f64; MAX_PLAYERS] {
    if players.len() == 1 {
        let mut result = [0.0; MAX_PLAYERS];
        let score = players[0].cached_score as f64;
        result[0] = if score >= 16.0 { 1.0 } else { 0.0 } + score / 100.0;
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
                Card::Alum => params.alum_quality.unwrap_or(0.0),
                Card::CreamOfTartar => params.cream_of_tartar_quality.unwrap_or(0.0),
                Card::GumArabic => params.gum_arabic_quality.unwrap_or(0.0),
                Card::Potash => params.potash_quality.unwrap_or(0.0),
                Card::Vinegar => params.vinegar_quality.unwrap_or(0.0),
                Card::LinseedOil => params.linseed_oil_quality,
                _ => 0.0,
            }
        }
        CardKind::Dye => {
            match card {
                Card::Kermes | Card::Weld | Card::Woad => params.pure_primary_dye_quality.unwrap_or(0.0),
                Card::Lac | Card::Brazilwood | Card::Pomegranate
                | Card::Sumac | Card::Elderberry | Card::Turnsole => params.primary_dye_quality.unwrap_or(0.0),
                Card::Madder | Card::Turmeric | Card::DyersGreenweed
                | Card::Verdigris | Card::Orchil | Card::Logwood => params.secondary_dye_quality.unwrap_or(0.0),
                Card::VermilionDye | Card::Saffron | Card::PersianBerries
                | Card::Azurite | Card::IndigoDye | Card::Cochineal => params.tertiary_dye_quality.unwrap_or(0.0),
                _ => 0.0,
            }
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
    for cards in [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards] {
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

    score + color_score + material_score + deck_quality + best_alignment
}

/// Compute heuristic rewards for truncated early-game rollouts.
/// Highest heuristic score gets 1.0, others get 0.0. Ties split evenly.
/// Solo mode: same formula as terminal (win bonus + score/100) using cached_score.
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
        let score = players[0].cached_score as f64;
        result[0] = if score >= 16.0 { 1.0 } else { 0.0 } + score / 100.0;
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
