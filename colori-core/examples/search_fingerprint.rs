//! A fingerprint of search behaviour at fixed positions and seeds, for checking
//! a change that claims to leave the search alone:
//!
//! ```sh
//! cargo run --release -p colori-core --example search_fingerprint > before.txt
//! # make the change, or run the same command in a checkout that has it
//! cargo run --release -p colori-core --example search_fingerprint > after.txt
//! diff before.txt after.txt
//! ```
//!
//! The baseline has to come from the code before the change; one taken after
//! it proves nothing.
//!
//! It records more than the chosen move, deliberately. A search can change
//! materially and still land on the same move at most positions — the chosen
//! move is the *last* thing to move, so it is a poor tripwire on its own. The
//! per-child visit and reward vector is what catches drift.
//!
//! It prints the tree search, then the transposition graph over the same
//! positions.

use std::fmt::Write as _;

use colori_core::choices::enumerate_choices;
use colori_core::apply_choice::apply_choice;
use colori_core::colori_game::{get_game_status, GameStatus};
use colori_core::ismcts::MctsConfig;
use colori_core::mcts_impl::{ColoriSearcher, SearchGraph, SearchNode, StopReason};
use colori_core::scoring::HeuristicParams;
use colori_core::types::{GamePhase, GameState};
use mcts::StopReason as SearchStop;
use rand::{RngExt, SeedableRng};
use wyrand::WyRand;

const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");

fn params() -> HeuristicParams {
    serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
}

/// Walk a fresh game forward, choosing uniformly at random from a seeded RNG.
fn position(players: usize, seed: u64, steps: usize) -> Option<GameState> {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai = vec![true; players];
    let mut state = colori_core::setup::create_initial_game_state(players, &ai, &mut rng);

    for _ in 0..steps {
        match &state.phase {
            GamePhase::GameOver => return None,
            _ => {}
        }
        let player = match get_game_status(&state, None) {
            GameStatus::AwaitingAction { player_index } => player_index,
            GameStatus::Terminated { .. } => return None,
        };
        let choices = enumerate_choices(&state, player);
        if choices.is_empty() {
            return None;
        }
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice(&mut state, player, &choice, &mut rng);
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

/// One row per root link, sorted by choice: the link's own visits, and its
/// successor's visits and reward over every route into it — which is where a
/// merged position differs from a tree child.
fn describe_graph(out: &mut String, graph: &SearchGraph) {
    let root = graph.root();
    let _ = writeln!(
        out,
        "  root visits={} nodes={} edges={} depth={}",
        root.visits(),
        graph.node_count(),
        graph.edge_count(),
        graph.max_depth()
    );
    let mut links: Vec<String> = root
        .edges()
        .map(|edge| {
            let (visits, reward) = edge
                .target()
                .map_or((0, 0.0), |t| (t.visits(), t.cumulative_reward()));
            format!(
                "  link {:?} visits={} position_visits={visits} reward={reward:.12e}",
                edge.choice(),
                edge.visits(),
            )
        })
        .collect();
    links.sort();
    for line in links {
        let _ = writeln!(out, "{line}");
    }
}

/// The search's own reason for stopping. No position here is far enough on to
/// be the game's last turn, which is solved rather than searched.
fn searched(stop_reason: StopReason) -> SearchStop {
    match stop_reason {
        StopReason::Search(reason) => reason,
        StopReason::Solved => panic!("a fingerprint position is the game's last turn"),
    }
}

/// Write the tree of a tree search, or the graph of a transposition search.
fn describe(out: &mut String, searcher: &ColoriSearcher, stop_reason: SearchStop) {
    match (searcher.tree(), searcher.graph()) {
        (Some(tree), _) => describe_tree(out, tree),
        (None, Some(graph)) => describe_graph(out, graph),
        // A position with one legal choice returns before a root is built, so
        // there is no tree to describe; the `chose ... stop=SingleChoice` line
        // above is all that position contributes.
        (None, None) => {
            assert_eq!(
                stop_reason,
                SearchStop::SingleChoice,
                "a search that ran leaves a tree or a graph"
            );
            let _ = writeln!(out, "  no search: one legal choice");
        }
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
fn fingerprint_reuse(
    out: &mut String,
    players: usize,
    seed: u64,
    steps: usize,
    iterations: u32,
    transpositions: bool,
) {
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
            transpositions,
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
            outcome.choice,
            outcome.iterations_used,
            outcome.reused_iterations,
            searched(outcome.stop_reason)
        );
        describe(out, &searcher, searched(outcome.stop_reason));

        let choice = outcome.choice.clone();
        apply_choice(&mut state, player, &choice, &mut rng);
        // Carry the subtree forward, which is what makes this a reuse test at
        // all. A searcher that is never re-rooted never exercises the paths
        // where the bugs have actually lived. The tree is the searching
        // player's, so it carries over only to their next decision.
        let same_player = matches!(
            get_game_status(&state, None),
            GameStatus::AwaitingAction { player_index } if player_index == player
        );
        if same_player {
            let reused = searcher.reuse_subtree(&choice);
            let _ = writeln!(out, "  reuse_subtree -> {reused}");
        } else {
            searcher.clear_tree();
            let _ = writeln!(out, "  clear_tree: player {player} is not next");
        }
    }
}

fn generate(transpositions: bool) -> String {
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
                            transpositions,
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
                            searched(outcome.stop_reason)
                        );
                        describe(&mut out, &searcher, searched(outcome.stop_reason));
                    }
                }
            }
        }
    }
    for (players, seed, steps, iterations) in
        [(2usize, 11u64, 12usize, 400u32), (3, 23, 50, 400), (4, 11, 30, 800)]
    {
        fingerprint_reuse(&mut out, players, seed, steps, iterations, transpositions);
    }

    out
}

fn main() {
    println!("== tree search ==");
    print!("{}", generate(false));
    println!("== graph search ==");
    print!("{}", generate(true));
}
