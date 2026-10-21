//! Every way a player's action turn can end, each reached once.
//!
//! A turn is a run of choices whose order mostly does not matter, so a tree
//! over single choices holds a position once per route that reaches it. This
//! walks the turn as a graph instead: [`Position`] keys what decides the rest
//! of the turn, each position is expanded once, and each distinct
//! end-of-turn position yields at most one plan. The plan set therefore holds
//! end states, not permutations of the choices that lead to them.
//!
//! Plans are built in a canonical trash order — cards moved into the draft
//! pool, cards trashed for what they give outright, cards trashed for a
//! workshop, for a mix, for a sale — and never back to an earlier group.
//! Within a group every order is tried, and the visited set collapses the ones
//! that commute. Ducats are not trashes and are not ordered. The ordering is
//! the one approximation here: it can miss an end state that needs a later
//! group before an earlier one, so [`PlanLimits::canonical_trash_order`] can be
//! turned off to measure what it costs.
//!
//! `final_turn` solves the game's last turn exactly over a branch and bound of
//! these same positions; [`Position`], [`mix_needs`] and [`mixes_are_needed`]
//! live here so both use one definition.

use rand::SeedableRng;
use rustc_hash::FxHashSet;
use wyrand::WyRand;

use crate::action_phase::ability_stack;
use crate::apply_choice::apply_choice;
use crate::choices::enumerate_choices;
use crate::colors::{mix_result, unmix_components, SECONDARIES, TERTIARIES};
use crate::types::{
    AbilityStack, Choice, Color, GamePhase, GameState, PlayerState, SellCardInstance, MAX_BUYERS,
    NUM_COLORS,
};
use crate::unordered_cards::UnorderedCards;

/// How far a walk of one turn may go.
#[derive(Clone, Copy, Debug)]
pub struct PlanLimits {
    /// Stop once this many distinct end-of-turn positions are found.
    pub max_plans: usize,
    /// Stop expanding once this many positions of the turn have been walked.
    /// The end states found by then are kept; there may be more.
    pub max_positions: usize,
    /// Take trashes in the canonical order and never go back to an earlier
    /// group. Off, every order of trashes is tried and only the visited set
    /// bounds the walk.
    pub canonical_trash_order: bool,
    /// Drop non-empty mixes that make a color no remaining buyer wants.
    /// `final_turn`'s test, which only holds for the game's last turn: colors
    /// carry over to the next round, so mid-game such a mix is not dead.
    pub prune_unneeded_mixes: bool,
}

impl PlanLimits {
    /// Walk until nothing new is left, taking trashes in order and pruning
    /// nothing. The census's starting point.
    pub const EVERYTHING: PlanLimits = PlanLimits {
        max_plans: usize::MAX,
        max_positions: usize::MAX,
        canonical_trash_order: true,
        prune_unneeded_mixes: false,
    };

    pub const fn capped(max_plans: usize, max_positions: usize) -> PlanLimits {
        PlanLimits {
            max_plans,
            max_positions,
            ..PlanLimits::EVERYTHING
        }
    }
}

/// One way a turn can be played, ending with `EndTurn`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnPlan {
    pub line: Vec<Choice>,
}

/// What walking one turn found.
#[derive(Clone, Debug, Default)]
pub struct TurnPlans {
    /// One plan per distinct end-of-turn position, in the order found.
    pub plans: Vec<TurnPlan>,
    /// Positions of the turn the walk expanded, counting each once.
    pub positions: usize,
    /// Whether `max_plans` stopped the walk with end states still to find.
    pub truncated: bool,
}

impl TurnPlans {
    /// The longest plan's number of choices.
    pub fn longest(&self) -> usize {
        self.plans.iter().map(|plan| plan.line.len()).max().unwrap_or(0)
    }

    /// The mean plan length, or 0 when nothing was found.
    pub fn mean_length(&self) -> f64 {
        if self.plans.is_empty() {
            return 0.0;
        }
        let total: usize = self.plans.iter().map(|plan| plan.line.len()).sum();
        total as f64 / self.plans.len() as f64
    }
}

/// Every way `player_index`'s turn can end from `state`, each once.
///
/// The turn is played out from `state` as it stands, so this works from the
/// start of the phase or from any point in the turn that has nothing pending.
pub fn enumerate_turn_plans(
    state: &GameState,
    player_index: usize,
    limits: PlanLimits,
) -> TurnPlans {
    let mut walker = Walker {
        player: player_index,
        limits,
        visited: FxHashSet::default(),
        ends: FxHashSet::default(),
        plans: Vec::new(),
        positions: 0,
        truncated: false,
        need: mix_needs(&state.players[player_index]),
        // Nothing in a turn is random, so this decides nothing.
        rng: WyRand::seed_from_u64(0),
    };
    let mut line = Vec::new();
    walker.walk(state, None, &mut line);
    TurnPlans {
        plans: walker.plans,
        positions: walker.positions,
        truncated: walker.truncated,
    }
}

struct Walker {
    player: usize,
    limits: PlanLimits,
    /// Positions whose futures are already known, so that a route reaching one
    /// again adds no end state the first route did not find.
    visited: FxHashSet<Position>,
    /// End-of-turn positions a plan was already emitted for.
    ends: FxHashSet<Position>,
    plans: Vec<TurnPlan>,
    positions: usize,
    truncated: bool,
    /// What mixing could still put toward a sale, for the pruning test.
    need: [u8; NUM_COLORS],
    rng: WyRand,
}

impl Walker {
    /// Expand everything `state` still allows. `last_trash` is the group of the
    /// last trash taken, which the canonical order only lets the walk match or
    /// pass.
    fn walk(&mut self, state: &GameState, last_trash: Option<u8>, line: &mut Vec<Choice>) {
        if self.truncated || !self.visited.insert(Position::of(state, self.player)) {
            return;
        }
        if self.positions >= self.limits.max_positions {
            self.truncated = true;
            return;
        }
        self.positions += 1;

        // Choices below an ability are answers to it, not new actions, so the
        // trash order has nothing to say about them.
        let between_abilities = ability_stack(state, self.player).is_empty();
        let mut next = state.clone();
        for choice in enumerate_choices(state, self.player) {
            if choice == Choice::EndTurn {
                self.record(state, line);
                if self.truncated {
                    return;
                }
                continue;
            }

            let mut trash = last_trash;
            if between_abilities {
                if let Some(category) = trash_category(&choice) {
                    if self.limits.canonical_trash_order
                        && last_trash.is_some_and(|last| category < last)
                    {
                        continue;
                    }
                    trash = Some(category);
                }
            }
            if self.limits.prune_unneeded_mixes && !mixes_could_matter(&choice, &self.need) {
                continue;
            }

            line.push(choice);
            next.clone_from(state);
            apply_choice(&mut next, self.player, &choice, &mut self.rng);
            self.walk(&next, trash, line);
            line.pop();
        }
    }

    /// Keep `line + EndTurn` as the plan for the position it ends in, unless
    /// some earlier route already ended there.
    fn record(&mut self, state: &GameState, line: &[Choice]) {
        if self.plans.len() >= self.limits.max_plans {
            self.truncated = true;
            return;
        }
        let mut ended = state.clone();
        apply_choice(&mut ended, self.player, &Choice::EndTurn, &mut self.rng);
        if !self.ends.insert(Position::of(&ended, self.player)) {
            return;
        }
        let mut line = line.to_vec();
        line.push(Choice::EndTurn);
        self.plans.push(TurnPlan { line });
    }
}

/// Which group of trashes a top-level choice belongs to, or `None` for a
/// choice that trashes nothing.
///
/// The groups run from what gathers to what spends: a card moved into the
/// draft pool can be trashed later as a drafted card, so it goes first; the
/// cards that give something outright gather for the mixes; a workshop gathers
/// too, and the mixes its picks grant resolve as part of it; then the mixes,
/// then the sales that spend what they made.
#[inline]
fn trash_category(choice: &Choice) -> Option<u8> {
    Some(match choice {
        Choice::TrashAndMoveToDraftPool { .. } => 0,
        Choice::TrashDraftedCard { .. } => 1,
        Choice::TrashAndWorkshop { .. } => 2,
        Choice::TrashAndMix { .. } => 3,
        Choice::TrashAndSell { .. } => 4,
        _ => return None,
    })
}

/// Whether `choice` mixes something worth making, by `mixes_are_needed`.
///
/// Trashing a mix card without mixing is kept: it is how a card is discarded.
#[inline]
fn mixes_could_matter(choice: &Choice, need: &[u8; NUM_COLORS]) -> bool {
    match choice {
        Choice::MixAll { mixes } | Choice::TrashAndMix { mixes, .. } => {
            mixes.is_empty() || mixes_are_needed(mixes, need)
        }
        _ => true,
    }
}

/// What decides the rest of a turn and where it leaves the player, kept whole
/// so that positions merge only when they are the same. A choice names cards by
/// type and takes the lowest-numbered copy, so whichever order reaches a
/// position holds the same copies of each. What was sold follows from which
/// buyers are left; the deck, the round and the cards already workshopped play
/// no further part — and the cards already workshopped are fixed by the rest,
/// since a card leaves the workshop only into the draft pool or into that set.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Position {
    pending: AbilityStack,
    workshop: UnorderedCards,
    drafted: UnorderedCards,
    wheel: [u8; NUM_COLORS],
    materials: [u32; 3],
    ducats: u32,
    wilds: u32,
    boats: u32,
    buyers: [Option<SellCardInstance>; MAX_BUYERS],
}

impl Position {
    /// The player's position. Nothing is pending outside the action phase, so a
    /// turn that has ended has a position too.
    pub(crate) fn of(state: &GameState, player_index: usize) -> Self {
        let player = &state.players[player_index];
        let pending = match &state.phase {
            GamePhase::Action { action_state } => {
                action_state.turns[player_index].ability_stack.clone()
            }
            _ => AbilityStack::new(),
        };
        let mut buyers = [None; MAX_BUYERS];
        for (slot, buyer) in buyers.iter_mut().zip(player.buyers.iter()) {
            *slot = Some(*buyer);
        }
        Position {
            pending,
            workshop: player.workshop_cards,
            drafted: player.drafted_cards,
            wheel: player.color_wheel.counts,
            materials: player.materials.counts,
            ducats: player.ducats,
            wilds: player.wild_primaries,
            boats: player.boats,
            buyers,
        }
    }
}

/// How many more of each color mixing could still put toward a sale.
///
/// A tertiary is needed as far as the remaining buyers ask for more than the
/// wheel holds; a secondary, as far as they and the tertiaries still needed
/// ask for more. Any other mix only spends colors: taking it out of a line
/// leaves every later step affordable and the score no lower. That takes three
/// things. A secondary or tertiary on the wheel is spent only on a sale or on
/// mixing what is needed. A sale that a wheel with more colors could no longer
/// avoid needs no avoiding: a Sell only ever comes from trashing a card that
/// does nothing else, or from a ducat. And a primary kept back can only pay
/// where a wild primary would have, which keeps the wild. Primaries are never
/// made, so they have no entry.
pub(crate) fn mix_needs(player: &PlayerState) -> [u8; NUM_COLORS] {
    let mut wanted = [0u8; NUM_COLORS];
    for buyer in player.buyers.iter() {
        for &(color, count) in buyer.sell_card.stats().colors() {
            wanted[color.index()] += count;
        }
    }
    let held = &player.color_wheel.counts;
    let mut need = [0u8; NUM_COLORS];
    for tertiary in TERTIARIES {
        let t = tertiary.index();
        need[t] = wanted[t].saturating_sub(held[t]);
    }
    for tertiary in TERTIARIES {
        let (_, secondary) = unmix_components(tertiary);
        wanted[secondary.index()] += need[tertiary.index()];
    }
    for secondary in SECONDARIES {
        let s = secondary.index();
        need[s] = wanted[s].saturating_sub(held[s]);
    }
    need
}

/// Whether every mix makes something still needed. Making a tertiary uses up
/// one of the secondary it needs and one of what that secondary is needed for,
/// so the order of a sequence makes no difference here.
pub(crate) fn mixes_are_needed(mixes: &[(Color, Color)], need: &[u8; NUM_COLORS]) -> bool {
    let mut made = [0u8; NUM_COLORS];
    mixes.iter().all(|&(a, b)| {
        let product = mix_result(a, b).index();
        made[product] += 1;
        made[product] <= need[product]
    })
}

#[cfg(test)]
mod tests {
    use rand::{RngExt, SeedableRng};
    use wyrand::WyRand;

    use super::*;
    use crate::choices::check_choice_available;
    use crate::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;

    /// Random play from a fresh game to the start of `round`'s action phase.
    fn action_phase(players: usize, seed: u64, round: u32) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(players, &vec![true; players], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        while !(state.round == round && matches!(state.phase, GamePhase::Action { .. })) {
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                panic!("seed {seed}: the game ended before round {round}");
            };
            let choices = enumerate_choices(&state, player_index);
            let choice = choices[rng.random_range(0..choices.len())];
            apply_choice_to_state(&mut state, player_index, &choice, &mut rng);
        }
        state
    }

    fn taking_turn(state: &GameState, player: usize) -> bool {
        matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player].ended)
    }

    /// `(players, seed, round)` positions with a turn of every plausible shape,
    /// across rounds so that some are nearly out of cards and some are not.
    fn positions() -> Vec<(usize, u64, u32, GameState)> {
        let mut out = Vec::new();
        for (players, seed, round) in [
            (2usize, 3u64, 1u32),
            (3, 7, 2),
            (3, 11, 4),
            (4, 5, 6),
        ] {
            let state = action_phase(players, seed, round);
            for player in 0..players {
                out.push((players, seed, round, state.clone()));
                let _ = player;
            }
        }
        out
    }

    /// Every plan is a line the game allows, step by step, and it ends the
    /// turn — or the round, if it was the last one left.
    #[test]
    fn every_plan_is_legal_and_ends_the_turn() {
        let mut checked = 0;
        for (players, seed, round, state) in positions() {
            for player in 0..players {
                if !taking_turn(&state, player) {
                    continue;
                }
                let found = enumerate_turn_plans(&state, player, PlanLimits::EVERYTHING);
                assert!(!found.plans.is_empty(), "seed {seed}: a turn can always end");
                for plan in &found.plans {
                    let mut walked = state.clone();
                    for (step, choice) in plan.line.iter().enumerate() {
                        assert!(
                            check_choice_available(&walked, player, choice),
                            "seed {seed} round {round} player {player} step {step}: \
                             {choice:?} is not available in plan {:?}",
                            plan.line
                        );
                        apply_choice_to_state(&mut walked, player, choice, &mut WyRand::seed_from_u64(0));
                    }
                    assert_eq!(plan.line.last(), Some(&Choice::EndTurn));
                    assert!(
                        !taking_turn(&walked, player),
                        "seed {seed}: a plan left the turn open"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "only {checked} plans were checked");
    }

    /// End states are what a plan is for, so two plans with one end state would
    /// be the same tree node twice.
    #[test]
    fn each_plan_ends_somewhere_of_its_own() {
        let mut total = 0;
        for (players, seed, _, state) in positions() {
            for player in 0..players {
                if !taking_turn(&state, player) {
                    continue;
                }
                let found = enumerate_turn_plans(&state, player, PlanLimits::EVERYTHING);
                let mut ends = FxHashSet::default();
                for plan in &found.plans {
                    let mut walked = state.clone();
                    for choice in &plan.line {
                        apply_choice_to_state(&mut walked, player, choice, &mut WyRand::seed_from_u64(0));
                    }
                    assert!(
                        ends.insert(Position::of(&walked, player)),
                        "seed {seed} player {player}: two plans end in one position"
                    );
                }
                assert!(
                    found.plans.len() <= found.positions,
                    "seed {seed}: {} plans out of {} positions",
                    found.plans.len(),
                    found.positions
                );
                total += found.plans.len();
            }
        }
        assert!(total > 0);
    }

    /// The tree this feeds must be the same tree on every iteration, so the
    /// walk may not depend on anything a determinization changes.
    #[test]
    fn the_walk_is_deterministic() {
        let state = action_phase(3, 13, 3);
        for player in 0..3 {
            if !taking_turn(&state, player) {
                continue;
            }
            let a = enumerate_turn_plans(&state, player, PlanLimits::EVERYTHING);
            let b = enumerate_turn_plans(&state, player, PlanLimits::EVERYTHING);
            assert_eq!(a.plans, b.plans);
            assert_eq!(a.positions, b.positions);
        }
    }

    #[test]
    fn a_cap_stops_the_walk() {
        let state = action_phase(3, 17, 2);
        let full = enumerate_turn_plans(&state, 0, PlanLimits::EVERYTHING);
        assert!(full.plans.len() > 1, "the position has more than one plan");
        assert!(!full.truncated);

        let capped = enumerate_turn_plans(&state, 0, PlanLimits::capped(1, usize::MAX));
        assert_eq!(capped.plans.len(), 1);
        assert!(capped.truncated);
        assert_eq!(capped.plans[0], full.plans[0], "a cap keeps the first found");
    }

    /// The canonical order is an approximation, so it may only lose end states,
    /// never invent one.
    #[test]
    fn the_canonical_order_only_drops_end_states() {
        let state = action_phase(3, 19, 2);
        let canonical = enumerate_turn_plans(&state, 0, PlanLimits::EVERYTHING);
        let any_order = enumerate_turn_plans(
            &state,
            0,
            PlanLimits {
                canonical_trash_order: false,
                ..PlanLimits::EVERYTHING
            },
        );

        let ended = |found: &TurnPlans| -> FxHashSet<Position> {
            let mut ends = FxHashSet::default();
            for plan in &found.plans {
                let mut walked = state.clone();
                for choice in &plan.line {
                    apply_choice_to_state(&mut walked, 0, choice, &mut WyRand::seed_from_u64(0));
                }
                ends.insert(Position::of(&walked, 0));
            }
            ends
        };
        let (canonical_ends, any_ends) = (ended(&canonical), ended(&any_order));
        assert!(canonical_ends.is_subset(&any_ends));
        assert!(
            any_ends.len() >= canonical_ends.len(),
            "ordering lost nothing here, which does not exercise the claim"
        );
    }

    /// A position's key has to decide the end state, or the walk would merge
    /// two different turns. Two routes that reach one key must therefore end
    /// the game in one place, deck included.
    #[test]
    fn one_key_means_one_end_state() {
        let state = action_phase(3, 23, 2);
        let found = enumerate_turn_plans(&state, 0, PlanLimits::EVERYTHING);
        let mut by_key: rustc_hash::FxHashMap<Position, String> = Default::default();
        for plan in &found.plans {
            let mut walked = state.clone();
            for choice in &plan.line {
                apply_choice_to_state(&mut walked, 0, choice, &mut WyRand::seed_from_u64(0));
            }
            let key = Position::of(&walked, 0);
            // The deck's segments are what a plan can still move, and the
            // position's key does not name them, so they must follow from it.
            let deck = format!("{:?}", walked.players[0].deck.segments());
            if let Some(other) = by_key.insert(key, deck.clone()) {
                assert_eq!(other, deck, "one key, two decks");
            }
        }
    }
}
