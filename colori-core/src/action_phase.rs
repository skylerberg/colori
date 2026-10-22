use crate::colors::{bowls_to_cover_sale, can_supply_mix, pay_cost, perform_mix, VALID_MIX_PAIRS};
use crate::round::{draw_into_workshop, end_round};
use crate::types::{
    Ability, AbilityStack, ActionState, Buyer, Card, Color, GamePhase, GameState, GoldCoinPurchase,
    PlayerState, PlayerTurn,
};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use smallvec::SmallVec;

/// Iterate over unique card types in a card set, deduplicating by card variant.
pub(crate) fn for_each_unique_card_type(
    cards: &UnorderedCards,
    card_lookup: &[Card; 256],
    mut f: impl FnMut(Card),
) {
    let mut seen: u64 = 0;
    for id in cards.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit != 0 { continue; }
        seen |= bit;
        f(card);
    }
}

pub fn initialize_action_phase(state: &mut GameState) {
    let turns = state.players.iter().map(|_| PlayerTurn::default()).collect();
    state.phase = GamePhase::Action {
        action_state: ActionState { turns },
    };
}

/// Take a card out of the top cards because it is being used.
///
/// Every path that trashes a top card goes through here: the choice
/// resolver, and the rollouts' fused mix and sale steps, which skip the ability
/// stack.
#[inline]
pub(crate) fn take_top_card(state: &mut GameState, player_index: usize, id: u8) {
    assert!(
        state.players[player_index].top_cards.contains(id),
        "Card not found in player's topCards"
    );
    state.players[player_index].top_cards.remove(id);
}

/// Keep a top card instead of using it: it comes out of the top cards and
/// goes on top of the player's deck, to be drawn again next round.
pub fn resolve_keep_card(state: &mut GameState, player_index: usize, card_instance_id: u32) {
    let id = card_instance_id as u8;
    let player = &mut state.players[player_index];
    assert!(
        player.top_cards.contains(id),
        "Card not found in player's topCards"
    );
    player.top_cards.remove(id);
    let mut kept = UnorderedCards::new();
    kept.insert(id);
    player.deck.push_top(kept);
}

pub fn trash_top_card(state: &mut GameState, player_index: usize, card_instance_id: u32) {
    let id = card_instance_id as u8;
    take_top_card(state, player_index, id);

    let card = state.card_lookup[id as usize];
    state.trash.insert(id);

    // The stack is LIFO, so the first ability listed goes on last.
    let stack = ability_stack_mut(state, player_index);
    for &ability in card.top_actions().iter().rev() {
        stack.push(ability);
    }
    process_ability_stack(state, player_index);
}

pub fn process_ability_stack(state: &mut GameState, player_index: usize) {
    loop {
        let Some(ability) = ability_stack(state, player_index).last().copied() else {
            return;
        };

        match ability {
            Ability::Workshop { .. } => {
                if state.players[player_index].workshop_cards.is_empty() {
                    ability_stack_mut(state, player_index).pop();
                    continue;
                } else {
                    return; // waiting for input
                }
            }
            Ability::GainGoldCoins { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].gold_coins += count;
                state.players[player_index].cached_whole_points += count;
                continue;
            }
            Ability::GainBowls { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].bowls += count;
                continue;
            }
            Ability::GainBoats { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].boats += count;
                continue;
            }
            Ability::MixColors { .. } => {
                // A mix nobody can make has one answer, mixing nothing, which
                // `enumerate_mix_sequences` spells as an empty sequence. Popping
                // it here is that answer, taken without asking: a position no
                // one can choose differently at is not a decision. The same
                // reason the `Workshop` and `Sell` arms above fizzle.
                if can_mix_any(&state.players[player_index]) {
                    return; // waiting for input
                }
                ability_stack_mut(state, player_index).pop();
                continue;
            }
            Ability::LiftCard => {
                return; // always needs input
            }
            Ability::Sell => {
                if can_sell_to_any_buyer(&state.players[player_index]) {
                    return; // waiting for input
                } else {
                    ability_stack_mut(state, player_index).pop();
                    continue;
                }
            }
            Ability::GainSecondary => {
                return; // always needs input
            }
        }
    }
}

/// Boats a sale would take from `player`: those the card asks for outright,
/// plus one for each material it asks for that the player has not stored.
#[inline]
pub(crate) fn boats_to_sell(player: &PlayerState, buyer: Buyer) -> u32 {
    let stats = buyer.stats();
    let mut boats = stats.boats;
    for (stored, &needed) in player.materials_wheel.counts.iter().zip(&stats.material_units) {
        boats += (needed as u32).saturating_sub(*stored);
    }
    boats
}

#[inline]
pub(crate) fn can_afford_buyer(player: &PlayerState, buyer: &Buyer) -> bool {
    boats_to_sell(player, *buyer) <= player.boats
        && bowls_to_cover_sale(&player.color_wheel, buyer.stats())
            .is_some_and(|needed| needed <= player.bowls)
}

/// Sell to the buyer at `index`, paying with the materials and colours it asks
/// for before any boat or bowl standing in for them.
pub(crate) fn sell_to_buyer(player: &mut PlayerState, index: usize) {
    // Order-preserving, so the slot a buyer occupied is not visibly shuffled
    // when an earlier one sells.
    let sold = player.buyers.remove(index);

    let mut boats = sold.buyer.required_boats();
    for &material in sold.buyer.required_materials() {
        if !player.materials_wheel.decrement(material) {
            boats += 1;
        }
    }
    assert!(player.boats >= boats, "Not enough stored material or boats");
    player.boats -= boats;
    let paid = pay_cost(&mut player.color_wheel, &mut player.bowls, sold.buyer.color_cost());
    assert!(paid, "Cannot pay buyer color cost");

    player.cached_whole_points += sold.buyer.points();
    player.completed_orders.push(sold);
}

/// Whether `player` can sell to one of their own buyers. The shared display is
/// not sellable to — cards must be claimed first, in the buyers phase.
#[inline]
pub fn can_sell_to_any_buyer(player: &PlayerState) -> bool {
    player
        .buyers
        .iter()
        .any(|b| can_afford_buyer(player, &b.buyer))
}

/// Whether the player can make any mix at all, counting a bowl as whichever
/// primary is missing.
#[inline]
pub fn can_mix_any(player: &PlayerState) -> bool {
    VALID_MIX_PAIRS
        .iter()
        .any(|&(a, b)| can_supply_mix(&player.color_wheel, player.bowls, a, b))
}

/// Whether spending a gold_coin on `purchase` would actually do something.
///
/// A purchase that cannot act would pop straight off the stack and burn the
/// gold_coin, so it is never offered.
pub fn can_buy_with_gold_coin(player: &PlayerState, purchase: GoldCoinPurchase) -> bool {
    if player.gold_coins == 0 {
        return false;
    }
    match purchase {
        GoldCoinPurchase::Workshop => !player.workshop_cards.is_empty(),
        GoldCoinPurchase::MixColors => can_mix_any(player),
        GoldCoinPurchase::Sell => can_sell_to_any_buyer(player),
    }
}

/// Pay a gold_coin and put what it bought on the ability stack.
///
/// GoldCoins are score, so the cached score has to move with them. The purchase
/// is pushed rather than fused into the choice: the abilities it buys already
/// have resolution paths and choice enumerators, and pushing keeps the
/// top-level menu three choices wider instead of multiplying by what each
/// ability could then do.
pub fn resolve_spend_gold_coin(state: &mut GameState, player_index: usize, purchase: GoldCoinPurchase) {
    let player = &mut state.players[player_index];
    assert!(player.gold_coins >= 1, "Cannot spend a gold_coin without one");
    player.gold_coins -= 1;
    player.cached_whole_points -= 1;

    ability_stack_mut(state, player_index).push(purchase.ability());
    process_ability_stack(state, player_index);
}

/// Workshop the selected cards. Each stores what it holds, and the workshop
/// abilities of action cards go on the stack with gains above mixes, so
/// anything gained is there to mix.
///
/// Picks left unspent are dropped rather than carried over: nothing enters the
/// workshop mid-turn, so a saved pick could only take a card on offer now.
pub fn resolve_workshop_choice(
    state: &mut GameState,
    player_index: usize,
    selected_cards: UnorderedCards,
) {
    let popped = ability_stack_mut(state, player_index).pop();
    assert!(
        matches!(popped, Some(Ability::Workshop { .. })),
        "No pending workshop choice"
    );

    let player = &mut state.players[player_index];
    let mut mixes: SmallVec<[Ability; 4]> = SmallVec::new();
    let mut gains: SmallVec<[Ability; 4]> = SmallVec::new();
    for id in selected_cards.iter() {
        let card = state.card_lookup[id as usize];
        player.workshop_cards.remove(id);
        player.used_cards.insert(id);
        for &material in card.material_types() {
            player.materials_wheel.increment(material);
        }
        for &color in card.colors() {
            player.color_wheel.increment(color);
        }
        for &ability in card.workshop_abilities() {
            if matches!(ability, Ability::MixColors { .. }) {
                mixes.push(ability);
            } else {
                gains.push(ability);
            }
        }
    }

    let stack = ability_stack_mut(state, player_index);
    stack.extend(mixes);
    stack.extend(gains);

    process_ability_stack(state, player_index);
}

pub fn skip_workshop(state: &mut GameState, player_index: usize) {
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index);
}

pub fn resolve_mix_colors(state: &mut GameState, player_index: usize, color_a: Color, color_b: Color) {
    let player = &mut state.players[player_index];
    let success = perform_mix(&mut player.color_wheel, &mut player.bowls, color_a, color_b);
    if !success {
        panic!("Cannot mix {:?} and {:?}", color_a, color_b);
    }

    let stack = ability_stack_mut(state, player_index);
    if let Some(Ability::MixColors { count }) = stack.last_mut() {
        *count -= 1;
        if *count == 0 {
            stack.pop();
        }
    }
    process_ability_stack(state, player_index);
}

pub fn skip_mix(state: &mut GameState, player_index: usize) {
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index);
}

/// Lift an unused card from the workshop onto the player's top cards, or move
/// nothing when there is none.
///
/// `None` is the fizzle when the workshop is empty, and also how a turn that
/// is ending declines a lift it could have made — see
/// [`skip_pending_abilities`].
///
/// Nothing is trashed here. A card in the top cards can be trashed later
/// in the turn through the ordinary top-card path, which triggers its
/// ability on the stack exactly as before — and when this ability resolves the
/// stack beneath it is always empty, since no card's *workshop* abilities
/// include this one, so trashing now and trashing later reach the same
/// state. What is new is the option to leave the card there, from where
/// ending the turn keeps it on top of the deck rather than letting the
/// workshop's shuffle send it to the bottom.
pub fn resolve_lift_card(state: &mut GameState, player_index: usize, target: Option<u8>) {
    ability_stack_mut(state, player_index).pop();

    if let Some(id) = target {
        let player = &mut state.players[player_index];
        assert!(
            player.workshop_cards.contains(id),
            "Card not found in player's workshopCards"
        );
        player.workshop_cards.remove(id);
        player.top_cards.insert(id);
    }

    process_ability_stack(state, player_index);
}

/// Decline a sale. The rules make a sale mandatory while a buyer is
/// affordable, so this is only for a turn that is ending: see
/// [`skip_pending_abilities`].
pub fn skip_sell(state: &mut GameState, player_index: usize) {
    let popped = ability_stack_mut(state, player_index).pop();
    debug_assert!(matches!(popped, Some(Ability::Sell)), "No pending sell to skip");
    process_ability_stack(state, player_index);
}

pub fn resolve_sell_to_buyer(state: &mut GameState, player_index: usize, buyer_instance_id: u32) {
    ability_stack_mut(state, player_index).pop();

    let player = &mut state.players[player_index];
    let buyer_index = player
        .buyers
        .iter()
        .position(|c| c.instance_id == buyer_instance_id)
        .expect("Buyer card not found in the player's buyers");

    sell_to_buyer(player, buyer_index);

    // The freed slot stays empty until the next buyers phase.
    process_ability_stack(state, player_index);
}

pub fn resolve_gain_color(state: &mut GameState, player_index: usize, color: Color) {
    state.players[player_index].color_wheel.increment(color);
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index);
}

/// Decline whatever is waiting on the player, so that ending a turn is always
/// possible.
///
/// Workshop, Mix Colors and Any Secondary are declined by dropping them,
/// exactly as their own skips do. Sell and Lift a Card are declined here:
/// while each can still act the rules leave no other way to say no, and a
/// turn that is over has no decision left in it. Automatic abilities never
/// wait for an answer, so `process_ability_stack` has already resolved them.
pub fn skip_pending_abilities(state: &mut GameState, player_index: usize) {
    while let Some(ability) = ability_stack(state, player_index).last().copied() {
        match ability {
            Ability::Sell => skip_sell(state, player_index),
            Ability::LiftCard => resolve_lift_card(state, player_index, None),
            // Instant abilities resolve as the stack is processed, so one is
            // never on top here.
            Ability::Workshop { .. } | Ability::MixColors { .. } | Ability::GainSecondary => {
                ability_stack_mut(state, player_index).pop();
                process_ability_stack(state, player_index);
            }
            Ability::GainGoldCoins { .. } | Ability::GainBowls { .. } | Ability::GainBoats { .. } => {
                unreachable!("an instant ability was waiting for a choice")
            }
        }
    }
}

/// End `player_index`'s turn. Everyone acts at once, so the round ends with
/// whoever finishes last.
/// End `player_index`'s turn. Everyone acts at once, so the round ends with
/// whoever finishes last.
///
/// `keep_remaining` decides what becomes of the top cards the turn never
/// decided. Kept, they go where keeping puts them: on top of the deck, to be
/// drawn back into the workshop. Otherwise each is used, taking its top action
/// and skipping whatever that action asks — the same as using it and declining
/// every choice it opens.
pub fn end_player_turn<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    keep_remaining: bool,
    rng: &mut R,
) {
    // Ending the turn declines whatever was still waiting on the player.
    skip_pending_abilities(state, player_index);

    if !keep_remaining {
        while let Some(id) = state.players[player_index].top_cards.lowest_bit() {
            trash_top_card(state, player_index, id as u32);
            skip_pending_abilities(state, player_index);
        }
    }

    let round_is_over = {
        let player = &mut state.players[player_index];

        // The whole workshop area is shuffled onto the bottom of the deck. Any
        // card the turn never decided is a card it kept, so it goes where
        // keeping puts one: on top, to be drawn back into the workshop.
        player
            .deck
            .push_bottom(player.workshop_cards.union(player.used_cards));
        if keep_remaining {
            player.deck.push_top(player.top_cards);
        }
        player.workshop_cards = UnorderedCards::new();
        player.used_cards = UnorderedCards::new();
        player.top_cards = UnorderedCards::new();

        let action_state = get_action_state_mut(state);
        let turn = &mut action_state.turns[player_index];
        debug_assert!(
            !turn.ended && turn.ability_stack.is_empty(),
            "Player {player_index} ended a turn that was over or still had abilities pending"
        );
        turn.ended = true;
        action_state.turns.iter().all(|turn| turn.ended)
    };

    // Draw straight away, as the turn's last act. A workshop is topped up
    // rather than dealt, so this is also how a kept card comes back. The last
    // round ends with this turn, so its workshop is never used again and
    // nothing is drawn: a final turn stays free of randomness.
    if state.round < state.max_rounds {
        draw_into_workshop(state, player_index, rng);
    }

    if round_is_over {
        end_round(state, rng);
    }
}

#[inline]
pub(crate) fn get_action_state(state: &GameState) -> &ActionState {
    match &state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}

#[inline]
pub(crate) fn get_action_state_mut(state: &mut GameState) -> &mut ActionState {
    match &mut state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}

#[inline]
pub(crate) fn ability_stack(state: &GameState, player_index: usize) -> &AbilityStack {
    &get_action_state(state).turns[player_index].ability_stack
}

#[inline]
pub(crate) fn ability_stack_mut(state: &mut GameState, player_index: usize) -> &mut AbilityStack {
    &mut get_action_state_mut(state).turns[player_index].ability_stack
}
