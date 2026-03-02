use crate::action_phase::{
    destroy_drafted_card, end_player_turn, initialize_action_phase,
    process_ability_stack, resolve_choose_tertiary_to_gain, resolve_choose_tertiary_to_lose,
    resolve_destroy_cards, resolve_gain_primary, resolve_gain_secondary,
    resolve_keep_workshop_cards, resolve_select_buyer, resolve_workshop_choice, skip_workshop,
};
use crate::choices::can_afford_buyer;
use crate::colors::{pay_cost, perform_mix_unchecked, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::deck_utils::draw_from_deck;
use crate::draft_phase::player_pick;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use rand::RngExt;

// ── Rollout draw+draft shortcut ──

fn rollout_draw_and_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    // Step 1: Draw up to 5 cards from each player's personal deck (same as execute_draw_phase)
    for i in 0..num_players {
        let player = &mut state.players[i];
        let current = player.workshop_cards.len() as usize;
        let to_draw = if current >= 5 { 0 } else { 5 - current };
        if to_draw > 0 {
            draw_from_deck(
                &mut player.deck,
                &mut player.discard,
                &mut player.workshop_cards,
                to_draw,
                rng,
            );
        }
    }

    // Step 2: Pool all available draft cards
    let mut pool = state.draft_deck;
    state.draft_deck = UnorderedCards::new();

    let total_needed = 5 * num_players as u32;
    if pool.len() < total_needed && !state.destroyed_pile.is_empty() {
        pool = pool.union(state.destroyed_pile);
        state.destroyed_pile = UnorderedCards::new();
    }

    // Step 3: Check if enough cards for all players
    if pool.len() < 4 * num_players as u32 {
        state.destroyed_pile = state.destroyed_pile.union(pool);
        initialize_action_phase(state);
        return;
    }

    // Step 4: Distribute 4 random cards per player to drafted_cards
    for i in 0..num_players {
        let drawn = pool.draw_multiple(4, rng);
        state.players[i].drafted_cards = drawn;
    }

    // Step 5: Remaining cards go to destroyed_pile
    state.destroyed_pile = state.destroyed_pile.union(pool);

    // Step 6: Go directly to action phase
    initialize_action_phase(state);
}

// ── Fused rollout step ──

#[inline(always)]
fn random_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    remaining: u32,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    let mut mixes = [(Color::Red, Color::Red); 2];
    let mut count = 0usize;
    let mut sim_wheel = wheel.clone();
    for _ in 0..remaining {
        if count >= 2 {
            break;
        }
        let mut pairs: [(Color, Color); 9] = [(Color::Red, Color::Red); 9];
        let mut pair_count = 0usize;
        for &(a, b) in &VALID_MIX_PAIRS {
            if sim_wheel.get(a) > 0 && sim_wheel.get(b) > 0 {
                pairs[pair_count] = (a, b);
                pair_count += 1;
            }
        }
        if pair_count == 0 {
            break;
        }
        let target = rng.random_range(0..pair_count + 1);
        if target == pair_count {
            break;
        }
        let (a, b) = pairs[target];
        mixes[count] = (a, b);
        count += 1;
        perform_mix_unchecked(&mut sim_wheel, a, b);
    }
    (mixes, count)
}

#[inline(always)]
fn pick_random_affordable_buyer<R: Rng>(
    player: &PlayerState,
    buyer_display: &[BuyerInstance],
    rng: &mut R,
) -> Option<u32> {
    let mut affordable = [0u32; MAX_BUYER_DISPLAY];
    let mut count = 0usize;
    for buyer in buyer_display {
        if can_afford_buyer(player, &buyer.buyer) {
            affordable[count] = buyer.instance_id;
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(affordable[rng.random_range(0..count)])
    }
}

#[inline(always)]
fn handle_action_no_pending<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    let mut copy = state.players[player_index].drafted_cards;
    let sel = copy.draw_up_to(1, rng);
    if sel.is_empty() {
        // No drafted cards left — end turn, handle cleanup, and advance to next round
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Cleanup { .. }) {
            while matches!(state.phase, GamePhase::Cleanup { .. }) {
                if let GamePhase::Cleanup { ref cleanup_state } = state.phase {
                    let keep =
                        state.players[cleanup_state.current_player_index].workshop_cards;
                    resolve_keep_workshop_cards(state, keep, rng);
                }
            }
        }
        if matches!(state.phase, GamePhase::Draw) {
            rollout_draw_and_draft(state, rng);
        }
        return;
    }

    let card_id = sel.lowest_bit().unwrap();
    let card = state.card_lookup[card_id as usize];
    match card.ability() {
        Ability::MixColors { count } => {
            // Fused: ability stack is guaranteed empty when stack is empty,
            // so we can skip all process_ability_stack calls.
            let (mixes, mix_count) =
                random_mix_seq(&state.players[player_index].color_wheel, count, rng);
            state.players[player_index].drafted_cards.remove(card_id);
            state.destroyed_pile.insert(card_id);
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
            }
        }
        Ability::Sell => {
            match pick_random_affordable_buyer(
                &state.players[player_index],
                &state.buyer_display,
                rng,
            ) {
                Some(buyer_id) => {
                    // Fused: ability stack is guaranteed empty when stack is empty,
                    // so we can skip all process_ability_stack calls.
                    state.players[player_index].drafted_cards.remove(card_id);
                    state.destroyed_pile.insert(card_id);
                    let buyer_index = state
                        .buyer_display
                        .iter()
                        .position(|c| c.instance_id == buyer_id)
                        .unwrap();
                    let buyer = state.buyer_display.swap_remove(buyer_index);
                    let player = &mut state.players[player_index];
                    player.materials.decrement(buyer.buyer.required_material());
                    pay_cost(&mut player.color_wheel, buyer.buyer.color_cost());
                    player.cached_score += buyer.buyer.stars();
                    player.completed_buyers.push(buyer);
                    if let Some(id) = state.buyer_deck.draw(rng) {
                        state.buyer_display.push(BuyerInstance {
                            instance_id: id as u32,
                            buyer: state.buyer_lookup[id as usize],
                        });
                    }
                }
                None => {
                    destroy_drafted_card(state, card_id as u32, rng);
                }
            }
        }
        _ => {
            destroy_drafted_card(state, card_id as u32, rng);
        }
    }
}

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, rng: &mut R) {
    // Fast path: complete entire draft in one step
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            draft_state.waiting_for_pass = false;
        }
        loop {
            let card_id = {
                if let GamePhase::Draft { ref draft_state } = state.phase {
                    let hand = draft_state.hands[draft_state.current_player_index];
                    hand.pick_random(rng).unwrap() as u32
                } else {
                    break;
                }
            };
            player_pick(state, card_id);
            if let GamePhase::Draft { ref mut draft_state } = state.phase {
                draft_state.waiting_for_pass = false;
            }
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player_index = action_state.current_player_index;
            match action_state.ability_stack.last() {
                None => {
                    handle_action_no_pending(state, player_index, rng);
                }
                Some(Ability::Workshop { count }) => {
                    let count = *count;
                    let mut copy = state.players[player_index].workshop_cards;
                    let selected = copy.draw_up_to(count as u8, rng);
                    if selected.is_empty() {
                        skip_workshop(state, rng);
                    } else {
                        resolve_workshop_choice(state, selected, rng);
                    }
                }
                Some(Ability::DestroyCards) => {
                    let mut copy = state.players[player_index].workshop_cards;
                    let selected = copy.draw_up_to(1, rng);
                    resolve_destroy_cards(state, selected, rng);
                }
                Some(Ability::MixColors { count }) => {
                    // Fused: apply all mixes directly, then process_ability_stack once.
                    // Ability stack may have more items below, so we must call process_ability_stack.
                    let remaining_mixes = *count;
                    let (mixes, mix_count) =
                        random_mix_seq(&state.players[player_index].color_wheel, remaining_mixes, rng);
                    for i in 0..mix_count {
                        let (a, b) = mixes[i];
                        perform_mix_unchecked(
                            &mut state.players[player_index].color_wheel,
                            a,
                            b,
                        );
                    }
                    if let GamePhase::Action { ref mut action_state } = state.phase {
                        action_state.ability_stack.pop();
                    }
                    process_ability_stack(state, rng);
                }
                Some(Ability::Sell) => {
                    let buyer_id = pick_random_affordable_buyer(
                        &state.players[player_index],
                        &state.buyer_display,
                        rng,
                    )
                    .unwrap();
                    resolve_select_buyer(state, buyer_id, rng);
                }
                Some(Ability::GainSecondary) => {
                    let color = SECONDARIES[rng.random_range(0..SECONDARIES.len())];
                    resolve_gain_secondary(state, color, rng);
                }
                Some(Ability::GainPrimary) => {
                    let color = PRIMARIES[rng.random_range(0..PRIMARIES.len())];
                    resolve_gain_primary(state, color, rng);
                }
                Some(Ability::ChangeTertiary) => {
                    let player = &state.players[player_index];
                    let mut owned_tertiaries = [Color::Red; 6];
                    let mut own_count = 0usize;
                    for &c in &TERTIARIES {
                        if player.color_wheel.get(c) > 0 {
                            owned_tertiaries[own_count] = c;
                            own_count += 1;
                        }
                    }
                    let r = rng.random_range(0..own_count * 5);
                    let lose_idx = r / 5;
                    let gain_local_idx = r % 5;
                    let lose_color = owned_tertiaries[lose_idx];
                    let mut options = [Color::Red; 6];
                    let mut opt_count = 0usize;
                    for &c in &TERTIARIES {
                        if c != lose_color {
                            options[opt_count] = c;
                            opt_count += 1;
                        }
                    }
                    let gain_color = options[gain_local_idx];
                    resolve_choose_tertiary_to_lose(state, lose_color);
                    resolve_choose_tertiary_to_gain(state, gain_color, rng);
                }
                // Instant abilities should never be on top waiting — they get processed immediately
                Some(_) => panic!("Unexpected ability on stack top during rollout"),
            }
        }
        GamePhase::Cleanup { cleanup_state } => {
            let mut all_workshop_cards = state.players[cleanup_state.current_player_index].workshop_cards;
            let selected = all_workshop_cards.draw_up_to(all_workshop_cards.len() as u8, rng);
            resolve_keep_workshop_cards(state, selected, rng);
            if matches!(state.phase, GamePhase::Draw) {
                rollout_draw_and_draft(state, rng);
            }
        }
        _ => panic!("Cannot apply rollout step for current state"),
    }
}
