use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;
use smallvec::SmallVec;

pub fn calculate_score(player: &PlayerState) -> u32 {
    let buyer_stars: u32 = player.completed_buyers.iter().map(|bi| bi.buyer.stars()).sum();
    buyer_stars + player.ducats
}

/// Returns a comparable ranking tuple: (score, completed_buyers_count, color_wheel_total).
/// Rust tuples compare lexicographically, giving correct tiebreak order.
pub fn player_ranking(player: &PlayerState) -> (u32, usize, u32) {
    (
        calculate_score(player),
        player.completed_buyers.len(),
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
                p.completed_buyers.len(),
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

fn card_quality(card: Card) -> f64 {
    if matches!(card, Card::Chalk) {
        return 0.20;
    }
    match card.kind() {
        CardKind::Action => 0.50,
        CardKind::Dye => 0.50,
        CardKind::BasicDye => 0.05,
        CardKind::Material => {
            let pips = card.pips();
            let mat_types = card.material_types();
            if pips.is_empty() && mat_types.len() == 1 {
                0.10 // starter material
            } else if !pips.is_empty() {
                0.25 // draft material with pip
            } else {
                0.30 // dual material
            }
        }
    }
}

fn heuristic_score(
    player: &PlayerState,
    buyer_display: &FixedVec<BuyerInstance, MAX_BUYER_DISPLAY>,
    card_lookup: &[Card; 256],
) -> f64 {
    let points = player.cached_score as f64;

    // Color wheel score: primary 0.10, secondary 0.20, tertiary 0.30 per pip
    let mut color_score = 0.0;
    for &c in &PRIMARIES {
        color_score += 0.10 * player.color_wheel.get(c) as f64;
    }
    for &c in &SECONDARIES {
        color_score += 0.20 * player.color_wheel.get(c) as f64;
    }
    for &c in &TERTIARIES {
        color_score += 0.30 * player.color_wheel.get(c) as f64;
    }

    // Stored materials: 0.20 each
    let material_score = 0.20 * player.materials.counts.iter().sum::<u32>() as f64;

    // Deck quality: average card quality across all owned cards
    let mut total_quality = 0.0;
    let mut card_count = 0u32;
    for cards in [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards] {
        for id in cards.iter() {
            total_quality += card_quality(card_lookup[id as usize]);
            card_count += 1;
        }
    }
    let deck_quality = if card_count > 0 {
        total_quality / card_count as f64
    } else {
        0.0
    };

    // Buyer alignment: max across visible buyers
    let mut best_alignment = 0.0f64;
    for bi in buyer_display.iter() {
        let buyer = bi.buyer;
        let stars = buyer.stars() as f64;
        let mut alignment = 0.0;

        // Has required material type?
        if player.materials.get(buyer.required_material()) > 0 {
            alignment += 0.5 * stars;
        }

        // Color progress
        let cost = buyer.color_cost();
        let cost_len = cost.len() as f64;
        for &color in cost {
            if player.color_wheel.get(color) > 0 {
                alignment += (0.5 / cost_len) * stars;
            }
        }

        best_alignment = best_alignment.max(alignment);
    }

    // Glass cards: 2.0 each
    let glass_score = 2.0 * player.completed_glass.len() as f64;

    points + color_score + material_score + deck_quality + best_alignment + glass_score
}

/// Compute heuristic rewards for truncated early-game rollouts.
/// Highest heuristic score gets 1.0, others get 0.0. Ties split evenly.
pub fn compute_heuristic_rewards(
    players: &FixedVec<PlayerState, MAX_PLAYERS>,
    buyer_display: &FixedVec<BuyerInstance, MAX_BUYER_DISPLAY>,
    card_lookup: &[Card; 256],
) -> SmallVec<[f64; 4]> {
    let scores: SmallVec<[f64; 4]> = players
        .iter()
        .map(|p| heuristic_score(p, buyer_display, card_lookup))
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
    use crate::types::{BuyerCard, BuyerInstance, ColorWheel, Materials, PlayerState};
    use crate::unordered_cards::UnorderedCards;

    fn make_player(ducats: u32, buyers: &[BuyerCard], color_counts: [u32; 12]) -> PlayerState {
        let completed_buyers: SmallVec<[BuyerInstance; 12]> = buyers
            .iter()
            .enumerate()
            .map(|(i, &buyer)| BuyerInstance {
                instance_id: i as u32,
                buyer,
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
            completed_buyers,
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
    fn test_tie_broken_by_buyers() {
        // p1: 2 three-star buyers = 6 points, 2 buyers
        // p2: 1 three-star buyer + 3 ducats = 6 points, 1 buyer
        let p1 = make_player(0, &[BuyerCard::Ceramics3VermilionRed, BuyerCard::Ceramics3AmberRed], [0; 12]);
        let p2 = make_player(3, &[BuyerCard::Ceramics3VermilionRed], [0; 12]);
        assert_eq!(p1.cached_score, 6);
        assert_eq!(p2.cached_score, 6);
        let mut players = FixedVec::new();
        players.push(p1);
        players.push(p2);
        let rewards = compute_terminal_rewards(&players);
        assert_eq!(rewards[0], 1.0); // p1 wins on buyer count
        assert_eq!(rewards[1], 0.0);
    }

    #[test]
    fn test_tie_broken_by_colors() {
        // Same score, same buyer count, different color wheel totals
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
        // Same score, same buyer count, same color wheel total
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
