use colori_core::ismcts::ismcts;
use colori_core::types::{CardInstance, ColoriChoice, GameState};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_ismcts(
    game_state_json: &str,
    player_index: u32,
    iterations: u32,
    seen_hands_json: &str,
) -> String {
    let game_state: GameState =
        serde_json::from_str(game_state_json).expect("Failed to parse game state JSON");

    let seen_hands: Option<Vec<Vec<CardInstance>>> = if seen_hands_json.is_empty() {
        None
    } else {
        serde_json::from_str(seen_hands_json).ok()
    };

    let max_round = std::cmp::max(6, game_state.round + 2);

    let mut rng = SmallRng::from_entropy();

    let choice: ColoriChoice = ismcts(
        &game_state,
        player_index as usize,
        iterations,
        &seen_hands,
        Some(max_round),
        &mut rng,
    );

    serde_json::to_string(&choice).expect("Failed to serialize choice")
}
