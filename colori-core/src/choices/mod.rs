mod trash;
mod mix_sequences;
mod multiset;

use crate::action_phase::{
    can_afford_buyer, can_buy_with_gold_coin, can_sell_to_any_buyer, for_each_unique_card_type,
};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, can_supply_mix, perform_mix_unchecked, SECONDARIES};
use crate::types::*;

use trash::{enumerate_trash_choices, WorkshopTally};
use mix_sequences::enumerate_mix_sequences;
use multiset::{count_card_types, enumerate_multiset_subsets};

/// What deciding which workshop picks are worth taking turns on: how many
/// cards there are to take, and how many moves of a card into the top cards
/// the turn still has in it.
///
/// Nothing enters the workshop mid-turn, so holding a pick back only pays off
/// when a card in the top cards could still move the card it left behind into
/// the pool, to be used this turn or kept for the next. Each such move takes
/// one card out of the workshop, and a card it takes that moves cards itself
/// can move another, so a mover in the workshop counts too — once a drafted
/// mover is there to bring it in. Without one, no move can ever happen.
pub(crate) struct WorkshopPruning {
    available: usize,
    movers: usize,
}

impl WorkshopPruning {
    pub(crate) fn new(state: &GameState, player: &PlayerState) -> Self {
        let moves_to_pool = |id: &u8| {
            state.card_lookup[*id as usize]
                .top_actions()
                .contains(&Ability::LiftCard)
        };
        let drafted_movers = player.top_cards.iter().filter(moves_to_pool).count();
        let workshop_movers = if drafted_movers == 0 {
            0
        } else {
            player.workshop_cards.iter().filter(moves_to_pool).count()
        };
        Self {
            available: player.workshop_cards.len() as usize,
            movers: drafted_movers + workshop_movers,
        }
    }

    /// The picks worth taking: the fewest that leaves no card idle, up to the
    /// most the ability allows.
    ///
    /// At most `movers` cards can leave the workshop for the top cards, so a
    /// workspace smaller than `available - movers` picks leaves a card behind
    /// that nothing will ever move: it sits in the workshop and drops to the
    /// bottom of the deck. Taking it instead costs nothing, and gains its
    /// colors, materials and workshop abilities, so every such workspace is
    /// dominated by one that takes an idle card too.
    pub(crate) fn pick_range(&self, count: u32) -> (usize, usize) {
        let most = (count as usize).min(self.available);
        let fewest = most.min(self.available.saturating_sub(self.movers));
        (fewest, most)
    }
}

// ── Choice enumeration ──

/// What `player_index` may choose now. The buyers phase and the draft seat one
/// player at a time and offer nothing to anyone else; in the action phase
/// everyone still taking their turn has choices.
pub fn enumerate_choices_into(state: &GameState, player_index: usize, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Buyers { buyers_state } => {
            if buyers_state.current_player_index == player_index {
                enumerate_buyer_choices(state, choices);
            }
        }
        GamePhase::Draft { draft_state } => {
            if draft_state.current_player_index == player_index {
                let hand = draft_state.hands[player_index];
                for_each_unique_card_type(&hand, &state.card_lookup, |card| {
                    choices.push(Choice::DraftPick { card });
                });
            }
        }
        GamePhase::Action { action_state } => {
            let turn = &action_state.turns[player_index];
            if turn.ended {
                return;
            }
            let player = &state.players[player_index];

            match turn.ability_stack.last() {
                None => {
                    // Every top card is decided: used for its action, or
                    // kept on top of the deck. Keeping costs nothing and can
                    // be done at any point, so it is offered beside using.
                    let mut tally = WorkshopTally::default();
                    for_each_unique_card_type(&player.top_cards, &state.card_lookup, |card| {
                        enumerate_trash_choices(state, player, card, &mut tally, choices);
                        choices.push(Choice::KeepCard { card });
                    });
                    for purchase in ALL_GOLD_COIN_PURCHASES {
                        if can_buy_with_gold_coin(player, purchase) {
                            choices.push(Choice::SpendGoldCoin { purchase });
                        }
                    }
                    // The turn ends once nothing is left to decide, which is
                    // when the last gold_coins can still be spent on a purchase.
                    if player.top_cards.is_empty() {
                        choices.push(Choice::EndTurn);
                    }
                }
                Some(Ability::Workshop { count }) => {
                    let (fewest, most) = WorkshopPruning::new(state, player).pick_range(*count);
                    let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    if fewest == 0 {
                        // Every card left could still be moved, so forfeiting the
                        // picks is a real option — even when a gold_coin paid for
                        // them, since a search can begin, and the last turn be
                        // solved, at this position rather than at the purchase.
                        // It is also the only answer to an empty workshop, which
                        // `process_ability_stack` pops before anyone is asked.
                        choices.push(Choice::SkipWorkshop);
                    }
                    if most > 0 {
                        enumerate_multiset_subsets(
                            &card_types[..len],
                            &type_counts[..len],
                            fewest.max(1),
                            most,
                            &mut WorkshopCards::new(),
                            choices,
                            &|card_types| Choice::Workshop { card_types },
                        );
                    }
                }
                Some(Ability::LiftCard) => {
                    if player.workshop_cards.is_empty() {
                        choices.push(Choice::LiftCard { card: None });
                    } else {
                        for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |card| {
                            choices.push(Choice::LiftCard { card: Some(card) });
                        });
                    }
                }
                Some(Ability::MixColors { count }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        player.bowls,
                        *count,
                        choices,
                        |mixes| Choice::MixAll { mixes },
                    );
                }
                Some(Ability::Sell) => {
                    for buyer in player.buyers.iter() {
                        if can_afford_buyer(player, &buyer.buyer) {
                            choices.push(Choice::SellToBuyer {
                                buyer: buyer.buyer,
                            });
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(Choice::GainSecondary { color: c });
                    }
                }
                // Instant abilities (GainGoldCoins, GainBowls, GainPrimaries,
                // GainBoats) should never be on top when waiting for a choice —
                // they get processed immediately.
                Some(_) => {}
            }
        }
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState, player_index: usize) -> Vec<Choice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, player_index, &mut choices);
    choices
}

// ── Choice availability ──

/// Whether `mixes` can be made one after another, within the `granted` number
/// of mixes.
fn mix_sequence_is_valid(player: &PlayerState, mixes: &[(Color, Color)], granted: u32) -> bool {
    if mixes.len() as u32 > granted {
        return false;
    }
    let mut wheel = player.color_wheel.clone();
    let mut bowls = player.bowls;
    for &(a, b) in mixes {
        if !can_mix(a, b) || !can_supply_mix(&wheel, bowls, a, b) {
            return false;
        }
        perform_mix_unchecked(&mut wheel, &mut bowls, a, b);
    }
    true
}

/// The player's pending abilities, while they are still taking their action
/// turn.
fn open_turn_stack(state: &GameState, player_index: usize) -> Option<&AbilityStack> {
    match &state.phase {
        GamePhase::Action { action_state } => {
            let turn = &action_state.turns[player_index];
            (!turn.ended).then_some(&turn.ability_stack)
        }
        _ => None,
    }
}

/// The ability the player must answer next, if they are mid-turn and one is
/// waiting.
fn pending_ability(state: &GameState, player_index: usize) -> Option<Ability> {
    open_turn_stack(state, player_index).and_then(|stack| stack.last().copied())
}

/// Whether `target` is a legal answer to Move to Draft Pool. The move is
/// mandatory when the workshop has a card to move, so `None` stands only for
/// the fizzle.
fn lift_target_is_valid(state: &GameState, player: &PlayerState, target: &Option<Card>) -> bool {
    match target {
        None => player.workshop_cards.is_empty(),
        Some(card) => player
            .workshop_cards
            .iter()
            .any(|id| state.card_lookup[id as usize] == *card),
    }
}

/// Check common preconditions for all trash-drafted-card variants: the
/// player is between abilities and holds the card in their top cards.
fn check_trash_preconditions(state: &GameState, player_index: usize, card: &Card) -> bool {
    open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty())
        && state.players[player_index]
            .top_cards
            .iter()
            .any(|id| state.card_lookup[id as usize] == *card)
}

pub fn check_choice_available(state: &GameState, player_index: usize, choice: &Choice) -> bool {
    let player = &state.players[player_index];
    match choice {
        Choice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                draft_state.current_player_index == player_index
                    && draft_state.hands[player_index]
                        .iter()
                        .any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::TrashTopCard { card } => {
            if !check_trash_preconditions(state, player_index, card) {
                return false;
            }
            match card.top_actions()[0] {
                Ability::Sell => !can_sell_to_any_buyer(player),
                Ability::MixColors { .. } => false,
                Ability::Workshop { .. } => false,
                Ability::LiftCard => false,
                _ => true,
            }
        }
        Choice::EndTurn => {
            open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty())
                && player.top_cards.is_empty()
        }
        Choice::KeepCard { card } => check_trash_preconditions(state, player_index, card),
        Choice::Workshop { card_types } => {
            matches!(pending_ability(state, player_index), Some(Ability::Workshop { .. }))
                && !card_types.is_empty()
                && resolve_card_types_to_ids(card_types, &player.workshop_cards, &state.card_lookup).is_some()
        }
        Choice::SkipWorkshop => {
            matches!(pending_ability(state, player_index), Some(Ability::Workshop { .. }))
        }
        Choice::LiftCard { card } => {
            if !matches!(pending_ability(state, player_index), Some(Ability::LiftCard)) {
                return false;
            }
            lift_target_is_valid(state, player, card)
        }
        Choice::SellToBuyer { buyer } => {
            matches!(pending_ability(state, player_index), Some(Ability::Sell))
                && player.buyers.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
        }
        Choice::GainSecondary { color } => {
            matches!(pending_ability(state, player_index), Some(Ability::GainSecondary))
                && SECONDARIES.contains(color)
        }
        Choice::MixAll { mixes } => match pending_ability(state, player_index) {
            Some(Ability::MixColors { count }) => mix_sequence_is_valid(player, mixes, count),
            _ => false,
        },
        Choice::TrashAndMix { card, mixes } => {
            if !check_trash_preconditions(state, player_index, card) {
                return false;
            }
            let Ability::MixColors { count } = card.top_actions()[0] else {
                return false;
            };
            mix_sequence_is_valid(player, mixes, count)
        }
        Choice::TrashAndSell { card, buyer } => {
            check_trash_preconditions(state, player_index, card)
                && player.buyers.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
        }
        Choice::TrashAndWorkshop { card, workshop_cards } => {
            check_trash_preconditions(state, player_index, card)
                && (workshop_cards.is_empty()
                    || resolve_card_types_to_ids(workshop_cards, &player.workshop_cards, &state.card_lookup).is_some())
        }
        Choice::TrashAndLift { card, target } => {
            if !check_trash_preconditions(state, player_index, card) {
                return false;
            }
            lift_target_is_valid(state, player, target)
        }
        Choice::SpendGoldCoin { purchase } => {
            open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty())
                && can_buy_with_gold_coin(player, *purchase)
        }
        Choice::ClaimBuyer { buyer } => {
            matches!(&state.phase, GamePhase::Buyers { buyers_state } if buyers_state.current_player_index == player_index)
                && state
                    .buyer_display
                    .iter()
                    .any(|c| c.buyer == *buyer)
        }
        Choice::DrawBuyer => {
            matches!(&state.phase, GamePhase::Buyers { buyers_state } if buyers_state.current_player_index == player_index)
                && !state.buyer_deck.is_empty()
        }
    }
}

/// What the acting player may claim: one entry per distinct face-up buyer,
/// plus the top of the deck when there is one.
///
/// Deduplicating by card type keeps the tree from branching on which of two
/// identical cards was taken, exactly as draft picks do. The phase only ever
/// seats a player who has an empty slot, so there is no "decline" — but both
/// piles can run dry, and then there is nothing to enumerate and the phase
/// ends without seating anyone.
fn enumerate_buyer_choices(state: &GameState, choices: &mut Vec<Choice>) {
    let display = &state.buyer_display;
    for (i, instance) in display.iter().enumerate() {
        // At most `MAX_BUYER_DISPLAY` entries, so a scan beats any
        // bookkeeping.
        if display[..i].iter().any(|c| c.buyer == instance.buyer) {
            continue;
        }
        choices.push(Choice::ClaimBuyer {
            buyer: instance.buyer,
        });
    }
    if !state.buyer_deck.is_empty() {
        choices.push(Choice::DrawBuyer);
    }
}


#[cfg(test)]
mod tests {
    use rand::{RngExt, SeedableRng};
    use wyrand::WyRand;

    use super::*;
    use crate::apply_choice::apply_choice;
    use crate::colori_game::{get_game_status, GameStatus};
    use crate::fixed_vec::FixedVec;
    use crate::round::WORKSHOP_SIZE;
    use crate::unordered_cards::UnorderedCards;
    use crate::setup::create_initial_game_state;

    /// Random play from a fresh game to its first action phase.
    fn action_phase(players: usize, seed: u64) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(players, &vec![true; players], &mut rng);
        while !matches!(state.phase, GamePhase::Action { .. }) {
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                panic!("seed {seed}: the game ended before its first action phase");
            };
            let choices = enumerate_choices(&state, player_index);
            let choice = choices[rng.random_range(0..choices.len())];
            apply_choice(&mut state, player_index, &choice, &mut rng);
        }
        state
    }

    fn choices_for(state: &GameState, player: usize) -> Vec<Choice> {
        enumerate_choices(state, player)
    }

    /// Every top card is used or kept, the turn cannot end until the pool is
    /// empty, and a kept card goes on top of the deck — the segment the draw
    /// the turn ends with takes its cards from.
    #[test]
    fn a_turn_decides_every_top_card_before_it_can_end() {
        for seed in 0..4 {
            let state = action_phase(3, seed);
            let player = 0;
            let drafted = state.players[player].top_cards;
            assert!(!drafted.is_empty(), "seed {seed}: a hand to decide");

            // A use and a keep for every card, and no ending yet.
            let offered = choices_for(&state, player);
            assert!(
                !offered.contains(&Choice::EndTurn),
                "seed {seed}: a turn with cards left to decide cannot end"
            );
            for_each_unique_card_type(&drafted, &state.card_lookup, |card| {
                assert!(
                    offered.contains(&Choice::KeepCard { card }),
                    "seed {seed}: {card:?} can be kept"
                );
            });
            assert!(!check_choice_available(&state, player, &Choice::EndTurn));

            // Keep everything: the pool empties, the deck grows by the kept
            // cards, and only then may the turn end.
            let mut kept = state.clone();
            let mut kept_ids = UnorderedCards::new();
            let deck_before = state.players[player].deck.len();
            while let Some(id) = kept.players[player].top_cards.lowest_bit() {
                let card = kept.card_lookup[id as usize];
                kept_ids.insert(id);
                apply_choice(
                    &mut kept,
                    player,
                    &Choice::KeepCard { card },
                    &mut WyRand::seed_from_u64(0),
                );
            }
            assert!(kept.players[player].top_cards.is_empty());
            assert_eq!(
                kept.players[player].deck.len(),
                deck_before + kept_ids.len(),
                "seed {seed}: kept cards go into the deck"
            );
            assert!(choices_for(&kept, player).contains(&Choice::EndTurn));

            for id in kept_ids.iter() {
                assert!(
                    kept.players[player].deck.contains(id),
                    "seed {seed}: a kept card goes on top of the deck"
                );
            }

            let workshop_before = kept.players[player].workshop_cards.len();
            let deck_before = kept.players[player].deck.len();
            let mut ended = kept.clone();
            apply_choice(&mut ended, player, &Choice::EndTurn, &mut WyRand::seed_from_u64(0));
            assert_eq!(
                ended.players[player].workshop_cards.len() as u32,
                WORKSHOP_SIZE,
                "seed {seed}: the turn ending draws the workshop back up"
            );
            assert_eq!(
                ended.players[player].deck.len(),
                deck_before + workshop_before - WORKSHOP_SIZE,
                "seed {seed}: the top-up draw comes out of the deck, one card per missing one"
            );
        }
    }

    /// A card moved into the top cards is just another card to decide: use it
    /// this turn, or keep it for the next.
    #[test]
    fn a_card_moved_into_the_pool_is_used_or_kept() {
        let (state, moving) = (0..40)
            .find_map(|seed| {
                let state = action_phase(4, seed);
                let player = 0;
                choices_for(&state, player)
                    .into_iter()
                    .find(|choice| {
                        matches!(choice, Choice::TrashAndLift { target: Some(_), .. })
                            || matches!(choice, Choice::TrashAndLift { target: None, .. })
                                && !state.players[player].workshop_cards.is_empty()
                    })
                    .map(|choice| (state, choice))
            })
            .expect("some turn can move a card into the top cards");
        let Choice::TrashAndLift { card, target } = moving else {
            unreachable!("the found choice moves a card into the pool");
        };
        let moved = target.expect("a workshop card to move");

        let mut used = state.clone();
        apply_choice(
            &mut used,
            0,
            &Choice::TrashAndLift { card, target: Some(moved) },
            &mut WyRand::seed_from_u64(0),
        );
        let offered = choices_for(&used, 0);
        assert!(
            offered.contains(&Choice::KeepCard { card: moved })
                || !used.players[0].top_cards.contains(
                    used.players[0]
                        .top_cards
                        .iter()
                        .find(|id| used.card_lookup[*id as usize] == moved)
                        .expect("the moved card is in the pool")
                ),
            "the moved card is a decision of its own"
        );
        assert!(
            offered.iter().any(|choice| matches!(
                choice,
                Choice::TrashTopCard { card } | Choice::TrashAndWorkshop { card, .. }
                    | Choice::TrashAndMix { card, .. } | Choice::TrashAndSell { card, .. }
                if *card == moved
            )),
            "the moved card can be used this turn"
        );
    }

    /// The ids of the first unused instances of `cards` in `card_lookup`.
    fn card_ids(card_lookup: &[Card; 256], cards: &[Card], taken: &mut UnorderedCards) -> Vec<u8> {
        cards
            .iter()
            .map(|&card| {
                let id = (0..=u8::MAX)
                    .find(|&id| card_lookup[id as usize] == card && !taken.contains(id))
                    .unwrap_or_else(|| panic!("{card:?} is in the game"));
                taken.insert(id);
                id
            })
            .collect()
    }

    /// A position whose workshop and top cards hold exactly these cards, for
    /// asking what the workshop enumeration may skip.
    fn position(workshop: &[Card], drafted: &[Card]) -> GameState {
        let mut rng = WyRand::seed_from_u64(0);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        let mut taken = UnorderedCards::new();
        let workshop_ids = card_ids(&state.card_lookup, workshop, &mut taken);
        let drafted_ids = card_ids(&state.card_lookup, drafted, &mut taken);
        let player = &mut state.players[0];
        player.workshop_cards = UnorderedCards::new();
        player.top_cards = UnorderedCards::new();
        for id in workshop_ids {
            player.workshop_cards.insert(id);
        }
        for id in drafted_ids {
            player.top_cards.insert(id);
        }
        state
    }

    fn pick_range(workshop: &[Card], drafted: &[Card], count: u32) -> (usize, usize) {
        let state = position(workshop, drafted);
        WorkshopPruning::new(&state, &state.players[0]).pick_range(count)
    }

    /// The same position with player 0 taking their action turn and `pending`
    /// abilities to answer.
    fn open_turn(mut state: GameState, pending: &[Ability]) -> GameState {
        let mut turns: FixedVec<PlayerTurn, MAX_PLAYERS> =
            state.players.iter().map(|_| PlayerTurn::default()).collect();
        for &ability in pending {
            turns[0].ability_stack.push(ability);
        }
        state.phase = GamePhase::Action {
            action_state: ActionState { turns },
        };
        state
    }

    /// The sizes of the workshop picks `offered` makes for `card`.
    fn offered_picks(offered: &[Choice], card: Card) -> Vec<usize> {
        offered
            .iter()
            .filter_map(|choice| match choice {
                Choice::TrashAndWorkshop {
                    card: trashed,
                    workshop_cards,
                } if *trashed == card => Some(workshop_cards.len()),
                _ => None,
            })
            .collect()
    }

    /// With nothing that moves a card into the top cards, no card can go idle
    /// and the workshop always takes its maximum.
    #[test]
    fn a_turn_that_cannot_move_a_card_workshops_the_maximum() {
        let workshop = [Card::Realgar, Card::Woad, Card::GumArabic];
        assert_eq!(pick_range(&workshop, &[], 2), (2, 2));
        assert_eq!(pick_range(&workshop, &[], 5), (3, 3), "more picks than cards");
        assert_eq!(pick_range(&workshop, &[Card::Realgar], 3), (3, 3));
    }

    /// Each move can take one card out of the workshop, so the picks may stop
    /// where the moves run out — but no earlier, since a card nothing can move
    /// is a free gain.
    #[test]
    fn the_picks_start_where_a_card_would_go_idle() {
        let workshop = [Card::Realgar, Card::Woad, Card::Azurite];
        let mover = [Card::Alum];
        assert_eq!(pick_range(&workshop, &mover, 2), (2, 2), "one card left for Alum");
        assert_eq!(
            pick_range(&workshop, &mover, 3),
            (2, 3),
            "three picks may be held back to two"
        );

        // Two moves and four cards left the picks no room below two: taking
        // one would leave two cards idle.
        let four = [Card::Realgar, Card::Woad, Card::Azurite, Card::Cinnabar];
        let two_movers = [Card::Alum, Card::Gondola];
        assert_eq!(pick_range(&four, &two_movers, 3), (2, 3));

        let three_moves = [Card::Alum, Card::GumArabic, Card::Gondola];
        assert_eq!(
            pick_range(&workshop, &three_moves, 1),
            (0, 1),
            "every card could be moved, so none of them must be taken"
        );
    }

    /// A mover in the workshop becomes another move once a drafted mover
    /// brings it into the pool, so it takes a card of its own to leave behind.
    #[test]
    fn a_mover_in_the_workshop_can_move_a_second_card() {
        let workshop = [Card::Alum, Card::Realgar, Card::Woad];
        let drafted_mover = [Card::Gondola];
        assert_eq!(pick_range(&workshop, &drafted_mover, 1), (1, 1));
        assert_eq!(
            pick_range(&workshop, &drafted_mover, 2),
            (1, 2),
            "the Alum the workshop takes could have moved the other card on"
        );

        assert_eq!(
            pick_range(&workshop, &[], 3),
            (3, 3),
            "with no drafted mover, nothing can bring Alum into the pool"
        );
    }

    /// The enumeration follows the pruning: a pending workshop offers only the
    /// sizes in the range, and forfeiting only while every card can be moved.
    #[test]
    fn a_pending_workshop_offers_only_the_picks_in_range() {
        let workshop = [Card::Realgar, Card::Woad, Card::Azurite];
        let state = open_turn(position(&workshop, &[]), &[Ability::Workshop { count: 2 }]);
        let offered = choices_for(&state, 0);
        let picked: Vec<usize> = offered
            .iter()
            .filter_map(|choice| match choice {
                Choice::Workshop { card_types } => Some(card_types.len()),
                _ => None,
            })
            .collect();
        assert!(!picked.is_empty(), "the workshop offers its picks");
        assert!(picked.iter().all(|&picks| picks == 2), "only the maximum: {picked:?}");
        assert!(!offered.contains(&Choice::SkipWorkshop));

        // One mover leaves two cards behind, so two or three picks are left.
        let state = open_turn(position(&workshop, &[Card::Alum]), &[Ability::Workshop { count: 3 }]);
        let picked: Vec<usize> = choices_for(&state, 0)
            .iter()
            .filter_map(|choice| match choice {
                Choice::Workshop { card_types } => Some(card_types.len()),
                _ => None,
            })
            .collect();
        assert!(picked.contains(&2) && picked.contains(&3), "two or three: {picked:?}");
        assert!(!picked.contains(&1), "one would leave two cards idle");

        // Every card can be moved, so taking none is coherent: the position can
        // be reached (by the last turn's solver, or a re-rooted search) with
        // the gold_coin that bought the picks already spent.
        let state = open_turn(
            position(&workshop[..2], &[Card::Alum, Card::Gondola]),
            &[Ability::Workshop { count: 3 }],
        );
        let offered = choices_for(&state, 0);
        assert!(offered.contains(&Choice::SkipWorkshop));
        assert!(offered.iter().any(|c| matches!(c, Choice::Workshop { card_types } if card_types.len() == 1)));
    }

    /// Trashing a Workshop card offers giving up its picks only while every
    /// card left could still be moved: otherwise taking an idle card is free.
    #[test]
    fn trashing_a_workshop_card_keeps_the_empty_picks_only_while_cards_can_move() {
        let workshop = [Card::Realgar, Card::Woad];
        let drafted = [Card::BasicRed, Card::Alum, Card::Gondola];
        let offered = choices_for(&open_turn(position(&workshop, &drafted), &[]), 0);
        let picks = offered_picks(&offered, Card::BasicRed);
        assert!(picks.contains(&0), "both cards could be moved: {picks:?}");
        assert!(picks.contains(&2));

        // One mover cannot reach both cards, so one would go idle.
        let drafted = [Card::BasicRed, Card::Alum];
        let offered = choices_for(&open_turn(position(&workshop, &drafted), &[]), 0);
        let picks = offered_picks(&offered, Card::BasicRed);
        assert!(!picks.contains(&0), "a card would go idle: {picks:?}");
        assert!(
            picks.contains(&1) && picks.contains(&2),
            "one card can be left for the mover: {picks:?}"
        );
    }

    /// A hand can hold Workshop cards that grant different numbers of picks,
    /// and they share one tally: each asks about its own count.
    #[test]
    fn a_hand_of_workshop_cards_asks_about_each_count_apart() {
        let workshop = [Card::Realgar, Card::Woad, Card::Azurite];
        // One mover leaves two cards behind: two picks still take the whole
        // workshop's worth, three may be held back to two.
        let drafted = [Card::Alum, Card::BasicRed, Card::Cinnabar];
        let offered = choices_for(&open_turn(position(&workshop, &drafted), &[]), 0);

        let two_picks = offered_picks(&offered, Card::BasicRed);
        assert!(!two_picks.is_empty(), "Red grants two picks");
        assert!(
            two_picks.iter().all(|&picks| picks == 2),
            "two picks leave no room below their maximum: {two_picks:?}"
        );

        let three_picks = offered_picks(&offered, Card::Cinnabar);
        assert!(three_picks.contains(&2), "three picks may be held back to two");
        assert!(three_picks.contains(&3), "and may take the maximum");
        assert!(!three_picks.contains(&0), "but never give up the picks");
    }
}
