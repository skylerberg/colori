use colori_core::apply_choice::apply_choice;
use colori_core::colori_game::enumerate_choices;
use colori_core::draft_phase::{advance_draft, confirm_pass, simultaneous_pick};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::nn_mcts::{nn_ismcts, NnEvaluator, NnMctsConfig};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::state_encoding::{encode_legal_actions, encode_state};
use colori_core::types::{CardInstance, ColoriChoice, GameState, PlayerState};
use colori_core::unordered_cards::{
    get_buyer_registry, get_card_registry, set_buyer_registry, set_card_registry,
};
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

/// Encode game state for NN input.
/// Returns a Float32Array of length STATE_ENCODING_SIZE (768).
#[wasm_bindgen]
pub fn wasm_encode_state(game_state_json: &str, perspective_player: u32) -> Vec<f32> {
    let state = deserialize_state(game_state_json);
    encode_state(&state, perspective_player as usize)
}

/// Encode all legal actions for NN input.
/// Returns a JSON string containing an array of float arrays.
#[wasm_bindgen]
pub fn wasm_encode_legal_actions(game_state_json: &str) -> String {
    let state = deserialize_state(game_state_json);
    let encodings = encode_legal_actions(&state);
    serde_json::to_string(&encodings).expect("Failed to serialize action encodings")
}

/// Get legal actions as JSON.
#[wasm_bindgen]
pub fn wasm_get_legal_actions(game_state_json: &str) -> String {
    let state = deserialize_state(game_state_json);
    let choices = enumerate_choices(&state);
    serde_json::to_string(&choices).expect("Failed to serialize choices")
}

/// JS-backed NN evaluator that calls a JavaScript function for inference.
struct JsEvaluator {
    eval_fn: js_sys::Function,
}

// SAFETY: WASM is single-threaded, so Send + Sync are trivially safe.
// These bounds are required by the NnEvaluator trait, which is designed
// for native multithreaded MCTS. In the WASM context, there is no actual
// concurrent access.
unsafe impl Send for JsEvaluator {}
unsafe impl Sync for JsEvaluator {}

impl NnEvaluator for JsEvaluator {
    fn evaluate(&self, state_encoding: &[f32], action_encodings: &[&[f32]]) -> (Vec<f32>, f32) {
        // Convert state encoding to JS Float32Array
        let state_arr = js_sys::Float32Array::new_with_length(state_encoding.len() as u32);
        state_arr.copy_from(state_encoding);

        // Convert action encodings to JS array of Float32Arrays
        let actions_arr = js_sys::Array::new_with_length(action_encodings.len() as u32);
        for (i, enc) in action_encodings.iter().enumerate() {
            let arr = js_sys::Float32Array::new_with_length(enc.len() as u32);
            arr.copy_from(enc);
            actions_arr.set(i as u32, arr.into());
        }

        // Call JS function: (Float32Array, Array<Float32Array>) -> {priors: Float32Array, value: number}
        let this = JsValue::NULL;
        let result = self
            .eval_fn
            .call2(&this, &state_arr.into(), &actions_arr.into())
            .expect("JS evaluation callback failed");

        // Parse result
        let priors_js = js_sys::Reflect::get(&result, &JsValue::from_str("priors"))
            .expect("Missing 'priors' in eval result");
        let value_js = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
            .expect("Missing 'value' in eval result");

        let priors_typed = js_sys::Float32Array::from(priors_js);
        let mut priors = vec![0.0f32; priors_typed.length() as usize];
        priors_typed.copy_to(&mut priors);

        let value = value_js.as_f64().unwrap_or(0.5) as f32;

        (priors, value)
    }
}

/// Run NN-MCTS with externally-provided evaluation via a JS callback.
///
/// The eval_fn is a JavaScript function with signature:
///   (stateEncoding: Float32Array, actionEncodings: Float32Array[]) =>
///     { priors: Float32Array, value: number }
///
/// Returns a JSON-serialized ColoriChoice.
#[wasm_bindgen]
pub fn wasm_run_nn_mcts(
    game_state_json: &str,
    player_index: u32,
    iterations: u32,
    c_puct: f32,
    eval_fn: &js_sys::Function,
    seen_hands_json: &str,
) -> String {
    let state = deserialize_state(game_state_json);
    let seen_hands: Option<Vec<Vec<CardInstance>>> = if seen_hands_json.is_empty() {
        None
    } else {
        serde_json::from_str(seen_hands_json).ok()
    };

    let evaluator = JsEvaluator {
        eval_fn: eval_fn.clone(),
    };
    let config = NnMctsConfig {
        iterations,
        c_puct,
    };

    let max_round = std::cmp::max(8, state.round + 2);
    let mut rng = SmallRng::from_os_rng();
    let (choice, _visit_dist) = nn_ismcts(
        &state,
        player_index as usize,
        &config,
        &evaluator,
        &seen_hands,
        Some(max_round),
        &mut rng,
    );

    serde_json::to_string(&choice).expect("Failed to serialize choice")
}
