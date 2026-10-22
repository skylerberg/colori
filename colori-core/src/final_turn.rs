//! The game's last turn, solved exactly.
//!
//! Once the last draft is over, nothing is hidden and nothing is random: a turn
//! draws no cards, and no turn touches anyone else's cards, colors, materials
//! or buyers. What a player can still do is fixed, and so is what it scores,
//! whatever anyone else does. So their last turn has a best line, found here
//! outright rather than estimated: the one that ends with them ranked highest,
//! by points, then buyers, then colors, as the game breaks ties. Everyone
//! else's result is out of their hands, so no other line wins a game this one
//! would lose.
//!
//! A turn can reach millions of positions, nearly all of them mixes that make
//! nothing a buyer wants or gold_coins spent on nothing that sells. The first are
//! never tried, since no best line needs them, and the second are bounded: a
//! position is searched only while what it could still score beats the best
//! line found so far.

use rand::SeedableRng;
use rustc_hash::FxHashSet;
use wyrand::WyRand;

use crate::apply_choice::apply_choice;
use crate::choices::enumerate_choices;
use crate::colors::{
    can_supply_mix, mix_result, unmix_components, PRIMARIES, SECONDARIES, TERTIARIES,
    VALID_MIX_PAIRS,
};
use crate::scoring::player_ranking;
use crate::types::{
    Ability, AbilityStack, Choice, Color, GoldCoinPurchase, GamePhase, GameState, PlayerState,
    BuyerInstance, MAX_BUYERS, NUM_COLORS,
};
use crate::unordered_cards::UnorderedCards;

type Ranking = (u32, usize, u32);

/// `player`'s best choice, if this is their last turn of the game.
///
/// Where several lines rank the same, ending the turn is preferred, so a
/// player who can gain nothing more stops.
pub fn solve(state: &GameState, player: usize) -> Option<Choice> {
    let last_turn = state.round == state.max_rounds
        && matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player].ended);
    if !last_turn {
        return None;
    }

    let root = state.clone();
    let mut solver = Solver {
        player,
        best: None,
        searched: FxHashSet::default(),
        rng: WyRand::seed_from_u64(0),
    };
    let mut choice = None;
    for candidate in worth_trying(&root, player) {
        let before = solver.best;
        solver.try_choice(&root, &candidate);
        if choice.is_none() || solver.best > before {
            choice = Some(candidate);
        }
    }
    choice
}

/// A branch and bound over the rest of the turn.
struct Solver {
    player: usize,
    /// The best ranking any line found so far ends with.
    best: Option<Ranking>,
    /// Positions already searched. No line from one beats `best`, which only
    /// rises, so none needs searching again however it is reached.
    searched: FxHashSet<Position>,
    /// Nothing in a turn is random, so this decides nothing.
    rng: WyRand,
}

impl Solver {
    fn search(&mut self, state: &GameState) {
        if self
            .best
            .is_some_and(|best| ceiling(state, self.player) <= best)
            || !self.searched.insert(Position::of(state, self.player))
        {
            return;
        }
        for choice in worth_trying(state, self.player) {
            self.try_choice(state, &choice);
        }
    }

    fn try_choice(&mut self, state: &GameState, choice: &Choice) {
        if matches!(choice, Choice::EndTurn { .. }) {
            // Ending the turn moves cards about, but nothing that ranks.
            let ranking = player_ranking(&state.players[self.player]);
            self.best = self.best.max(Some(ranking));
        } else {
            let mut next = state.clone();
            apply_choice(&mut next, self.player, choice, &mut self.rng);
            self.search(&next);
        }
    }
}

/// What decides the rest of a turn and where it leaves the player, kept whole
/// so that positions merge only when they are the same. A choice names cards by
/// type and takes the lowest-numbered copy, so whichever order reaches a
/// position holds the same copies of each. What was sold follows from which
/// buyers are left; the deck, the round and the cards already workshopped play
/// no further part.
#[derive(PartialEq, Eq, Hash)]
struct Position {
    pending: AbilityStack,
    workshop: UnorderedCards,
    drafted: UnorderedCards,
    wheel: [u8; NUM_COLORS],
    materials: [u32; 3],
    gold_coins: u32,
    wilds: u32,
    boats: u32,
    buyers: [Option<BuyerInstance>; MAX_BUYERS],
}

impl Position {
    fn of(state: &GameState, player_index: usize) -> Self {
        let player = &state.players[player_index];
        let GamePhase::Action { action_state } = &state.phase else {
            unreachable!("a turn is played in the action phase");
        };
        let mut buyers = [None; MAX_BUYERS];
        for (slot, buyer) in buyers.iter_mut().zip(player.buyers.iter()) {
            *slot = Some(*buyer);
        }
        Position {
            pending: action_state.turns[player_index].ability_stack.clone(),
            workshop: player.workshop_cards,
            drafted: player.top_cards,
            wheel: player.color_wheel.counts,
            materials: player.materials_wheel.counts,
            gold_coins: player.gold_coins,
            wilds: player.bowls,
            boats: player.boats,
            buyers,
        }
    }
}

/// A ranking no line from `state` can beat.
///
/// For each set of buyers the player might still sell to, it charges what the
/// sales must cost at the least: a gold_coin for each sale beyond the Sells that
/// are free and each mix beyond the mixes that are, a boat for each material
/// short, and a bowl for each primary short. Everything the workshop
/// could still give is counted as gained, and nothing as spent on getting it.
fn ceiling(state: &GameState, player_index: usize) -> Ranking {
    let player = &state.players[player_index];
    let (mut points, sold, mut colors) = player_ranking(player);

    let (mut gold_coins, mut wilds, mut boats) = (player.gold_coins, player.bowls, player.boats);
    let (mut free_sells, mut free_mixes, mut moves, mut any_secondaries) = (0, 0, 0, 0);
    let pending = match &state.phase {
        GamePhase::Action { action_state } => {
            action_state.turns[player_index].ability_stack.as_slice()
        }
        _ => &[],
    };
    let drafted = player
        .top_cards
        .iter()
        .map(|id| state.card_lookup[id as usize]);
    let workshop = || {
        player
            .workshop_cards
            .iter()
            .map(|id| state.card_lookup[id as usize])
    };
    let abilities = pending
        .iter()
        .chain(drafted.flat_map(|card| card.top_actions()))
        .chain(workshop().flat_map(|card| card.workshop_abilities()));
    for &ability in abilities {
        match ability {
            Ability::Sell => free_sells += 1,
            Ability::MixColors { count } => free_mixes += count,
            Ability::LiftCard => moves += 1,
            Ability::GainGoldCoins { count } => {
                points += 2 * count;
                gold_coins += count;
            }
            Ability::GainBowls { count } => {
                points += count;
                wilds += count;
            }
            Ability::GainBoats { count } => {
                points += count;
                boats += count;
            }
            Ability::GainSecondary => {
                any_secondaries += 1;
                colors += 1;
            }
            Ability::Workshop { .. } => {}
        }
    }

    let mut supply = player.color_wheel.counts.map(u32::from);
    let mut materials = player.materials_wheel.counts;
    let (mut movable_sells, mut movable_mixes) = (0, 0);
    for card in workshop() {
        for &color in card.colors() {
            supply[color.index()] += 1;
            colors += 1;
        }
        for &material in card.material_types() {
            materials[material as usize] += 1;
        }
        for &ability in card.top_actions() {
            match ability {
                Ability::Sell => movable_sells += 1,
                Ability::MixColors { count } => movable_mixes += count,
                _ => {}
            }
        }
    }
    free_sells += moves.min(movable_sells);
    if moves > 0 {
        free_mixes += movable_mixes;
    }

    let buyers = &player.buyers[..];
    let (mut most_points, mut most_sales) = (0, 0);
    for subset in 1u32..1 << buyers.len() {
        let mut wanted = [0u32; NUM_COLORS];
        let mut wanted_materials = [0u32; 3];
        let mut boats_spent = 0;
        let mut worth = 0;
        for (_, buyer) in buyers
            .iter()
            .enumerate()
            .filter(|(i, _)| subset >> i & 1 == 1)
        {
            let stats = buyer.buyer.stats();
            for &(color, count) in stats.colors() {
                wanted[color.index()] += count as u32;
            }
            for (w, &m) in wanted_materials.iter_mut().zip(&stats.material_units) {
                *w += m as u32;
            }
            boats_spent += stats.boats;
            worth += 2 * stats.points;
        }
        boats_spent += wanted_materials
            .iter()
            .zip(&materials)
            .map(|(w, have)| w.saturating_sub(*have))
            .sum::<u32>();

        // A tertiary short takes a mix, which takes a secondary; a secondary
        // short takes a mix too, unless a secondary of choice covers it.
        let mut made = [0u32; NUM_COLORS];
        for tertiary in TERTIARIES {
            let t = tertiary.index();
            made[t] = wanted[t].saturating_sub(supply[t]);
            let (primary, secondary) = unmix_components(tertiary);
            wanted[primary.index()] += made[t];
            wanted[secondary.index()] += made[t];
        }
        for secondary in SECONDARIES {
            let s = secondary.index();
            made[s] = wanted[s].saturating_sub(supply[s]);
            let unless_chosen = made[s].saturating_sub(any_secondaries);
            let (a, b) = unmix_components(secondary);
            wanted[a.index()] += unless_chosen;
            wanted[b.index()] += unless_chosen;
        }
        let secondaries_short: u32 = SECONDARIES.iter().map(|s| made[s.index()]).sum();
        let mixes = TERTIARIES.iter().map(|t| made[t.index()]).sum::<u32>()
            + secondaries_short.saturating_sub(any_secondaries);
        let wilds_spent: u32 = PRIMARIES
            .iter()
            .map(|p| wanted[p.index()].saturating_sub(supply[p.index()]))
            .sum();

        let sales = subset.count_ones();
        let gold_coins_spent = sales.saturating_sub(free_sells) + mixes.saturating_sub(free_mixes);
        if boats_spent > boats || wilds_spent > wilds || gold_coins_spent > gold_coins {
            continue;
        }
        let spent = 2 * gold_coins_spent + boats_spent + wilds_spent;
        most_points = most_points.max(worth.saturating_sub(spent));
        most_sales = most_sales.max(sales as usize);
    }
    (points + most_points, sold + most_sales, colors)
}

/// The choices that could be part of a best line.
fn worth_trying(state: &GameState, player_index: usize) -> Vec<Choice> {
    let mut choices = enumerate_choices(state, player_index);
    let player = &state.players[player_index];
    let need = mix_needs(player);
    let mut kept_a_surplus_secondary = false;
    choices.retain(|choice| match choice {
        Choice::MixAll { mixes } => mixes_are_needed(mixes, &need),
        // A trash that mixes nothing, sells to nobody or moves nothing does
        // nothing at all.
        Choice::TrashAndMix { mixes, .. } => !mixes.is_empty() && mixes_are_needed(mixes, &need),
        Choice::TrashTopCard { card } => {
            // A Buyer card nobody can buy from trashes for nothing; a card that
            // trashes for what it gives is worth taking.
            !matches!(card.top_actions()[0], Ability::Sell)
        }
        Choice::TrashAndLift { target: None, .. } => false,

        Choice::SpendGoldCoin {
            purchase: GoldCoinPurchase::MixColors,
        } => VALID_MIX_PAIRS.iter().any(|&(a, b)| {
            need[mix_result(a, b).index()] > 0
                && can_supply_mix(&player.color_wheel, player.bowls, a, b)
        }),
        Choice::GainSecondary { color } => {
            if need[color.index()] > 0 {
                true
            } else if SECONDARIES.iter().any(|s| need[s.index()] > 0) {
                false
            } else {
                // Every secondary is surplus, so which one makes no difference.
                !std::mem::replace(&mut kept_a_surplus_secondary, true)
            }
        }
        _ => true,
    });
    // Sales first, so a good line is found early and bounds the rest.
    choices.sort_by_key(|choice| match choice {
        Choice::EndTurn { .. } => 0,
        Choice::TrashAndSell { .. } | Choice::SellToBuyer { .. } => 1,
        Choice::SpendGoldCoin {
            purchase: GoldCoinPurchase::Sell,
        } => 2,
        Choice::SpendGoldCoin { .. } => 4,
        _ => 3,
    });
    choices
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
/// does nothing else, or from a gold_coin. And a primary kept back can only pay
/// where a bowl would have, which keeps the wild. Primaries are never
/// made, so they have no entry.
fn mix_needs(player: &PlayerState) -> [u8; NUM_COLORS] {
    let mut wanted = [0u8; NUM_COLORS];
    for buyer in player.buyers.iter() {
        for &(color, count) in buyer.buyer.stats().colors() {
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
fn mixes_are_needed(mixes: &[(Color, Color)], need: &[u8; NUM_COLORS]) -> bool {
    let mut made = [0u8; NUM_COLORS];
    mixes.iter().all(|&(a, b)| {
        let product = mix_result(a, b).index();
        made[product] += 1;
        made[product] <= need[product]
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::{RngExt, SeedableRng};
    use wyrand::WyRand;

    use super::*;
    use crate::cards::{draft_deck_cards, starter_deck};
    use crate::apply_choice::apply_choice;
    use crate::colori_game::{get_game_status, GameStatus};
    use crate::setup::create_initial_game_state;
    use crate::types::{Card, WorkshopCards};

    fn taking_turn(state: &GameState, player: usize) -> bool {
        matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player].ended)
    }

    /// Random play from a fresh game of `rounds` rounds to its last action phase.
    fn last_turn(players: usize, rounds: u32, seed: u64) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(players, &vec![true; players], &mut rng);
        state.max_rounds = rounds;
        while !(state.round == rounds && matches!(state.phase, GamePhase::Action { .. })) {
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                unreachable!("the game ends after its last action phase");
            };
            let choices = enumerate_choices(&state, player_index);
            let choice = choices[rng.random_range(0..choices.len())];
            apply_choice(&mut state, player_index, &choice, &mut rng);
        }
        state
    }

    /// Every choice the game allows, including the workshop subsets the
    /// solver's pruning sets aside. The brute force must not share the pruning
    /// it is checking, so it widens the workshop choices itself: any non-empty
    /// subset up to the picks granted, plus taking none.
    fn all_choices(state: &GameState, player: usize) -> Vec<Choice> {
        let cards: Vec<Card> = state.players[player]
            .workshop_cards
            .iter()
            .map(|id| state.card_lookup[id as usize])
            .collect();
        let subsets = |count: u32| -> Vec<WorkshopCards> {
            let mut out = Vec::new();
            for mask in 1u32..1 << cards.len() {
                if mask.count_ones() > count {
                    continue;
                }
                let mut picked = WorkshopCards::new();
                for (i, &card) in cards.iter().enumerate() {
                    if mask >> i & 1 == 1 {
                        picked.push(card);
                    }
                }
                out.push(picked);
            }
            out
        };
        let pending_workshop = match &state.phase {
            GamePhase::Action { action_state } => match action_state.turns[player].ability_stack.last() {
                Some(Ability::Workshop { count }) => Some(*count),
                _ => None,
            },
            _ => None,
        };

        let mut choices = Vec::new();
        for choice in enumerate_choices(state, player) {
            match choice {
                Choice::Workshop { .. } => {
                    let count = pending_workshop.expect("a workshop choice answers a workshop");
                    choices.push(Choice::SkipWorkshop);
                    choices.extend(
                        subsets(count)
                            .into_iter()
                            .map(|card_types| Choice::Workshop { card_types }),
                    );
                }
                Choice::TrashAndWorkshop { card, .. } => {
                    let Ability::Workshop { count } = card.top_actions()[0] else {
                        unreachable!("a workshop follows only a card that grants one")
                    };
                    choices.push(Choice::TrashAndWorkshop {
                        card,
                        workshop_cards: WorkshopCards::new(),
                    });
                    choices.extend(subsets(count).into_iter().map(|workshop_cards| {
                        Choice::TrashAndWorkshop { card, workshop_cards }
                    }));
                }
                other => choices.push(other),
            }
        }
        // Two copies of a card make the same subset more than once.
        let mut unique: Vec<Choice> = Vec::with_capacity(choices.len());
        for choice in choices {
            if !unique.contains(&choice) {
                unique.push(choice);
            }
        }
        unique
    }

    /// The best ranking any line of legal choices ends with, found by trying
    /// them all, or `None` if that takes more than `budget` choices. Checks on
    /// the way that no position's ceiling is below it.
    fn exhaustive(
        state: &GameState,
        player: usize,
        best: &mut HashMap<u128, Ranking>,
        budget: &mut u32,
    ) -> Option<Ranking> {
        let key = state.own_key(player);
        if let Some(&ranking) = best.get(&key) {
            return Some(ranking);
        }
        let mut rng = WyRand::seed_from_u64(0);
        let mut ranking = (0, 0, 0);
        for choice in all_choices(state, player) {
            *budget = budget.checked_sub(1)?;
            let after = if matches!(choice, Choice::EndTurn { .. }) {
                player_ranking(&state.players[player])
            } else {
                let mut next = state.clone();
                apply_choice(&mut next, player, &choice, &mut rng);
                exhaustive(&next, player, best, budget)?
            };
            ranking = ranking.max(after);
        }
        assert!(
            ceiling(state, player) >= ranking,
            "the ceiling {:?} is below {ranking:?}, which a line reaches",
            ceiling(state, player)
        );
        best.insert(key, ranking);
        Some(ranking)
    }

    /// Play `player`'s turn out the way `solve` says to.
    fn play_solved(mut state: GameState, player: usize) -> (Ranking, Vec<Choice>) {
        let mut rng = WyRand::seed_from_u64(0);
        let mut line = Vec::new();
        while taking_turn(&state, player) {
            let choice = solve(&state, player).expect("the last turn is solved");
            apply_choice(&mut state, player, &choice, &mut rng);
            line.push(choice);
        }
        (player_ranking(&state.players[player]), line)
    }

    /// The pruning and the bound only ever set aside lines no better than one
    /// the solver keeps, so its line ends as well as the best of every legal
    /// line.
    ///
    /// Trying every line of a whole turn is too slow for a debug build, so this
    /// checks each position down a random line of the turn that can be tried
    /// in full quickly: mostly later ones, many with abilities half resolved.
    #[test]
    fn the_solved_line_ranks_with_the_best_of_every_line() {
        let mut checked = 0;
        for seed in 0..20 {
            let players = 2 + seed as usize % 3;
            let rounds = 2 + seed as u32 % 3;
            let state = last_turn(players, rounds, seed);
            let mut rng = WyRand::seed_from_u64(seed);
            for player in 0..players {
                let mut position = state.clone();
                while taking_turn(&position, player) {
                    if let Some(best) =
                        exhaustive(&position, player, &mut HashMap::new(), &mut 2_000)
                    {
                        let (solved, line) = play_solved(position.clone(), player);
                        assert_eq!(solved, best, "seed {seed} player {player} played {line:?}");
                        checked += 1;
                    }
                    let mut choices = enumerate_choices(&position, player);
                    if choices.len() > 1 {
                        choices.retain(|choice| !matches!(choice, Choice::EndTurn { .. }));
                    }
                    let choice = choices[rng.random_range(0..choices.len())];
                    apply_choice(&mut position, player, &choice, &mut rng);
                }
            }
        }
        assert!(checked >= 150, "only {checked} positions were checked");
    }

    #[test]
    fn only_the_last_turn_is_solved() {
        let state = last_turn(3, 3, 5);
        assert!(solve(&state, 1).is_some());

        let mut earlier = state.clone();
        earlier.max_rounds += 1;
        assert!(
            solve(&earlier, 1).is_none(),
            "another round is still to come"
        );

        let mut ended = state.clone();
        let mut rng = WyRand::seed_from_u64(0);
        while taking_turn(&ended, 1) {
            let choice = solve(&ended, 1).unwrap();
            apply_choice(&mut ended, 1, &choice, &mut rng);
        }
        assert!(solve(&ended, 1).is_none(), "the turn is over");
    }

    #[test]
    fn a_player_with_nothing_to_gain_ends_their_turn() {
        let mut state = last_turn(2, 3, 9);
        let player = &mut state.players[0];
        player.buyers = Default::default();
        player.top_cards = Default::default();
        player.gold_coins = 3;
        player.cached_whole_points += 3;
        assert!(
            !player.workshop_cards.is_empty(),
            "a gold_coin could buy a workshop"
        );
        assert_eq!(solve(&state, 0), Some(Choice::EndTurn { keep_remaining: true }));
    }

    /// What the pruning takes on trust about the cards: a Sell is the only
    /// thing the card it comes from does, so no sale ever needs dodging, and
    /// trashing a card to mix nothing or move nothing does nothing else.
    #[test]
    fn the_cards_do_what_the_pruning_assumes() {
        let cards = starter_deck()
            .into_iter()
            .chain(draft_deck_cards().map(|(card, _)| card));
        for card in cards {
            let trash = card.top_actions();
            let stands_alone = trash
                .iter()
                .any(|ability| matches!(ability, Ability::Sell | Ability::LiftCard))
                || matches!(trash.first(), Some(Ability::MixColors { .. }));
            assert!(
                !stands_alone || trash.len() == 1,
                "{card:?} trashes for {trash:?}"
            );
            assert!(
                !card.workshop_abilities().contains(&Ability::Sell),
                "{card:?} sells when workshopped"
            );
        }
    }
}
