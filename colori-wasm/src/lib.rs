use colori_core::apply_choice::apply_choice;
use colori_core::draft_phase::{advance_draft, confirm_pass, simultaneous_pick};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{CardInstance, ColoriChoice, GameState, PlayerState};
use colori_core::unordered_cards::{get_buyer_registry, get_card_registry, set_buyer_registry, set_card_registry};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

fn deserialize_state(json: &str) -> GameState {
    let mut state: GameState =
        serde_json::from_str(json).expect("Failed to parse game state JSON");
    state.card_lookup = get_card_registry();
    state.buyer_lookup = get_buyer_registry();
    for p in state.players.iter_mut() {
        p.cached_score = calculate_score(p);
    }
    state
}

fn serialize_state(state: &GameState) -> String {
    set_card_registry(&state.card_lookup);
    set_buyer_registry(&state.buyer_lookup);
    serde_json::to_string(state).expect("Failed to serialize game state")
}

#[wasm_bindgen]
pub fn run_ismcts(
    game_state_json: &str,
    player_index: u32,
    iterations: u32,
    seen_hands_json: &str,
) -> String {
    let game_state = deserialize_state(game_state_json);

    let seen_hands: Option<Vec<Vec<CardInstance>>> = if seen_hands_json.is_empty() {
        None
    } else {
        serde_json::from_str(seen_hands_json).ok()
    };

    let max_round = std::cmp::max(8, game_state.round + 2);

    let mut rng = SmallRng::from_os_rng();

    let config = MctsConfig { iterations, ..MctsConfig::default() };
    let choice: ColoriChoice = ismcts(
        &game_state,
        player_index as usize,
        &config,
        &seen_hands,
        Some(max_round),
        &mut rng,
    );

    serde_json::to_string(&choice).expect("Failed to serialize choice")
}

#[wasm_bindgen]
pub fn wasm_create_initial_game_state(num_players: u32, ai_players_json: &str) -> String {
    let ai_players: Vec<bool> =
        serde_json::from_str(ai_players_json).expect("Failed to parse ai players JSON");
    let mut rng = SmallRng::from_os_rng();
    let state = create_initial_game_state(num_players as usize, &ai_players, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_execute_draw_phase(state_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let mut rng = SmallRng::from_os_rng();
    execute_draw_phase(&mut state, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_apply_choice(state_json: &str, choice_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let choice: ColoriChoice =
        serde_json::from_str(choice_json).expect("Failed to parse choice JSON");
    let mut rng = SmallRng::from_os_rng();
    apply_choice(&mut state, &choice, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_confirm_pass(state_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    confirm_pass(&mut state);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_simultaneous_pick(
    state_json: &str,
    player_index: u32,
    card_instance_id: u32,
) -> String {
    let mut state = deserialize_state(state_json);
    simultaneous_pick(&mut state, player_index as usize, card_instance_id);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_advance_draft(state_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    advance_draft(&mut state);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_calculate_scores(players_json: &str) -> String {
    let players: Vec<PlayerState> =
        serde_json::from_str(players_json).expect("Failed to parse players JSON");
    let scores: Vec<u32> = players
        .iter()
        .map(|p| calculate_score(p))
        .collect();
    serde_json::to_string(&scores).expect("Failed to serialize scores")
}
