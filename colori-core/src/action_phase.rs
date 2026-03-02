use crate::colors::{can_pay_cost, pay_cost, perform_mix, perform_mix_unchecked, TERTIARIES};
use crate::deck_utils::draw_from_deck;
use crate::types::{
    Ability, ActionState, BuyerInstance, CleanupState, Color, GamePhase, GameState, PendingChoice,
};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use smallvec::SmallVec;

pub fn initialize_action_phase(state: &mut GameState) {
    let action_state = ActionState {
        current_player_index: ((state.round - 1) as usize) % state.players.len(),
        ability_stack: SmallVec::new(),
        pending_choice: None,
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

pub fn process_ability_stack<R: Rng>(state: &mut GameState, rng: &mut R) {
    loop {
        let action_state = get_action_state(state);
        if action_state.pending_choice.is_some() {
            return;
        }
        if action_state.ability_stack.is_empty() {
            return;
        }

        let ability = get_action_state_mut(state).ability_stack.pop().unwrap();
        let player_index = get_action_state(state).current_player_index;

        match ability {
            Ability::DrawCards { count } => {
                let player = &mut state.players[player_index];
                draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, count as usize, rng);
                continue;
            }
            Ability::Workshop { count } => {
                if state.players[player_index].workshop_cards.is_empty() {
                    continue;
                } else {
                    get_action_state_mut(state).pending_choice =
                        Some(PendingChoice::ChooseCardsForWorkshop { remaining_picks: count });
                    return;
                }
            }
            Ability::GainDucats { count } => {
                state.players[player_index].ducats += count;
                state.players[player_index].cached_score += count;
                continue;
            }
            Ability::MixColors { count } => {
                get_action_state_mut(state).pending_choice =
                    Some(PendingChoice::ChooseMix { remaining_mixes: count });
                return;
            }
            Ability::DestroyCards => {
                get_action_state_mut(state).pending_choice =
                    Some(PendingChoice::ChooseCardsToDestroy);
                return;
            }
            Ability::Sell => {
                if can_sell_to_any_buyer(state) {
                    get_action_state_mut(state).pending_choice = Some(PendingChoice::ChooseBuyer);
                    return;
                } else {
                    continue;
                }
            }
            Ability::GainSecondary => {
                get_action_state_mut(state).pending_choice =
                    Some(PendingChoice::ChooseSecondaryColor);
                return;
            }
            Ability::GainPrimary => {
                get_action_state_mut(state).pending_choice = Some(PendingChoice::ChoosePrimaryColor);
                return;
            }
            Ability::ChangeTertiary => {
                let player = &state.players[player_index];
                let has_tertiary = TERTIARIES.iter().any(|&c| player.color_wheel.get(c) > 0);
                if has_tertiary {
                    get_action_state_mut(state).pending_choice =
                        Some(PendingChoice::ChooseTertiaryToLose);
                    return;
                } else {
                    continue;
                }
            }
        }
    }
}

#[inline]
pub fn can_sell_to_any_buyer(state: &GameState) -> bool {
    let action_state = get_action_state(state);
    let player = &state.players[action_state.current_player_index];
    for buyer_instance in &state.buyer_display {
        if player.materials.get(buyer_instance.buyer.required_material()) >= 1
            && can_pay_cost(&player.color_wheel, buyer_instance.buyer.color_cost())
        {
            return true;
        }
    }
    false
}

pub fn resolve_workshop_choice<R: Rng>(
    state: &mut GameState,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    let action_state = get_action_state(state);
    let remaining_picks = match &action_state.pending_choice {
        Some(PendingChoice::ChooseCardsForWorkshop { remaining_picks }) => *remaining_picks,
        _ => panic!("No pending workshop choice"),
    };
    let player_index = action_state.current_player_index;
    let remaining = remaining_picks - selected_cards.len();

    // Partition selected cards into action and non-action using card_lookup
    let mut action_ids = [0u8; 16];
    let mut action_count = 0usize;
    let mut non_action_ids = [0u8; 16];
    let mut non_action_count = 0usize;
    for id in selected_cards.iter() {
        let card = state.card_lookup[id as usize];
        if card.is_action() {
            action_ids[action_count] = id;
            action_count += 1;
        } else {
            non_action_ids[non_action_count] = id;
            non_action_count += 1;
        }
    }

    // Process non-action cards: extract materials/colors, move to discard
    let player = &mut state.players[player_index];
    for i in 0..non_action_count {
        let id = non_action_ids[i];
        let card = state.card_lookup[id as usize];
        player.workshop_cards.remove(id);
        for mt in card.material_types() {
            player.materials.increment(*mt);
        }
        for pip in card.pips() {
            player.color_wheel.increment(*pip);
        }
        player.workshopped_cards.insert(id);
    }

    // Remove action cards from workshop, move to discard, collect abilities
    let mut regular_abilities = [Ability::Sell; 8];
    let mut regular_abilities_count = 0usize;
    let mut draw_card_abilities = [Ability::Sell; 8];
    let mut draw_card_abilities_count = 0usize;
    let mut change_tertiary_abilities = [Ability::Sell; 8];
    let mut change_tertiary_abilities_count = 0usize;
    let mut potash_base_count: Option<u32> = None;
    let mut has_draw_cards = false;

    for i in 0..action_count {
        let id = action_ids[i];
        let card = state.card_lookup[id as usize];
        player.workshop_cards.remove(id);

        for &ability in card.workshop_abilities() {
            match ability {
                Ability::ChangeTertiary => {
                    change_tertiary_abilities[change_tertiary_abilities_count] = ability;
                    change_tertiary_abilities_count += 1;
                }
                Ability::Workshop { count: c } => {
                    potash_base_count = Some(potash_base_count.unwrap_or(0) + c);
                }
                Ability::DrawCards { .. } => {
                    has_draw_cards = true;
                    draw_card_abilities[draw_card_abilities_count] = ability;
                    draw_card_abilities_count += 1;
                }
                _ => {
                    regular_abilities[regular_abilities_count] = ability;
                    regular_abilities_count += 1;
                }
            }
        }

        player.workshopped_cards.insert(id);
    }

    // Clear pending choice and push abilities onto LIFO stack in reverse resolution order
    get_action_state_mut(state).pending_choice = None;
    let stack = &mut get_action_state_mut(state).ability_stack;

    if let Some(base) = potash_base_count {
        let potash_count = if has_draw_cards {
            base
        } else {
            base + remaining
        };
        stack.push(Ability::Workshop { count: potash_count });
    }

    for i in 0..change_tertiary_abilities_count {
        stack.push(change_tertiary_abilities[i]);
    }

    if has_draw_cards && remaining > 0 {
        stack.push(Ability::Workshop { count: remaining });
    }

    for i in 0..draw_card_abilities_count {
        stack.push(draw_card_abilities[i]);
    }

    for i in 0..regular_abilities_count {
        stack.push(regular_abilities[i]);
    }

    process_ability_stack(state, rng);
}

pub fn skip_workshop<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).pending_choice = None;
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

    let new_remaining = {
        let action_state = get_action_state(state);
        match &action_state.pending_choice {
            Some(PendingChoice::ChooseMix { remaining_mixes }) => remaining_mixes - 1,
            _ => 0,
        }
    };

    if new_remaining > 0 {
        get_action_state_mut(state).pending_choice =
            Some(PendingChoice::ChooseMix {
                remaining_mixes: new_remaining,
            });
    } else {
        get_action_state_mut(state).pending_choice = None;
    }

    process_ability_stack(state, rng);
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

    let new_remaining = {
        let action_state = get_action_state(state);
        match &action_state.pending_choice {
            Some(PendingChoice::ChooseMix { remaining_mixes }) => remaining_mixes - 1,
            _ => 0,
        }
    };

    if new_remaining > 0 {
        get_action_state_mut(state).pending_choice =
            Some(PendingChoice::ChooseMix {
                remaining_mixes: new_remaining,
            });
    } else {
        get_action_state_mut(state).pending_choice = None;
    }

    process_ability_stack(state, rng);
}

pub fn skip_mix<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).pending_choice = None;
    process_ability_stack(state, rng);
}

pub fn resolve_destroy_cards<R: Rng>(
    state: &mut GameState,
    selected_cards: UnorderedCards,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    for id in selected_cards.iter() {
        assert!(
            state.players[player_index].workshop_cards.contains(id),
            "Card not found in workshopCards"
        );
        state.players[player_index].workshop_cards.remove(id);

        let card = state.card_lookup[id as usize];
        let ability = card.ability();
        state.destroyed_pile.insert(id);
        get_action_state_mut(state).ability_stack.push(ability);
    }

    get_action_state_mut(state).pending_choice = None;
    process_ability_stack(state, rng);
}

pub fn resolve_select_buyer<R: Rng>(
    state: &mut GameState,
    buyer_instance_id: u32,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    let buyer_index = state
        .buyer_display
        .iter()
        .position(|c| c.instance_id == buyer_instance_id)
        .expect("Buyer not found in buyer display");

    let buyer = state.buyer_display.swap_remove(buyer_index);

    let player = &mut state.players[player_index];
    if !player.materials.decrement(buyer.buyer.required_material()) {
        panic!("Not enough stored material");
    }
    let success = pay_cost(&mut player.color_wheel, buyer.buyer.color_cost());
    if !success {
        panic!("Cannot pay buyer color cost");
    }
    player.cached_score += buyer.buyer.stars();
    player.completed_buyers.push(buyer);

    // Refill buyer display from buyer_deck
    if let Some(id) = state.buyer_deck.draw(rng) {
        state.buyer_display.push(BuyerInstance {
            instance_id: id as u32,
            buyer: state.buyer_lookup[id as usize],
        });
    }

    get_action_state_mut(state).pending_choice = None;
    process_ability_stack(state, rng);
}

pub fn resolve_gain_secondary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    state.players[player_index].color_wheel.increment(color);
    get_action_state_mut(state).pending_choice = None;
    process_ability_stack(state, rng);
}

pub fn resolve_gain_primary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    state.players[player_index].color_wheel.increment(color);
    get_action_state_mut(state).pending_choice = None;
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
    get_action_state_mut(state).pending_choice = None;
    process_ability_stack(state, rng);
}

pub fn end_player_turn<R: Rng>(state: &mut GameState, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    // Move remaining cards to discard
    player.discard = player.discard.union(player.drafted_cards).union(player.workshopped_cards);
    player.drafted_cards = UnorderedCards::new();
    player.workshopped_cards = UnorderedCards::new();

    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;

    let action_state = get_action_state_mut(state);
    action_state.current_player_index = (action_state.current_player_index + 1) % num_players;

    if action_state.current_player_index == starting_player {
        let is_last_round = state.round >= 10
            || state.players.iter().any(|p| p.cached_score >= 15);
        if is_last_round {
            end_round(state, rng);
        } else {
            initialize_cleanup_phase(state, rng);
        }
    } else {
        let action_state = get_action_state_mut(state);
        action_state.ability_stack.clear();
        action_state.pending_choice = None;
    }
}

pub fn end_round<R: Rng>(state: &mut GameState, _rng: &mut R) {
    state.round += 1;
    let any_reached_15 = state.players.iter().any(|p| p.cached_score >= 15);
    if any_reached_15 || state.round > 10 {
        state.phase = GamePhase::GameOver;
    } else {
        state.phase = GamePhase::Draw;
    }
}

pub fn initialize_cleanup_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    state.phase = GamePhase::Cleanup {
        cleanup_state: CleanupState {
            current_player_index: starting_player,
        },
    };
    advance_cleanup_to_next_nonempty(state, rng);
}

fn advance_cleanup_to_next_nonempty<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    loop {
        let current = get_cleanup_state(state).current_player_index;
        if !state.players[current].workshop_cards.is_empty() {
            return; // This player has workshop cards; wait for their choice
        }
        // Advance to next player
        let next = (current + 1) % num_players;
        if next == starting_player {
            // All players done, end round
            end_round(state, rng);
            return;
        }
        get_cleanup_state_mut(state).current_player_index = next;
    }
}

pub fn resolve_keep_workshop_cards<R: Rng>(
    state: &mut GameState,
    keep_ids: UnorderedCards,
    rng: &mut R,
) {
    let current = get_cleanup_state(state).current_player_index;
    let player = &mut state.players[current];

    // Discard cards NOT in keep_ids
    let to_discard = player.workshop_cards.difference(keep_ids);
    player.discard = player.discard.union(to_discard);
    player.workshop_cards = keep_ids;

    // Advance to next player
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    let next = (current + 1) % num_players;
    if next == starting_player {
        end_round(state, rng);
    } else {
        get_cleanup_state_mut(state).current_player_index = next;
        advance_cleanup_to_next_nonempty(state, rng);
    }
}

#[inline]
fn get_cleanup_state(state: &GameState) -> &CleanupState {
    match &state.phase {
        GamePhase::Cleanup { cleanup_state } => cleanup_state,
        _ => panic!("Expected cleanup phase"),
    }
}

#[inline]
fn get_cleanup_state_mut(state: &mut GameState) -> &mut CleanupState {
    match &mut state.phase {
        GamePhase::Cleanup { cleanup_state } => cleanup_state,
        _ => panic!("Expected cleanup phase"),
    }
}

#[inline]
fn get_action_state(state: &GameState) -> &ActionState {
    match &state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}

#[inline]
fn get_action_state_mut(state: &mut GameState) -> &mut ActionState {
    match &mut state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}
