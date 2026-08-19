//! Search throughput at fixed positions.
//!
//! A tournament measures strength but is far too slow to gate a change on, and
//! per-game timing mixes search cost with position difficulty. This times one
//! search from a reproducible position.
//!
//! Positions are a pure function of `(seed, steps)`: the game is created from a
//! seeded RNG and walked forward by picking uniformly among legal choices, so
//! nothing needs to be checked in and a clean checkout reproduces them.
//!
//! `early_termination` is off in every timed configuration. With it on,
//! `iterations_used` varies per run and wall-clock is no longer comparable
//! across implementations — the feature gets its own bench instead.

use colori_core::choices::enumerate_choices;
use colori_core::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::{ismcts, MctsConfig};
use colori_core::mcts_impl::ColoriSearcher;
use colori_core::scoring::HeuristicParams;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{GamePhase, GameState};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{RngExt, SeedableRng};
use wyrand::WyRand;

const PLAYERS: usize = 3;
const SEED: u64 = 20_260_819;

/// The same generation the tests pin, so an ongoing GA run cannot move the bar
/// underneath a measurement. It is already a tracked, immutable snapshot, so it
/// is referenced directly rather than copied into a fixtures directory.
fn frozen_params() -> HeuristicParams {
    const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
    serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
}

fn config(iterations: u32) -> MctsConfig {
    MctsConfig {
        iterations,
        // See the module comment: this must stay off for timings to compare.
        early_termination: false,
        time_limit_ms: None,
        ..MctsConfig::new(frozen_params())
    }
}

/// Walk a fresh game forward by `steps` decisions, choosing uniformly at random.
fn position(seed: u64, steps: usize) -> (GameState, usize) {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; PLAYERS];
    let mut state = create_initial_game_state(PLAYERS, &ai_players, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    for taken in 0..steps {
        match &state.phase {
            GamePhase::GameOver => return (state, taken),
            GamePhase::Draw => {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            _ => {}
        }
        if matches!(get_game_status(&state, None), GameStatus::Terminated { .. }) {
            return (state, taken);
        }
        let choices = enumerate_choices(&state);
        if choices.is_empty() {
            return (state, taken);
        }
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice_to_state(&mut state, &choice, &mut rng);
    }
    (state, steps)
}

fn active_player(state: &GameState) -> usize {
    match get_game_status(state, None) {
        GameStatus::AwaitingAction { player_index } => player_index,
        GameStatus::Terminated { .. } => panic!("bench position is terminal"),
    }
}

/// Matches what the runner passes in production.
fn max_rollout_round(state: &GameState) -> Option<u32> {
    Some(std::cmp::max(8, state.round + 2))
}

fn bench_search(c: &mut Criterion) {
    // Spread across rounds and branching factors, so a change that helps one
    // phase and hurts another cannot hide behind a single number.
    let positions = [("early", 12usize), ("middle", 50), ("late", 110)];

    let mut group = c.benchmark_group("ismcts");
    group.sample_size(10);

    for (name, steps) in positions {
        let (state, reached) = position(SEED, steps);
        if reached < steps {
            eprintln!("{name}: game ended after {reached} steps; skipping");
            continue;
        }
        let player = active_player(&state);
        let horizon = max_rollout_round(&state);
        let branching = enumerate_choices(&state).len();
        // One legal choice takes the fast path and returns without searching,
        // so such a position would time nothing at all.
        assert!(branching >= 2, "{name}: branching {branching} is a forced move");
        eprintln!("{name}: round={} player={player} branching={branching}", state.round);

        for iterations in [1_000u32, 10_000] {
            group.throughput(Throughput::Elements(iterations as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("legacy/{name}"), iterations),
                &iterations,
                |b, &iterations| {
                    let config = config(iterations);
                    b.iter(|| {
                        let mut rng = WyRand::seed_from_u64(0xC0_10_71);
                        ismcts(&state, player, &config, horizon, None, &mut rng)
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::new(format!("crate/{name}"), iterations),
                &iterations,
                |b, &iterations| {
                    let config = config(iterations);
                    b.iter(|| {
                        let mut rng = WyRand::seed_from_u64(0xC0_10_71);
                        let mut searcher = ColoriSearcher::new(&state);
                        searcher.search(&state, player, &config, horizon, &mut rng)
                    });
                },
            );
        }
    }
    group.finish();
}

/// Early termination changes how many iterations actually run, so it is
/// measured on its own rather than left on during the throughput benches.
fn bench_early_termination(c: &mut Criterion) {
    let (state, _) = position(SEED, 90);
    let player = active_player(&state);
    let horizon = max_rollout_round(&state);

    let mut group = c.benchmark_group("early_termination");
    group.sample_size(10);

    for (name, early) in [("off", false), ("on", true)] {
        group.bench_function(name, |b| {
            let config = MctsConfig {
                early_termination: early,
                ..config(10_000)
            };
            b.iter(|| {
                let mut rng = WyRand::seed_from_u64(0xC0_10_71);
                ismcts(&state, player, &config, horizon, None, &mut rng)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_search, bench_early_termination);
criterion_main!(benches);
