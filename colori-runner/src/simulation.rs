use colori_core::colori_game::{apply_choice_to_state, enumerate_choices};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::game_log::{DrawEvent, DrawLog, FinalPlayerStats, FinalScore, PlayerVariant};
use colori_core::ismcts::{ismcts, MctsConfig, MctsNode};
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;
use colori_core::unordered_cards::{set_sell_card_registry, set_card_registry};

use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::RngExt;
use rand::SeedableRng;
use wyrand::WyRand;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{NamedVariant, SimulateArgs, load_variants_from_file, parse_inline_variants};
use crate::generate_batch_id;

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_draws: Vec<DrawEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub player_time_ms: Vec<u64>,
    pub player_iterations: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_variants: Option<Vec<PlayerVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip)]
    pub variant_order: Vec<usize>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub draws: Vec<DrawEvent>,
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

fn format_time_limit(ms: u64) -> String {
    if ms >= 1000 && ms % 1000 == 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}ms", ms)
    }
}

pub fn format_variant_label(variant: &NamedVariant, differing: &DifferingFields) -> String {
    if let Some(name) = &variant.name {
        return name.clone();
    }
    let config = &variant.ai;
    let mut parts = Vec::new();
    if differing.time_limit_differs {
        if let Some(tl) = config.time_limit_ms {
            parts.push(format_time_limit(tl));
        }
    }
    if differing.iterations_differs && config.time_limit_ms.is_none() {
        parts.push(format_iterations(config.iterations));
    }
    if differing.exploration_constant_differs {
        parts.push(format!("c={:.2}", config.exploration_constant));
    }
    if differing.max_rollout_steps_differs {
        parts.push(format!("rollout={}", config.max_rollout_steps));
    }
    if parts.is_empty() {
        if let Some(tl) = config.time_limit_ms {
            parts.push(format_time_limit(tl));
        } else {
            parts.push(format_iterations(config.iterations));
        }
    }
    parts.join(", ")
}

pub struct DifferingFields {
    pub iterations_differs: bool,
    pub exploration_constant_differs: bool,
    pub max_rollout_steps_differs: bool,
    pub time_limit_differs: bool,
}

pub fn compute_differing_fields(variants: &[NamedVariant]) -> DifferingFields {
    if variants.len() <= 1 {
        return DifferingFields {
            iterations_differs: false,
            exploration_constant_differs: false,
            max_rollout_steps_differs: false,
            time_limit_differs: false,
        };
    }
    let first = &variants[0].ai;
    DifferingFields {
        iterations_differs: variants.iter().any(|v| v.ai.iterations != first.iterations),
        exploration_constant_differs: variants.iter().any(|v| v.ai.exploration_constant != first.exploration_constant),
        max_rollout_steps_differs: variants.iter().any(|v| v.ai.max_rollout_steps != first.max_rollout_steps),
        time_limit_differs: variants.iter().any(|v| v.ai.time_limit_ms != first.time_limit_ms),
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
    diff.iterations_differs || diff.exploration_constant_differs || diff.max_rollout_steps_differs || diff.time_limit_differs
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
        time_limit_ms: config.time_limit_ms,
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
        random_first_pick: if config.random_first_pick { Some(true) } else { None },
        first_pick_params: config.first_pick_params.as_ref().map(|p| (**p).clone()),
    }
}

// ── Game loop ──

pub fn run_game(
    _game_index: usize,
    player_variants: &[NamedVariant],
    note: Option<String>,
    max_rounds: Option<u32>,
    rng: &mut WyRand,
) -> GameRunOutput {
    let start = std::time::Instant::now();
    let num_players = player_variants.len();

    // Shuffle variant assignment to eliminate position bias
    let mut variant_order: Vec<usize> = (0..num_players).collect();
    variant_order.shuffle(rng);
    let shuffled_variants: Vec<NamedVariant> = variant_order.iter().map(|&i| player_variants[i].clone()).collect();

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
    let mut state = create_initial_game_state(num_players, &ai_players, rng);
    if let Some(mr) = max_rounds {
        state.max_rounds = mr;
    }
    let initial_state = state.clone();

    let game_started_at = now_epoch_secs_string();

    // Start first round (draw phase -> draft phase)
    state.draw_log = Some(DrawLog::Recording(Vec::new()));
    execute_draw_phase(&mut state, rng);
    let initial_draws = match state.draw_log.take() {
        Some(DrawLog::Recording(events)) => events,
        _ => Vec::new(),
    };

    let mut entries: Vec<StructuredLogEntry> = Vec::new();
    let mut seq: u32 = 0;
    let mut reuse_tree: Option<MctsNode> = None;
    let mut player_time = vec![std::time::Duration::ZERO; num_players];
    let mut player_iterations_count = vec![0u64; num_players];

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

        // Check if this is the first draft pick of round 1
        let is_first_pick = state.round == 1
            && matches!(&state.phase, GamePhase::Draft { draft_state } if draft_state.pick_number == 0);

        let (choice, mcts_tree): (Choice, Option<MctsNode>) = if is_first_pick && config.first_pick_params.is_some() {
            let fpp = config.first_pick_params.as_ref().unwrap();
            let choices = enumerate_choices(&state);
            let best = choices.iter()
                .max_by(|a, b| {
                    let score_a = match a { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    let score_b = match b { Choice::DraftPick { card } => fpp.score_card(*card, &state.sell_card_display), _ => f64::NEG_INFINITY };
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("No choices available")
                .clone();
            (best, None)
        } else if is_first_pick && config.random_first_pick {
            let choices = enumerate_choices(&state);
            (choices.choose(rng).expect("No choices available").clone(), None)
        } else {
            let max_rollout_round = std::cmp::max(8, state.round + 2);
            let mcts_start = std::time::Instant::now();
            let result = ismcts(&state, player_index, config, Some(max_rollout_round), reuse_tree.take(), rng);
            player_time[player_index] += mcts_start.elapsed();
            player_iterations_count[player_index] += result.iterations_used as u64;
            (result.choice.clone(), result.tree)
        };

        seq += 1;

        // Snapshot state for information revelation check
        let prev_workshop_len = state.players[player_index].workshop_cards.len();
        let prev_sell_deck_len = state.sell_card_deck.len();

        // Capture round before applying choice (end_round may increment it)
        let round = state.round;

        // Enable draw recording before applying the choice
        state.draw_log = Some(DrawLog::Recording(Vec::new()));
        apply_choice_to_state(&mut state, &choice, rng);
        let draws = match state.draw_log.take() {
            Some(DrawLog::Recording(events)) => events,
            _ => Vec::new(),
        };

        entries.push(StructuredLogEntry {
            seq,
            timestamp: now_epoch_millis(),
            round,
            phase: phase_str.to_string(),
            player_index,
            choice: choice.clone(),
            draws,
        });

        // Reuse subtree only if same player is still active and no information was revealed
        let same_player_action = matches!(&state.phase, GamePhase::Action { action_state }
            if action_state.current_player_index == player_index);
        let info_revealed =
            state.players[player_index].workshop_cards.len() != prev_workshop_len ||
            state.sell_card_deck.len() != prev_sell_deck_len;
        reuse_tree = if same_player_action && !info_revealed {
            mcts_tree.and_then(|t| t.into_subtree(&choice))
        } else {
            None
        };
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
    let player_time_ms: Vec<u64> = player_time.iter().map(|d| d.as_millis() as u64).collect();

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
        initial_draws,
        duration_ms,
        player_time_ms,
        player_iterations: player_iterations_count,
        iterations: log_iterations,
        player_variants: log_player_variants,
        note,
        variant_order,
    }
}

pub fn run_simulation(args: &SimulateArgs, threads: usize, output: &str) {
    let player_variants = if let Some(ref v) = args.variants {
        parse_inline_variants(v)
    } else {
        load_variants_from_file(&args.variants_file)
    };
    let num_players = player_variants.len();
    let solo = num_players == 1;
    let max_rounds = if solo { Some(args.max_rounds) } else { None };

    if solo {
        eprintln!(
            "Running {} solo games ({} rounds, {} MCTS iterations, {} threads)",
            args.games, args.max_rounds, player_variants[0].ai.iterations, threads
        );
    } else if has_any_difference(&player_variants) {
        let differing = compute_differing_fields(&player_variants);
        let labels: Vec<String> = player_variants.iter().map(|v| format_variant_label(v, &differing)).collect();
        eprintln!(
            "Running {} games with variants: {}, {} threads",
            args.games,
            labels.join(", "),
            threads
        );
    } else if let Some(tl) = player_variants[0].ai.time_limit_ms {
        eprintln!(
            "Running {} games with {} players, {}ms MCTS time limit, {} threads",
            args.games, num_players, tl, threads
        );
    } else {
        eprintln!(
            "Running {} games with {} players, {} ISMCTS iterations, {} threads",
            args.games, num_players,
            player_variants[0].ai.iterations,
            threads
        );
    }

    std::fs::create_dir_all(output).expect("Failed to create output directory");

    let batch_id = generate_batch_id();
    let completed = AtomicUsize::new(0);
    let variant_time_ms: Vec<AtomicU64> = (0..num_players).map(|_| AtomicU64::new(0)).collect();
    let variant_iterations: Vec<AtomicU64> = (0..num_players).map(|_| AtomicU64::new(0)).collect();
    let solo_wins = AtomicUsize::new(0);
    let solo_total_score = AtomicU64::new(0);
    let total_games = args.games;
    let num_threads = threads;
    let batch_id = batch_id.as_str();
    let note = &args.note;
    let player_variants = player_variants.as_slice();

    std::thread::scope(|s| {
        let games_per_thread = total_games / num_threads;
        let remainder = total_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let completed = &completed;
            let variant_time_ms = &variant_time_ms;
            let variant_iterations = &variant_iterations;
            let solo_wins = &solo_wins;
            let solo_total_score = &solo_total_score;

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                for _i in 0..count {
                    let log = run_game(
                        0,
                        player_variants,
                        note.clone(),
                        max_rounds,
                        &mut rng,
                    );
                    for (player_pos, &orig_idx) in log.variant_order.iter().enumerate() {
                        variant_time_ms[orig_idx].fetch_add(log.player_time_ms[player_pos], Ordering::Relaxed);
                        variant_iterations[orig_idx].fetch_add(log.player_iterations[player_pos], Ordering::Relaxed);
                    }
                    if solo {
                        let score = log.final_scores.as_ref()
                            .and_then(|fs| fs.first())
                            .map(|fs| fs.score)
                            .unwrap_or(0);
                        if score >= 16 {
                            solo_wins.fetch_add(1, Ordering::Relaxed);
                        }
                        solo_total_score.fetch_add(score as u64, Ordering::Relaxed);
                    }
                    set_card_registry(&log.initial_state.card_lookup);
                    set_sell_card_registry(&log.initial_state.sell_card_lookup);
                    let epoch_millis = now_epoch_millis();
                    let game_id: String = {
                        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                        (0..4)
                            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
                            .collect()
                    };
                    let path = format!("{}/game-{}-{}-{}.json", output, epoch_millis, batch_id, game_id);
                    let json = serde_json::to_string_pretty(&log).unwrap();
                    std::fs::write(&path, json).unwrap();
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if solo {
                        if done % 100 == 0 || done == total_games {
                            let w = solo_wins.load(Ordering::Relaxed);
                            eprintln!(
                                "Game {}/{} — win rate: {:.1}%",
                                done, total_games, w as f64 / done as f64 * 100.0
                            );
                        }
                    } else {
                        eprintln!("Game {}/{} complete", done, total_games);
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    if solo {
        let total_wins = solo_wins.load(Ordering::Relaxed);
        let total_score = solo_total_score.load(Ordering::Relaxed);
        let avg_ducats = total_score as f64 / total_games as f64;
        let win_rate = total_wins as f64 / total_games as f64 * 100.0;
        eprintln!();
        eprintln!("=== Solo Results ({} rounds) ===", args.max_rounds);
        eprintln!("Games:      {}", total_games);
        eprintln!("Wins:       {} ({:.1}%)", total_wins, win_rate);
        eprintln!("Avg ducats: {:.1}", avg_ducats);
    } else if has_any_difference(player_variants) {
        let differing = compute_differing_fields(player_variants);
        for (i, v) in player_variants.iter().enumerate() {
            let total_ms = variant_time_ms[i].load(Ordering::Relaxed);
            let total_iters = variant_iterations[i].load(Ordering::Relaxed);
            let avg_secs = total_ms as f64 / total_games as f64 / 1000.0;
            let avg_iters = total_iters as f64 / total_games as f64;
            eprintln!("{}: {:.1}s avg, {:.0} avg iters per game", format_variant_label(v, &differing), avg_secs, avg_iters);
        }
    } else {
        let total_ms: u64 = variant_time_ms.iter().map(|a| a.load(Ordering::Relaxed)).sum();
        let total_iters: u64 = variant_iterations.iter().map(|a| a.load(Ordering::Relaxed)).sum();
        let avg_secs = total_ms as f64 / (total_games as f64 * num_players as f64) / 1000.0;
        let avg_iters = total_iters as f64 / (total_games as f64 * num_players as f64);
        eprintln!("Average per player per game: {:.1}s, {:.0} iters", avg_secs, avg_iters);
    }
    eprintln!("All {} games written to {}/", total_games, output);
}
