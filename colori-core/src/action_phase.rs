use crate::colors::{can_pay_cost, can_supply_mix, pay_cost, perform_mix, VALID_MIX_PAIRS};
use crate::draw_log_helpers::{is_replaying, record_player_deck_draw, replay_player_deck_draw};
use crate::types::{
    Ability, AbilityStack, ActionState, Card, Color, DucatPurchase, GamePhase, GameState,
    MaterialType, PlayerState, PlayerTurn, SellCard,
};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

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

/// Iterate over unique card types across the player's workshop area —
/// both freshly drawn workshop cards and already-processed workshopped cards
/// — deduplicating by card variant.
pub(crate) fn for_each_unique_card_type_in_workshop_area(
    player: &PlayerState,
    card_lookup: &[Card; 256],
    f: impl FnMut(Card),
) {
    let union = player.workshop_cards.union(player.workshopped_cards);
    for_each_unique_card_type(&union, card_lookup, f);
}

/// Remove `id` from whichever of the player's workshop-area buckets
/// (`workshop_cards` or `workshopped_cards`) contains it. Returns `true` on
/// success.
pub(crate) fn remove_from_workshop_area(player: &mut PlayerState, id: u8) -> bool {
    if player.workshop_cards.contains(id) {
        player.workshop_cards.remove(id);
        true
    } else if player.workshopped_cards.contains(id) {
        player.workshopped_cards.remove(id);
        true
    } else {
        false
    }
}

/// Partitions card IDs from `selected_cards` into action and non-action cards.
/// Returns `(action_ids, action_count, non_action_ids, non_action_count)`.
fn partition_action_cards(
    selected_cards: &UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([u8; 16], usize, [u8; 16], usize) {
    let mut action_ids = [0u8; 16];
    let mut action_count = 0usize;
    let mut non_action_ids = [0u8; 16];
    let mut non_action_count = 0usize;
    for id in selected_cards.iter() {
        let card = card_lookup[id as usize];
        if card.is_action() {
            action_ids[action_count] = id;
            action_count += 1;
        } else {
            non_action_ids[non_action_count] = id;
            non_action_count += 1;
        }
    }
    (action_ids, action_count, non_action_ids, non_action_count)
}

/// Processes non-action cards: extracts materials/colors, moves from workshop to workshopped.
fn process_non_action_cards(
    player: &mut PlayerState,
    card_lookup: &[Card; 256],
    non_action_ids: &[u8; 16],
    non_action_count: usize,
) {
    for i in 0..non_action_count {
        let id = non_action_ids[i];
        let card = card_lookup[id as usize];
        player.workshop_cards.remove(id);
        for mt in card.material_types() {
            player.materials.increment(*mt);
        }
        for color in card.colors() {
            player.color_wheel.increment(*color);
        }
        player.workshopped_cards.insert(id);
    }
}

/// Categorized abilities collected from action cards during workshop processing.
struct CollectedAbilities {
    regular: [Ability; 8],
    regular_count: usize,
    mix_colors: [Ability; 8],
    mix_colors_count: usize,
    draw_card: [Ability; 8],
    draw_card_count: usize,
    potash_base_count: Option<u32>,
    has_draw_cards: bool,
}

impl CollectedAbilities {
    fn new() -> Self {
        Self {
            regular: [Ability::Sell; 8],
            regular_count: 0,
            mix_colors: [Ability::Sell; 8],
            mix_colors_count: 0,
            draw_card: [Ability::Sell; 8],
            draw_card_count: 0,
            potash_base_count: None,
            has_draw_cards: false,
        }
    }

    /// Categorize a single ability into the appropriate bucket.
    fn add_ability(&mut self, ability: Ability) {
        match ability {
            Ability::Workshop { count: c } => {
                self.potash_base_count = Some(self.potash_base_count.unwrap_or(0) + c);
            }
            Ability::DrawCards { .. } => {
                self.has_draw_cards = true;
                self.draw_card[self.draw_card_count] = ability;
                self.draw_card_count += 1;
            }
            Ability::MixColors { .. } => {
                self.mix_colors[self.mix_colors_count] = ability;
                self.mix_colors_count += 1;
            }
            _ => {
                self.regular[self.regular_count] = ability;
                self.regular_count += 1;
            }
        }
    }
}

/// Processes action cards: removes from workshop, moves to workshopped, and collects abilities.
fn collect_abilities_from_action_cards(
    player: &mut PlayerState,
    card_lookup: &[Card; 256],
    action_ids: &[u8; 16],
    action_count: usize,
) -> CollectedAbilities {
    let mut collected = CollectedAbilities::new();

    for i in 0..action_count {
        let id = action_ids[i];
        let card = card_lookup[id as usize];
        player.workshop_cards.remove(id);

        for &ability in card.workshop_abilities() {
            collected.add_ability(ability);
        }

        player.workshopped_cards.insert(id);
    }

    collected
}

/// Pushes collected abilities onto the LIFO stack in the correct resolution order.
/// `remaining` is the number of unused workshop slots; if `None`, no remaining-based
/// workshop ability is pushed (used by reworkshop path).
fn push_abilities_to_stack(
    stack: &mut AbilityStack,
    collected: &CollectedAbilities,
    remaining: Option<u32>,
) {
    let needs_deferred_remaining = collected.has_draw_cards;

    if let Some(base) = collected.potash_base_count {
        let potash_count = match remaining {
            Some(rem) if !needs_deferred_remaining => base + rem,
            _ => base,
        };
        stack.push(Ability::Workshop { count: potash_count });
    }

    if let Some(remaining) = remaining {
        if needs_deferred_remaining && remaining > 0 {
            stack.push(Ability::Workshop { count: remaining });
        }
    }

    for i in 0..collected.draw_card_count {
        stack.push(collected.draw_card[i]);
    }

    for i in 0..collected.mix_colors_count {
        stack.push(collected.mix_colors[i]);
    }

    for i in 0..collected.regular_count {
        stack.push(collected.regular[i]);
    }
}

pub fn initialize_action_phase(state: &mut GameState) {
    let turns = state.players.iter().map(|_| PlayerTurn::default()).collect();
    state.phase = GamePhase::Action {
        action_state: ActionState { turns },
    };
}

pub fn destroy_drafted_card<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    card_instance_id: u32,
    rng: &mut R,
) {
    let id = card_instance_id as u8;
    let player = &mut state.players[player_index];

    assert!(
        player.drafted_cards.contains(id),
        "Card not found in player's draftedCards"
    );
    player.drafted_cards.remove(id);

    let card = state.card_lookup[id as usize];
    state.destroyed_pile.insert(id);

    // The stack is LIFO, so the first ability listed goes on last.
    let stack = ability_stack_mut(state, player_index);
    for &ability in card.destroy_abilities().iter().rev() {
        stack.push(ability);
    }
    process_ability_stack(state, player_index, rng);
}

pub fn process_ability_stack<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    loop {
        let Some(ability) = ability_stack(state, player_index).last().copied() else {
            return;
        };

        match ability {
            Ability::DrawCards { count } => {
                ability_stack_mut(state, player_index).pop();
                if is_replaying(state) {
                    replay_player_deck_draw(state, player_index);
                } else {
                    let before = state.players[player_index].workshop_cards;
                    let player = &mut state.players[player_index];
                    player.deck.draw_into(&mut player.workshop_cards, count, rng);
                    record_player_deck_draw(state, player_index, before);
                }
                continue;
            }
            Ability::Workshop { .. } => {
                if state.players[player_index].workshop_cards.is_empty() {
                    ability_stack_mut(state, player_index).pop();
                    continue;
                } else {
                    return; // waiting for input
                }
            }
            Ability::GainDucats { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].ducats += count;
                state.players[player_index].cached_ducat_score += count;
                continue;
            }
            Ability::GainWildPrimaries { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].wild_primaries += count;
                continue;
            }
            Ability::GainBoats { count } => {
                ability_stack_mut(state, player_index).pop();
                state.players[player_index].boats += count;
                continue;
            }
            Ability::MixColors { .. } => {
                return; // always needs input
            }
            Ability::MoveToDraftPool => {
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
            Ability::GainPrimary => {
                return; // always needs input
            }
            Ability::GainMaterial => {
                return; // always needs input
            }
        }
    }
}

/// Boats a sale would take from `player`: those the card asks for outright,
/// plus one for each material it asks for that the player has not stored.
#[inline]
pub(crate) fn boats_to_sell(player: &PlayerState, sell_card: SellCard) -> u32 {
    let materials = sell_card.required_materials();
    let mut boats = sell_card.required_boats();
    for (i, &material) in materials.iter().enumerate() {
        let needed = 1 + materials[..i].iter().filter(|&&earlier| earlier == material).count() as u32;
        if player.materials.get(material) < needed {
            boats += 1;
        }
    }
    boats
}

#[inline]
pub(crate) fn can_afford_sell_card(player: &PlayerState, sell_card: &SellCard) -> bool {
    boats_to_sell(player, *sell_card) <= player.boats
        && can_pay_cost(&player.color_wheel, player.wild_primaries, sell_card.color_cost())
}

/// Sell to the buyer at `index`, paying with the materials and colours it asks
/// for before any boat or wild primary standing in for them.
pub(crate) fn sell_to_buyer(player: &mut PlayerState, index: usize) {
    // Order-preserving, so the slot a buyer occupied is not visibly shuffled
    // when an earlier one sells.
    let sold = player.buyers.remove(index);

    let mut boats = sold.sell_card.required_boats();
    for &material in sold.sell_card.required_materials() {
        if !player.materials.decrement(material) {
            boats += 1;
        }
    }
    assert!(player.boats >= boats, "Not enough stored material or boats");
    player.boats -= boats;
    let paid = pay_cost(&mut player.color_wheel, &mut player.wild_primaries, sold.sell_card.color_cost());
    assert!(paid, "Cannot pay sell card color cost");

    player.cached_ducat_score += sold.sell_card.ducats();
    player.completed_sell_cards.push(sold);
}

/// Whether `player` can sell to one of their own buyers. The shared display is
/// not sellable to — cards must be claimed first, in the buyers phase.
#[inline]
pub fn can_sell_to_any_buyer(player: &PlayerState) -> bool {
    player
        .buyers
        .iter()
        .any(|b| can_afford_sell_card(player, &b.sell_card))
}

/// Whether the player can make any mix at all, counting a wild primary as whichever
/// primary is missing.
#[inline]
pub fn can_mix_any(player: &PlayerState) -> bool {
    VALID_MIX_PAIRS
        .iter()
        .any(|&(a, b)| can_supply_mix(&player.color_wheel, player.wild_primaries, a, b))
}

/// Whether spending a ducat on `purchase` would actually do something.
///
/// A purchase that cannot act would pop straight off the stack and burn the
/// ducat, so it is never offered.
pub fn can_buy_with_ducat(player: &PlayerState, purchase: DucatPurchase) -> bool {
    if player.ducats == 0 {
        return false;
    }
    match purchase {
        DucatPurchase::Workshop => !player.workshop_cards.is_empty(),
        DucatPurchase::MixColors => can_mix_any(player),
        DucatPurchase::Sell => can_sell_to_any_buyer(player),
    }
}

/// Pay a ducat and put what it bought on the ability stack.
///
/// Ducats are score, so the cached score has to move with them. The purchase
/// is pushed rather than fused into the choice: the abilities it buys already
/// have resolution paths and choice enumerators, and pushing keeps the
/// top-level menu three choices wider instead of multiplying by what each
/// ability could then do.
pub fn resolve_spend_ducat<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    purchase: DucatPurchase,
    rng: &mut R,
) {
    let player = &mut state.players[player_index];
    assert!(player.ducats >= 1, "Cannot spend a ducat without one");
    player.ducats -= 1;
    player.cached_ducat_score -= 1;

    ability_stack_mut(state, player_index).push(purchase.ability());
    process_ability_stack(state, player_index, rng);
}

pub fn resolve_workshop_choice<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    let count = match ability_stack(state, player_index).last() {
        Some(Ability::Workshop { count }) => *count,
        _ => panic!("No pending workshop choice"),
    };
    let remaining = count - selected_cards.len();

    ability_stack_mut(state, player_index).pop();

    let (action_ids, action_count, non_action_ids, non_action_count) =
        partition_action_cards(&selected_cards, &state.card_lookup);

    process_non_action_cards(
        &mut state.players[player_index],
        &state.card_lookup,
        &non_action_ids,
        non_action_count,
    );

    let collected = collect_abilities_from_action_cards(
        &mut state.players[player_index],
        &state.card_lookup,
        &action_ids,
        action_count,
    );

    push_abilities_to_stack(ability_stack_mut(state, player_index), &collected, Some(remaining));

    process_ability_stack(state, player_index, rng);
}

pub fn skip_workshop<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index, rng);
}

pub fn resolve_mix_colors<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    color_a: Color,
    color_b: Color,
    rng: &mut R,
) {
    let player = &mut state.players[player_index];
    let success = perform_mix(&mut player.color_wheel, &mut player.wild_primaries, color_a, color_b);
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
    process_ability_stack(state, player_index, rng);
}

pub fn skip_mix<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index, rng);
}

/// Move the selected workshop-area cards into the player's draft pool.
///
/// Nothing is destroyed here. A card in the draft pool can be destroyed later
/// in the turn through the ordinary drafted-card path, which triggers its
/// ability on the stack exactly as before — and when this ability resolves the
/// stack beneath it is always empty, since no card's *workshop* abilities
/// include this one, so destroying now and destroying later reach the same
/// state. What is new is the option to leave the card in the pool, from where
/// end of turn returns it to the workshop instead of the bottom of the deck.
pub fn resolve_move_to_draft_pool<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    ability_stack_mut(state, player_index).pop();

    for id in selected_cards.iter() {
        let removed = remove_from_workshop_area(&mut state.players[player_index], id);
        assert!(
            removed,
            "Card not found in player's workshop area (workshopCards or workshoppedCards)"
        );
        state.players[player_index].drafted_cards.insert(id);
    }

    process_ability_stack(state, player_index, rng);
}

pub fn resolve_select_sell_card<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    sell_card_instance_id: u32,
    rng: &mut R,
) {
    ability_stack_mut(state, player_index).pop();

    let player = &mut state.players[player_index];
    let sell_card_index = player
        .buyers
        .iter()
        .position(|c| c.instance_id == sell_card_instance_id)
        .expect("Sell card not found in the player's buyers");

    sell_to_buyer(player, sell_card_index);

    // The freed slot stays empty until the next buyers phase.
    process_ability_stack(state, player_index, rng);
}

pub fn resolve_gain_color<R: Rng>(state: &mut GameState, player_index: usize, color: Color, rng: &mut R) {
    state.players[player_index].color_wheel.increment(color);
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index, rng);
}

pub fn resolve_gain_material<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    material: MaterialType,
    rng: &mut R,
) {
    state.players[player_index].materials.increment(material);
    ability_stack_mut(state, player_index).pop();
    process_ability_stack(state, player_index, rng);
}

/// End `player_index`'s turn. Everyone acts at once, so the round ends with
/// whoever finishes last.
pub fn end_player_turn<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    let player = &mut state.players[player_index];

    // The whole workshop area is shuffled onto the bottom of the deck, and
    // whatever survived in the draft pool becomes the new workshop.
    player
        .deck
        .push_bottom(player.workshop_cards.union(player.workshopped_cards));
    player.workshop_cards = player.drafted_cards;
    player.workshopped_cards = UnorderedCards::new();
    player.drafted_cards = UnorderedCards::new();

    let action_state = get_action_state_mut(state);
    let turn = &mut action_state.turns[player_index];
    debug_assert!(
        !turn.ended && turn.ability_stack.is_empty(),
        "Player {player_index} ended a turn that was over or still had abilities pending"
    );
    turn.ended = true;

    if action_state.turns.iter().all(|turn| turn.ended) {
        end_round(state, rng);
    }
}

pub fn end_round<R: Rng>(state: &mut GameState, _rng: &mut R) {
    state.round += 1;
    if state.round > state.max_rounds {
        state.phase = GamePhase::GameOver;
    } else {
        state.phase = GamePhase::Draw;
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
