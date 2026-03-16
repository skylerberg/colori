use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;

use rand::seq::SliceRandom;
use wyrand::WyRand;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::NamedVariant;

// ── Serialization types ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRunOutput {
    pub version: u32,
    pub game_started_at: String,
    pub game_ended_at: Option<String>,
    pub player_names: Vec<String>,
    pub ai_players: Vec<bool>,
    pub initial_state: GameState,
    pub final_scores: Option<Vec<FinalScore>>,
    pub final_player_stats: Option<Vec<FinalPlayerStats>>,
    pub entries: Vec<StructuredLogEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_variants: Option<Vec<PlayerVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredLogEntry {
    pub seq: u32,
    pub timestamp: u64,
    pub round: u32,
    pub phase: String,
    pub player_index: usize,
    pub choice: Choice,
}

// ── Helpers ──

pub fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn now_epoch_secs_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", secs)
}

fn format_iterations(iters: u32) -> String {
    if iters >= 1000 && iters % 1000 == 0 {
        format!("{}k", iters / 1000)
    } else {
        format!("{}", iters)
    }
}

pub fn format_variant_label(variant: &NamedVariant, differing: &DifferingFields) -> String {
    if let Some(name) = &variant.name {
        return name.clone();
    }
    let config = &variant.ai;
    let mut parts = Vec::new();
    if differing.iterations_differs {
        parts.push(format_iterations(config.iterations));
    }
    if differing.exploration_constant_differs {
        parts.push(format!("c={:.2}", config.exploration_constant));
    }
    if differing.max_rollout_steps_differs {
        parts.push(format!("rollout={}", config.max_rollout_steps));
    }
    if parts.is_empty() {
        parts.push(format_iterations(config.iterations));
    }
    parts.join(", ")
}

pub struct DifferingFields {
    pub iterations_differs: bool,
    pub exploration_constant_differs: bool,
    pub max_rollout_steps_differs: bool,
}

pub fn compute_differing_fields(variants: &[NamedVariant]) -> DifferingFields {
    if variants.len() <= 1 {
        return DifferingFields {
            iterations_differs: false,
            exploration_constant_differs: false,
            max_rollout_steps_differs: false,
        };
    }
    let first = &variants[0].ai;
    DifferingFields {
        iterations_differs: variants.iter().any(|v| v.ai.iterations != first.iterations),
        exploration_constant_differs: variants.iter().any(|v| v.ai.exploration_constant != first.exploration_constant),
        max_rollout_steps_differs: variants.iter().any(|v| v.ai.max_rollout_steps != first.max_rollout_steps),
    }
}

pub fn has_any_difference(variants: &[NamedVariant]) -> bool {
    if variants.len() <= 1 {
        return false;
    }
    if variants.iter().any(|v| v.name.is_some()) {
        return true;
    }
    let diff = compute_differing_fields(variants);
    diff.iterations_differs || diff.exploration_constant_differs || diff.max_rollout_steps_differs
}

fn is_default_heuristic_params(params: &HeuristicParams) -> bool {
    let d = HeuristicParams::default();
    let json_params = serde_json::to_string(params).unwrap_or_default();
    let json_default = serde_json::to_string(&d).unwrap_or_default();
    json_params == json_default
}

fn variant_to_player_variant(variant: &NamedVariant) -> PlayerVariant {
    let defaults = MctsConfig::default();
    let config = &variant.ai;
    PlayerVariant {
        name: variant.name.clone(),
        algorithm: Some("ucb".to_string()),
        iterations: config.iterations,
        exploration_constant: if config.exploration_constant != defaults.exploration_constant {
            Some(config.exploration_constant)
        } else {
            None
        },
        max_rollout_steps: if config.max_rollout_steps != defaults.max_rollout_steps {
            Some(config.max_rollout_steps)
        } else {
            None
        },
        heuristic_params: if !is_default_heuristic_params(&config.heuristic_params) {
            Some(config.heuristic_params.clone())
        } else {
            None
        },
    }
}

// ── Game loop ──

pub fn run_game(
    _game_index: usize,
    player_variants: &[NamedVariant],
    note: Option<String>,
    glass: bool,
    rng: &mut WyRand,
) -> GameRunOutput {
    let start = std::time::Instant::now();
    let num_players = player_variants.len();

    // Shuffle variant assignment to eliminate position bias
    let mut shuffled_variants = player_variants.to_vec();
    shuffled_variants.shuffle(rng);

    let has_variants = has_any_difference(&shuffled_variants);
    let differing = compute_differing_fields(&shuffled_variants);
    let names: Vec<String> = (1..=num_players)
        .map(|i| {
            if has_variants {
                format!("Player {} ({})", i, format_variant_label(&shuffled_variants[i - 1], &differing))
            } else {
                format!("Player {}", i)
            }
        })
        .collect();

    let ai_players = vec![true; num_players];
    let expansions = Expansions { glass };
    let mut state = create_initial_game_state_with_expansions(num_players, &ai_players, expansions, rng);
    let initial_state = state.clone();

    let game_started_at = now_epoch_secs_string();

    // Start first round (draw phase -> draft phase)
    execute_draw_phase(&mut state, rng);

    let mut entries: Vec<StructuredLogEntry> = Vec::new();
    let mut seq: u32 = 0;

    // Main game loop
    while !matches!(state.phase, GamePhase::GameOver) {
        let (player_index, phase_str) = match &state.phase {
            GamePhase::Draft { draft_state } => {
                (draft_state.current_player_index, "draft")
            }
            GamePhase::Action { action_state } => {
                (action_state.current_player_index, "action")
            }
            GamePhase::Draw => {
                break;
            }
            GamePhase::GameOver => break,
        };

        let config = &shuffled_variants[player_index].ai;
        let max_rollout_round = std::cmp::max(8, state.round + 2);
        let choice = ismcts(&state, player_index, config, &None, Some(max_rollout_round), rng);

        seq += 1;
        entries.push(StructuredLogEntry {
            seq,
            timestamp: now_epoch_millis(),
            round: state.round,
            phase: phase_str.to_string(),
            player_index,
            choice: choice.clone(),
        });

        apply_choice_to_state(&mut state, &choice, rng);
    }

    let game_ended_at = Some(now_epoch_secs_string());

    // Compute final scores
    let final_scores: Option<Vec<FinalScore>> = Some(
        state
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| FinalScore {
                name: names[i].clone(),
                score: calculate_score(p),
                completed_sell_cards: p.completed_sell_cards.len() as u32,
                color_wheel_total: p.color_wheel.counts.iter().sum(),
            })
            .collect(),
    );

    // Compute final player stats
    let final_player_stats: Option<Vec<FinalPlayerStats>> = Some(
        state
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| FinalPlayerStats {
                name: names[i].clone(),
                deck_size: (p.deck.len() + p.discard.len() + p.workshop_cards.len() + p.workshopped_cards.len()) as usize,
                completed_sell_cards: p.completed_sell_cards.to_vec(),
                ducats: p.ducats,
                color_wheel: p.color_wheel.clone(),
                materials: p.materials.clone(),
            })
            .collect(),
    );

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    let (log_iterations, log_player_variants) = if has_variants {
        (
            None,
            Some(
                shuffled_variants
                    .iter()
                    .map(|v| variant_to_player_variant(v))
                    .collect(),
            ),
        )
    } else {
        (Some(shuffled_variants[0].ai.iterations), None)
    };

    GameRunOutput {
        version: 1,
        game_started_at,
        game_ended_at,
        player_names: names,
        ai_players,
        initial_state,
        final_scores,
        final_player_stats,
        entries,
        duration_ms,
        iterations: log_iterations,
        player_variants: log_player_variants,
        note,
    }
}
