use colori_core::apply_choice::apply_choice;
use colori_core::colori_game::enumerate_choices;
use colori_core::draft_phase::{advance_draft, simultaneous_pick};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, DiffEvalParams, HeuristicParams};
use colori_core::setup::{create_initial_game_state, create_initial_game_state_with_expansions};
use colori_core::types::{Card, CardInstance, Choice, Expansions, GameState, PlayerState};
use colori_core::unordered_cards::{
    get_card_registry, get_sell_card_registry, set_card_registry, set_sell_card_registry,
};
use rand::SeedableRng;
use wyrand::WyRand;
use wasm_bindgen::prelude::*;

const TRAINED_PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-rqo1vv-gen-18.json");
const NN_PARAMS_JSON: &str = include_str!("../../diff-eval-training/diff-eval-epoch-213.json");

fn deserialize_state(json: &str) -> GameState {
    let mut state: GameState =
        serde_json::from_str(json).expect("Failed to parse game state JSON");
    state.card_lookup = get_card_registry();
    state.sell_card_lookup = get_sell_card_registry();
    for p in state.players.iter_mut() {
        p.cached_score = calculate_score(p);
    }
    state
}

fn serialize_state(state: &GameState) -> String {
    set_card_registry(&state.card_lookup);
    set_sell_card_registry(&state.sell_card_lookup);
    serde_json::to_string(state).expect("Failed to serialize game state")
}

#[wasm_bindgen]
pub fn wasm_run_ismcts(
    game_state_json: &str,
    player_index: u32,
    iterations: u32,
    known_draft_hands_json: &str,
    ai_style: &str,
) -> String {
    let game_state = deserialize_state(game_state_json);

    let known_draft_hands: Option<Vec<Vec<CardInstance>>> = if known_draft_hands_json.is_empty() {
        None
    } else {
        serde_json::from_str(known_draft_hands_json).ok()
    };

    let max_rollout_round = std::cmp::max(8, game_state.round + 2);

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let heuristic_params: HeuristicParams = serde_json::from_str(TRAINED_PARAMS_JSON)
        .expect("Failed to parse trained heuristic params");
    let diff_eval_params = if ai_style == "nn" {
        Some(serde_json::from_str::<DiffEvalParams>(NN_PARAMS_JSON)
            .expect("Failed to parse NN eval params"))
    } else {
        None
    };
    let config = MctsConfig { iterations, heuristic_params, diff_eval_params, ..MctsConfig::default() };
    let result = ismcts(
        &game_state,
        player_index as usize,
        &config,
        &known_draft_hands,
        Some(max_rollout_round),
        None,
        &mut rng,
    );

    serde_json::to_string(&result.choice).expect("Failed to serialize choice")
}

#[wasm_bindgen]
pub fn wasm_create_initial_game_state(num_players: u32, ai_players_json: &str) -> String {
    let ai_players: Vec<bool> =
        serde_json::from_str(ai_players_json).expect("Failed to parse ai players JSON");
    let mut rng = WyRand::from_rng(&mut rand::rng());
    let state = create_initial_game_state(num_players as usize, &ai_players, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_create_initial_game_state_with_expansions(
    num_players: u32,
    ai_players_json: &str,
    expansions_json: &str,
) -> String {
    let ai_players: Vec<bool> =
        serde_json::from_str(ai_players_json).expect("Failed to parse ai players JSON");
    let expansions: Expansions =
        serde_json::from_str(expansions_json).expect("Failed to parse expansions JSON");
    let mut rng = WyRand::from_rng(&mut rand::rng());
    let state = create_initial_game_state_with_expansions(
        num_players as usize,
        &ai_players,
        expansions,
        &mut rng,
    );
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_execute_draw_phase(state_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let mut rng = WyRand::from_rng(&mut rand::rng());
    execute_draw_phase(&mut state, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_apply_choice(state_json: &str, choice_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let choice: Choice =
        serde_json::from_str(choice_json).expect("Failed to parse choice JSON");
    let mut rng = WyRand::from_rng(&mut rand::rng());
    apply_choice(&mut state, &choice, &mut rng);
    serialize_state(&state)
}

#[wasm_bindgen]
pub fn wasm_simultaneous_pick(
    state_json: &str,
    player_index: u32,
    card_json: &str,
) -> String {
    let mut state = deserialize_state(state_json);
    let card: Card = serde_json::from_str(card_json).expect("Failed to parse card JSON");
    simultaneous_pick(&mut state, player_index as usize, card);
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

/// Get legal actions as JSON.
#[wasm_bindgen]
pub fn wasm_get_legal_actions(game_state_json: &str) -> String {
    let state = deserialize_state(game_state_json);
    let choices = enumerate_choices(&state);
    serde_json::to_string(&choices).expect("Failed to serialize choices")
}
