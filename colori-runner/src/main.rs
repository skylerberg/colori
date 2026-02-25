use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::ismcts;
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;

use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ── CLI args ──

struct Args {
    games: usize,
    iterations: u32,
    players: usize,
    threads: usize,
    output: String,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut games = 10usize;
    let mut iterations = 100u32;
    let mut players = 3usize;
    let mut threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut output = "game-logs".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" => {
                i += 1;
                games = args[i].parse().expect("Invalid --games value");
            }
            "--iterations" => {
                i += 1;
                iterations = args[i].parse().expect("Invalid --iterations value");
            }
            "--players" => {
                i += 1;
                players = args[i].parse().expect("Invalid --players value");
            }
            "--threads" => {
                i += 1;
                threads = args[i].parse().expect("Invalid --threads value");
            }
            "--output" => {
                i += 1;
                output = args[i].clone();
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    Args {
        games,
        iterations,
        players,
        threads,
        output,
    }
}

// ── Serialization types ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StructuredGameLog {
    version: u32,
    game_started_at: String,
    game_ended_at: Option<String>,
    player_names: Vec<String>,
    ai_players: Vec<bool>,
    initial_state: GameState,
    final_scores: Option<Vec<FinalScore>>,
    final_player_stats: Option<Vec<FinalPlayerStats>>,
    entries: Vec<StructuredLogEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalScore {
    name: String,
    score: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalPlayerStats {
    name: String,
    deck_size: usize,
    completed_buyers: Vec<BuyerInstance>,
    ducats: u32,
    color_wheel: ColorWheel,
    materials: Materials,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StructuredLogEntry {
    seq: u32,
    timestamp: u64,
    round: u32,
    phase: String,
    player_index: usize,
    choice: ColoriChoice,
}

// ── Helpers ──

fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", secs)
}

// ── Game loop ──

fn run_game(
    _game_index: usize,
    num_players: usize,
    iterations: u32,
    rng: &mut SmallRng,
) -> StructuredGameLog {
    let names: Vec<String> = (1..=num_players)
        .map(|i| format!("Player {}", i))
        .collect();

    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(&names, &ai_players, rng);
    let initial_state = state.clone();

    let game_started_at = now_iso();

    // Start first round (draw phase -> draft phase)
    execute_draw_phase(&mut state, rng);

    let mut entries: Vec<StructuredLogEntry> = Vec::new();
    let mut seq: u32 = 0;

    // Main game loop
    while !matches!(state.phase, GamePhase::GameOver) {
        let (player_index, phase_str) = match &state.phase {
            GamePhase::Draft { draft_state } => {
                if draft_state.waiting_for_pass {
                    // This shouldn't happen since apply_choice_to_state handles it,
                    // but guard against it
                    break;
                }
                (draft_state.current_player_index, "draft")
            }
            GamePhase::Action { action_state } => {
                (action_state.current_player_index, "action")
            }
            GamePhase::Draw => {
                // Draw phase is handled internally by apply_choice_to_state on EndTurn
                break;
            }
            GamePhase::GameOver => break,
        };

        let max_round = std::cmp::max(8, state.round + 2);
        let choice = ismcts(&state, player_index, iterations, &None, Some(max_round), rng);

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

    let game_ended_at = Some(now_iso());

    // Compute final scores
    let final_scores: Option<Vec<FinalScore>> = Some(
        state
            .players
            .iter()
            .map(|p| FinalScore {
                name: p.name.clone(),
                score: calculate_score(p),
            })
            .collect(),
    );

    // Compute final player stats
    let final_player_stats: Option<Vec<FinalPlayerStats>> = Some(
        state
            .players
            .iter()
            .map(|p| FinalPlayerStats {
                name: p.name.clone(),
                deck_size: p.deck.len() + p.discard.len() + p.workshop_cards.len(),
                completed_buyers: p.completed_buyers.clone(),
                ducats: p.ducats,
                color_wheel: p.color_wheel.clone(),
                materials: p.materials.clone(),
            })
            .collect(),
    );

    StructuredGameLog {
        version: 1,
        game_started_at,
        game_ended_at,
        player_names: names,
        ai_players,
        initial_state,
        final_scores,
        final_player_stats,
        entries,
    }
}

// ── Main ──

fn generate_batch_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = SmallRng::from_os_rng();
    (0..6)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

fn main() {
    let args = parse_args();

    eprintln!(
        "Running {} games with {} players, {} ISMCTS iterations, {} threads",
        args.games, args.players, args.iterations, args.threads
    );

    std::fs::create_dir_all(&args.output).expect("Failed to create output directory");

    let batch_id = generate_batch_id();
    let completed = AtomicUsize::new(0);
    let total_games = args.games;
    let num_players = args.players;
    let iterations = args.iterations;
    let num_threads = args.threads;
    let output_dir = &args.output;
    let batch_id = batch_id.as_str();

    std::thread::scope(|s| {
        let games_per_thread = total_games / num_threads;
        let remainder = total_games % num_threads;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let completed = &completed;

            handles.push(s.spawn(move || {
                let mut rng = SmallRng::from_os_rng();
                for _i in 0..count {
                    let log = run_game(0, num_players, iterations, &mut rng);
                    let epoch_millis = now_epoch_millis();
                    let path = format!("{}/game-{}-{}.json", output_dir, epoch_millis, batch_id);
                    let json = serde_json::to_string_pretty(&log).unwrap();
                    std::fs::write(&path, json).unwrap();
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    eprintln!("Game {}/{} complete", done, total_games);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    eprintln!("All {} games written to {}/", total_games, args.output);
}
