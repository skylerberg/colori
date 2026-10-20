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
/// With `challenger` the candidate takes one seat, rotating over the games, and
/// the challenger takes the rest; without one, every seat is the candidate.
pub fn play_batch(
    candidate: &CompiledPolicy,
    challenger: Option<&CompiledPolicy>,
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
                        let candidate_seat = (start + offset) % players;
                        let seats: Vec<&CompiledPolicy> = (0..players)
                            .map(|seat| match challenger {
                                Some(challenger) if seat != candidate_seat => challenger,
                                _ => candidate,
                            })
                            .collect();
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

/// The score the candidate took in each game of a batch.
///
/// In a match the candidate is one seat, so its score is that seat's; in a
/// mirror match it is every seat, averaged.
pub fn candidate_scores(outcomes: &[GameOutcome], players: usize, mirrored: bool) -> Vec<f64> {
    outcomes
        .iter()
        .enumerate()
        .map(|(game, outcome)| {
            if mirrored {
                let total: f64 = outcome.seats.iter().map(|seat| seat.score).sum();
                total / players as f64
            } else {
                outcome.seats[game % players].score
            }
        })
        .collect()
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

fn quantile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

impl ScoreSummary {
    pub fn summarize(scores: &[f64], outcomes: &[GameOutcome], seat_of: impl Fn(usize) -> usize) -> Self {
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

        let seats: Vec<&SeatOutcome> = outcomes
            .iter()
            .enumerate()
            .map(|(game, outcome)| &outcome.seats[seat_of(game)])
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
            "mean {:.2} ± {:.2} (se)  median {:.1}  p10 {:.1}  p90 {:.1}  range {:.1}–{:.1}  \
             ≥30 {:.0}%  ≥35 {:.0}%  ≥40 {:.0}%",
            self.mean,
            self.std_error,
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

pub fn load_params(path: Option<&str>) -> HeuristicParams {
    match path {
        Some(path) => crate::cli::load_heuristic_params(path),
        None => HeuristicParams::default(),
    }
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
    challenger: Option<String>,
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
    let challenger = args
        .challenger
        .as_deref()
        .map(|spec| compile(&load_genome(spec), &params, "the challenger genome"));

    let seeds: Vec<u64> = (0..args.games).map(|i| args.seed_base + i as u64).collect();
    eprintln!(
        "Playing {} games, {} players, {} rules, {} threads — {}",
        args.games,
        args.players,
        candidate.terms().len(),
        threads,
        match &challenger {
            Some(_) => "candidate on one seat against the challenger",
            None => "mirror match",
        }
    );

    let started = std::time::Instant::now();
    let outcomes = play_batch(
        &candidate,
        challenger.as_ref(),
        args.players,
        &seeds,
        threads,
        false,
    );
    let elapsed = started.elapsed();

    let mirrored = challenger.is_none();
    let scores = candidate_scores(&outcomes, args.players, mirrored);
    let seat_of = |game: usize| {
        if mirrored {
            0
        } else {
            game % args.players
        }
    };
    let summary = ScoreSummary::summarize(&scores, &outcomes, seat_of);

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
        write_trace(&candidate, challenger.as_ref(), args, threads, seed, output);
    }

    std::fs::create_dir_all(output).expect("failed to create the output directory");
    let report = PlayReport {
        candidate: args.genome.clone(),
        challenger: args.challenger.clone(),
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
    challenger: Option<&CompiledPolicy>,
    args: &PolicyPlayArgs,
    _threads: usize,
    seed: u64,
    output: &str,
) {
    let candidate_seat = seed.saturating_sub(args.seed_base) as usize % args.players;
    let seats: Vec<&CompiledPolicy> = (0..args.players)
        .map(|seat| match challenger {
            Some(challenger) if seat != candidate_seat => challenger,
            _ => candidate,
        })
        .collect();
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
        // The challenger is the same rules, so only the bookkeeping is under
        // test here.
        let seeds: Vec<u64> = (0..8).collect();
        let outcomes = play_batch(&candidate, Some(&candidate), 4, &seeds, 3, false);
        assert_eq!(outcomes.len(), seeds.len());
        assert_eq!(
            outcomes.iter().map(|outcome| outcome.seed).collect::<Vec<_>>(),
            seeds,
            "seeds must come back in the order they were asked for"
        );
        assert_eq!(candidate_scores(&outcomes, 4, false).len(), seeds.len());
    }
}
