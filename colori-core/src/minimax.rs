use crate::action_phase::can_afford_buyer;
use crate::colori_game::{
    apply_choice_to_state, determinize_in_place, enumerate_choices_into, get_game_status,
    GameStatus,
};
use crate::scoring::calculate_score;
use crate::types::*;
use rand::Rng;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct MinimaxConfig {
    pub depth: u32,
    pub num_determinizations: u32,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        MinimaxConfig {
            depth: 10,
            num_determinizations: 5,
        }
    }
}

impl<'de> Deserialize<'de> for MinimaxConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper {
            #[serde(default = "default_depth")]
            depth: u32,
            #[serde(default = "default_num_determinizations")]
            num_determinizations: u32,
        }

        fn default_depth() -> u32 {
            10
        }
        fn default_num_determinizations() -> u32 {
            5
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(MinimaxConfig {
            depth: helper.depth,
            num_determinizations: helper.num_determinizations,
        })
    }
}

fn evaluate(state: &GameState, player_index: usize) -> f64 {
    let player = &state.players[player_index];
    let score = calculate_score(player) as f64;

    // Bonus for affordable buyers in display
    let affordable_bonus: f64 = state
        .buyer_display
        .iter()
        .filter(|bi| can_afford_buyer(player, &bi.buyer))
        .map(|bi| bi.buyer.stars() as f64)
        .sum();

    // Bonus for color wheel diversity (distinct nonzero colors)
    let diversity: f64 = player
        .color_wheel
        .counts
        .iter()
        .filter(|&&c| c > 0)
        .count() as f64;

    // Bonus for material counts
    let materials: f64 = player.materials.counts.iter().sum::<u32>() as f64;

    // Small bonus for total color counts
    let color_total: f64 = player.color_wheel.counts.iter().sum::<u32>() as f64;

    score * 10.0 + affordable_bonus * 3.0 + diversity * 1.0 + materials * 0.5 + color_total * 0.2
}

pub fn minimax<R: Rng>(
    state: &GameState,
    player_index: usize,
    config: &MinimaxConfig,
    rng: &mut R,
) -> Choice {
    let mut choices_buf: Vec<Choice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let is_draft = matches!(state.phase, GamePhase::Draft { .. });

    if is_draft {
        // Determinize and average over multiple samples
        let mut score_sums: Vec<f64> = vec![0.0; choices_buf.len()];
        let mut det_state = state.clone();
        let mut cached_scores = [0u32; MAX_PLAYERS];
        for (i, p) in state.players.iter().enumerate() {
            cached_scores[i] = calculate_score(p);
        }

        for _ in 0..config.num_determinizations {
            determinize_in_place(
                &mut det_state,
                state,
                player_index,
                &None,
                &cached_scores,
                rng,
            );
            enumerate_choices_into(&det_state, &mut choices_buf);
            for (idx, choice) in choices_buf.iter().enumerate() {
                let mut child = det_state.clone();
                apply_choice_to_state(&mut child, choice, rng);
                let val = alpha_beta(
                    &mut child,
                    player_index,
                    config.depth - 1,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    &mut choices_buf.clone(),
                    rng,
                );
                score_sums[idx] += val;
            }
        }

        let best_idx = score_sums
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0;
        choices_buf[best_idx].clone()
    } else {
        // Perfect information - direct search
        let mut best_val = f64::NEG_INFINITY;
        let mut best_choice = choices_buf[0].clone();

        for choice in &choices_buf {
            let mut child = state.clone();
            apply_choice_to_state(&mut child, choice, rng);
            let val = alpha_beta(
                &mut child,
                player_index,
                config.depth - 1,
                f64::NEG_INFINITY,
                f64::INFINITY,
                &mut Vec::new(),
                rng,
            );
            if val > best_val {
                best_val = val;
                best_choice = choice.clone();
            }
        }

        best_choice
    }
}

fn alpha_beta<R: Rng>(
    state: &mut GameState,
    perspective_player: usize,
    depth: u32,
    mut alpha: f64,
    mut beta: f64,
    choices_buf: &mut Vec<Choice>,
    rng: &mut R,
) -> f64 {
    let active_player = match get_game_status(state, None) {
        GameStatus::Terminated { .. } => {
            // Terminal: large bonus/penalty based on win
            let my_score = calculate_score(&state.players[perspective_player]);
            let max_score = state
                .players
                .iter()
                .map(|p| calculate_score(p))
                .max()
                .unwrap();
            if my_score >= max_score {
                return 10000.0 + my_score as f64;
            } else {
                return -(10000.0 + max_score as f64);
            }
        }
        GameStatus::AwaitingAction { player_index } => player_index,
    };

    if depth == 0 {
        return evaluate(state, perspective_player);
    }

    enumerate_choices_into(state, choices_buf);

    if active_player == perspective_player {
        // Maximize
        let mut value = f64::NEG_INFINITY;
        let choices: Vec<Choice> = choices_buf.clone();
        for choice in &choices {
            let mut child = state.clone();
            apply_choice_to_state(&mut child, choice, rng);
            let child_val = alpha_beta(
                &mut child,
                perspective_player,
                depth - 1,
                alpha,
                beta,
                choices_buf,
                rng,
            );
            if child_val > value {
                value = child_val;
            }
            if value > alpha {
                alpha = value;
            }
            if alpha >= beta {
                break;
            }
        }
        value
    } else {
        // Minimize (paranoid: all opponents minimize perspective player's score)
        let mut value = f64::INFINITY;
        let choices: Vec<Choice> = choices_buf.clone();
        for choice in &choices {
            let mut child = state.clone();
            apply_choice_to_state(&mut child, choice, rng);
            let child_val = alpha_beta(
                &mut child,
                perspective_player,
                depth - 1,
                alpha,
                beta,
                choices_buf,
                rng,
            );
            if child_val < value {
                value = child_val;
            }
            if value < beta {
                beta = value;
            }
            if alpha >= beta {
                break;
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colori_game::{
        apply_choice_to_state, check_choice_available, enumerate_choices_into,
    };
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn run_full_game_validating_choices(num_players: usize, seed: u64) {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

        let config = MinimaxConfig {
            depth: 3,
            num_determinizations: 2,
        };

        execute_draw_phase(&mut state, &mut rng);

        let mut choices_buf: Vec<Choice> = Vec::new();
        let max_steps = 5000;

        for step in 0..max_steps {
            match &state.phase {
                GamePhase::GameOver => return,
                GamePhase::Draw => {
                    execute_draw_phase(&mut state, &mut rng);
                    continue;
                }
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let choice = minimax(&state, player_index, &config, &mut rng);

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: minimax choice {choice:?} \
                 not in enumerated choices",
                state.round, state.phase
            );

            assert!(
                check_choice_available(&state, &choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: check_choice_available returned \
                 false for {choice:?}",
                state.round, state.phase
            );

            apply_choice_to_state(&mut state, &choice, &mut rng);
        }

        panic!(
            "seed={seed}, players={num_players}: \
             game did not finish within {max_steps} steps"
        );
    }

    #[test]
    fn test_minimax_valid_moves_2_players() {
        for seed in 0..3 {
            run_full_game_validating_choices(2, seed);
        }
    }

    #[test]
    fn test_minimax_valid_moves_3_players() {
        for seed in 0..3 {
            run_full_game_validating_choices(3, seed);
        }
    }

    #[test]
    fn test_minimax_valid_moves_4_players() {
        for seed in 0..3 {
            run_full_game_validating_choices(4, seed);
        }
    }
}
