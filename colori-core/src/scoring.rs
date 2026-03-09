use crate::fixed_vec::FixedVec;
use crate::types::{PlayerState, MAX_PLAYERS};
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
