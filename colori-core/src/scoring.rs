use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicParams {
    pub primary_pip_weight: f64,
    pub secondary_pip_weight: f64,
    pub tertiary_pip_weight: f64,
    pub stored_material_weight: f64,
    pub chalk_quality: f64,
    pub action_quality: f64,
    pub dye_quality: f64,
    pub basic_dye_quality: f64,
    pub starter_material_quality: f64,
    pub draft_material_quality: f64,
    pub dual_material_quality: f64,
    pub buyer_material_weight: f64,
    pub buyer_color_weight: f64,
    pub glass_weight: f64,
    pub heuristic_round_threshold: u32,
    pub heuristic_lookahead: u32,
}

impl Default for HeuristicParams {
    fn default() -> Self {
        HeuristicParams {
            primary_pip_weight: 0.10,
            secondary_pip_weight: 0.20,
            tertiary_pip_weight: 0.30,
            stored_material_weight: 0.20,
            chalk_quality: 0.20,
            action_quality: 1.00,
            dye_quality: 1.00,
            basic_dye_quality: 0.10,
            starter_material_quality: 0.20,
            draft_material_quality: 0.50,
            dual_material_quality: 0.60,
            buyer_material_weight: 0.50,
            buyer_color_weight: 0.50,
            glass_weight: 1.0,
            heuristic_round_threshold: 3,
            heuristic_lookahead: 3,
        }
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
pub fn compute_terminal_rewards(players: &FixedVec<PlayerState, MAX_PLAYERS>) -> SmallVec<[f64; 4]> {
    let rankings: SmallVec<[(u32, usize, u32); 4]> = players
        .iter()
        .map(|p| {
            (
                p.cached_score,
                p.completed_sell_cards.len(),
                p.color_wheel.counts.iter().sum(),
            )
        })
        .collect();
    let best = rankings.iter().copied().max().unwrap_or((0, 0, 0));
    let num_winners = rankings.iter().filter(|&&r| r == best).count() as f64;
    rankings
        .iter()
        .map(|&r| if r == best { 1.0 / num_winners } else { 0.0 })
        .collect()
}

fn card_quality(card: Card, params: &HeuristicParams) -> f64 {
    if matches!(card, Card::Chalk) {
        return params.chalk_quality;
    }
    match card.kind() {
        CardKind::Action => params.action_quality,
        CardKind::Dye => params.dye_quality,
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

fn heuristic_score(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
) -> f64 {
    let score = player.cached_score as f64;

    let mut color_score = 0.0;
    for &c in &PRIMARIES {
        color_score += params.primary_pip_weight * player.color_wheel.get(c) as f64;
    }
    for &c in &SECONDARIES {
        color_score += params.secondary_pip_weight * player.color_wheel.get(c) as f64;
    }
    for &c in &TERTIARIES {
        color_score += params.tertiary_pip_weight * player.color_wheel.get(c) as f64;
    }

    let material_score = params.stored_material_weight * player.materials.counts.iter().sum::<u32>() as f64;

    let mut total_quality = 0.0;
    let mut card_count = 0u32;
    for cards in [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards] {
        for id in cards.iter() {
            total_quality += card_quality(card_lookup[id as usize], params);
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
            alignment += params.buyer_material_weight * ducats;
        }

        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        for &color in cost {
            if player.color_wheel.get(color) > 0 {
                alignment += (params.buyer_color_weight / cost_len) * ducats;
            }
        }

        best_alignment = best_alignment.max(alignment);
    }

    let glass_score = params.glass_weight * player.completed_glass.len() as f64;

    score + color_score + material_score + deck_quality + best_alignment + glass_score
}

/// Compute heuristic rewards for truncated early-game rollouts.
/// Highest heuristic score gets 1.0, others get 0.0. Ties split evenly.
pub fn compute_heuristic_rewards(
    players: &FixedVec<PlayerState, MAX_PLAYERS>,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
) -> SmallVec<[f64; 4]> {
    let scores: SmallVec<[f64; 4]> = players
        .iter()
        .map(|p| heuristic_score(p, sell_card_display, card_lookup, params))
        .collect();

    // Use integer comparison via bits for exact tie detection
    let best = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let num_winners = scores.iter().filter(|&&s| s == best).count() as f64;
    scores
        .iter()
        .map(|&s| if s == best { 1.0 / num_winners } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SellCard, SellCardInstance, ColorWheel, Materials, PlayerState};
    use crate::unordered_cards::UnorderedCards;

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
