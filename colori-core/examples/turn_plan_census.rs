//! How big a player's action turn is when it is planned whole.
//!
//! A search whose root children are whole turns needs one child per end state
//! of the turn, so the number of end states is the branching factor it would
//! have. This walks real turns — random play from a fresh game, then the census
//! at every action phase it reaches — and reports:
//!
//! * **plans**: distinct end-of-turn positions, which is the child count;
//! * **positions**: turn positions the walk expanded, which is the work of
//!   building the plan set once (the visited set is what keeps it a graph, so
//!   no permutation of the same choices is expanded twice);
//! * **kept**: how many of the end states an unordered walk finds survive the
//!   canonical trash order, which is the one approximation in the design;
//! * **pruned**: end states left after dropping mixes no buyer wants, which is
//!   `final_turn`'s test and only sound on the game's last turn.
//!
//! ```sh
//! cargo run --release -p colori-core --example turn_plan_census -- [players] [seeds] [budget] [plan cap]
//! ```
//!
//! `budget` bounds the total turn positions expanded, so a pathological turn
//! cannot run away with the machine.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use colori_core::choices::enumerate_choices;
use colori_core::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::setup::create_initial_game_state;
use colori_core::turn_plan::{enumerate_turn_plans, PlanLimits, TurnPlans};
use colori_core::types::{GamePhase, GameState};
use rand::{RngExt, SeedableRng};
use wyrand::WyRand;

/// The most end states one turn may produce before the walk is cut short,
/// unless the command line names another.
const DEFAULT_PLAN_CAP: usize = 20_000;
/// The most positions the unordered walk may expand in one turn. It is the
/// measurement of what the canonical order costs, not the thing being
/// measured, so it gets a bound of its own instead of the census budget.
const ANY_ORDER_CAP: usize = 20_000;

#[derive(Default)]
struct Sample {
    plans: usize,
    positions: usize,
    longest: usize,
    mean_length: f64,
    any_order_plans: usize,
    any_order_truncated: bool,
    pruned_plans: usize,
}

struct Census {
    /// The start of `round`'s action phase, for one seat.
    samples: BTreeMap<u32, Vec<Sample>>,
    positions_spent: usize,
    budget: usize,
    plan_cap: usize,
    truncated: usize,
}

impl Census {
    fn exhausted(&self) -> bool {
        self.positions_spent >= self.budget
    }

    fn sample(&mut self, state: &GameState, player: usize, round: u32) {
        if self.exhausted() {
            return;
        }
        let left = self.budget - self.positions_spent;
        let canonical = self.walk(state, player, PlanLimits::capped(self.plan_cap, left));
        let any_order = self.walk(
            state,
            player,
            PlanLimits {
                canonical_trash_order: false,
                ..PlanLimits::capped(self.plan_cap, ANY_ORDER_CAP)
            },
        );
        let pruned = self.walk(
            state,
            player,
            PlanLimits {
                prune_unneeded_mixes: true,
                ..PlanLimits::capped(self.plan_cap, self.budget - self.positions_spent)
            },
        );
        if canonical.truncated {
            self.truncated += 1;
        }
        self.samples.entry(round).or_default().push(Sample {
            plans: canonical.plans.len(),
            positions: canonical.positions,
            longest: canonical.longest(),
            mean_length: canonical.mean_length(),
            any_order_plans: any_order.plans.len(),
            any_order_truncated: any_order.truncated,
            pruned_plans: pruned.plans.len(),
        });
    }

    fn walk(&mut self, state: &GameState, player: usize, limits: PlanLimits) -> TurnPlans {
        let found = enumerate_turn_plans(state, player, limits);
        self.positions_spent += found.positions;
        found
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |i: usize, default: usize| -> usize {
        args.get(i)
            .map(|arg| arg.parse().unwrap_or_else(|_| panic!("{arg:?} is not a number")))
            .unwrap_or(default)
    };
    let players = number(0, 3);
    let seeds = number(1, 2);
    let budget = number(2, 3_000_000);
    let plan_cap = number(3, DEFAULT_PLAN_CAP);
    assert!((1..=4).contains(&players), "Colori seats 1 to 4 players");

    let mut census = Census {
        samples: BTreeMap::new(),
        positions_spent: 0,
        budget,
        plan_cap,
        truncated: 0,
    };

    for seed in 0..seeds as u64 {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(players, &vec![true; players], &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        // A round reaches its action phase through the buyers and the draft,
        // so the first state of a round seen in that phase is its start. A
        // seat's turn stays open for several choices, so "nobody has ended
        // their turn" would also catch mid-turn positions.
        let mut sampled_round = 0;
        while census.positions_spent < budget {
            if matches!(state.phase, GamePhase::GameOver) {
                break;
            }
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            if matches!(state.phase, GamePhase::Action { .. }) && state.round != sampled_round {
                sampled_round = state.round;
                let round = state.round;
                for player in 0..players {
                    census.sample(&state, player, round);
                }
            }
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                break;
            };
            let choices = enumerate_choices(&state, player_index);
            let choice = choices[rng.random_range(0..choices.len())];
            apply_choice_to_state(&mut state, player_index, &choice, &mut rng);
        }
    }

    report(&census, players, seeds, budget);
}

fn median(values: &mut [usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn percentile(values: &mut [usize], percent: usize) -> usize {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let index = (values.len() - 1) * percent / 100;
    values[index]
}

fn report(census: &Census, players: usize, seeds: usize, budget: usize) {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{players} players, {seeds} seeds, budget {budget} turn positions, plan cap {}\n",
        census.plan_cap
    );
    let _ = writeln!(
        out,
        "{:>5}  {:>5}  {:>10}  {:>10}  {:>9}  {:>6}  {:>14}  {:>7}  {:>8}  {:>10}",
        "round",
        "seats",
        "plans(med)",
        "plans(p90)",
        "plans(max)",
        "kept%",
        "positions(med)",
        "longest",
        "mean-len",
        "pruned(med)"
    );
    let _ = writeln!(out, "{}", "-".repeat(104));

    let mut all_plans = Vec::new();
    let mut all_positions = Vec::new();
    let mut all_pruned = Vec::new();
    let mut all_kept = Vec::new();
    for (round, samples) in &census.samples {
        let plans: Vec<usize> = samples.iter().map(|s| s.plans).collect();
        let positions: Vec<usize> = samples.iter().map(|s| s.positions).collect();
        let pruned: Vec<usize> = samples.iter().map(|s| s.pruned_plans).collect();
        let kept: Vec<usize> = samples
            .iter()
            .filter(|s| !s.any_order_truncated)
            .map(|s| if s.any_order_plans == 0 { 100 } else { s.plans * 100 / s.any_order_plans })
            .collect();
        let longest = samples.iter().map(|s| s.longest).max().unwrap_or(0);
        let mean = samples.iter().map(|s| s.mean_length).sum::<f64>() / samples.len() as f64;
        let _ = writeln!(
            out,
            "{round:>5}  {:>5}  {:>10}  {:>10}  {:>9}  {:>5}%  {:>14}  {:>7}  {mean:>8.1}  {:>10}",
            samples.len(),
            median(&mut plans.clone()),
            percentile(&mut plans.clone(), 90),
            plans.iter().max().copied().unwrap_or(0),
            median(&mut kept.clone()),
            median(&mut positions.clone()),
            longest,
            median(&mut pruned.clone()),
        );
        all_plans.extend(plans);
        all_positions.extend(positions);
        all_pruned.extend(pruned);
        all_kept.extend(kept);
    }

    let _ = writeln!(out, "{}", "-".repeat(104));
    let _ = writeln!(
        out,
        "plans      median {}  p90 {}  max {}",
        median(&mut all_plans.clone()),
        percentile(&mut all_plans.clone(), 90),
        all_plans.iter().max().copied().unwrap_or(0),
    );
    let _ = writeln!(
        out,
        "positions  median {}  p90 {}  max {}  ({} expanded in all)",
        median(&mut all_positions.clone()),
        percentile(&mut all_positions.clone(), 90),
        all_positions.iter().max().copied().unwrap_or(0),
        census.positions_spent,
    );
    let _ = writeln!(
        out,
        "canonical order keeps {}% of the end states (median)",
        median(&mut all_kept),
    );
    let _ = writeln!(
        out,
        "needed-mixes pruning leaves a median of {} plans",
        median(&mut all_pruned),
    );
    if census.truncated > 0 {
        let _ = writeln!(
            out,
            "{} walks hit the {} plan cap",
            census.truncated, census.plan_cap
        );
    }
    print!("{out}");
}
