use crate::color_wheel::{can_pay_cost, pay_cost, perform_mix, remove_color, store_color};
use crate::colors::TERTIARIES;
use crate::deck_utils::draw_from_deck;
use crate::scoring::calculate_score;
use crate::types::{
    Ability, ActionState, BuyerInstance, Color, GamePhase, GameState, PendingChoice,
};
use crate::unordered_cards::UnorderedCards;
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
            let player = &mut state.players[player_index];
            draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, count as usize, rng);
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
            state.players[player_index].cached_score += count;
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
    let remaining = count - selected_card_ids.len() as u32;

    // Partition selected cards into action and non-action using card_lookup
    let mut action_ids = [0u8; 16];
    let mut action_count = 0usize;
    let mut non_action_ids = [0u8; 16];
    let mut non_action_count = 0usize;
    for &raw_id in selected_card_ids {
        let id = raw_id as u8;
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
            store_color(&mut player.color_wheel, *pip);
        }
        player.discard.insert(id);
    }

    // Remove action cards from workshop, move to discard, collect abilities
    let mut regular_abilities = [Ability::Sell; 8];
    let mut regular_abilities_count = 0usize;
    let mut cot_abilities = [Ability::Sell; 8];
    let mut cot_abilities_count = 0usize;
    let mut vinegar_abilities = [Ability::Sell; 8];
    let mut vinegar_abilities_count = 0usize;
    let mut potash_base_count: Option<u32> = None;
    let mut has_cot = false;

    for i in 0..action_count {
        let id = action_ids[i];
        let card = state.card_lookup[id as usize];
        player.workshop_cards.remove(id);

        for &ability in card.workshop_abilities() {
            match ability {
                Ability::ChangeTertiary => {
                    vinegar_abilities[vinegar_abilities_count] = ability;
                    vinegar_abilities_count += 1;
                }
                Ability::Workshop { count: c } => {
                    potash_base_count = Some(potash_base_count.unwrap_or(0) + c);
                }
                Ability::DrawCards { .. } => {
                    has_cot = true;
                    cot_abilities[cot_abilities_count] = ability;
                    cot_abilities_count += 1;
                }
                _ => {
                    regular_abilities[regular_abilities_count] = ability;
                    regular_abilities_count += 1;
                }
            }
        }

        player.discard.insert(id);
    }

    // Clear pending choice and push abilities onto LIFO stack in reverse resolution order
    get_action_state_mut(state).pending_choice = None;
    let stack = &mut get_action_state_mut(state).ability_stack;

    if let Some(base) = potash_base_count {
        let potash_count = if has_cot {
            base
        } else {
            base + remaining
        };
        stack.push(Ability::Workshop { count: potash_count });
    }

    for i in 0..vinegar_abilities_count {
        stack.push(vinegar_abilities[i]);
    }

    if has_cot && remaining > 0 {
        stack.push(Ability::Workshop { count: remaining });
    }

    for i in 0..cot_abilities_count {
        stack.push(cot_abilities[i]);
    }

    for i in 0..regular_abilities_count {
        stack.push(regular_abilities[i]);
    }

    process_queue(state, rng);
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
        let id = card_id as u8;
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
    player.discard = player.discard.union(player.workshop_cards).union(player.drafted_cards);
    player.workshop_cards = UnorderedCards::new();
    player.drafted_cards = UnorderedCards::new();

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
