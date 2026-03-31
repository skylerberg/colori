use colori_core::colori_game::apply_choice_to_state;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig, MctsNode};
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state_with_expansions;
use colori_core::types::*;

use rand::SeedableRng;
use wyrand::WyRand;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};

use crate::cli::{SoloArgs, load_variants_from_file};

fn run_solo_game(
    config: &MctsConfig,
    max_rounds: u32,
    glass: bool,
    rng: &mut WyRand,
) -> (bool, u32, u32) {
    let ai_players = vec![true];
    let expansions = Expansions { glass };
    let mut state = create_initial_game_state_with_expansions(1, &ai_players, expansions, rng);
    state.max_rounds = max_rounds;

    execute_draw_phase(&mut state, rng);

    let mut reuse_tree: Option<MctsNode> = None;

    while !matches!(state.phase, GamePhase::GameOver) {
        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => {
                break;
            }
            GamePhase::GameOver => break,
        };

        let max_rollout_round = std::cmp::min(
            max_rounds,
            std::cmp::max(state.round + 2, 4),
        );
        let mcts_start = std::time::Instant::now();
        let result = ismcts(&state, player_index, config, Some(max_rollout_round), reuse_tree.take(), rng);
        let _ = mcts_start.elapsed();
        let choice = result.choice.clone();
        let mcts_tree = result.tree;

        let prev_workshop_len = state.players[player_index].workshop_cards.len();
        let prev_sell_deck_len = state.sell_card_deck.len();
        let prev_glass_deck_len = state.glass_deck.len();

        apply_choice_to_state(&mut state, &choice, rng);

        let same_player_action = matches!(&state.phase, GamePhase::Action { action_state }
            if action_state.current_player_index == player_index);
        let info_revealed =
            state.players[player_index].workshop_cards.len() != prev_workshop_len
                || state.sell_card_deck.len() != prev_sell_deck_len
                || state.glass_deck.len() != prev_glass_deck_len;
        reuse_tree = if same_player_action && !info_revealed {
            mcts_tree.and_then(|t| t.into_subtree(&choice))
        } else {
            None
        };
    }

    let score = calculate_score(&state.players[0]);
    let final_round = state.round - 1;
    (score >= 16, score, final_round)
}

pub fn run_solo(args: &SoloArgs, threads: usize, glass: bool) {
    let config = if let Some(ref path) = args.variant_file {
        let variants = load_variants_from_file(path);
        variants.into_iter().next().expect("Variant file is empty").ai
    } else {
        MctsConfig {
            iterations: args.iterations,
            ..MctsConfig::default()
        }
    };

    eprintln!(
        "Running {} solo games ({} rounds, {} MCTS iterations, {} threads)",
        args.games, args.max_rounds, config.iterations, threads
    );

    let wins = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let total_ducats = AtomicU64::new(0);
    let total_games = args.games;
    let max_rounds = args.max_rounds;

    std::thread::scope(|s| {
        let games_per_thread = total_games / threads;
        let remainder = total_games % threads;
        let mut handles = Vec::new();

        for t in 0..threads {
            let count = games_per_thread + if t < remainder { 1 } else { 0 };
            let wins = &wins;
            let completed = &completed;
            let total_ducats = &total_ducats;
            let config = &config;

            handles.push(s.spawn(move || {
                let mut rng = WyRand::from_rng(&mut rand::rng());

                for _ in 0..count {
                    let (won, ducats, _final_round) = run_solo_game(config, max_rounds, glass, &mut rng);
                    if won {
                        wins.fetch_add(1, Ordering::Relaxed);
                    }
                    total_ducats.fetch_add(ducats as u64, Ordering::Relaxed);
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if done % 100 == 0 || done == total_games {
                        let w = wins.load(Ordering::Relaxed);
                        eprintln!(
                            "Game {}/{} — win rate: {:.1}%",
                            done,
                            total_games,
                            w as f64 / done as f64 * 100.0
                        );
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    let total_wins = wins.load(Ordering::Relaxed);
    let total_d = total_ducats.load(Ordering::Relaxed);
    let avg_ducats = total_d as f64 / total_games as f64;
    let win_rate = total_wins as f64 / total_games as f64 * 100.0;

    eprintln!();
    eprintln!("=== Solo Results ({} rounds) ===", max_rounds);
    eprintln!("Games:      {}", total_games);
    eprintln!("Wins:       {} ({:.1}%)", total_wins, win_rate);
    eprintln!("Avg ducats: {:.1}", avg_ducats);
}
