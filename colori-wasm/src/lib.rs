use colori_core::apply_choice::apply_choice;
use colori_core::draft_phase::{advance_draft, simultaneous_pick};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{DrawEvent, DrawLog};
use colori_core::ismcts::{MctsConfig};
use colori_core::mcts_impl::{search_refusal, ColoriSearcher};
use colori_core::scoring::{calculate_ducat_score, score_in_half_points, HeuristicParams};
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Card, Choice, GameState, PlayerState};
use colori_core::unordered_cards::{
    get_card_registry, get_sell_card_registry, set_card_registry, set_sell_card_registry,
};
use rand::SeedableRng;
use serde::Serialize;
use wyrand::WyRand;
use wasm_bindgen::prelude::*;

const TRAINED_PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: &str);
}

/// Report panics to the browser console when the module is initialized.
///
/// Without a hook a panic is only the wasm `unreachable` trap, which reaches
/// JS as `RuntimeError: unreachable` with no message or location — undebuggable
/// from the browser side.
#[wasm_bindgen(start)]
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        error(&format!("colori wasm panicked: {info}"));
    }));
}

fn deserialize_state(json: &str) -> GameState {
    let mut state: GameState =
        serde_json::from_str(json).expect("Failed to parse game state JSON");
    state.card_lookup = get_card_registry();
    state.sell_card_lookup = get_sell_card_registry();
    for p in state.players.iter_mut() {
        p.cached_ducat_score = calculate_ducat_score(p);
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
    _ai_style: &str,
) -> Result<String, JsError> {
    let game_state = deserialize_state(game_state_json);
    let player_index = player_index as usize;
    if let Some(refusal) = search_refusal(&game_state, player_index) {
        return Err(JsError::new(&format!("cannot run the AI here: {refusal}")));
    }

    let max_rollout_round = std::cmp::max(8, game_state.round + 2);

    let mut rng = WyRand::from_rng(&mut rand::rng());

    let heuristic_params: HeuristicParams = serde_json::from_str(TRAINED_PARAMS_JSON)
        .expect("Failed to parse trained heuristic params");
    let config = MctsConfig { iterations, ..MctsConfig::new(heuristic_params) };
    let mut searcher = ColoriSearcher::new(&game_state);
    let result = searcher.search(
        &game_state,
        player_index,
        &config,
        Some(max_rollout_round),
        &mut rng,
    );

    Ok(serde_json::to_string(&result.choice).expect("Failed to serialize choice"))
}

#[wasm_bindgen]
pub fn wasm_create_initial_game_state(num_players: u32, ai_players_json: &str) -> String {
    let ai_players: Vec<bool> =
        serde_json::from_str(ai_players_json).expect("Failed to parse ai players JSON");
    let mut rng = WyRand::from_rng(&mut rand::rng());
    let state = create_initial_game_state(num_players as usize, &ai_players, &mut rng);
    serialize_state(&state)
}

#[derive(Serialize)]
struct StateWithDraws {
    state: serde_json::Value,
    draws: Vec<DrawEvent>,
}

fn serialize_state_with_draws(state: &mut GameState) -> String {
    let draws = match state.draw_log.take() {
        Some(DrawLog::Recording(events)) => events,
        _ => Vec::new(),
    };
    let state_json = {
        set_card_registry(&state.card_lookup);
        set_sell_card_registry(&state.sell_card_lookup);
        serde_json::to_value(state).expect("Failed to serialize game state")
    };
    serde_json::to_string(&StateWithDraws { state: state_json, draws })
        .expect("Failed to serialize state with draws")
}

#[wasm_bindgen]
pub fn wasm_execute_draw_phase(state_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let mut rng = WyRand::from_rng(&mut rand::rng());
    state.draw_log = Some(DrawLog::Recording(Vec::new()));
    execute_draw_phase(&mut state, &mut rng);
    serialize_state_with_draws(&mut state)
}

#[wasm_bindgen]
pub fn wasm_apply_choice(state_json: &str, player_index: u32, choice_json: &str) -> String {
    let mut state = deserialize_state(state_json);
    let choice: Choice =
        serde_json::from_str(choice_json).expect("Failed to parse choice JSON");
    let mut rng = WyRand::from_rng(&mut rand::rng());
    state.draw_log = Some(DrawLog::Recording(Vec::new()));
    apply_choice(&mut state, player_index as usize, &choice, &mut rng);
    serialize_state_with_draws(&mut state)
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
    let scores: Vec<f64> = players
        .iter()
        .map(|p| score_in_half_points(p) as f64 / 2.0)
        .collect();
    serde_json::to_string(&scores).expect("Failed to serialize scores")
}
