use crate::color_wheel::{can_pay_cost, pay_cost, perform_mix, remove_color, store_color};
use crate::colors::TERTIARIES;
use crate::deck_utils::draw_from_deck;
use crate::scoring::calculate_score;
use crate::types::{
    Ability, ActionState, Color, GamePhase, GameState, PendingChoice,
};
use rand::Rng;

pub fn initialize_action_phase(state: &mut GameState) {
    let action_state = ActionState {
        current_player_index: ((state.round - 1) as usize) % state.players.len(),
        ability_stack: vec![],
        pending_choice: None,
    };
    state.phase = GamePhase::Action { action_state };
}

pub fn destroy_drafted_card<R: Rng>(state: &mut GameState, card_instance_id: u32, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    let card_index = player
        .drafted_cards
        .iter()
        .position(|c| c.instance_id == card_instance_id)
        .expect("Card not found in player's draftedCards");

    let card = player.drafted_cards.remove(card_index);
    let ability = card.card.ability();
    state.destroyed_pile.push(card);

    get_action_state_mut(state).ability_stack.push(ability);
    process_queue(state, rng);
}

pub fn process_queue<R: Rng>(state: &mut GameState, rng: &mut R) {
    let as_ = get_action_state(state);
    if as_.pending_choice.is_some() {
        return;
    }
    if as_.ability_stack.is_empty() {
        return;
    }

    let ability = get_action_state_mut(state).ability_stack.pop().unwrap();
    let player_index = get_action_state(state).current_player_index;

    match ability {
        Ability::DrawCards { count } => {
            let drawn = draw_from_deck(&mut state.players[player_index], count as usize, rng);
            state.players[player_index].workshop_cards.extend(drawn);
            process_queue(state, rng);
        }
        Ability::Workshop { count } => {
            if state.players[player_index].workshop_cards.is_empty() {
                process_queue(state, rng);
            } else {
                get_action_state_mut(state).pending_choice =
                    Some(PendingChoice::ChooseCardsForWorkshop { count });
            }
        }
        Ability::GainDucats { count } => {
            state.players[player_index].ducats += count;
            process_queue(state, rng);
        }
        Ability::MixColors { count } => {
            get_action_state_mut(state).pending_choice =
                Some(PendingChoice::ChooseMix { remaining: count });
        }
        Ability::DestroyCards { count } => {
            get_action_state_mut(state).pending_choice =
                Some(PendingChoice::ChooseCardsToDestroy { count });
        }
        Ability::Sell => {
            if can_sell_to_any_buyer(state) {
                get_action_state_mut(state).pending_choice = Some(PendingChoice::ChooseBuyer);
            } else {
                process_queue(state, rng);
            }
        }
        Ability::GainSecondary => {
            get_action_state_mut(state).pending_choice =
                Some(PendingChoice::ChooseSecondaryColor);
        }
        Ability::GainPrimary => {
            get_action_state_mut(state).pending_choice = Some(PendingChoice::ChoosePrimaryColor);
        }
        Ability::ChangeTertiary => {
            let player = &state.players[player_index];
            let has_tertiary = TERTIARIES.iter().any(|&c| player.color_wheel.get(c) > 0);
            if has_tertiary {
                get_action_state_mut(state).pending_choice =
                    Some(PendingChoice::ChooseTertiaryToLose);
            } else {
                process_queue(state, rng);
            }
        }
    }
}

fn can_sell_to_any_buyer(state: &GameState) -> bool {
    let as_ = get_action_state(state);
    let player = &state.players[as_.current_player_index];
    for buyer_instance in &state.buyer_display {
        if player.materials.get(buyer_instance.buyer.required_material()) >= 1
            && can_pay_cost(&player.color_wheel, buyer_instance.buyer.color_cost())
        {
            return true;
        }
    }
    false
}

#[allow(dead_code)]
pub fn can_sell(state: &GameState, buyer_instance_id: u32) -> bool {
    let as_ = get_action_state(state);
    let player = &state.players[as_.current_player_index];
    let buyer_instance = state
        .buyer_display
        .iter()
        .find(|g| g.instance_id == buyer_instance_id);
    match buyer_instance {
        Some(bi) => {
            player.materials.get(bi.buyer.required_material()) >= 1
                && can_pay_cost(&player.color_wheel, bi.buyer.color_cost())
        }
        None => false,
    }
}

pub fn resolve_workshop_choice<R: Rng>(
    state: &mut GameState,
    selected_card_ids: &[u32],
    rng: &mut R,
) {
    let as_ = get_action_state(state);
    let count = match &as_.pending_choice {
        Some(PendingChoice::ChooseCardsForWorkshop { count }) => *count,
        _ => panic!("No pending workshop choice"),
    };
    let player_index = as_.current_player_index;

    // Check if selection contains an action card
    let action_card_id = selected_card_ids.iter().find(|&&id| {
        state.players[player_index]
            .workshop_cards
            .iter()
            .any(|c| c.instance_id == id && c.card.is_action())
    });

    if let Some(&action_id) = action_card_id {
        // Action card selected: consume 1 pick
        let card_index = state.players[player_index]
            .workshop_cards
            .iter()
            .position(|c| c.instance_id == action_id)
            .unwrap();
        let card = state.players[player_index].workshop_cards.remove(card_index);

        let remaining = count - 1;
        get_action_state_mut(state).pending_choice = None;

        // Push remaining workshop picks onto stack first (bottom)
        if remaining > 0 {
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::Workshop { count: remaining });
        }

        // Push workshopAbilities in reverse order
        for ability in card.card.workshop_abilities().iter().rev() {
            get_action_state_mut(state)
                .ability_stack
                .push(*ability);
        }

        // Move card to discard
        state.players[player_index].discard.push(card);
        process_queue(state, rng);
    } else {
        // Non-action cards: process all at once
        let player = &mut state.players[player_index];
        for &card_id in selected_card_ids {
            let card_index = player
                .workshop_cards
                .iter()
                .position(|c| c.instance_id == card_id)
                .expect("Card not found in workshopCards");

            let card = player.workshop_cards.remove(card_index);

            for mt in card.card.material_types() {
                player.materials.increment(*mt);
            }
            for pip in card.card.pips() {
                store_color(&mut player.color_wheel, *pip);
            }
            player.discard.push(card);
        }

        get_action_state_mut(state).pending_choice = None;
        process_queue(state, rng);
    }
}

pub fn skip_workshop<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
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
        let as_ = get_action_state(state);
        match &as_.pending_choice {
            Some(PendingChoice::ChooseMix { remaining }) => remaining - 1,
            _ => 0,
        }
    };

    if new_remaining > 0 {
        get_action_state_mut(state).pending_choice =
            Some(PendingChoice::ChooseMix {
                remaining: new_remaining,
            });
    } else {
        get_action_state_mut(state).pending_choice = None;
    }

    process_queue(state, rng);
}

pub fn skip_mix<R: Rng>(state: &mut GameState, rng: &mut R) {
    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
}

pub fn resolve_destroy_cards<R: Rng>(
    state: &mut GameState,
    selected_card_ids: &[u32],
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    for &card_id in selected_card_ids {
        let card_index = state.players[player_index]
            .workshop_cards
            .iter()
            .position(|c| c.instance_id == card_id)
            .expect("Card not found in workshopCards");

        let card = state.players[player_index]
            .workshop_cards
            .remove(card_index);
        let ability = card.card.ability();
        state.destroyed_pile.push(card);
        get_action_state_mut(state).ability_stack.push(ability);
    }

    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
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

    let buyer = state.buyer_display.remove(buyer_index);

    let player = &mut state.players[player_index];
    if !player.materials.decrement(buyer.buyer.required_material()) {
        panic!("Not enough stored material");
    }
    let success = pay_cost(&mut player.color_wheel, buyer.buyer.color_cost());
    if !success {
        panic!("Cannot pay buyer color cost");
    }
    player.completed_buyers.push(buyer);

    // Refill buyer display
    if let Some(new_buyer) = state.buyer_deck.pop() {
        state.buyer_display.push(new_buyer);
    }

    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
}

pub fn resolve_gain_secondary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    store_color(&mut state.players[player_index].color_wheel, color);
    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
}

pub fn resolve_gain_primary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    store_color(&mut state.players[player_index].color_wheel, color);
    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
}

pub fn resolve_choose_tertiary_to_lose(state: &mut GameState, color: Color) {
    let player_index = get_action_state(state).current_player_index;
    remove_color(&mut state.players[player_index].color_wheel, color);
    get_action_state_mut(state).pending_choice = Some(PendingChoice::ChooseTertiaryToGain {
        lost_color: color,
    });
}

pub fn resolve_choose_tertiary_to_gain<R: Rng>(
    state: &mut GameState,
    color: Color,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;
    store_color(&mut state.players[player_index].color_wheel, color);
    get_action_state_mut(state).pending_choice = None;
    process_queue(state, rng);
}

pub fn end_player_turn<R: Rng>(state: &mut GameState, rng: &mut R) {
    let player_index = get_action_state(state).current_player_index;
    let player = &mut state.players[player_index];

    // Move remaining cards to discard
    let mut workshop = std::mem::take(&mut player.workshop_cards);
    let mut drafted = std::mem::take(&mut player.drafted_cards);
    player.discard.append(&mut workshop);
    player.discard.append(&mut drafted);

    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;

    let as_ = get_action_state_mut(state);
    as_.current_player_index = (as_.current_player_index + 1) % num_players;

    if as_.current_player_index == starting_player {
        end_round(state, rng);
    } else {
        let as_ = get_action_state_mut(state);
        as_.ability_stack.clear();
        as_.pending_choice = None;
    }
}

pub fn end_round<R: Rng>(state: &mut GameState, _rng: &mut R) {
    state.round += 1;
    let any_reached_15 = state.players.iter().any(|p| calculate_score(p) >= 15);
    if any_reached_15 || state.round > 10 {
        state.phase = GamePhase::GameOver;
    } else {
        state.phase = GamePhase::Draw;
    }
}

fn get_action_state(state: &GameState) -> &ActionState {
    match &state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}

fn get_action_state_mut(state: &mut GameState) -> &mut ActionState {
    match &mut state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => panic!("Expected action phase"),
    }
}
