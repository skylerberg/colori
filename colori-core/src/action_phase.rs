use crate::colors::{can_pay_cost, pay_cost, perform_mix, perform_mix_unchecked, TERTIARIES};
use crate::deck_utils::draw_from_deck;
use crate::draw_log_helpers::{is_replaying, record_player_deck_draw, replay_player_deck_draw, replay_sell_card_reveal};
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{
    Ability, AbilityStack, ActionState, Card, Color, GamePhase, GameState,
    PlayerState, SellCard, SellCardInstance,
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
    change_tertiary: [Ability; 8],
    change_tertiary_count: usize,
    move_to_workshop: [Ability; 8],
    move_to_workshop_count: usize,
    potash_base_count: Option<u32>,
    has_draw_cards: bool,
    has_move_to_workshop: bool,
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
            change_tertiary: [Ability::Sell; 8],
            change_tertiary_count: 0,
            move_to_workshop: [Ability::Sell; 8],
            move_to_workshop_count: 0,
            potash_base_count: None,
            has_draw_cards: false,
            has_move_to_workshop: false,
        }
    }

    /// Categorize a single ability into the appropriate bucket.
    fn add_ability(&mut self, ability: Ability) {
        match ability {
            Ability::ChangeTertiary => {
                self.change_tertiary[self.change_tertiary_count] = ability;
                self.change_tertiary_count += 1;
            }
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
            Ability::MoveToWorkshop => {
                self.has_move_to_workshop = true;
                self.move_to_workshop[self.move_to_workshop_count] = ability;
                self.move_to_workshop_count += 1;
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
    let needs_deferred_remaining = collected.has_draw_cards || collected.has_move_to_workshop;

    if let Some(base) = collected.potash_base_count {
        let potash_count = match remaining {
            Some(rem) if !needs_deferred_remaining => base + rem,
            _ => base,
        };
        stack.push(Ability::Workshop { count: potash_count });
    }

    for i in 0..collected.change_tertiary_count {
        stack.push(collected.change_tertiary[i]);
    }

    if let Some(remaining) = remaining {
        if needs_deferred_remaining && remaining > 0 {
            stack.push(Ability::Workshop { count: remaining });
        }
    }

    for i in 0..collected.draw_card_count {
        stack.push(collected.draw_card[i]);
    }

    for i in 0..collected.move_to_workshop_count {
        stack.push(collected.move_to_workshop[i]);
    }

    for i in 0..collected.mix_colors_count {
        stack.push(collected.mix_colors[i]);
    }

    for i in 0..collected.regular_count {
        stack.push(collected.regular[i]);
    }
}

pub fn initialize_action_phase(state: &mut GameState) {
    let action_state = ActionState {
        current_player_index: ((state.round - 1) as usize) % state.players.len(),
        ability_stack: SmallVec::new(),
    };
    state.phase = GamePhase::Action { action_state };
}

pub fn destroy_drafted_card<R: Rng>(state: &mut GameState, card_instance_id: u32, rng: &mut R) {
    let id = card_instance_id as u8;
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    assert!(
        player.drafted_cards.contains(id),
        "Card not found in player's draftedCards"
    );
    player.drafted_cards.remove(id);

    let card = state.card_lookup[id as usize];
    let ability = card.ability();
    state.destroyed_pile.insert(id);

    let action_state = get_action_state_mut(state);
    action_state.ability_stack.push(ability);
    process_ability_stack(state, rng);
}

/// Destroy a card currently in the player's workshop area (either
/// `workshop_cards` or `workshopped_cards`) and trigger its ability — without
/// requiring `DestroyCards` to be on the stack. Used by the human UI's
/// deferred move-to-draft-pool flow: after staging a move (which pops
/// DestroyCards), the user can later commit by clicking the card in the draft
/// pool, which invokes this helper. The MCTS enumerator never emits the
/// corresponding Choice, so the AI's game tree is unchanged by this path.
pub fn destroy_workshop_card_and_trigger<R: Rng>(
    state: &mut GameState,
    card_instance_id: u32,
    rng: &mut R,
) {
    let id = card_instance_id as u8;
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    let removed = remove_from_workshop_area(player, id);
    assert!(
        removed,
        "Card not found in player's workshop area (workshopCards or workshoppedCards)"
    );

    let card = state.card_lookup[id as usize];
    let ability = card.ability();
    state.destroyed_pile.insert(id);

    let action_state = get_action_state_mut(state);
    action_state.ability_stack.push(ability);
    process_ability_stack(state, rng);
}

pub fn process_ability_stack<R: Rng>(state: &mut GameState, rng: &mut R) {
    loop {
        let action_state = get_action_state(state);
        let Some(ability) = action_state.ability_stack.last().copied() else {
            return;
        };
        let player_index = action_state.current_player_index;

        match ability {
            Ability::DrawCards { count } => {
                get_action_state_mut(state).ability_stack.pop();
                if is_replaying(state) {
                    replay_player_deck_draw(state, player_index);
                } else {
                    let before = state.players[player_index].workshop_cards;
                    let player = &mut state.players[player_index];
                    draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, count as usize, rng);
                    record_player_deck_draw(state, player_index, before);
                }
                continue;
            }
            Ability::Workshop { .. } => {
                if state.players[player_index].workshop_cards.is_empty() {
                    get_action_state_mut(state).ability_stack.pop();
                    continue;
                } else {
                    return; // waiting for input
                }
            }
            Ability::GainDucats { count } => {
                get_action_state_mut(state).ability_stack.pop();
                state.players[player_index].ducats += count;
                state.players[player_index].cached_score += count;
                continue;
            }
            Ability::MixColors { .. } => {
                return; // always needs input
            }
            Ability::DestroyCards => {
                return; // always needs input
            }
            Ability::Sell => {
                if can_sell_to_any_sell_card(state) {
                    return; // waiting for input
                } else {
                    get_action_state_mut(state).ability_stack.pop();
                    continue;
                }
            }
            Ability::GainSecondary => {
                return; // always needs input
            }
            Ability::GainPrimary => {
                return; // always needs input
            }
            Ability::ChangeTertiary => {
                let player = &state.players[player_index];
                let has_tertiary = TERTIARIES.iter().any(|&c| player.color_wheel.get(c) > 0);
                if has_tertiary {
                    return; // waiting for input
                } else {
                    get_action_state_mut(state).ability_stack.pop();
                    continue;
                }
            }
            Ability::MoveToDrafted => {
                let player = &state.players[player_index];
                if player.workshop_cards.is_empty() && player.workshopped_cards.is_empty() {
                    get_action_state_mut(state).ability_stack.pop();
                    continue;
                } else {
                    return; // waiting for input
                }
            }
            Ability::MoveToWorkshop => {
                if state.players[player_index].drafted_cards.is_empty() {
                    get_action_state_mut(state).ability_stack.pop();
                    continue;
                } else {
                    return; // waiting for input
                }
            }
        }
    }
}

#[inline]
pub(crate) fn can_afford_sell_card(player: &PlayerState, sell_card: &SellCard) -> bool {
    player.materials.get(sell_card.required_material()) >= 1
        && can_pay_cost(&player.color_wheel, sell_card.color_cost())
}

#[inline]
pub fn can_sell_to_any_sell_card(state: &GameState) -> bool {
    let action_state = get_action_state(state);
    let player = &state.players[action_state.current_player_index];
    state
        .sell_card_display
        .iter()
        .any(|b| can_afford_sell_card(player, &b.sell_card))
}


pub fn resolve_workshop_choice<R: Rng>(
    state: &mut GameState,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    let action_state = get_action_state(state);
    let count = match action_state.ability_stack.last() {
        Some(Ability::Workshop { count }) => *count,
        _ => panic!("No pending workshop choice"),
    };
    let player_index = action_state.current_player_index;
    let remaining = count - selected_cards.len();

    // Pop the Workshop ability from the stack
    get_action_state_mut(state).ability_stack.pop();

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

    let stack = &mut get_action_state_mut(state).ability_stack;
    push_abilities_to_stack(stack, &collected, Some(remaining));

    process_ability_stack(state, rng);
}


pub fn skip_workshop<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).ability_stack.pop();
    process_ability_stack(state, rng);
}

pub fn resolve_mix_colors<R: Rng>(
    state: &mut GameState,
    color_a: Color,
    color_b: Color,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;
    let success = perform_mix(&mut state.players[player_index].color_wheel, color_a, color_b);
    if !success {
        panic!("Cannot mix {:?} and {:?}", color_a, color_b);
    }
    finish_mix(state, rng);
}

/// Same as `resolve_mix_colors` but skips the `can_mix` and wheel amount checks.
/// The caller must guarantee the mix is valid.
pub fn resolve_mix_colors_unchecked<R: Rng>(
    state: &mut GameState,
    color_a: Color,
    color_b: Color,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;
    perform_mix_unchecked(&mut state.players[player_index].color_wheel, color_a, color_b);
    finish_mix(state, rng);
}

/// Decrement the remaining mix count and process the ability stack.
fn finish_mix<R: Rng>(state: &mut GameState, rng: &mut R) {
    let action_state = get_action_state_mut(state);
    match action_state.ability_stack.last_mut() {
        Some(Ability::MixColors { count }) => {
            *count -= 1;
            if *count == 0 {
                action_state.ability_stack.pop();
            }
        }
        _ => {}
    }
    process_ability_stack(state, rng);
}

pub fn skip_mix<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).ability_stack.pop();
    process_ability_stack(state, rng);
}

pub fn resolve_destroy_cards<R: Rng>(
    state: &mut GameState,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    // Pop the DestroyCards ability from the stack
    get_action_state_mut(state).ability_stack.pop();

    for id in selected_cards.iter() {
        let removed = remove_from_workshop_area(&mut state.players[player_index], id);
        assert!(
            removed,
            "Card not found in player's workshop area (workshopCards or workshoppedCards)"
        );

        let card = state.card_lookup[id as usize];
        let ability = card.ability();
        state.destroyed_pile.insert(id);
        get_action_state_mut(state).ability_stack.push(ability);
    }

    process_ability_stack(state, rng);
}

pub fn resolve_select_sell_card<R: Rng>(
    state: &mut GameState,
    sell_card_instance_id: u32,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    // Pop the Sell ability from the stack
    get_action_state_mut(state).ability_stack.pop();

    let sell_card_index = state
        .sell_card_display
        .iter()
        .position(|c| c.instance_id == sell_card_instance_id)
        .expect("Sell card not found in sell card display");

    let sell_card_instance = state.sell_card_display.swap_remove(sell_card_index);

    let player = &mut state.players[player_index];
    if !player.materials.decrement(sell_card_instance.sell_card.required_material()) {
        panic!("Not enough stored material");
    }
    let success = pay_cost(&mut player.color_wheel, sell_card_instance.sell_card.color_cost());
    if !success {
        panic!("Cannot pay sell card color cost");
    }
    player.cached_score += sell_card_instance.sell_card.ducats();
    player.completed_sell_cards.push(sell_card_instance);

    // Refill sell card display from sell_card_deck
    if is_replaying(state) {
        replay_sell_card_reveal(state);
    } else if let Some(id) = state.sell_card_deck.draw(rng) {
        let revealed = SellCardInstance {
            instance_id: id as u32,
            sell_card: state.sell_card_lookup[id as usize],
        };
        if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
            log.push(DrawEvent::SellCardReveal {
                sell_card: revealed,
            });
        }
        state.sell_card_display.push(revealed);
    }

    process_ability_stack(state, rng);
}

pub fn resolve_gain_color<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    state.players[player_index].color_wheel.increment(color);
    get_action_state_mut(state).ability_stack.pop();
    process_ability_stack(state, rng);
}

pub fn resolve_choose_tertiary_to_lose(state: &mut GameState, color: Color) {
    let player_index = get_action_state(state).current_player_index;
    state.players[player_index].color_wheel.decrement(color);
}

pub fn resolve_choose_tertiary_to_gain<R: Rng>(
    state: &mut GameState,
    color: Color,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;
    state.players[player_index].color_wheel.increment(color);
    get_action_state_mut(state).ability_stack.pop();
    process_ability_stack(state, rng);
}


pub fn end_player_turn<R: Rng>(state: &mut GameState, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    // Move remaining cards to discard
    player.discard = player.discard.union(player.drafted_cards).union(player.workshopped_cards).union(player.workshop_cards);
    player.drafted_cards = UnorderedCards::new();
    player.workshopped_cards = UnorderedCards::new();
    player.workshop_cards = UnorderedCards::new();

    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;

    let action_state = get_action_state_mut(state);
    action_state.current_player_index = (action_state.current_player_index + 1) % num_players;

    if action_state.current_player_index == starting_player {
        end_round(state, rng);
    } else {
        let action_state = get_action_state_mut(state);
        action_state.ability_stack.clear();
    }
}

pub fn end_round<R: Rng>(state: &mut GameState, _rng: &mut R) {
    state.round += 1;
    let any_reached_15 = state.players.iter().any(|p| p.cached_score >= 16);
    if any_reached_15 || state.round > state.max_rounds {
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
