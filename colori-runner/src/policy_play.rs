//! Playing games with the deterministic policy, and measuring how they went.
//!
//! The measurement is the point. A rules bot is cheap enough to run in the
//! thousands, so the answer to "is this genome better" should be an interval
//! rather than an impression — and the number that matters is the **final
//! score**, not the win count. Thirty is a good score, thirty-five a great one,
//! and forty is above what a human plays at.
//!
//! # The yardstick
//!
//! A candidate is measured on one seat of a four-player table whose other three
//! seats are played by a **frozen anchor** genome. That gives a number that
//! means the same thing in every generation: what this genome scores against a
//! fixed opponent. Comparing candidates against each other on their own is how
//! a search drifts into being champion of its own harness.
//!
//! The candidate's seat rotates over the games, so a favourable seat position
//! averages out instead of accumulating.
//!
//! # Seeds
//!
//! Game `i` uses seed `seed_base + i`, and a genome's decisions consume no
//! randomness at all, so two genomes measured on the same seeds see the same
//! starting decks. Their random streams do diverge — how many cards a draw
//! pulls depends on choices made earlier — so this is variance reduction rather
//! than a truly paired comparison. It still removes the largest source of noise,
//! the opening hands.

use colori_core::colori_game::{apply_choice_to_state, enumerate_choices, ActionPhaseViews};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::policy::{CompiledPolicy, Decision, PolicyGenome};
use colori_core::scoring::{score_in_half_points, HeuristicParams};
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;
use rand::SeedableRng;
use serde::Serialize;
use wyrand::WyRand;

use crate::cli::PolicyPlayArgs;

/// A game that never ends is a policy bug, not a slow game. Failing loudly
/// beats hanging an overnight run.
const MAX_DECISIONS: u32 = 20_000;

// ── Outcomes ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatOutcome {
    pub seat: usize,
    pub score: f64,
    pub half_points: u32,
    pub sell_cards: u32,
    pub ducats: u32,
    pub wilds: u32,
    pub boats: u32,
    pub stored_colors: u32,
    pub stored_materials: u32,
    pub owned_cards: u32,
    pub decisions: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub seq: u32,
    pub round: u32,
    pub phase: &'static str,
    pub seat: usize,
    pub decision: Decision,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcome {
    pub seed: u64,
    pub rounds: u32,
    pub seats: Vec<SeatOutcome>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<DecisionRecord>,
}

// ── Playing ──

fn is_taking_turn(state: &GameState, seat: usize) -> bool {
    matches!(
        &state.phase,
        GamePhase::Action { action_state } if !action_state.turns[seat].ended
    )
}

fn decide(
    policy: &CompiledPolicy,
    state: &GameState,
    seat: usize,
    record: bool,
) -> Decision {
    let choices = enumerate_choices(state, seat);
    assert!(
        !choices.is_empty(),
        "seat {seat} has nothing to choose from in {:?}",
        state.phase
    );
    let decision = if record {
        policy.decide_with_trace(state, seat, &choices)
    } else {
        policy.decide_scored(state, seat, &choices)
    };
    decision.expect("choices were offered and scored")
}

/// Play one game.
///
/// Every seat decides on its own view of an action phase, as the game's own AI
/// drivers do: turns are simultaneous, and a policy that read the live state
/// would be seeing moves nobody has seen yet.
pub fn play_game(policies: &[&CompiledPolicy], seed: u64, record: bool) -> GameOutcome {
    let num_players = policies.len();
    assert!(
        (1..=MAX_PLAYERS).contains(&num_players),
        "{num_players} players is not a game"
    );

    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    let mut decisions_per_seat = vec![0u32; num_players];
    let mut trace = Vec::new();
    let mut seq = 0u32;
    let mut guard = 0u32;

    while !matches!(state.phase, GamePhase::GameOver) {
        guard += 1;
        assert!(
            guard < MAX_DECISIONS,
            "seed {seed}: {MAX_DECISIONS} decisions and the game has not ended"
        );

        match &state.phase {
            GamePhase::Buyers { buyers_state } => {
                let seat = buyers_state.current_player_index;
                let decision = decide(policies[seat], &state, seat, record);
                play(&mut state, &mut rng, seat, &decision, "buyers", &mut seq, &mut trace, record);
                decisions_per_seat[seat] += 1;
            }
            GamePhase::Draft { draft_state } => {
                let seat = draft_state.current_player_index;
                let decision = decide(policies[seat], &state, seat, record);
                play(&mut state, &mut rng, seat, &decision, "draft", &mut seq, &mut trace, record);
                decisions_per_seat[seat] += 1;
            }
            GamePhase::Action { .. } => {
                // Everyone acts at once, each on the phase as it began plus
                // their own moves.
                let mut views = ActionPhaseViews::new(&state);
                for seat in 0..num_players {
                    while is_taking_turn(views.view(seat), seat) {
                        let decision = {
                            let view = views.view(seat);
                            decide(policies[seat], view, seat, record)
                        };
                        play(&mut state, &mut rng, seat, &decision, "action", &mut seq, &mut trace, record);
                        views.apply(seat, &decision.choice);
                        decisions_per_seat[seat] += 1;
                    }
                }
            }
            GamePhase::Draw | GamePhase::GameOver => break,
        }
    }

    let seats = state
        .players
        .iter()
        .enumerate()
        .map(|(seat, player)| {
            let half_points = score_in_half_points(player);
            SeatOutcome {
                seat,
                score: half_points as f64 / 2.0,
                half_points,
                sell_cards: player.completed_sell_cards.len() as u32,
                ducats: player.ducats,
                wilds: player.wild_primaries,
                boats: player.boats,
                stored_colors: player.color_wheel.counts.iter().map(|&c| c as u32).sum(),
                stored_materials: player.materials.counts.iter().sum(),
                owned_cards: player.deck.len()
                    + player.workshop_cards.len()
                    + player.workshopped_cards.len()
                    + player.drafted_cards.len(),
                decisions: decisions_per_seat[seat],
            }
        })
        .collect();

    GameOutcome {
        seed,
        rounds: state.round,
        seats,
        trace,
    }
}

#[allow(clippy::too_many_arguments)]
fn play(
    state: &mut GameState,
    rng: &mut WyRand,
    seat: usize,
    decision: &Decision,
    phase: &'static str,
    seq: &mut u32,
    trace: &mut Vec<DecisionRecord>,
    record: bool,
) {
    *seq += 1;
    let round = state.round;
    apply_choice_to_state(state, seat, &decision.choice, rng);
    if record {
        trace.push(DecisionRecord {
            seq: *seq,
            round,
            phase,
            seat,
            decision: decision.clone(),
        });
    }
}

/// Play `seeds.len()` games, spreading them over `threads`.
///
/// The candidate takes one seat, rotating over the games so a favourable seat
/// position averages out. Every other seat is filled from the frozen ladder:
/// with one rung that is a straight match, with several the candidate faces a
/// varied table while the ladder itself never moves. Without a ladder every
/// seat is the candidate.
///
/// Which rung sits where is decided by the game's index, not sampled, so every
/// candidate measured on the same seed range faces exactly the same tables. A
/// random draw would put a candidate's fitness at the mercy of whose turn it was
/// to face the strong rung.
pub fn play_batch(
    candidate: &CompiledPolicy,
    ladder: Option<&[&CompiledPolicy]>,
    players: usize,
    seeds: &[u64],
    threads: usize,
    record: bool,
) -> Vec<GameOutcome> {
    let threads = threads.max(1).min(seeds.len().max(1));
    let chunk = (seeds.len() + threads - 1) / threads.max(1);
    let mut collected: Vec<GameOutcome> = Vec::with_capacity(seeds.len());

    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk_index in 0..threads {
            let start = chunk_index * chunk;
            if start >= seeds.len() {
                break;
            }
            let end = (start + chunk).min(seeds.len());
            let slice = &seeds[start..end];
            handles.push(scope.spawn(move || {
                slice
                    .iter()
                    .enumerate()
                    .map(|(offset, &seed)| {
                        let game_index = start + offset;
                        let seats = seat_table(candidate, ladder, players, game_index);
                        play_game(&seats, seed, record)
                    })
                    .collect::<Vec<_>>()
            }));
        }
        for handle in handles {
            collected.extend(handle.join().expect("a game worker panicked"));
        }
    });

    collected.sort_by_key(|outcome| outcome.seed);
    collected
}

/// Who sits where in game `game_index`.
pub fn seat_table<'a>(
    candidate: &'a CompiledPolicy,
    ladder: Option<&[&'a CompiledPolicy]>,
    players: usize,
    game_index: usize,
) -> Vec<&'a CompiledPolicy> {
    let candidate_seat = game_index % players;
    let mut rung = 0usize;
    (0..players)
        .map(|seat| {
            if seat == candidate_seat {
                return candidate;
            }
            match ladder {
                Some(ladder) if !ladder.is_empty() => {
                    let pick = ladder[(game_index + rung) % ladder.len()];
                    rung += 1;
                    pick
                }
                _ => candidate,
            }
        })
        .collect()
}

/// What a batch says about the candidate: its own score, and how it compared
/// with the table it was dealt.
///
/// [`Batch::deltas`] is the statistic worth trusting in a search. Two games with
/// the same seed still diverge, so a game's absolute total carries a lot of the
/// table's luck; the candidate's score minus the other seats' mean strips that
/// luck out, because it is measured within the same game. The absolute score
/// stays the yardstick — it is what "thirty points" means.
pub struct Batch {
    pub scores: Vec<f64>,
    pub deltas: Vec<f64>,
    pub opponent_means: Vec<f64>,
}

impl Batch {
    pub fn of(outcomes: &[GameOutcome], players: usize, mirrored: bool) -> Self {
        let mut batch = Batch {
            scores: Vec::with_capacity(outcomes.len()),
            deltas: Vec::with_capacity(outcomes.len()),
            opponent_means: Vec::with_capacity(outcomes.len()),
        };
        for (game, outcome) in outcomes.iter().enumerate() {
            let seat = game % players;
            if mirrored || players < 2 {
                let total: f64 = outcome.seats.iter().map(|other| other.score).sum();
                let mean = total / outcome.seats.len().max(1) as f64;
                batch.scores.push(mean);
                batch.opponent_means.push(mean);
                batch.deltas.push(0.0);
                continue;
            }
            let score = outcome.seats[seat].score;
            let others: f64 = outcome
                .seats
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != seat)
                .map(|(_, other)| other.score)
                .sum::<f64>()
                / (outcome.seats.len() - 1) as f64;
            batch.scores.push(score);
            batch.opponent_means.push(others);
            batch.deltas.push(score - others);
        }
        batch
    }
}

// ── Summarising ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSummary {
    pub games: usize,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    /// Standard error of the mean: how much of the mean is noise. A difference
    /// between two genomes that is smaller than this is not a difference.
    pub std_error: f64,
    /// The candidate's score minus the rest of the table's, averaged over
    /// games. Measured within each game, so much of the table's luck cancels;
    /// this is the number a search should compare.
    pub mean_delta: f64,
    pub std_error_delta: f64,
    /// What the other seats scored, so a table's strength is visible.
    pub mean_opponents: f64,
    pub min: f64,
    pub p10: f64,
    pub p25: f64,
    pub p75: f64,
    pub p90: f64,
    pub max: f64,
    pub share_at_least_30: f64,
    pub share_at_least_35: f64,
    pub share_at_least_40: f64,
    pub mean_sell_cards: f64,
    pub mean_ducats: f64,
    pub mean_unspent_wilds: f64,
    pub mean_unspent_boats: f64,
    pub mean_stored_colors: f64,
    pub mean_stored_materials: f64,
    pub mean_owned_cards: f64,
    pub mean_decisions: f64,
}

fn mean_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn quantile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

impl ScoreSummary {
    pub fn summarize(
        batch: &Batch,
        outcomes: &[GameOutcome],
        players: usize,
        mirrored: bool,
    ) -> Self {
        let scores = &batch.scores;
        let mut sorted = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let games = sorted.len().max(1);
        let mean = sorted.iter().sum::<f64>() / games as f64;
        let variance = if games > 1 {
            sorted.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / (games - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();
        let delta_mean = mean_of(&batch.deltas);
        let delta_variance = if games > 1 {
            batch
                .deltas
                .iter()
                .map(|delta| (delta - delta_mean).powi(2))
                .sum::<f64>()
                / (games - 1) as f64
        } else {
            0.0
        };

        let seats: Vec<&SeatOutcome> = outcomes
            .iter()
            .enumerate()
            .map(|(game, outcome)| {
                let seat = if mirrored || players < 2 {
                    0
                } else {
                    game % players
                };
                &outcome.seats[seat]
            })
            .collect();
        let seat_mean = |f: fn(&SeatOutcome) -> f64| -> f64 {
            if seats.is_empty() {
                0.0
            } else {
                seats.iter().map(|seat| f(seat)).sum::<f64>() / seats.len() as f64
            }
        };
        let share = |threshold: f64| {
            sorted.iter().filter(|score| **score >= threshold).count() as f64 / games as f64
        };

        ScoreSummary {
            games,
            mean,
            mean_delta: delta_mean,
            std_error_delta: delta_variance.sqrt() / (games as f64).sqrt(),
            mean_opponents: mean_of(&batch.opponent_means),
            median: quantile(&sorted, 0.5),
            std_dev,
            std_error: std_dev / (games as f64).sqrt(),
            min: sorted.first().copied().unwrap_or(0.0),
            p10: quantile(&sorted, 0.10),
            p25: quantile(&sorted, 0.25),
            p75: quantile(&sorted, 0.75),
            p90: quantile(&sorted, 0.90),
            max: sorted.last().copied().unwrap_or(0.0),
            share_at_least_30: share(30.0),
            share_at_least_35: share(35.0),
            share_at_least_40: share(40.0),
            mean_sell_cards: seat_mean(|seat| seat.sell_cards as f64),
            mean_ducats: seat_mean(|seat| seat.ducats as f64),
            mean_unspent_wilds: seat_mean(|seat| seat.wilds as f64),
            mean_unspent_boats: seat_mean(|seat| seat.boats as f64),
            mean_stored_colors: seat_mean(|seat| seat.stored_colors as f64),
            mean_stored_materials: seat_mean(|seat| seat.stored_materials as f64),
            mean_owned_cards: seat_mean(|seat| seat.owned_cards as f64),
            mean_decisions: seat_mean(|seat| seat.decisions as f64),
        }
    }
}

impl std::fmt::Display for ScoreSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mean {:.2} ± {:.2} (se)  vs table {:+.2} ± {:.2}  (table {:.2})  median {:.1}  \
             p10 {:.1}  p90 {:.1}  range {:.1}–{:.1}  ≥30 {:.0}%  ≥35 {:.0}%  ≥40 {:.0}%",
            self.mean,
            self.std_error,
            self.mean_delta,
            self.std_error_delta,
            self.mean_opponents,
            self.median,
            self.p10,
            self.p90,
            self.min,
            self.max,
            self.share_at_least_30 * 100.0,
            self.share_at_least_35 * 100.0,
            self.share_at_least_40 * 100.0,
        )
    }
}

// ── Feature activity ──

/// How much each rule actually moved the decisions it was in.
///
/// A rule with no activity is either switched off, inert on this position, or
/// shadowed by another. It is the first thing to look at when a genome stops
/// improving: a search cannot tune a gene that never fires.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureActivity {
    pub feature: String,
    pub mean_absolute_weight: f64,
    pub fired: u32,
    pub of_decisions: u32,
}

pub fn feature_activity(activity: &[(String, f64, u32)], decisions: u32) -> Vec<FeatureActivity> {
    let mut rows: Vec<FeatureActivity> = activity
        .iter()
        .map(|(feature, weight, fired)| FeatureActivity {
            feature: feature.clone(),
            mean_absolute_weight: weight / decisions.max(1) as f64,
            fired: *fired,
            of_decisions: decisions,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.mean_absolute_weight
            .partial_cmp(&a.mean_absolute_weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

// ── Loading ──

/// `starter` for the built-in rules, or a path to a genome file.
pub fn load_genome(spec: &str) -> PolicyGenome {
    if spec == "starter" {
        return PolicyGenome::starter();
    }
    let text = std::fs::read_to_string(spec)
        .unwrap_or_else(|error| panic!("failed to read genome {spec}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("failed to parse genome {spec}: {error}"))
}

/// The trained parameters the policy's card-quality rules read when no file is
/// named.
///
/// Baked in rather than read from disk: this runs from whichever worktree
/// happens to be current, and a card-quality table that silently failed to load
/// would quietly switch several rules off. `--heuristic-params-file` overrides
/// it.
pub const DEFAULT_PARAMS_JSON: &str =
    include_str!("../../genetic-algorithm/batch-alobyo-gen-41.json");

pub fn default_params() -> HeuristicParams {
    serde_json::from_str(DEFAULT_PARAMS_JSON)
        .expect("the baked-in heuristic params are current-schema")
}

pub fn load_params(path: Option<&str>) -> HeuristicParams {
    match path {
        Some(path) => crate::cli::load_heuristic_params(path),
        None => default_params(),
    }
}

/// Compile a frozen ladder from genome specs, in order.
///
/// `none`, or an empty list, is no ladder: the candidate plays every seat.
pub fn compile_ladder(specs: &[String], params: &HeuristicParams) -> Vec<CompiledPolicy> {
    specs
        .iter()
        .filter(|spec| spec.as_str() != "none")
        .map(|spec| compile(&load_genome(spec), params, &format!("ladder genome {spec}")))
        .collect()
}

pub fn compile(genome: &PolicyGenome, params: &HeuristicParams, what: &str) -> CompiledPolicy {
    CompiledPolicy::from_genome(genome, params)
        .unwrap_or_else(|error| panic!("{what} will not compile: {error}"))
}

// ── The command ──

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayReport {
    candidate: String,
    anchors: Vec<String>,
    players: usize,
    seed_base: u64,
    summary: ScoreSummary,
    seat_means: Vec<f64>,
    activity: Vec<FeatureActivity>,
}

pub fn run_policy_play(args: &PolicyPlayArgs, threads: usize, output: &str) {
    let params = load_params(args.heuristic_params_file.as_deref());
    let candidate_genome = load_genome(&args.genome);
    let candidate = compile(&candidate_genome, &params, "the candidate genome");
    let ladder_policies = compile_ladder(&args.anchors, &params);
    let ladder: Vec<&CompiledPolicy> = ladder_policies.iter().collect();
    let mirrored = ladder.is_empty();

    let seeds: Vec<u64> = (0..args.games).map(|i| args.seed_base + i as u64).collect();
    eprintln!(
        "Playing {} games, {} players, {} rules, {} threads — {}",
        args.games,
        args.players,
        candidate.terms().len(),
        threads,
        if mirrored {
            "mirror match".to_string()
        } else {
            format!("candidate on one seat against {}", args.anchors.join(", "))
        }
    );

    let started = std::time::Instant::now();
    let outcomes = play_batch(
        &candidate,
        if mirrored { None } else { Some(&ladder) },
        args.players,
        &seeds,
        threads,
        false,
    );
    let elapsed = started.elapsed();

    let batch = Batch::of(&outcomes, args.players, mirrored);
    let summary = ScoreSummary::summarize(&batch, &outcomes, args.players, mirrored);

    let mut seat_means = vec![0.0; args.players];
    for outcome in &outcomes {
        for (seat, seat_outcome) in outcome.seats.iter().enumerate() {
            seat_means[seat] += seat_outcome.score;
        }
    }
    for mean in seat_means.iter_mut() {
        *mean /= outcomes.len().max(1) as f64;
    }

    // Activity needs full traces, so it is measured on a handful of games
    // rather than all of them.
    let activity = activity_of(&candidate, args.players, 4, args.seed_base, threads);

    eprintln!();
    eprintln!("{}", summary);
    eprintln!(
        "Per seat: {}",
        seat_means
            .iter()
            .map(|mean| format!("{mean:.1}"))
            .collect::<Vec<_>>()
            .join("  ")
    );
    eprintln!(
        "Per game: {:.2} sell cards, {:.2} ducats, {:.2} colours stored, {:.0} decisions",
        summary.mean_sell_cards,
        summary.mean_ducats,
        summary.mean_stored_colors,
        summary.mean_decisions
    );
    eprintln!(
        "{:.1}ms per game ({:.0} games/sec across {} threads)",
        elapsed.as_secs_f64() * 1000.0 / outcomes.len().max(1) as f64,
        outcomes.len() as f64 / elapsed.as_secs_f64().max(1e-9),
        threads
    );
    if !activity.is_empty() {
        eprintln!("\nRules that moved decisions (mean |contribution| per decision):");
        for row in activity.iter().take(12) {
            eprintln!(
                "  {:<26} {:>8.3}   fired in {:>5.1}% of decisions",
                row.feature,
                row.mean_absolute_weight,
                row.fired as f64 / row.of_decisions.max(1) as f64 * 100.0
            );
        }
        let inert: Vec<&str> = activity
            .iter()
            .filter(|row| row.fired == 0)
            .map(|row| row.feature.as_str())
            .collect();
        if !inert.is_empty() {
            eprintln!("  never fired: {}", inert.join(", "));
        }
    }

    if let Some(seed) = args.trace_seed {
        write_trace(
            &candidate,
            if mirrored { None } else { Some(&ladder) },
            args,
            seed,
            output,
        );
    }

    std::fs::create_dir_all(output).expect("failed to create the output directory");
    let report = PlayReport {
        candidate: args.genome.clone(),
        anchors: args.anchors.clone(),
        players: args.players,
        seed_base: args.seed_base,
        summary,
        seat_means,
        activity,
    };
    let path = format!("{output}/play-{}.json", epoch_millis());
    std::fs::write(&path, serde_json::to_string_pretty(&report).expect("report serializes"))
        .expect("failed to write the report");
    eprintln!("\nReport written to {path}");
}

/// How much each rule moved the decisions across a handful of games.
///
/// Tracing every decision of a long run is expensive, so this is measured on a
/// few games rather than all of them — enough to tell an inert rule from a
/// load-bearing one, which is all the report needs it for.
pub fn activity_of(
    policy: &CompiledPolicy,
    players: usize,
    games: usize,
    seed_base: u64,
    threads: usize,
) -> Vec<FeatureActivity> {
    let games = games.max(1);
    let seeds: Vec<u64> = (0..games).map(|i| seed_base + i as u64).collect();
    let outcomes = play_batch(policy, None, players, &seeds, threads.min(games), true);

    let mut totals: Vec<(String, f64, u32)> = Vec::new();
    let mut decisions = 0u32;
    for outcome in &outcomes {
        for record in &outcome.trace {
            decisions += 1;
            for contribution in &record.decision.contributions {
                match totals
                    .iter_mut()
                    .find(|(name, _, _)| name == contribution.feature)
                {
                    Some((_, weight, fired)) => {
                        *weight += contribution.weighted.abs();
                        if contribution.weighted != 0.0 {
                            *fired += 1;
                        }
                    }
                    None => totals.push((
                        contribution.feature.to_string(),
                        contribution.weighted.abs(),
                        u32::from(contribution.weighted != 0.0),
                    )),
                }
            }
        }
    }
    if decisions == 0 {
        return Vec::new();
    }

    let mut rows = feature_activity(&totals, decisions);
    // A rule the genome switched off never appears in a trace; say so
    // explicitly rather than letting it look merely quiet.
    for term in policy.terms() {
        if !rows.iter().any(|row| row.feature == term.name) {
            rows.push(FeatureActivity {
                feature: term.name.to_string(),
                mean_absolute_weight: 0.0,
                fired: 0,
                of_decisions: decisions,
            });
        }
    }
    rows
}

fn write_trace(
    candidate: &CompiledPolicy,
    ladder: Option<&[&CompiledPolicy]>,
    args: &PolicyPlayArgs,
    seed: u64,
    output: &str,
) {
    let game_index = seed.saturating_sub(args.seed_base) as usize;
    let seats = seat_table(candidate, ladder, args.players, game_index);
    let outcome = play_game(&seats, seed, true);

    std::fs::create_dir_all(output).expect("failed to create the output directory");
    let path = format!("{output}/trace-{seed}.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&outcome).expect("a trace serializes"),
    )
    .expect("failed to write the trace");

    eprintln!("\nTrace of seed {seed} written to {path}");
    eprintln!("Seat scores: {}", outcome.seats.iter()
        .map(|seat| format!("{:.1}", seat.score))
        .collect::<Vec<_>>()
        .join("  "));
    eprintln!("Decisions the rules came closest to getting wrong — read these first:");
    let mut near: Vec<&DecisionRecord> = outcome
        .trace
        .iter()
        .filter(|record| record.decision.margin().is_some())
        .collect();
    near.sort_by(|a, b| {
        a.decision
            .margin()
            .unwrap_or(f64::INFINITY)
            .partial_cmp(&b.decision.margin().unwrap_or(f64::INFINITY))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for record in near.iter().take(10) {
        let decision = &record.decision;
        eprintln!(
            "  round {} {:>6} seat {} — took {:?} ({:.2}) over {:?} ({:.2})",
            record.round,
            record.phase,
            record.seat,
            decision.choice,
            decision.score,
            decision.runner_up.unwrap_or(decision.choice),
            decision.runner_up_score.unwrap_or(decision.score),
        );
        for contribution in decision
            .contributions
            .iter()
            .filter(|c| c.weighted != 0.0)
            .take(5)
        {
            eprintln!("      {:<26} {:+8.3}", contribution.feature, contribution.weighted);
        }
    }
}

fn epoch_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use colori_core::scoring::HeuristicParams;

    fn starter() -> CompiledPolicy {
        CompiledPolicy::starter(&HeuristicParams::default())
    }

    fn scores(outcome: &GameOutcome) -> Vec<u32> {
        outcome.seats.iter().map(|seat| seat.half_points).collect()
    }

    #[test]
    fn the_baked_in_params_are_current_and_price_cards() {
        // A stale or partial file would silently strip the card-quality rules
        // of any meaning, so the default is checked rather than assumed.
        let params = default_params();
        let table = colori_core::scoring::CardHeuristicTable::new(&params);
        use colori_core::types::Card;
        assert!(table.quality(Card::Chalk) > 0.0, "chalk is worth nothing");
        assert!(
            table.quality(Card::StarterCeramics) > 0.0,
            "a starter material is worth nothing"
        );
        assert_ne!(
            table.quality(Card::Chalk),
            table.quality(Card::StarterCeramics),
            "two different cards weigh the same, so the table is the neutral default"
        );
    }

    #[test]
    fn the_same_seed_plays_the_same_game() {
        let policy = starter();
        let seats: [&CompiledPolicy; 4] = [&policy; 4];
        for seed in 0..6 {
            let first = play_game(&seats, seed, false);
            let second = play_game(&seats, seed, false);
            assert_eq!(scores(&first), scores(&second), "seed {seed} scored differently");
            assert_eq!(first.rounds, second.rounds);
            for (a, b) in first.seats.iter().zip(&second.seats) {
                assert_eq!(a.sell_cards, b.sell_cards);
                assert_eq!(a.decisions, b.decisions, "the same seed made a different number of decisions");
            }
        }
    }

    #[test]
    fn a_game_runs_to_a_score() {
        let policy = starter();
        let seats: [&CompiledPolicy; 4] = [&policy; 4];
        let outcome = play_game(&seats, 3, false);
        assert_eq!(outcome.seats.len(), 4);
        assert!(outcome.rounds > 1, "the game ended after {} rounds", outcome.rounds);
        assert!(
            outcome.seats.iter().any(|seat| seat.half_points > 0),
            "a finished game scored nothing at all"
        );
        assert!(
            outcome.seats.iter().any(|seat| seat.sell_cards > 0),
            "nobody finished a sell card in a whole game"
        );
    }

    #[test]
    fn different_seeds_play_different_games() {
        let policy = starter();
        let seats: [&CompiledPolicy; 4] = [&policy; 4];
        let played: Vec<Vec<u32>> = (0..5)
            .map(|seed| scores(&play_game(&seats, seed, false)))
            .collect();
        assert!(
            played.windows(2).any(|pair| pair[0] != pair[1]),
            "every seed produced the same scores, so something is ignoring the seed"
        );
    }

    #[test]
    fn a_traced_game_records_its_decisions() {
        let policy = starter();
        let seats: [&CompiledPolicy; 4] = [&policy; 4];
        let outcome = play_game(&seats, 11, true);
        assert!(!outcome.trace.is_empty(), "a traced game recorded nothing");
        for record in &outcome.trace {
            assert!(!record.decision.contributions.is_empty());
            assert!(record.decision.score.is_finite());
        }
    }

    #[test]
    fn a_batch_plays_every_seed_once_and_returns_them_in_order() {
        let candidate = starter();
        // The only rung is the same rules, so only the bookkeeping is under
        // test here.
        let seeds: Vec<u64> = (0..8).collect();
        let ladder: [&CompiledPolicy; 1] = [&candidate];
        let outcomes = play_batch(&candidate, Some(&ladder), 4, &seeds, 3, false);
        assert_eq!(outcomes.len(), seeds.len());
        assert_eq!(
            outcomes.iter().map(|outcome| outcome.seed).collect::<Vec<_>>(),
            seeds,
            "seeds must come back in the order they were asked for"
        );
        let batch = Batch::of(&outcomes, 4, false);
        assert_eq!(batch.scores.len(), seeds.len());
        assert_eq!(batch.deltas.len(), seeds.len());
    }
}
