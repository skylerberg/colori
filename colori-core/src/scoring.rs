use crate::types::{AnyCard, PlayerState};

pub fn calculate_score(player: &PlayerState) -> u32 {
    let buyer_stars: u32 = player
        .completed_buyers
        .iter()
        .map(|ci| match &ci.card {
            AnyCard::Buyer { stars, .. } => *stars,
            _ => 0,
        })
        .sum();
    buyer_stars + player.ducats
}
