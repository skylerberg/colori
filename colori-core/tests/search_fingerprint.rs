//! A golden fingerprint of search behaviour at fixed positions and seeds.
//!
//! `BASELINE.md` proposed this and it did not get built. The occasion for
//! building it is a breaking change to the `mcts` crate that claims sequential
//! search behaviour is unchanged: a fingerprint captured *before* that change is
//! an oracle for the claim, and one captured after proves nothing. Its value is
//! entirely in predating what it is used to check.
//!
//! It records more than the chosen move, deliberately. A search can change
//! materially and still land on the same move at most positions — the chosen
//! move is the *last* thing to move, so it is a poor tripwire on its own. The
//! per-child visit and reward vector is what catches drift.
//!
//! To regenerate after an intentional change:
//!
//! ```text
//! COLORI_WRITE_FINGERPRINT=1 cargo test -p colori-core --test search_fingerprint
//! ```
//!
//! That is an env var and not a flag on purpose. Regenerating should be a
//! decision, never the quickest way to turn a red test green.

use std::fmt::Write as _;

use colori_core::choices::enumerate_choices;
use colori_core::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::MctsConfig;
use colori_core::mcts_impl::{ColoriSearcher, SearchNode};
use colori_core::scoring::HeuristicParams;
use colori_core::types::{GamePhase, GameState};
use rand::{RngExt, SeedableRng};
use wyrand::WyRand;

const EXPECTED: &str = include_str!("fingerprints/search.txt");
const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");

fn params() -> HeuristicParams {
    serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
}

/// Walk a fresh game forward, choosing uniformly at random from a seeded RNG.
fn position(players: usize, seed: u64, steps: usize) -> Option<GameState> {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai = vec![true; players];
    let mut state = colori_core::setup::create_initial_game_state(players, &ai, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    for _ in 0..steps {
        match &state.phase {
            GamePhase::GameOver => return None,
            GamePhase::Draw => {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            _ => {}
        }
        let choices = enumerate_choices(&state);
        if choices.is_empty() {
            return None;
        }
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice_to_state(&mut state, &choice, &mut rng);
    }

    match get_game_status(&state, None) {
        GameStatus::AwaitingAction { .. } => Some(state),
        GameStatus::Terminated { .. } => None,
    }
}

/// One row per child, sorted by choice so a benign reordering of expansion does
/// not read as a behaviour change while the values still do.
fn describe_tree(out: &mut String, root: &SearchNode) {
    let _ = writeln!(
        out,
        "  root visits={} nodes={} depth={}",
        root.visits(),
        root.node_count(),
        root.max_depth()
    );
    let mut children: Vec<String> = root
        .children()
        .iter()
        .map(|child| {
            format!(
                "  child {:?} visits={} reward={:.12e}",
                child.edge().choice().expect("child has a choice"),
                child.visits(),
                child.cumulative_reward()
            )
        })
        .collect();
    children.sort();
    for line in children {
        let _ = writeln!(out, "{line}");
    }
}

/// Search, re-root, search again — the usage the docs prescribe, and the one a
/// fresh searcher per case cannot exercise.
///
/// Every defect found in this crate so far has been state on the `Searcher`
/// outliving the call that created it: a retained tree used for a new position,
/// a re-rooted root never re-expanded, a cached legality mask surviving into the
/// next search. None of those are reachable by constructing a searcher, calling
/// `search` once and throwing it away, so a fingerprint that only does that is
/// blind to the entire class.
fn fingerprint_reuse(out: &mut String, players: usize, seed: u64, steps: usize, iterations: u32) {
    let Some(mut state) = position(players, seed, steps) else {
        let _ = writeln!(out, "reuse players={players} seed={seed}: no position");
        return;
    };

    let mut searcher = ColoriSearcher::new(&state);
    let mut rng = WyRand::seed_from_u64(0xBEE_5EED);

    for turn in 0..4 {
        let player = match get_game_status(&state, None) {
            GameStatus::AwaitingAction { player_index } => player_index,
            GameStatus::Terminated { .. } => {
                let _ = writeln!(out, "reuse players={players} seed={seed} turn={turn}: over");
                return;
            }
        };
        let horizon = Some(std::cmp::max(8, state.round + 2));
        let config = MctsConfig {
            iterations,
            early_termination: true,
            time_limit_ms: None,
            ..MctsConfig::new(params())
        };

        let outcome = searcher.search(&state, player, &config, horizon, &mut rng);
        let _ = writeln!(
            out,
            "reuse players={players} seed={seed} turn={turn} round={} actor={player} iters={iterations}",
            state.round
        );
        let _ = writeln!(
            out,
            "  chose {:?} used={} reused={} stop={:?}",
            outcome.choice, outcome.iterations_used, outcome.reused_iterations, outcome.stop_reason
        );
        describe_tree(out, searcher.tree().expect("a search leaves a tree"));

        let choice = outcome.choice.clone();
        apply_choice_to_state(&mut state, &choice, &mut rng);
        // Carry the subtree forward, which is what makes this a reuse test at
        // all. A searcher that is never re-rooted never exercises the paths
        // where the bugs have actually lived.
        let reused = searcher.reuse_subtree(&choice);
        let _ = writeln!(out, "  reuse_subtree -> {reused}");
    }
}

fn generate() -> String {
    let mut out = String::new();

    // Player count is the branching-factor axis. Budget and the two policy
    // toggles are the axes that shift how much RNG the search consumes, which
    // is where a change to draw *order* shows up — small budgets reveal it,
    // large ones wash it out.
    for players in [2usize, 3, 4] {
        for (seed, steps) in [(11u64, 12usize), (23, 50)] {
            let Some(state) = position(players, seed, steps) else {
                let _ = writeln!(out, "position players={players} seed={seed} steps={steps}: none");
                continue;
            };
            let player = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => unreachable!("filtered above"),
            };
            let horizon = Some(std::cmp::max(8, state.round + 2));

            for iterations in [200u32, 1_000] {
                for early_termination in [false, true] {
                    for progressive_bias_weight in [0.0f64, 0.5] {
                        let config = MctsConfig {
                            iterations,
                            early_termination,
                            progressive_bias_weight,
                            time_limit_ms: None,
                            ..MctsConfig::new(params())
                        };
                        let mut searcher = ColoriSearcher::new(&state);
                        let mut rng = WyRand::seed_from_u64(0xF1_1E_5E_ED);
                        let outcome =
                            searcher.search(&state, player, &config, horizon, &mut rng);

                        let _ = writeln!(
                            out,
                            "players={players} seed={seed} steps={steps} round={} actor={player} \
                             iters={iterations} early={early_termination} bias={progressive_bias_weight}",
                            state.round
                        );
                        let _ = writeln!(
                            out,
                            "  chose {:?} used={} reused={} stop={:?}",
                            outcome.choice,
                            outcome.iterations_used,
                            outcome.reused_iterations,
                            outcome.stop_reason
                        );
                        describe_tree(&mut out, searcher.tree().expect("a search leaves a tree"));
                    }
                }
            }
        }
    }
    for (players, seed, steps, iterations) in
        [(2usize, 11u64, 12usize, 400u32), (3, 23, 50, 400), (4, 11, 30, 800)]
    {
        fingerprint_reuse(&mut out, players, seed, steps, iterations);
    }

    out
}

#[test]
fn search_behaviour_matches_the_recorded_fingerprint() {
    let actual = generate();

    if std::env::var("COLORI_WRITE_FINGERPRINT").is_ok() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fingerprints/search.txt"),
            &actual,
        )
        .expect("write fingerprint");
        eprintln!("fingerprint rewritten; commit it deliberately");
        return;
    }

    if actual != EXPECTED {
        let first_difference = actual
            .lines()
            .zip(EXPECTED.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(line, (a, b))| format!("line {}:\n  now      {a}\n  recorded {b}", line + 1))
            .unwrap_or_else(|| {
                format!(
                    "no differing line; lengths differ ({} vs {} lines)",
                    actual.lines().count(),
                    EXPECTED.lines().count()
                )
            });
        panic!(
            "search behaviour changed against the recorded fingerprint.\n\n{first_difference}\n\n\
             If the change was intended, regenerate with \
             COLORI_WRITE_FINGERPRINT=1 cargo test -p colori-core --test search_fingerprint \
             and review the diff — it is the record of what the search used to do."
        );
    }
}
