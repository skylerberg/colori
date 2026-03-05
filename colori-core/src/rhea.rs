use crate::colori_game::{
    apply_choice_to_state, determinize_in_place, enumerate_choices_into, get_game_status,
    GameStatus,
};
use crate::rollout::apply_rollout_step;
use crate::scoring::{calculate_score, compute_terminal_rewards};
use crate::types::*;
use rand::Rng;
use rand::RngExt;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct RheaConfig {
    pub generations: u32,
    pub population_size: u32,
    pub horizon_length: u32,
    pub mutation_rate: f64,
    pub max_rollout_steps: u32,
    pub elitism_count: u32,
    pub tournament_size: u32,
}

impl Default for RheaConfig {
    fn default() -> Self {
        RheaConfig {
            generations: 100,
            population_size: 20,
            horizon_length: 15,
            mutation_rate: 0.3,
            max_rollout_steps: 1000,
            elitism_count: 2,
            tournament_size: 3,
        }
    }
}

impl<'de> Deserialize<'de> for RheaConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RheaConfigHelper {
            #[serde(default = "default_generations")]
            generations: u32,
            #[serde(default = "default_population_size")]
            population_size: u32,
            #[serde(default = "default_horizon_length")]
            horizon_length: u32,
            #[serde(default = "default_mutation_rate")]
            mutation_rate: f64,
            #[serde(default = "default_max_rollout_steps")]
            max_rollout_steps: u32,
            #[serde(default = "default_elitism_count")]
            elitism_count: u32,
            #[serde(default = "default_tournament_size")]
            tournament_size: u32,
        }

        fn default_generations() -> u32 { 100 }
        fn default_population_size() -> u32 { 20 }
        fn default_horizon_length() -> u32 { 15 }
        fn default_mutation_rate() -> f64 { 0.3 }
        fn default_max_rollout_steps() -> u32 { 1000 }
        fn default_elitism_count() -> u32 { 2 }
        fn default_tournament_size() -> u32 { 3 }

        let helper = RheaConfigHelper::deserialize(deserializer)?;
        Ok(RheaConfig {
            generations: helper.generations,
            population_size: helper.population_size,
            horizon_length: helper.horizon_length,
            mutation_rate: helper.mutation_rate,
            max_rollout_steps: helper.max_rollout_steps,
            elitism_count: helper.elitism_count,
            tournament_size: helper.tournament_size,
        })
    }
}

struct Individual {
    genes: Vec<Option<Choice>>,
    fitness: f64,
}

#[inline]
fn is_terminal(state: &GameState, max_rollout_round: Option<u32>) -> bool {
    matches!(state.phase, GamePhase::GameOver)
        || max_rollout_round.is_some_and(|mr| state.round > mr)
}

fn generate_random_individual<R: Rng>(
    det_state: &mut GameState,
    state: &GameState,
    player_index: usize,
    horizon_length: u32,
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    cached_scores: &[u32; MAX_PLAYERS],
    max_rollout_round: Option<u32>,
    choices_buf: &mut Vec<Choice>,
    rng: &mut R,
) -> Individual {
    determinize_in_place(det_state, state, player_index, known_draft_hands, cached_scores, rng);

    let mut genes = Vec::with_capacity(horizon_length as usize);

    for _ in 0..horizon_length {
        if is_terminal(det_state, max_rollout_round) {
            break;
        }

        let active_player = match get_game_status(det_state, max_rollout_round) {
            GameStatus::AwaitingAction { player_index: p } => p,
            GameStatus::Terminated { .. } => break,
        };

        if active_player == player_index {
            enumerate_choices_into(det_state, choices_buf);
            if choices_buf.is_empty() {
                break;
            }
            let idx = rng.random_range(0..choices_buf.len());
            let choice = choices_buf[idx].clone();
            apply_choice_to_state(det_state, &choice, rng);
            genes.push(Some(choice));
        } else {
            apply_rollout_step(det_state, rng);
        }
    }

    Individual {
        genes,
        fitness: f64::NEG_INFINITY,
    }
}

fn evaluate<R: Rng>(
    individual: &mut Individual,
    det_state: &mut GameState,
    state: &GameState,
    player_index: usize,
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    cached_scores: &[u32; MAX_PLAYERS],
    max_rollout_round: Option<u32>,
    max_rollout_steps: u32,
    choices_buf: &mut Vec<Choice>,
    rng: &mut R,
) {
    determinize_in_place(det_state, state, player_index, known_draft_hands, cached_scores, rng);

    let mut gene_idx = 0;

    // Replay genes
    while gene_idx < individual.genes.len() {
        if is_terminal(det_state, max_rollout_round) {
            break;
        }

        let active_player = match get_game_status(det_state, max_rollout_round) {
            GameStatus::AwaitingAction { player_index: p } => p,
            GameStatus::Terminated { .. } => break,
        };

        if active_player == player_index {
            enumerate_choices_into(det_state, choices_buf);
            if choices_buf.is_empty() {
                break;
            }

            let choice = match &individual.genes[gene_idx] {
                Some(c) if choices_buf.contains(c) => c.clone(),
                _ => {
                    let idx = rng.random_range(0..choices_buf.len());
                    let c = choices_buf[idx].clone();
                    individual.genes[gene_idx] = Some(c.clone());
                    c
                }
            };

            apply_choice_to_state(det_state, &choice, rng);
            gene_idx += 1;
        } else {
            apply_rollout_step(det_state, rng);
        }
    }

    // Rollout to terminal
    for _ in 0..max_rollout_steps {
        if is_terminal(det_state, max_rollout_round) {
            break;
        }
        apply_rollout_step(det_state, rng);
    }

    if is_terminal(det_state, max_rollout_round) {
        let rewards = compute_terminal_rewards(&det_state.players);
        individual.fitness = if player_index < rewards.len() {
            rewards[player_index]
        } else {
            0.0
        };
    } else {
        individual.fitness = 0.0;
    }
}

fn tournament_select<'a, R: Rng>(
    population: &'a [Individual],
    tournament_size: u32,
    rng: &mut R,
) -> usize {
    let mut best_idx = rng.random_range(0..population.len());
    for _ in 1..tournament_size {
        let idx = rng.random_range(0..population.len());
        if population[idx].fitness > population[best_idx].fitness {
            best_idx = idx;
        }
    }
    best_idx
}

fn crossover<R: Rng>(parent1: &Individual, parent2: &Individual, rng: &mut R) -> Individual {
    let len = parent1.genes.len().max(parent2.genes.len());
    let mut genes = Vec::with_capacity(len);
    for i in 0..len {
        let gene = if rng.random_bool(0.5) {
            parent1.genes.get(i).cloned().flatten()
        } else {
            parent2.genes.get(i).cloned().flatten()
        };
        genes.push(gene);
    }
    Individual {
        genes,
        fitness: f64::NEG_INFINITY,
    }
}

fn mutate<R: Rng>(individual: &mut Individual, mutation_rate: f64, rng: &mut R) {
    for gene in individual.genes.iter_mut() {
        if rng.random_bool(mutation_rate) {
            *gene = None;
        }
    }
}

pub fn rhea<R: Rng>(
    state: &GameState,
    player_index: usize,
    config: &RheaConfig,
    known_draft_hands: &Option<Vec<Vec<CardInstance>>>,
    max_rollout_round: Option<u32>,
    rng: &mut R,
) -> Choice {
    let mut choices_buf: Vec<Choice> = Vec::new();
    enumerate_choices_into(state, &mut choices_buf);
    if choices_buf.len() == 1 {
        return choices_buf.swap_remove(0);
    }

    let mut cached_scores = [0u32; MAX_PLAYERS];
    for (i, p) in state.players.iter().enumerate() {
        cached_scores[i] = calculate_score(p);
    }

    let mut det_state = state.clone();

    // Initialize population
    let mut population: Vec<Individual> = Vec::with_capacity(config.population_size as usize);
    for _ in 0..config.population_size {
        det_state.clone_from(state);
        let mut ind = generate_random_individual(
            &mut det_state, state, player_index, config.horizon_length,
            known_draft_hands, &cached_scores, max_rollout_round, &mut choices_buf, rng,
        );
        det_state.clone_from(state);
        evaluate(
            &mut ind, &mut det_state, state, player_index,
            known_draft_hands, &cached_scores, max_rollout_round,
            config.max_rollout_steps, &mut choices_buf, rng,
        );
        population.push(ind);
    }

    // Evolution loop
    for _ in 0..config.generations {
        // Sort by fitness descending
        population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let mut new_population: Vec<Individual> = Vec::with_capacity(config.population_size as usize);

        // Keep elites
        for i in 0..config.elitism_count.min(config.population_size) as usize {
            new_population.push(Individual {
                genes: population[i].genes.clone(),
                fitness: population[i].fitness,
            });
        }

        // Fill rest via tournament selection + crossover + mutation
        while new_population.len() < config.population_size as usize {
            let p1 = tournament_select(&population, config.tournament_size, rng);
            let p2 = tournament_select(&population, config.tournament_size, rng);
            let mut child = crossover(&population[p1], &population[p2], rng);
            mutate(&mut child, config.mutation_rate, rng);

            det_state.clone_from(state);
            evaluate(
                &mut child, &mut det_state, state, player_index,
                known_draft_hands, &cached_scores, max_rollout_round,
                config.max_rollout_steps, &mut choices_buf, rng,
            );
            new_population.push(child);
        }

        population = new_population;
    }

    // Return first choice of best individual
    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

    for ind in &population {
        if let Some(Some(choice)) = ind.genes.first() {
            return choice.clone();
        }
    }

    // Fallback: random choice
    enumerate_choices_into(state, &mut choices_buf);
    let idx = rng.random_range(0..choices_buf.len());
    choices_buf[idx].clone()
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

        let config = RheaConfig {
            generations: 2,
            population_size: 5,
            ..RheaConfig::default()
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
                GamePhase::Draft { .. } => {}
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let choice = rhea(&state, player_index, &config, &None, None, &mut rng);

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed}, players={num_players}, \
                 step={step}, round={}, phase={:?}: RHEA choice {choice:?} \
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
    fn test_rhea_valid_moves_2_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(2, seed);
        }
    }

    #[test]
    fn test_rhea_valid_moves_3_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(3, seed);
        }
    }

    #[test]
    fn test_rhea_valid_moves_4_players() {
        for seed in 0..5 {
            run_full_game_validating_choices(4, seed);
        }
    }
}
