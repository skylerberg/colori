//! Engine cost, measured in retired instructions rather than wall clock.
//!
//! `cargo bench` reports wall clock, which on a shared desktop says as much
//! about what else is running as about the engine. macOS exposes a per-process
//! retired-instruction counter through `proc_pid_rusage(RUSAGE_INFO_V4)`, and
//! it needs no privileges. Repeats of an identical workload on an M4 land
//! within ~0.15% of each other, so a change of a percent or two is legible —
//! which wall clock on this machine is not.
//!
//! ```sh
//! cargo run --release -p colori-core --example instruction_count
//! ```
//!
//! Release matters: the workspace profile sets `lto = true` and
//! `codegen-units = 1`, and a debug build measures a different program.
//!
//! Everything here is seeded, single-threaded and fixed-iteration, so two
//! builds of the engine can be compared directly. `early_termination` is off
//! in the per-iteration workload for the reason `benches/ismcts_bench.rs`
//! gives: with it on, the iteration count itself varies and a per-iteration
//! figure stops meaning anything.

use colori_core::choices::enumerate_choices;
use colori_core::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::ismcts::MctsConfig;
use colori_core::mcts_impl::ColoriSearcher;
use colori_core::scoring::HeuristicParams;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{GamePhase, GameState, PlayerState};

use rand::{RngExt, SeedableRng};
use wyrand::WyRand;

// ── Counters ──

/// Retired instructions and cycles for this process.
///
/// Both come from the same `proc_pid_rusage` call so they describe the same
/// interval. Cycles are reported alongside instructions because IPC is what
/// separates "doing more work" from "doing the same work worse" — a change
/// that grows the state and thrashes cache moves cycles without moving
/// instructions.
#[derive(Clone, Copy)]
struct Counters {
    instructions: u64,
    cycles: u64,
}

impl std::ops::Sub for Counters {
    type Output = Counters;
    fn sub(self, rhs: Counters) -> Counters {
        Counters {
            instructions: self.instructions.saturating_sub(rhs.instructions),
            cycles: self.cycles.saturating_sub(rhs.cycles),
        }
    }
}

#[cfg(target_os = "macos")]
mod counters {
    use super::Counters;

    /// `struct rusage_info_v4` from `<sys/resource.h>`. Laid out field for
    /// field rather than read by offset so a future SDK adding a field in the
    /// middle is a compile-time diff and not a silently wrong number.
    #[repr(C)]
    #[derive(Default)]
    struct RusageInfoV4 {
        ri_uuid: [u8; 16],
        ri_user_time: u64,
        ri_system_time: u64,
        ri_pkg_idle_wkups: u64,
        ri_interrupt_wkups: u64,
        ri_pageins: u64,
        ri_wired_size: u64,
        ri_resident_size: u64,
        ri_phys_footprint: u64,
        ri_proc_start_abstime: u64,
        ri_proc_exit_abstime: u64,
        ri_child_user_time: u64,
        ri_child_system_time: u64,
        ri_child_pkg_idle_wkups: u64,
        ri_child_interrupt_wkups: u64,
        ri_child_pageins: u64,
        ri_child_elapsed_abstime: u64,
        ri_diskio_bytesread: u64,
        ri_diskio_byteswritten: u64,
        ri_cpu_time_qos_default: u64,
        ri_cpu_time_qos_maintenance: u64,
        ri_cpu_time_qos_background: u64,
        ri_cpu_time_qos_utility: u64,
        ri_cpu_time_qos_legacy: u64,
        ri_cpu_time_qos_user_initiated: u64,
        ri_cpu_time_qos_user_interactive: u64,
        ri_billed_system_time: u64,
        ri_serviced_system_time: u64,
        ri_logical_writes: u64,
        ri_lifetime_max_phys_footprint: u64,
        ri_instructions: u64,
        ri_cycles: u64,
        ri_billed_energy: u64,
        ri_serviced_energy: u64,
        ri_interval_max_phys_footprint: u64,
        ri_runnable_time: u64,
    }

    const RUSAGE_INFO_V4: i32 = 4;

    extern "C" {
        fn getpid() -> i32;
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
    }

    pub fn read() -> Counters {
        let mut info = RusageInfoV4::default();
        // SAFETY: `info` is a correctly sized and aligned `rusage_info_v4`,
        // which is what the kernel writes for `RUSAGE_INFO_V4`.
        let rc = unsafe {
            proc_pid_rusage(
                getpid(),
                RUSAGE_INFO_V4,
                &mut info as *mut RusageInfoV4 as *mut std::ffi::c_void,
            )
        };
        assert_eq!(rc, 0, "proc_pid_rusage failed");
        Counters {
            instructions: info.ri_instructions,
            cycles: info.ri_cycles,
        }
    }

    pub const SOURCE: &str = "proc_pid_rusage(RUSAGE_INFO_V4), retired instructions and cycles";
}

/// No portable equivalent exists, and a wall-clock stand-in would defeat the
/// point of the tool, so elsewhere it refuses rather than reporting a number
/// that cannot be compared.
#[cfg(not(target_os = "macos"))]
mod counters {
    use super::Counters;

    pub fn read() -> Counters {
        eprintln!(
            "instruction_count needs macOS hardware counters \
             (proc_pid_rusage RUSAGE_INFO_V4); nothing equivalent is wired up \
             for this platform."
        );
        std::process::exit(1);
    }

    pub const SOURCE: &str = "unsupported platform";
}

/// Run `f` `reps` times and return the per-rep counter deltas.
///
/// The first rep is dropped: it pays for lazily faulted pages and a cold
/// branch predictor, and is reliably the outlier.
fn measure(reps: usize, mut f: impl FnMut()) -> Vec<Counters> {
    let mut out = Vec::with_capacity(reps);
    for _ in 0..=reps {
        let before = counters::read();
        f();
        out.push(counters::read() - before);
    }
    out.remove(0);
    out
}

fn median_instructions(samples: &[Counters]) -> u64 {
    let mut v: Vec<u64> = samples.iter().map(|c| c.instructions).collect();
    v.sort_unstable();
    v[v.len() / 2]
}

fn median_cycles(samples: &[Counters]) -> u64 {
    let mut v: Vec<u64> = samples.iter().map(|c| c.cycles).collect();
    v.sort_unstable();
    v[v.len() / 2]
}

/// Spread of the samples around the median, as a percentage. A workload whose
/// spread is comparable to the effect being measured is not measuring it.
fn spread_pct(samples: &[Counters]) -> f64 {
    let mut v: Vec<u64> = samples.iter().map(|c| c.instructions).collect();
    v.sort_unstable();
    let median = v[v.len() / 2] as f64;
    if median == 0.0 {
        return 0.0;
    }
    (v[v.len() - 1] - v[0]) as f64 / median * 100.0
}

// ── Shared setup ──

const PLAYERS: usize = 3;
const POSITION_SEED: u64 = 20_260_819;
const REPS: usize = 5;

/// The generation the tests and benches pin, so an ongoing GA run cannot move
/// the bar underneath a measurement.
fn frozen_params() -> HeuristicParams {
    const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
    serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
}

fn config(iterations: u32, early_termination: bool) -> MctsConfig {
    MctsConfig {
        iterations,
        early_termination,
        time_limit_ms: None,
        ..MctsConfig::new(frozen_params())
    }
}

/// Matches what the runner passes in production.
fn max_rollout_round(state: &GameState) -> Option<u32> {
    Some(std::cmp::max(8, state.round + 2))
}

/// Walk a fresh game forward, choosing uniformly at random, and stop at the
/// first action-phase position in `round` offering at least `min_branching`
/// legal choices.
///
/// `benches/ismcts_bench.rs` selects positions by step index instead, which is
/// fine while the game is fixed but useless across a rules change: the same
/// index lands somewhere else entirely, and the reader cannot tell a slower
/// engine from an easier position. Selecting by round and width gives the two
/// builds the same *kind* of position, and the branching actually found is
/// printed so any remaining drift is visible rather than assumed away.
fn position(seed: u64, round: u32, min_branching: usize) -> Option<GameState> {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; PLAYERS];
    let mut state = create_initial_game_state(PLAYERS, &ai_players, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    loop {
        match &state.phase {
            GamePhase::GameOver => return None,
            GamePhase::Draw => {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            _ => {}
        }
        if matches!(get_game_status(&state, None), GameStatus::Terminated { .. }) {
            return None;
        }
        let choices = enumerate_choices(&state);
        if choices.is_empty() {
            return None;
        }
        if state.round > round {
            return None;
        }
        let wide_enough = choices.len() >= min_branching;
        if state.round == round && wide_enough && matches!(state.phase, GamePhase::Action { .. }) {
            return Some(state);
        }
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice_to_state(&mut state, &choice, &mut rng);
    }
}

// ── Workloads ──

fn report_sizes() {
    println!("sizes");
    println!("  {:<14}{:>7} B", "GameState", std::mem::size_of::<GameState>());
    println!("  {:<14}{:>7} B", "PlayerState", std::mem::size_of::<PlayerState>());
    println!();
}

/// Instructions per search iteration, at fixed positions.
///
/// This is the engine's own cost, isolated from how many decisions a game
/// takes — a rules change moves both, and they are worth separating.
fn report_search() {
    const ITERATIONS: u32 = 2_000;
    const MIN_BRANCHING: usize = 20;
    let positions = [("early", 1u32), ("middle", 3), ("late", 5)];

    println!(
        "search ({PLAYERS} players, {ITERATIONS} iterations, early termination off, \
         first action node per round with branching >= {MIN_BRANCHING})"
    );
    println!(
        "  {:<8}{:>6}{:>11}{:>14}{:>13}{:>7}{:>9}",
        "position", "round", "branching", "instr/iter", "cycles/iter", "IPC", "spread"
    );

    for (name, round) in positions {
        let Some(state) = position(POSITION_SEED, round, MIN_BRANCHING) else {
            println!("  {name:<8}no such position on this seed; skipped");
            continue;
        };
        let player = match get_game_status(&state, None) {
            GameStatus::AwaitingAction { player_index } => player_index,
            GameStatus::Terminated { .. } => unreachable!("position() rejects terminal states"),
        };
        let branching = enumerate_choices(&state).len();
        let horizon = max_rollout_round(&state);
        let config = config(ITERATIONS, false);

        let samples = measure(REPS, || {
            let mut rng = WyRand::seed_from_u64(0xC0_10_71);
            let mut searcher = ColoriSearcher::new(&state);
            let outcome = searcher.search(&state, player, &config, horizon, &mut rng);
            assert_eq!(
                outcome.iterations_used, ITERATIONS,
                "{name}: search stopped early, so per-iteration cost is not comparable"
            );
        });

        let instr = median_instructions(&samples) / ITERATIONS as u64;
        let cycles = median_cycles(&samples) / ITERATIONS as u64;
        println!(
            "  {:<8}{:>6}{:>11}{:>14}{:>13}{:>7.2}{:>8.2}%",
            name,
            state.round,
            branching,
            instr,
            cycles,
            instr as f64 / cycles as f64,
            spread_pct(&samples),
        );
    }
    println!();
}

/// What one whole game costs, plus the shape of the positions it passes
/// through.
///
/// Per-iteration cost is only half the story: a rules change that leaves the
/// engine untouched but adds decisions per turn, or widens the choice set,
/// costs the player just as much. Both halves are printed so a regression can
/// be attributed.
fn report_games() {
    const GAMES: u64 = 12;
    const ITERATIONS: u32 = 1_000;

    let config = config(ITERATIONS, true);
    let mut stats = GameStats::default();

    println!("games ({GAMES} seeded {PLAYERS}-player games, {ITERATIONS} iterations per move)");
    let samples = measure(REPS, || {
        stats = GameStats::default();
        for seed in 0..GAMES {
            play_game(seed, &config, &mut stats);
        }
    });

    let instr = median_instructions(&samples);
    let row = |label: &str, value: String| println!("  {label:<26}{value:>16}");
    row("instructions/game", (instr / GAMES).to_string());
    row("instructions/iteration", (instr / stats.iterations.max(1)).to_string());
    row("iterations/game", (stats.iterations / GAMES).to_string());
    row("decisions/game", (stats.decisions / GAMES).to_string());
    row("mean branching", format!("{:.2}", stats.per_decision(stats.branching)));
    row("mean workshop size", format!("{:.2}", stats.per_player(stats.workshop)));
    row("mean deck size", format!("{:.2}", stats.per_player(stats.deck)));
    row("max deck size", stats.max_deck.to_string());
    row("max deck segments", stats.max_segments.to_string());
    row("mean score", format!("{:.2}", stats.total_score as f64 / (GAMES * PLAYERS as u64) as f64));
    row("spread", format!("{:.2}%", spread_pct(&samples)));
    println!();
}

#[derive(Default)]
struct GameStats {
    decisions: u64,
    iterations: u64,
    branching: u64,
    workshop: u64,
    deck: u64,
    max_deck: u32,
    max_segments: u32,
    total_score: u64,
}

impl GameStats {
    fn per_decision(&self, total: u64) -> f64 {
        total as f64 / self.decisions.max(1) as f64
    }

    fn per_player(&self, total: u64) -> f64 {
        total as f64 / (self.decisions.max(1) * PLAYERS as u64) as f64
    }

    fn observe(&mut self, state: &GameState, branching: usize, iterations: u32) {
        self.decisions += 1;
        self.iterations += iterations as u64;
        self.branching += branching as u64;
        for player in state.players.iter() {
            self.workshop += (player.workshop_cards.len() + player.workshopped_cards.len()) as u64;
            let deck = deck_len(player);
            self.deck += deck as u64;
            self.max_deck = self.max_deck.max(deck);
            self.max_segments = self.max_segments.max(deck_segments(player));
        }
    }
}

/// Size of everything that will be drawn from later, so the figure stays
/// comparable across the change that folds the discard pile into the deck.
fn deck_len(player: &PlayerState) -> u32 {
    player.deck.len() + player.discard.len()
}

/// How many independently shuffled piles the personal deck is made of. One
/// today: the deck is a single bag and the discard is not drawn from until it
/// becomes one.
fn deck_segments(_player: &PlayerState) -> u32 {
    1
}

fn play_game(seed: u64, config: &MctsConfig, stats: &mut GameStats) {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; PLAYERS];
    let mut state = create_initial_game_state(PLAYERS, &ai_players, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    let mut searchers: Vec<ColoriSearcher> =
        (0..PLAYERS).map(|_| ColoriSearcher::new(&state)).collect();

    loop {
        match &state.phase {
            GamePhase::GameOver => break,
            GamePhase::Draw => {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            _ => {}
        }
        let player = match get_game_status(&state, None) {
            GameStatus::AwaitingAction { player_index } => player_index,
            GameStatus::Terminated { .. } => break,
        };

        let horizon = max_rollout_round(&state);
        let outcome = searchers[player].search(&state, player, config, horizon, &mut rng);
        stats.observe(&state, enumerate_choices(&state).len(), outcome.iterations_used);
        apply_choice_to_state(&mut state, &outcome.choice, &mut rng);
    }

    for player in state.players.iter() {
        stats.total_score += player.cached_score as u64;
    }
}

fn main() {
    println!("colori engine cost — {}\n", counters::SOURCE);
    report_sizes();
    report_search();
    report_games();
}
