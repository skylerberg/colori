use crate::colors::{can_pay_cost, pay_cost, perform_mix, perform_mix_unchecked, PRIMARIES, TERTIARIES};
use crate::deck_utils::draw_from_deck;
use crate::types::{
    Ability, ActionState, BuyerCard, BuyerInstance, Color, GamePhase, GameState, GlassCard,
    PlayerState,
};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use smallvec::SmallVec;

pub fn initialize_action_phase(state: &mut GameState) {
    let action_state = ActionState {
        current_player_index: ((state.round - 1) as usize) % state.players.len(),
        ability_stack: SmallVec::new(),
        used_glass: 0,
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
        let Some(ability) = action_state.ability_stack.last().copied() else {
            return;
        };
        let player_index = action_state.current_player_index;

        match ability {
            Ability::DrawCards { count } => {
                get_action_state_mut(state).ability_stack.pop();
                let player = &mut state.players[player_index];
                draw_from_deck(&mut player.deck, &mut player.discard, &mut player.workshop_cards, count as usize, rng);
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
                if can_sell_to_any_buyer(state) || can_sell_to_any_glass(state) {
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
        }
    }
}

#[inline]
pub(crate) fn can_afford_buyer(player: &PlayerState, buyer: &BuyerCard) -> bool {
    player.materials.get(buyer.required_material()) >= 1
        && can_pay_cost(&player.color_wheel, buyer.color_cost())
}

#[inline]
pub fn can_sell_to_any_buyer(state: &GameState) -> bool {
    let action_state = get_action_state(state);
    let player = &state.players[action_state.current_player_index];
    state
        .buyer_display
        .iter()
        .any(|b| can_afford_buyer(player, &b.buyer))
}

pub fn can_afford_glass(player: &PlayerState) -> bool {
    PRIMARIES.iter().any(|&c| player.color_wheel.get(c) >= 4)
}

pub fn can_sell_to_any_glass(state: &GameState) -> bool {
    if !state.expansions.glass || state.glass_display.is_empty() {
        return false;
    }
    let player_index = get_action_state(state).current_player_index;
    can_afford_glass(&state.players[player_index])
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

    // Push abilities onto LIFO stack in reverse resolution order
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

/// Workshop cards including one card that gets workshopped twice via GlassReworkshop.
/// The reworkshop_id card is processed with all others first, then un-rotated and processed again.
pub fn resolve_workshop_with_reworkshop<R: Rng>(
    state: &mut GameState,
    selected_cards: UnorderedCards,
    reworkshop_id: u8,
    rng: &mut R,
) {
    let action_state = get_action_state(state);
    let player_index = action_state.current_player_index;

    // Pop the Workshop ability from the stack
    get_action_state_mut(state).ability_stack.pop();

    // Partition selected cards into action and non-action
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

    // Process non-action cards: extract materials/colors, move to workshopped
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

    // Process action cards: move to workshopped, collect abilities
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

    // Now un-rotate the reworkshop card and process it a second time
    let player = &mut state.players[player_index];
    player.workshopped_cards.remove(reworkshop_id);
    player.workshop_cards.insert(reworkshop_id);

    let reworkshop_card = state.card_lookup[reworkshop_id as usize];
    let player = &mut state.players[player_index];
    player.workshop_cards.remove(reworkshop_id);
    if reworkshop_card.is_action() {
        for &ability in reworkshop_card.workshop_abilities() {
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
    } else {
        for mt in reworkshop_card.material_types() {
            player.materials.increment(*mt);
        }
        for pip in reworkshop_card.pips() {
            player.color_wheel.increment(*pip);
        }
    }
    player.workshopped_cards.insert(reworkshop_id);

    // The reworkshop card used 2 slots, other cards used 1 each.
    // Total slots used = 2 + other_cards.len() = selected_cards.len() + 1
    // remaining = count - total = already accounted for by the caller's Workshop count
    // We don't have the original count here, but we don't need remaining for the ability stack
    // since the Workshop was already popped.

    // Push abilities onto LIFO stack in reverse resolution order
    let stack = &mut get_action_state_mut(state).ability_stack;

    if let Some(base) = potash_base_count {
        let potash_count = if has_draw_cards {
            base
        } else {
            base
        };
        stack.push(Ability::Workshop { count: potash_count });
    }

    for i in 0..change_tertiary_abilities_count {
        stack.push(change_tertiary_abilities[i]);
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

    process_ability_stack(state, rng);
}

pub fn resolve_select_buyer<R: Rng>(
    state: &mut GameState,
    buyer_instance_id: u32,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;

    // Pop the Sell ability from the stack
    get_action_state_mut(state).ability_stack.pop();

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

    process_ability_stack(state, rng);
}

pub fn resolve_gain_secondary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    resolve_gain_color(state, color, rng);
}

pub fn resolve_gain_primary<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
    resolve_gain_color(state, color, rng);
}

fn resolve_gain_color<R: Rng>(state: &mut GameState, color: Color, rng: &mut R) {
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

#[allow(dead_code)]
pub(crate) fn glass_ability_available(state: &GameState, player_index: usize, glass: GlassCard) -> bool {
    let action_state = get_action_state(state);
    let bit = 1u16 << (glass as u16);
    if action_state.used_glass & bit != 0 {
        return false;
    }
    state.players[player_index].completed_glass.iter().any(|g| g.glass == glass)
}

pub(crate) fn mark_glass_used(state: &mut GameState, glass: GlassCard) {
    let action_state = get_action_state_mut(state);
    action_state.used_glass |= 1u16 << (glass as u16);
}

pub fn resolve_select_glass<R: Rng>(
    state: &mut GameState,
    glass_card: GlassCard,
    pay_color: Color,
    rng: &mut R,
) {
    let player_index = get_action_state(state).current_player_index;
    get_action_state_mut(state).ability_stack.pop();

    let glass_index = state.glass_display.iter()
        .position(|g| g.glass == glass_card)
        .expect("Glass card not found in display");
    let glass_instance = state.glass_display.swap_remove(glass_index);

    let player = &mut state.players[player_index];
    for _ in 0..4 {
        assert!(player.color_wheel.decrement(pay_color), "Not enough color to pay");
    }
    player.completed_glass.push(glass_instance);

    if let Some(next) = state.glass_deck.pop() {
        state.glass_display.push(next);
    }

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
        let is_last_round = state.round >= 10
            || state.players.iter().any(|p| p.cached_score >= 15);
        if is_last_round {
            end_round(state, rng);
        } else {
            end_round(state, rng);
        }
    } else {
        let action_state = get_action_state_mut(state);
        action_state.ability_stack.clear();
        action_state.used_glass = 0;
    }
}

pub fn end_round<R: Rng>(state: &mut GameState, _rng: &mut R) {
    state.round += 1;
    let any_reached_15 = state.players.iter().any(|p| p.cached_score >= 15);
    if any_reached_15 || state.round > 20 {
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
