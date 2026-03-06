use colori_core::atomic::{
    apply_atomic_choice, enumerate_atomic_choices, atomic_choice_to_index,
    index_to_atomic_choice, NUM_ATOMIC_ACTIONS,
};
use colori_core::encoding::{encode_legal_mask, encode_observation, OBS_SIZE};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::scoring::compute_terminal_rewards;
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;
use numpy::pyo3::Python;
use numpy::PyArray1;
use pyo3::prelude::*;
use rand::SeedableRng;
use wyrand::WyRand;

#[pyclass]
struct PyGameState {
    state: GameState,
    rng: WyRand,
}

#[pymethods]
impl PyGameState {
    #[new]
    fn new(num_players: usize, seed: u64) -> Self {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &ai_players, &mut rng);
        PyGameState { state, rng }
    }

    fn clone_state(&self) -> Self {
        PyGameState {
            state: self.state.clone(),
            rng: WyRand::seed_from_u64(0), // fresh rng for clone
        }
    }

    fn get_observation<'py>(&self, py: Python<'py>, player_index: usize) -> Bound<'py, PyArray1<f32>> {
        let mut buf = [0.0f32; OBS_SIZE];
        encode_observation(&self.state, player_index, &mut buf);
        PyArray1::from_slice(py, &buf)
    }

    fn get_legal_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        let mut mask = [0.0f32; NUM_ATOMIC_ACTIONS];
        encode_legal_mask(&self.state, &mut mask);
        PyArray1::from_slice(py, &mask)
    }

    fn apply_action(&mut self, action_index: usize) {
        let choice = index_to_atomic_choice(action_index);
        apply_atomic_choice(&mut self.state, &choice, &mut self.rng);
    }

    fn get_current_player(&self) -> usize {
        match &self.state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            _ => 0,
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self.state.phase, GamePhase::GameOver)
    }

    fn get_rewards(&self) -> Vec<f64> {
        compute_terminal_rewards(&self.state.players).to_vec()
    }

    fn get_num_players(&self) -> usize {
        self.state.players.len()
    }

    fn is_draw_phase(&self) -> bool {
        matches!(self.state.phase, GamePhase::Draw)
    }

    fn advance_draw_phase(&mut self) {
        if matches!(self.state.phase, GamePhase::Draw) {
            execute_draw_phase(&mut self.state, &mut self.rng);
        }
    }

    fn get_round(&self) -> u32 {
        self.state.round
    }

    fn get_legal_actions(&self) -> Vec<usize> {
        enumerate_atomic_choices(&self.state)
            .iter()
            .map(|c| atomic_choice_to_index(c))
            .collect()
    }
}

#[pymodule]
fn colori_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGameState>()?;
    m.add("NUM_ACTIONS", NUM_ATOMIC_ACTIONS)?;
    m.add("OBS_SIZE", OBS_SIZE)?;
    Ok(())
}
