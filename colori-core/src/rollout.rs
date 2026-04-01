use crate::action_phase::{
    can_afford_glass, can_afford_sell_card, destroy_drafted_card, end_player_turn,
    get_action_state_mut, initialize_action_phase, mark_glass_used,
    process_ability_stack, resolve_choose_tertiary_to_gain, resolve_choose_tertiary_to_lose,
    resolve_destroy_cards, resolve_gain_color, resolve_select_sell_card,
    resolve_select_glass, resolve_workshop_choice, resolve_workshop_with_reworkshop,
    skip_workshop,
};
use crate::colors::{
    is_primary, mix_result, pay_cost, perform_mix_unchecked, perform_unmix, PRIMARIES,
    SECONDARIES, TERTIARIES, VALID_MIX_PAIRS,
};
use crate::choices::is_glass_ability_available;
use crate::deck_utils::draw_from_deck;
use crate::draft_phase::player_pick;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use rand::RngExt;

// ── Rollout draw+draft shortcut ──

fn rollout_draw_and_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    // Step 1: Draw 5 cards from each player's personal deck
    for i in 0..num_players {
        let player = &mut state.players[i];
        draw_from_deck(
            &mut player.deck,
            &mut player.discard,
            &mut player.workshop_cards,
            5,
            rng,
        );
    }

    // Step 2: Draw 4 cards per player from draft_deck, restocking from destroyed_pile
    // only when the deck runs out (mirrors initialize_draft ordering)
    let mut dealt = [UnorderedCards::new(); MAX_PLAYERS];
    for i in 0..num_players {
        let deck_len = state.draft_deck.len();
        if deck_len >= 4 {
            dealt[i] = state.draft_deck.draw_multiple(4, rng);
        } else {
            // Take everything remaining from draft_deck
            dealt[i] = state.draft_deck;
            state.draft_deck = UnorderedCards::new();
            let remaining = 4 - deck_len;
            if remaining > 0 && !state.destroyed_pile.is_empty() {
                // Restock draft_deck from destroyed_pile
                state.draft_deck = state.destroyed_pile;
                state.destroyed_pile = UnorderedCards::new();
                let available = state.draft_deck.len().min(remaining);
                if available > 0 {
                    let drawn = state.draft_deck.draw_multiple(available, rng);
                    dealt[i] = dealt[i].union(drawn);
                }
            }
        }
    }

    // Step 3: If any player got 0 cards, return all dealt cards to destroyed_pile
    if (0..num_players).any(|i| dealt[i].is_empty()) {
        for i in 0..num_players {
            state.destroyed_pile = state.destroyed_pile.union(dealt[i]);
        }
        state.destroyed_pile = state.destroyed_pile.union(state.draft_deck);
        state.draft_deck = UnorderedCards::new();
        initialize_action_phase(state);
        return;
    }

    // Step 4: Assign dealt cards as drafted_cards
    for i in 0..num_players {
        state.players[i].drafted_cards = dealt[i];
    }

    // Step 5: Remaining draft_deck cards go to destroyed_pile
    state.destroyed_pile = state.destroyed_pile.union(state.draft_deck);
    state.draft_deck = UnorderedCards::new();

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
fn pick_random_affordable_sell_card<R: Rng>(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    rng: &mut R,
) -> Option<u32> {
    let mut affordable = [0u32; MAX_SELL_CARD_DISPLAY];
    let mut count = 0usize;
    for sell_card in sell_card_display {
        if can_afford_sell_card(player, &sell_card.sell_card) {
            affordable[count] = sell_card.instance_id;
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(affordable[rng.random_range(0..count)])
    }
}

// ── Glass ability helpers ──

/// Check if a glass ability has valid targets for the given player state.
#[inline(always)]
fn glass_has_valid_targets(player: &PlayerState, glass: GlassCard) -> bool {
    match glass {
        GlassCard::GlassWorkshop => !player.workshop_cards.is_empty(),
        GlassCard::GlassDraw => true,
        GlassCard::GlassMix => VALID_MIX_PAIRS
            .iter()
            .any(|&(a, b)| player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0),
        GlassCard::GlassGainPrimary => true,
        GlassCard::GlassExchange => ALL_MATERIAL_TYPES
            .iter()
            .any(|&m| player.materials.get(m) > 0),
        GlassCard::GlassMoveDrafted => !player.drafted_cards.is_empty(),
        GlassCard::GlassUnmix => ALL_COLORS
            .iter()
            .any(|&c| !is_primary(c) && player.color_wheel.get(c) > 0),
        GlassCard::GlassTertiaryDucat => TERTIARIES
            .iter()
            .any(|&c| player.color_wheel.get(c) > 0),
        GlassCard::GlassReworkshop => !player.workshopped_cards.is_empty(),
        GlassCard::GlassDestroyClean => !player.workshop_cards.is_empty(),
        GlassCard::GlassKeepBoth => false, // passive, no activation
    }
}

/// Activate a glass ability during rollout. Handles both stack-pushing and immediate abilities.
#[inline(always)]
fn activate_glass_in_rollout(
    state: &mut GameState,
    player_index: usize,
    glass: GlassCard,
    rng: &mut impl Rng,
) {
    mark_glass_used(state, glass);
    match glass {
        // Stack-pushing abilities
        GlassCard::GlassWorkshop => {
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::Workshop { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassDraw => {
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::DrawCards { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassMix => {
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::MixColors { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassGainPrimary => {
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::GainPrimary);
            process_ability_stack(state, rng);
        }
        // Immediate abilities
        GlassCard::GlassExchange => {
            let player = &state.players[player_index];
            let mut owned = [MaterialType::Textiles; 3];
            let mut own_count = 0usize;
            for &m in &ALL_MATERIAL_TYPES {
                if player.materials.get(m) > 0 {
                    owned[own_count] = m;
                    own_count += 1;
                }
            }
            let lose = owned[rng.random_range(0..own_count)];
            let mut gain_options = [MaterialType::Textiles; 2];
            let mut gain_count = 0usize;
            for &m in &ALL_MATERIAL_TYPES {
                if m != lose {
                    gain_options[gain_count] = m;
                    gain_count += 1;
                }
            }
            let gain = gain_options[rng.random_range(0..gain_count)];
            let player = &mut state.players[player_index];
            player.materials.decrement(lose);
            player.materials.increment(gain);
        }
        GlassCard::GlassMoveDrafted => {
            let player = &mut state.players[player_index];
            let card_id = player.drafted_cards.pick_random(rng).unwrap();
            player.drafted_cards.remove(card_id);
            player.workshop_cards.insert(card_id);
        }
        GlassCard::GlassUnmix => {
            let player = &state.players[player_index];
            let mut unmixable = [Color::Red; 9];
            let mut count = 0usize;
            for &c in &ALL_COLORS {
                if !is_primary(c) && player.color_wheel.get(c) > 0 {
                    unmixable[count] = c;
                    count += 1;
                }
            }
            let color = unmixable[rng.random_range(0..count)];
            perform_unmix(&mut state.players[player_index].color_wheel, color);
        }
        GlassCard::GlassTertiaryDucat => {
            let player = &state.players[player_index];
            let mut tertiaries = [Color::Red; 6];
            let mut count = 0usize;
            for &c in &TERTIARIES {
                if player.color_wheel.get(c) > 0 {
                    tertiaries[count] = c;
                    count += 1;
                }
            }
            let color = tertiaries[rng.random_range(0..count)];
            let player = &mut state.players[player_index];
            player.color_wheel.decrement(color);
            player.ducats += 1;
            player.cached_score += 1;
        }
        GlassCard::GlassReworkshop => {
            let player = &mut state.players[player_index];
            let card_id = player.workshopped_cards.pick_random(rng).unwrap();
            player.workshopped_cards.remove(card_id);
            player.workshop_cards.insert(card_id);
        }
        GlassCard::GlassDestroyClean => {
            let card_id = state.players[player_index].workshop_cards.pick_random(rng).unwrap();
            let card = state.card_lookup[card_id as usize];
            state.players[player_index].workshop_cards.remove(card_id);
            state.destroyed_pile.insert(card_id);
            let ability = card.ability();
            get_action_state_mut(state).ability_stack.push(ability);
            process_ability_stack(state, rng);
        }
        GlassCard::GlassKeepBoth => {} // passive, should never be activated
    }
}

/// Try to randomly activate a glass ability. Returns true if one was activated.
#[inline(always)]
fn try_activate_random_glass(
    state: &mut GameState,
    player_index: usize,
    rng: &mut impl Rng,
) -> bool {
    if !state.expansions.glass {
        return false;
    }
    // Collect available glass abilities
    const ALL_ACTIVATABLE: [GlassCard; 10] = [
        GlassCard::GlassWorkshop,
        GlassCard::GlassDraw,
        GlassCard::GlassMix,
        GlassCard::GlassGainPrimary,
        GlassCard::GlassExchange,
        GlassCard::GlassMoveDrafted,
        GlassCard::GlassUnmix,
        GlassCard::GlassTertiaryDucat,
        GlassCard::GlassReworkshop,
        GlassCard::GlassDestroyClean,
    ];
    let mut available = [GlassCard::GlassWorkshop; 10];
    let mut count = 0usize;
    for &glass in &ALL_ACTIVATABLE {
        if is_glass_ability_available(state, &state.players[player_index], glass)
            && glass_has_valid_targets(&state.players[player_index], glass)
        {
            available[count] = glass;
            count += 1;
        }
    }
    if count == 0 {
        return false;
    }
    // Pick randomly including a "skip" option
    let choice = rng.random_range(0..count + 1);
    if choice == count {
        return false; // skip
    }
    activate_glass_in_rollout(state, player_index, available[choice], rng);
    true
}

#[inline(always)]
fn handle_action_no_pending(state: &mut GameState, player_index: usize, heuristic_draft: bool, rng: &mut impl Rng) {
    // Try activating a random glass ability before picking a drafted card
    if try_activate_random_glass(state, player_index, rng) {
        return;
    }

    let mut copy = state.players[player_index].drafted_cards;
    let sel = copy.draw_up_to(1, rng);
    if sel.is_empty() {
        // No drafted cards left — end turn and advance to next round
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            if heuristic_draft {
                heuristic_rollout_draw_and_draft(state, rng);
            } else {
                rollout_draw_and_draft(state, rng);
            }
        }
        return;
    }

    let card_id = sel.lowest_bit().unwrap();
    let card = state.card_lookup[card_id as usize];
    match card.ability() {
        Ability::MixColors { count } => {
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
            let sell_card_id_opt = pick_random_affordable_sell_card(
                &state.players[player_index],
                &state.sell_card_display,
                rng,
            );
            let glass_available = state.expansions.glass
                && !state.glass_display.is_empty()
                && can_afford_glass(&state.players[player_index]);

            match (sell_card_id_opt, glass_available) {
                (Some(sell_card_id), true) => {
                    if rng.random_range(0..2u32) == 0 {
                        fused_buy(state, player_index, card_id, sell_card_id, rng);
                    } else {
                        fused_glass_acquire(state, player_index, card_id, rng);
                    }
                }
                (Some(sell_card_id), false) => {
                    fused_buy(state, player_index, card_id, sell_card_id, rng);
                }
                (None, true) => {
                    fused_glass_acquire(state, player_index, card_id, rng);
                }
                (None, false) => {
                    destroy_drafted_card(state, card_id as u32, rng);
                }
            }
        }
        _ => {
            destroy_drafted_card(state, card_id as u32, rng);
        }
    }
}

/// Fused sell card purchase (no ability stack involvement).
#[inline(always)]
fn fused_buy<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    card_id: u8,
    sell_card_id: u32,
    rng: &mut R,
) {
    state.players[player_index].drafted_cards.remove(card_id);
    state.destroyed_pile.insert(card_id);
    let sell_card_index = state
        .sell_card_display
        .iter()
        .position(|c| c.instance_id == sell_card_id)
        .unwrap();
    let sell_card = state.sell_card_display.swap_remove(sell_card_index);
    let player = &mut state.players[player_index];
    player.materials.decrement(sell_card.sell_card.required_material());
    pay_cost(&mut player.color_wheel, sell_card.sell_card.color_cost());
    player.cached_score += sell_card.sell_card.ducats();
    player.completed_sell_cards.push(sell_card);
    if let Some(id) = state.sell_card_deck.draw(rng) {
        state.sell_card_display.push(SellCardInstance {
            instance_id: id as u32,
            sell_card: state.sell_card_lookup[id as usize],
        });
    }
}

/// Fused glass acquisition (no ability stack involvement).
#[inline(always)]
fn fused_glass_acquire<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    card_id: u8,
    rng: &mut R,
) {
    state.players[player_index].drafted_cards.remove(card_id);
    state.destroyed_pile.insert(card_id);

    // Pick random glass from display
    let glass_idx = rng.random_range(0..state.glass_display.len());
    let glass_instance = state.glass_display.swap_remove(glass_idx);

    // Pick random affordable primary color (>= 4)
    let mut affordable_primaries = [Color::Red; 3];
    let mut aff_count = 0usize;
    for &c in &PRIMARIES {
        if state.players[player_index].color_wheel.get(c) >= 4 {
            affordable_primaries[aff_count] = c;
            aff_count += 1;
        }
    }
    let pay_color = affordable_primaries[rng.random_range(0..aff_count)];

    let player = &mut state.players[player_index];
    for _ in 0..4 {
        player.color_wheel.decrement(pay_color);
    }
    player.completed_glass.push(glass_instance);

    // Refill display from deck
    if let Some(next) = state.glass_deck.pop() {
        state.glass_display.push(next);
    }
}

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, heuristic_draft: bool, rng: &mut R) {
    // Fast path: complete entire draft in one step
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if heuristic_draft {
            heuristic_draft_loop(state, rng);
        } else {
            loop {
                let card_id = {
                    if let GamePhase::Draft { ref draft_state } = state.phase {
                        let player = draft_state.current_player_index;
                        let hand = draft_state.hands[player];
                        match hand.pick_random(rng) {
                            Some(id) => id as u32,
                            None => break,
                        }
                    } else {
                        break;
                    }
                };
                player_pick(state, card_id, rng);
            }
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player_index = action_state.current_player_index;
            match action_state.ability_stack.last() {
                None => {
                    handle_action_no_pending(state, player_index, heuristic_draft, rng);
                }
                Some(Ability::Workshop { count }) => {
                    let count = *count;

                    let use_reworkshop = state.expansions.glass
                        && is_glass_ability_available(state, &state.players[player_index], GlassCard::GlassReworkshop)
                        && count >= 2
                        && !state.players[player_index].workshop_cards.is_empty()
                        && rng.random_range(0..2u32) == 0;

                    if use_reworkshop {
                        let mut copy = state.players[player_index].workshop_cards;
                        let selected = copy.draw_up_to((count - 1) as u8, rng);
                        if selected.is_empty() {
                            skip_workshop(state, rng);
                        } else {
                            let reworkshop_id = selected.pick_random(rng).unwrap();
                            mark_glass_used(state, GlassCard::GlassReworkshop);
                            resolve_workshop_with_reworkshop(state, selected, reworkshop_id, rng);
                        }
                    } else {
                        let mut copy = state.players[player_index].workshop_cards;
                        let selected = copy.draw_up_to(count as u8, rng);
                        if selected.is_empty() {
                            skip_workshop(state, rng);
                        } else {
                            resolve_workshop_choice(state, selected, rng);
                        }
                    }
                }
                Some(Ability::DestroyCards) => {
                    let mut copy = state.players[player_index].workshop_cards;
                    let selected = copy.draw_up_to(1, rng);
                    resolve_destroy_cards(state, selected, rng);
                }
                Some(Ability::MixColors { count }) => {
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
                    let sell_card_id_opt = pick_random_affordable_sell_card(
                        &state.players[player_index],
                        &state.sell_card_display,
                        rng,
                    );
                    let glass_available = state.expansions.glass
                        && !state.glass_display.is_empty()
                        && can_afford_glass(&state.players[player_index]);

                    match (sell_card_id_opt, glass_available) {
                        (Some(sell_card_id), true) => {
                            if rng.random_range(0..2u32) == 0 {
                                resolve_select_sell_card(state, sell_card_id, rng);
                            } else {
                                let glass_idx = rng.random_range(0..state.glass_display.len());
                                let glass_card = state.glass_display[glass_idx].card;
                                let mut affordable_primaries = [Color::Red; 3];
                                let mut aff_count = 0usize;
                                for &c in &PRIMARIES {
                                    if state.players[player_index].color_wheel.get(c) >= 4 {
                                        affordable_primaries[aff_count] = c;
                                        aff_count += 1;
                                    }
                                }
                                let pay_color =
                                    affordable_primaries[rng.random_range(0..aff_count)];
                                resolve_select_glass(state, glass_card, pay_color, rng);
                            }
                        }
                        (Some(sell_card_id), false) => {
                            resolve_select_sell_card(state, sell_card_id, rng);
                        }
                        (None, true) => {
                            let glass_idx = rng.random_range(0..state.glass_display.len());
                            let glass_card = state.glass_display[glass_idx].card;
                            let mut affordable_primaries = [Color::Red; 3];
                            let mut aff_count = 0usize;
                            for &c in &PRIMARIES {
                                if state.players[player_index].color_wheel.get(c) >= 4 {
                                    affordable_primaries[aff_count] = c;
                                    aff_count += 1;
                                }
                            }
                            let pay_color =
                                affordable_primaries[rng.random_range(0..aff_count)];
                            resolve_select_glass(state, glass_card, pay_color, rng);
                        }
                        (None, false) => {
                            if let GamePhase::Action { ref mut action_state } = state.phase {
                                action_state.ability_stack.pop();
                            }
                            process_ability_stack(state, rng);
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    let color = SECONDARIES[rng.random_range(0..SECONDARIES.len())];
                    resolve_gain_color(state, color, rng);
                }
                Some(Ability::GainPrimary) => {
                    let color = PRIMARIES[rng.random_range(0..PRIMARIES.len())];
                    resolve_gain_color(state, color, rng);
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
                    if own_count == 0 {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    } else {
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
                }
                Some(Ability::MoveToDrafted) => {
                    let player = &mut state.players[player_index];
                    if player.workshop_cards.is_empty() || rng.random_range(0..2u32) == 0 {
                        // Skip
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    } else {
                        let card_id = player.workshop_cards.pick_random(rng).unwrap();
                        player.workshop_cards.remove(card_id);
                        player.drafted_cards.insert(card_id);
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    }
                }
                // Instant abilities should never be on top waiting — they get processed immediately
                Some(_) => panic!("Unexpected ability on stack top during rollout"),
            }
        }
        _ => panic!("Cannot apply rollout step for current state"),
    }
}

// ── Heuristic rollout helpers ──

/// Epsilon probability for falling back to random in the heuristic rollout.
const HEURISTIC_EPSILON: f64 = 0.2;

// ── Heuristic draft helpers ──

/// Classify a card's ability into a category index for redundancy counting.
/// Returns (category, workshop_count) where workshop_count is only meaningful for Workshop.
#[inline(always)]
fn ability_category(ability: Ability) -> (u8, u32) {
    match ability {
        Ability::Workshop { count } => (0, count),
        Ability::MixColors { .. } => (1, 0),
        Ability::Sell => (2, 0),
        Ability::DestroyCards => (3, 0),
        Ability::DrawCards { .. } => (4, 0),
        _ => (5, 0),
    }
}

/// Given a hand of cards, pick the instance ID of the card to drop.
/// Drops the most redundant card (most other cards share its ability type).
/// Among equally redundant Workshop cards, drops the one with the lowest count.
#[inline(always)]
fn pick_card_to_drop<R: Rng>(
    hand: &UnorderedCards,
    card_lookup: &[Card; 256],
    rng: &mut R,
) -> u8 {
    // Epsilon: random drop
    if rng.random_bool(HEURISTIC_EPSILON) {
        return hand.pick_random(rng).unwrap();
    }

    // Collect card info: (instance_id, category, workshop_count)
    let mut cards: [(u8, u8, u32); 8] = [(0, 0, 0); 8];
    let mut count = 0usize;
    for id in hand.iter() {
        let card = card_lookup[id as usize];
        let (cat, wc) = ability_category(card.ability());
        cards[count] = (id, cat, wc);
        count += 1;
    }

    // Count how many cards share each category
    let mut cat_counts = [0u32; 6];
    for i in 0..count {
        cat_counts[cards[i].1 as usize] += 1;
    }

    // For each card, its redundancy is cat_counts[its_category] - 1
    // Find the max redundancy
    let mut max_redundancy = 0u32;
    for i in 0..count {
        let redundancy = cat_counts[cards[i].1 as usize] - 1;
        if redundancy > max_redundancy {
            max_redundancy = redundancy;
        }
    }

    // Among cards with max redundancy, pick the worst to drop
    let mut best_drop: Option<u8> = None;
    let mut best_drop_wc = u32::MAX; // for Workshop: lower = worse = drop first
    let mut candidates = 0u32;

    for i in 0..count {
        let redundancy = cat_counts[cards[i].1 as usize] - 1;
        if redundancy != max_redundancy {
            continue;
        }
        let (id, cat, wc) = cards[i];
        if cat == 0 {
            // Workshop: prefer dropping lowest workshop count
            if wc < best_drop_wc {
                best_drop_wc = wc;
                best_drop = Some(id);
                candidates = 1;
            } else if wc == best_drop_wc {
                candidates += 1;
                if rng.random_range(0..candidates) == 0 {
                    best_drop = Some(id);
                }
            }
        } else {
            // Non-workshop: pick randomly among equal redundancy
            candidates += 1;
            if best_drop.is_none() || rng.random_range(0..candidates) == 0 {
                best_drop = Some(id);
            }
        }
    }

    best_drop.unwrap()
}

/// Like `rollout_draw_and_draft` but deals 5 cards per player and uses
/// the heuristic to drop the most redundant card, keeping the best 4.
fn heuristic_rollout_draw_and_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    // Step 1: Draw 5 cards from each player's personal deck
    for i in 0..num_players {
        let player = &mut state.players[i];
        draw_from_deck(
            &mut player.deck,
            &mut player.discard,
            &mut player.workshop_cards,
            5,
            rng,
        );
    }

    // Step 2: Draw 5 cards per player from draft_deck (instead of 4)
    let mut dealt = [UnorderedCards::new(); MAX_PLAYERS];
    for i in 0..num_players {
        let deck_len = state.draft_deck.len();
        if deck_len >= 5 {
            dealt[i] = state.draft_deck.draw_multiple(5, rng);
        } else {
            dealt[i] = state.draft_deck;
            state.draft_deck = UnorderedCards::new();
            let remaining = 5 - deck_len;
            if remaining > 0 && !state.destroyed_pile.is_empty() {
                state.draft_deck = state.destroyed_pile;
                state.destroyed_pile = UnorderedCards::new();
                let available = state.draft_deck.len().min(remaining);
                if available > 0 {
                    let drawn = state.draft_deck.draw_multiple(available, rng);
                    dealt[i] = dealt[i].union(drawn);
                }
            }
        }
    }

    // Step 3: If any player got 0 cards, return all dealt cards to destroyed_pile
    if (0..num_players).any(|i| dealt[i].is_empty()) {
        for i in 0..num_players {
            state.destroyed_pile = state.destroyed_pile.union(dealt[i]);
        }
        state.destroyed_pile = state.destroyed_pile.union(state.draft_deck);
        state.draft_deck = UnorderedCards::new();
        initialize_action_phase(state);
        return;
    }

    // Step 4: For each player, drop the most redundant card
    for i in 0..num_players {
        if dealt[i].len() > 4 {
            let drop_id = pick_card_to_drop(&dealt[i], &state.card_lookup, rng);
            dealt[i].remove(drop_id);
            state.destroyed_pile.insert(drop_id);
        }
    }

    // Step 5: Assign remaining cards as drafted_cards
    for i in 0..num_players {
        state.players[i].drafted_cards = dealt[i];
    }

    // Step 6: Remaining draft_deck cards go to destroyed_pile
    state.destroyed_pile = state.destroyed_pile.union(state.draft_deck);
    state.draft_deck = UnorderedCards::new();

    // Step 7: Go directly to action phase
    initialize_action_phase(state);
}

/// Heuristic draft loop: for each player's hand, identify the card to drop,
/// then pick all other cards via player_pick.
fn heuristic_draft_loop<R: Rng>(state: &mut GameState, rng: &mut R) {
    loop {
        let (_player, hand, card_to_drop) = {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                let player = draft_state.current_player_index;
                let hand = draft_state.hands[player];
                if hand.is_empty() {
                    break;
                }
                // Only compute drop card when hand has more than 1 card
                // (with 1 card, it'll be picked anyway — it's the last card which gets destroyed)
                let drop = if hand.len() > 1 {
                    Some(pick_card_to_drop(&hand, &state.card_lookup, rng))
                } else {
                    None
                };
                (player, hand, drop)
            } else {
                break;
            }
        };

        // Pick a card that is NOT the one we want to drop
        let card_id = if let Some(drop_id) = card_to_drop {
            // Find any card in hand that isn't the drop card
            let mut pick = None;
            for id in hand.iter() {
                if id != drop_id {
                    pick = Some(id);
                    break;
                }
            }
            // If all cards are the drop card (shouldn't happen with len > 1), just pick any
            pick.unwrap_or_else(|| hand.iter().next().unwrap()) as u32
        } else {
            // Single card in hand, just pick it (it'll be the last discarded card)
            hand.iter().next().unwrap() as u32
        };

        player_pick(state, card_id, rng);
    }
}

// ── Sell card cache for heuristic rollout performance ──

#[derive(Clone, Copy)]
struct SellCardCacheEntry {
    ducats: u32,
    required_material: MaterialType,
    colors_met: bool,
    have_colors: u32,
    total_colors: u32,
}

struct SellCardCache {
    flat_demand: [u32; 12],
    proximity_demand: [u32; 12],
    best_affordable_ducats: u32,
    best_affordable_id: Option<u32>,
    entries: [SellCardCacheEntry; MAX_SELL_CARD_DISPLAY],
    len: usize,
}

impl SellCardCache {
    #[inline(always)]
    fn new(sell_card_display: &[SellCardInstance], wheel: &ColorWheel, materials: &Materials) -> Self {
        let mut flat_demand = [0u32; 12];
        let mut proximity_demand = [0u32; 12];
        let mut best_affordable_ducats = 0u32;
        let mut best_affordable_id: Option<u32> = None;
        let len = sell_card_display.len();
        const EMPTY_ENTRY: SellCardCacheEntry = SellCardCacheEntry { ducats: 0, required_material: MaterialType::Textiles, colors_met: false, have_colors: 0, total_colors: 0 };
        let mut entries: [SellCardCacheEntry; MAX_SELL_CARD_DISPLAY] = [EMPTY_ENTRY; MAX_SELL_CARD_DISPLAY];

        for (i, sc) in sell_card_display.iter().enumerate() {
            let ducats = sc.sell_card.ducats();
            let cost = sc.sell_card.color_cost();
            let total_colors = cost.len() as u32;
            let has_material = materials.get(sc.sell_card.required_material()) >= 1;

            // Count missing colors — iterate cost directly (max 4 colors)
            // instead of using a [0u32; 12] tracking array. Sell card costs
            // have at most 3 distinct colors with at most 1 duplicate.
            let mut missing = 0u32;
            let mut have = 0u32;
            let counts = &wheel.counts;
            match cost.len() {
                1 => {
                    if counts[cost[0].index()] >= 1 { have = 1; } else { missing = 1; }
                }
                2 => {
                    let (c0, c1) = (cost[0].index(), cost[1].index());
                    if c0 == c1 {
                        let avail = counts[c0];
                        have = avail.min(2);
                        missing = 2 - have;
                    } else {
                        if counts[c0] >= 1 { have += 1; } else { missing += 1; }
                        if counts[c1] >= 1 { have += 1; } else { missing += 1; }
                    }
                }
                3 => {
                    let (c0, c1, c2) = (cost[0].index(), cost[1].index(), cost[2].index());
                    if c0 == c1 && c1 == c2 {
                        // All three same: need count >= 3
                        let avail = counts[c0];
                        have = avail.min(3); missing = 3 - have;
                    } else if c0 == c1 {
                        let avail = counts[c0];
                        have += avail.min(2); missing += 2 - avail.min(2);
                        if counts[c2] >= 1 { have += 1; } else { missing += 1; }
                    } else if c1 == c2 {
                        if counts[c0] >= 1 { have += 1; } else { missing += 1; }
                        let avail = counts[c1];
                        have += avail.min(2); missing += 2 - avail.min(2);
                    } else if c0 == c2 {
                        let avail = counts[c0];
                        have += avail.min(2); missing += 2 - avail.min(2);
                        if counts[c1] >= 1 { have += 1; } else { missing += 1; }
                    } else {
                        if counts[c0] >= 1 { have += 1; } else { missing += 1; }
                        if counts[c1] >= 1 { have += 1; } else { missing += 1; }
                        if counts[c2] >= 1 { have += 1; } else { missing += 1; }
                    }
                }
                _ => {
                    // Fallback for 4+ colors (shouldn't happen in current card set)
                    let mut used = [0u32; 12];
                    for &c in cost {
                        let idx = c.index();
                        let needed = used[idx] + 1;
                        if counts[idx] < needed { missing += 1; } else { have += 1; }
                        used[idx] = needed;
                    }
                }
            }

            let colors_met = missing == 0;
            let can_afford = has_material && colors_met;

            // Build flat and proximity demand — iterate cost directly (max 4 entries)
            let total_missing = missing + if has_material { 0 } else { 2 };
            let prox_weight = ducats * 10 / (total_missing + 1);
            for &c in cost {
                let idx = c.index();
                flat_demand[idx] += ducats;
                proximity_demand[idx] += prox_weight;
            }

            // Track best affordable
            if can_afford && ducats > best_affordable_ducats {
                best_affordable_ducats = ducats;
                best_affordable_id = Some(sc.instance_id);
            }

            entries[i] = SellCardCacheEntry {
                ducats,
                required_material: sc.sell_card.required_material(),
                colors_met,
                have_colors: have,
                total_colors,
            };
        }

        SellCardCache {
            flat_demand,
            proximity_demand,
            best_affordable_ducats,
            best_affordable_id,
            entries,
            len,
        }
    }

    #[inline(always)]
    fn color_demand(&self, color: Color) -> u32 {
        self.proximity_demand[color.index()]
    }

}

/// Destruction priority considering actual game state.
#[inline(always)]
fn destruction_priority(
    card: Card,
    player: &PlayerState,
    cache: &SellCardCache,
) -> u32 {
    match card.ability() {
        Ability::Sell => {
            if cache.best_affordable_ducats > 0 {
                cache.best_affordable_ducats * 25
            } else {
                35
            }
        }
        Ability::MixColors { count } => {
            let mut valid_pairs = 0u32;
            for &(a, b) in &VALID_MIX_PAIRS {
                if player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0 {
                    valid_pairs += 1;
                }
            }
            if valid_pairs == 0 {
                20
            } else {
                50 + valid_pairs.min(4) * 3 + count.min(2) * 5
            }
        }
        Ability::Workshop { count } => {
            if player.workshop_cards.is_empty() {
                15
            } else {
                60 + count.min(3) * 7
            }
        }
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() {
                25
            } else {
                45
            }
        }
        Ability::DrawCards { count } => 30 + count.min(3) * 2,
        _ => 10,
    }
}

/// Score a workshop card for selection priority.
#[inline(always)]
fn workshop_card_score(
    card: Card,
    cache: &SellCardCache,
) -> u32 {
    let mut score = 0u32;

    // Material cards: score by how much their material type is needed
    for &mt in card.material_types() {
        for i in 0..cache.len {
            let entry = &cache.entries[i];
            if entry.required_material == mt {
                let base = entry.ducats * 3;
                if entry.colors_met {
                    score += base * 3;
                } else {
                    score += base + base * entry.have_colors / (entry.total_colors + 1);
                }
            }
        }
    }

    // Color cards: score by how much their colors are needed
    for &color in card.colors() {
        score += cache.color_demand(color);
    }

    // Action cards get a moderate bonus
    if card.is_action() {
        score += 5;
    }

    score
}

/// Pick the best color from a slice, preferring colors needed for sell cards.
#[inline(always)]
fn pick_best_color<R: Rng>(
    colors: &[Color],
    cache: &SellCardCache,
    rng: &mut R,
) -> Color {
    if rng.random_bool(HEURISTIC_EPSILON) {
        return colors[rng.random_range(0..colors.len())];
    }
    let mut best_color = colors[0];
    let mut best_score = 0u32;
    for &c in colors {
        let score = cache.color_demand(c);
        if score > best_score {
            best_score = score;
            best_color = c;
        }
    }
    best_color
}

/// Heuristic mix sequence: prefer mixes whose output is useful for sell cards.
#[inline(always)]
fn heuristic_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    remaining: u32,
    cache: &SellCardCache,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    if rng.random_bool(HEURISTIC_EPSILON) {
        return random_mix_seq(wheel, remaining, rng);
    }

    let mut mixes = [(Color::Red, Color::Red); 2];
    let mut count = 0usize;
    let mut sim_wheel = wheel.clone();
    for _ in 0..remaining {
        if count >= 2 {
            break;
        }
        let mut best_pair: Option<(Color, Color)> = None;
        let mut best_score = 0u32;
        let mut any_valid = false;
        for &(a, b) in &VALID_MIX_PAIRS {
            if sim_wheel.get(a) > 0 && sim_wheel.get(b) > 0 {
                any_valid = true;
                let output = mix_result(a, b);
                let score = cache.color_demand(output);
                if score > best_score {
                    best_score = score;
                    best_pair = Some((a, b));
                }
            }
        }
        if !any_valid {
            break;
        }
        // If no mix is useful, skip mixing (50% chance) or do random
        if best_pair.is_none() {
            if rng.random_bool(0.5) {
                break;
            }
            // Fall back to random valid pair
            let mut pairs: [(Color, Color); 9] = [(Color::Red, Color::Red); 9];
            let mut pair_count = 0usize;
            for &(a, b) in &VALID_MIX_PAIRS {
                if sim_wheel.get(a) > 0 && sim_wheel.get(b) > 0 {
                    pairs[pair_count] = (a, b);
                    pair_count += 1;
                }
            }
            let (a, b) = pairs[rng.random_range(0..pair_count)];
            mixes[count] = (a, b);
            count += 1;
            perform_mix_unchecked(&mut sim_wheel, a, b);
        } else {
            let (a, b) = best_pair.unwrap();
            mixes[count] = (a, b);
            count += 1;
            perform_mix_unchecked(&mut sim_wheel, a, b);
        }
    }
    (mixes, count)
}

/// Two-step mix lookahead: evaluate all pair combinations when 2 mixes are available.
#[inline(always)]
fn two_step_heuristic_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    remaining: u32,
    cache: &SellCardCache,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    if remaining < 2 {
        return heuristic_mix_seq(wheel, remaining, cache, rng);
    }

    if rng.random_bool(HEURISTIC_EPSILON) {
        return random_mix_seq(wheel, remaining, rng);
    }

    // Collect valid first-mix pairs
    let mut pairs1: [(Color, Color); 9] = [(Color::Red, Color::Red); 9];
    let mut pair1_count = 0usize;
    for &(a, b) in &VALID_MIX_PAIRS {
        if wheel.get(a) > 0 && wheel.get(b) > 0 {
            pairs1[pair1_count] = (a, b);
            pair1_count += 1;
        }
    }

    if pair1_count == 0 {
        return ([(Color::Red, Color::Red); 2], 0);
    }

    let mut best_score = 0u32;
    let mut best_combo: ([(Color, Color); 2], usize) = ([(Color::Red, Color::Red); 2], 0);

    for p1 in 0..pair1_count {
        let (a1, b1) = pairs1[p1];
        let output1 = mix_result(a1, b1);
        let score1 = cache.color_demand(output1);

        // Consider doing only the first mix
        if score1 > best_score {
            best_score = score1;
            best_combo = ([(a1, b1), (Color::Red, Color::Red)], 1);
        }

        // Simulate first mix and evaluate second using flat demand (avoids recomputing proximity)
        let mut sim_wheel = wheel.clone();
        perform_mix_unchecked(&mut sim_wheel, a1, b1);

        for &(a2, b2) in &VALID_MIX_PAIRS {
            if sim_wheel.get(a2) > 0 && sim_wheel.get(b2) > 0 {
                let output2 = mix_result(a2, b2);
                let score2 = cache.flat_demand[output2.index()];
                let total = score1 + score2;
                if total > best_score {
                    best_score = total;
                    best_combo = ([(a1, b1), (a2, b2)], 2);
                }
            }
        }
    }

    // If no useful mixes, 50% chance to skip or fall back to random
    if best_score == 0 {
        if rng.random_bool(0.5) {
            return ([(Color::Red, Color::Red); 2], 0);
        }
        return random_mix_seq(wheel, remaining, rng);
    }

    best_combo
}

#[inline(always)]
fn handle_action_no_pending_heuristic(state: &mut GameState, player_index: usize, heuristic_draft: bool, cache: &SellCardCache, rng: &mut impl Rng) {
    // Glass: use same random policy (glass strategy is complex, not worth heuristic overhead)
    if try_activate_random_glass(state, player_index, rng) {
        return;
    }

    let drafted = state.players[player_index].drafted_cards;
    if drafted.is_empty() {
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            if heuristic_draft {
                heuristic_rollout_draw_and_draft(state, rng);
            } else {
                rollout_draw_and_draft(state, rng);
            }
        }
        return;
    }

    // Epsilon: random fallback
    if rng.random_bool(HEURISTIC_EPSILON) {
        handle_action_no_pending(state, player_index, heuristic_draft, rng);
        return;
    }

    // Score each drafted card and pick the best to destroy
    let mut best_id: Option<u8> = None;
    let mut best_priority = 0u32;
    let mut seen: u64 = 0;
    for id in drafted.iter() {
        let card = state.card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit != 0 { continue; }
        seen |= bit;
        let priority = destruction_priority(card, &state.players[player_index], &cache);
        if priority > best_priority {
            best_priority = priority;
            best_id = Some(id);
        }
    }

    // If best priority is low, 50% chance to just end turn
    if best_priority <= 30 && rng.random_bool(0.5) {
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            rollout_draw_and_draft(state, rng);
        }
        return;
    }

    let card_id = best_id.unwrap();
    let card = state.card_lookup[card_id as usize];
    match card.ability() {
        Ability::MixColors { count } => {
            let player = &state.players[player_index];
            let (mixes, mix_count) = two_step_heuristic_mix_seq(&player.color_wheel, count, &cache, rng);
            state.players[player_index].drafted_cards.remove(card_id);
            state.destroyed_pile.insert(card_id);
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
            }
        }
        Ability::Sell => {
            let sell_card_id_opt = cache.best_affordable_id;
            let glass_available = state.expansions.glass
                && !state.glass_display.is_empty()
                && can_afford_glass(&state.players[player_index]);

            match (sell_card_id_opt, glass_available) {
                (Some(sell_card_id), true) => {
                    // Prefer selling (ducats are the goal) unless glass is clearly better
                    if rng.random_range(0..3u32) == 0 {
                        fused_glass_acquire(state, player_index, card_id, rng);
                    } else {
                        fused_buy(state, player_index, card_id, sell_card_id, rng);
                    }
                }
                (Some(sell_card_id), false) => {
                    fused_buy(state, player_index, card_id, sell_card_id, rng);
                }
                (None, true) => {
                    fused_glass_acquire(state, player_index, card_id, rng);
                }
                (None, false) => {
                    destroy_drafted_card(state, card_id as u32, rng);
                }
            }
        }
        _ => {
            destroy_drafted_card(state, card_id as u32, rng);
        }
    }
}

pub fn apply_heuristic_rollout_step<R: Rng>(state: &mut GameState, heuristic_draft: bool, rng: &mut R) {
    // Draft phase
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if heuristic_draft {
            heuristic_draft_loop(state, rng);
        } else {
            loop {
                let card_id = {
                    if let GamePhase::Draft { ref draft_state } = state.phase {
                        let player = draft_state.current_player_index;
                        let hand = draft_state.hands[player];
                        match hand.pick_random(rng) {
                            Some(id) => id as u32,
                            None => break,
                        }
                    } else {
                        break;
                    }
                };
                player_pick(state, card_id, rng);
            }
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player_index = action_state.current_player_index;
            let cache = SellCardCache::new(&state.sell_card_display, &state.players[player_index].color_wheel, &state.players[player_index].materials);
            match action_state.ability_stack.last() {
                None => {
                    handle_action_no_pending_heuristic(state, player_index, heuristic_draft, &cache, rng);
                }
                Some(Ability::Workshop { count }) => {
                    let count = *count;

                    // Epsilon: fall back to random
                    if rng.random_bool(HEURISTIC_EPSILON) {
                        let mut copy = state.players[player_index].workshop_cards;
                        let selected = copy.draw_up_to(count as u8, rng);
                        if selected.is_empty() {
                            skip_workshop(state, rng);
                        } else {
                            resolve_workshop_choice(state, selected, rng);
                        }
                        return;
                    }

                    // Score each workshop card and pick the top N
                    let workshop = state.players[player_index].workshop_cards;
                    let mut scored: [(u8, u32); 16] = [(0, 0); 16];
                    let mut scored_count = 0usize;
                    for id in workshop.iter() {
                        let card = state.card_lookup[id as usize];
                        let score = workshop_card_score(card, &cache);
                        scored[scored_count] = (id, score);
                        scored_count += 1;
                    }

                    if scored_count == 0 {
                        skip_workshop(state, rng);
                        return;
                    }

                    // Sort descending by score (simple insertion sort, max 16 elements)
                    for i in 1..scored_count {
                        let mut j = i;
                        while j > 0 && scored[j].1 > scored[j - 1].1 {
                            scored.swap(j, j - 1);
                            j -= 1;
                        }
                    }

                    // Select top min(count, scored_count) cards
                    let take = (count as usize).min(scored_count);
                    let mut selected = UnorderedCards::new();
                    for i in 0..take {
                        selected.insert(scored[i].0);
                    }
                    resolve_workshop_choice(state, selected, rng);
                }
                Some(Ability::DestroyCards) => {
                    let workshop = state.players[player_index].workshop_cards;
                    if workshop.is_empty() {
                        resolve_destroy_cards(state, UnorderedCards::new(), rng);
                        return;
                    }

                    // Epsilon: random
                    if rng.random_bool(HEURISTIC_EPSILON) {
                        let mut copy = workshop;
                        let selected = copy.draw_up_to(1, rng);
                        resolve_destroy_cards(state, selected, rng);
                        return;
                    }

                    // Pick the card whose destroy ability is most useful
                    let mut best_id: Option<u8> = None;
                    let mut best_score = 0u32;
                    for id in workshop.iter() {
                        let card = state.card_lookup[id as usize];
                        let score = destruction_priority(card, &state.players[player_index], &cache);
                        if score > best_score || best_id.is_none() {
                            best_score = score;
                            best_id = Some(id);
                        }
                    }

                    let mut selected = UnorderedCards::new();
                    if let Some(id) = best_id {
                        selected.insert(id);
                    }
                    resolve_destroy_cards(state, selected, rng);
                }
                Some(Ability::MixColors { count }) => {
                    let remaining_mixes = *count;
                    let (mixes, mix_count) = two_step_heuristic_mix_seq(&state.players[player_index].color_wheel, remaining_mixes, &cache, rng);
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
                    // Epsilon: random
                    if rng.random_bool(HEURISTIC_EPSILON) {
                        let sell_card_id_opt = pick_random_affordable_sell_card(
                            &state.players[player_index],
                            &state.sell_card_display,
                            rng,
                        );
                        let glass_available = state.expansions.glass
                            && !state.glass_display.is_empty()
                            && can_afford_glass(&state.players[player_index]);
                        match (sell_card_id_opt, glass_available) {
                            (Some(sell_card_id), true) => {
                                if rng.random_range(0..2u32) == 0 {
                                    resolve_select_sell_card(state, sell_card_id, rng);
                                } else {
                                    let glass_idx = rng.random_range(0..state.glass_display.len());
                                    let glass_card = state.glass_display[glass_idx].card;
                                    let mut affordable_primaries = [Color::Red; 3];
                                    let mut aff_count = 0usize;
                                    for &c in &PRIMARIES {
                                        if state.players[player_index].color_wheel.get(c) >= 4 {
                                            affordable_primaries[aff_count] = c;
                                            aff_count += 1;
                                        }
                                    }
                                    let pay_color = affordable_primaries[rng.random_range(0..aff_count)];
                                    resolve_select_glass(state, glass_card, pay_color, rng);
                                }
                            }
                            (Some(sell_card_id), false) => {
                                resolve_select_sell_card(state, sell_card_id, rng);
                            }
                            (None, true) => {
                                let glass_idx = rng.random_range(0..state.glass_display.len());
                                let glass_card = state.glass_display[glass_idx].card;
                                let mut affordable_primaries = [Color::Red; 3];
                                let mut aff_count = 0usize;
                                for &c in &PRIMARIES {
                                    if state.players[player_index].color_wheel.get(c) >= 4 {
                                        affordable_primaries[aff_count] = c;
                                        aff_count += 1;
                                    }
                                }
                                let pay_color = affordable_primaries[rng.random_range(0..aff_count)];
                                resolve_select_glass(state, glass_card, pay_color, rng);
                            }
                            (None, false) => {
                                if let GamePhase::Action { ref mut action_state } = state.phase {
                                    action_state.ability_stack.pop();
                                }
                                process_ability_stack(state, rng);
                            }
                        }
                        return;
                    }

                    // Heuristic: pick best sell card
                    let sell_card_id_opt = cache.best_affordable_id;
                    let glass_available = state.expansions.glass
                        && !state.glass_display.is_empty()
                        && can_afford_glass(&state.players[player_index]);
                    match (sell_card_id_opt, glass_available) {
                        (Some(sell_card_id), true) => {
                            // Prefer selling over glass most of the time
                            if rng.random_range(0..3u32) == 0 {
                                let glass_idx = rng.random_range(0..state.glass_display.len());
                                let glass_card = state.glass_display[glass_idx].card;
                                let mut affordable_primaries = [Color::Red; 3];
                                let mut aff_count = 0usize;
                                for &c in &PRIMARIES {
                                    if state.players[player_index].color_wheel.get(c) >= 4 {
                                        affordable_primaries[aff_count] = c;
                                        aff_count += 1;
                                    }
                                }
                                let pay_color = affordable_primaries[rng.random_range(0..aff_count)];
                                resolve_select_glass(state, glass_card, pay_color, rng);
                            } else {
                                resolve_select_sell_card(state, sell_card_id, rng);
                            }
                        }
                        (Some(sell_card_id), false) => {
                            resolve_select_sell_card(state, sell_card_id, rng);
                        }
                        (None, true) => {
                            let glass_idx = rng.random_range(0..state.glass_display.len());
                            let glass_card = state.glass_display[glass_idx].card;
                            let mut affordable_primaries = [Color::Red; 3];
                            let mut aff_count = 0usize;
                            for &c in &PRIMARIES {
                                if state.players[player_index].color_wheel.get(c) >= 4 {
                                    affordable_primaries[aff_count] = c;
                                    aff_count += 1;
                                }
                            }
                            let pay_color = affordable_primaries[rng.random_range(0..aff_count)];
                            resolve_select_glass(state, glass_card, pay_color, rng);
                        }
                        (None, false) => {
                            if let GamePhase::Action { ref mut action_state } = state.phase {
                                action_state.ability_stack.pop();
                            }
                            process_ability_stack(state, rng);
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    let color = pick_best_color(&SECONDARIES, &cache, rng);
                    resolve_gain_color(state, color, rng);
                }
                Some(Ability::GainPrimary) => {
                    let color = pick_best_color(&PRIMARIES, &cache, rng);
                    resolve_gain_color(state, color, rng);
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
                    if own_count == 0 {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    } else if rng.random_bool(HEURISTIC_EPSILON) {
                        // Epsilon: random
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
                    } else {
                        // Heuristic: lose the least useful, gain the most useful
                        let mut best_lose = owned_tertiaries[0];
                        let mut best_lose_score = u32::MAX;
                        for i in 0..own_count {
                            let c = owned_tertiaries[i];
                            let score = cache.color_demand(c);
                            if score < best_lose_score {
                                best_lose_score = score;
                                best_lose = c;
                            }
                        }
                        let mut best_gain = TERTIARIES[0];
                        let mut best_gain_score = 0u32;
                        for &c in &TERTIARIES {
                            if c != best_lose {
                                let score = cache.color_demand(c);
                                if score > best_gain_score {
                                    best_gain_score = score;
                                    best_gain = c;
                                }
                            }
                        }
                        resolve_choose_tertiary_to_lose(state, best_lose);
                        resolve_choose_tertiary_to_gain(state, best_gain, rng);
                    }
                }
                Some(Ability::MoveToDrafted) => {
                    let player = &mut state.players[player_index];
                    if player.workshop_cards.is_empty() || rng.random_range(0..2u32) == 0 {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    } else {
                        let card_id = player.workshop_cards.pick_random(rng).unwrap();
                        player.workshop_cards.remove(card_id);
                        player.drafted_cards.insert(card_id);
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    }
                }
                Some(_) => panic!("Unexpected ability on stack top during rollout"),
            }
        }
        _ => panic!("Cannot apply heuristic rollout step for current state"),
    }
}

// ── Backward sell solver ──

enum SellPlanActionType {
    Workshop { selected_cards: UnorderedCards },
    Mix { mixes: [(Color, Color); 2], mix_count: usize },
    Sell { sell_card_instance_id: u32 },
    /// Destroy a workshop card via DestroyCards ability; heuristic resolves the resulting ability.
    Destroy { target_workshop_id: u8 },
    /// Destroy a Sell-ability workshop card via DestroyCards, then sell to a specific target.
    DestroyForSell { target_workshop_id: u8, sell_card_instance_id: u32 },
}

struct SellPlanAction {
    draft_card_id: u8,
    action_type: SellPlanActionType,
}

struct SellPlan {
    actions: [Option<SellPlanAction>; 8],
    action_count: usize,
    consumed_draft_ids: [u8; 8],
    consumed_count: usize,
}

fn compute_sell_plan(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    workshop_cards: &UnorderedCards,
) -> SellPlan {
    let mut plan = SellPlan {
        actions: [const { None }; 8],
        action_count: 0,
        consumed_draft_ids: [0; 8],
        consumed_count: 0,
    };

    // Step 1: Categorize drafted cards by ability
    let mut draft_workshop: [(u8, u32); 8] = [(0, 0); 8]; // (card_id, count)
    let mut draft_workshop_count = 0usize;
    let mut draft_mix: [(u8, u32); 8] = [(0, 0); 8];
    let mut draft_mix_count = 0usize;
    let mut draft_sell: [u8; 8] = [0; 8];
    let mut draft_sell_count = 0usize;
    let mut draft_destroy: [u8; 8] = [0; 8];
    let mut draft_destroy_count = 0usize;

    for id in player.drafted_cards.iter() {
        let card = card_lookup[id as usize];
        match card.ability() {
            Ability::Workshop { count } => {
                draft_workshop[draft_workshop_count] = (id, count);
                draft_workshop_count += 1;
            }
            Ability::MixColors { count } => {
                draft_mix[draft_mix_count] = (id, count);
                draft_mix_count += 1;
            }
            Ability::Sell => {
                draft_sell[draft_sell_count] = id;
                draft_sell_count += 1;
            }
            Ability::DestroyCards => {
                draft_destroy[draft_destroy_count] = id;
                draft_destroy_count += 1;
            }
            _ => {}
        }
    }

    // Step 2: Assign DestroyCards drafted cards to workshop card targets.
    // Each assignment consumes one DestroyCards card and one workshop card,
    // producing a virtual ability (Sell, Workshop, or MixColors).
    //
    // Priority: Workshop (most common target — starter materials give Workshop{2-4})
    //           then MixColors, then Sell (Sell-ability cards are rarer in workshop)
    struct DestroyAssignment {
        draft_id: u8,
        workshop_id: u8,
        ability: Ability,
    }
    let mut destroy_assignments: [Option<DestroyAssignment>; 8] = [const { None }; 8];
    let mut destroy_assign_count = 0usize;
    let mut reserved_ws = UnorderedCards::new(); // workshop cards reserved for destruction
    let mut used_destroy = [false; 8];

    // Pass 1: DestroyCards → Sell
    for di in 0..draft_destroy_count {
        if used_destroy[di] { continue; }
        for id in workshop_cards.iter() {
            if reserved_ws.contains(id) { continue; }
            let card = card_lookup[id as usize];
            if matches!(card.ability(), Ability::Sell) {
                destroy_assignments[destroy_assign_count] = Some(DestroyAssignment {
                    draft_id: draft_destroy[di],
                    workshop_id: id,
                    ability: Ability::Sell,
                });
                destroy_assign_count += 1;
                reserved_ws.insert(id);
                used_destroy[di] = true;
                break;
            }
        }
    }

    // Pass 2: DestroyCards → Workshop
    for di in 0..draft_destroy_count {
        if used_destroy[di] { continue; }
        let mut best_id: Option<u8> = None;
        let mut best_count = 0u32;
        for id in workshop_cards.iter() {
            if reserved_ws.contains(id) { continue; }
            let card = card_lookup[id as usize];
            if let Ability::Workshop { count } = card.ability() {
                if count > best_count {
                    best_count = count;
                    best_id = Some(id);
                }
            }
        }
        if let Some(ws_id) = best_id {
            destroy_assignments[destroy_assign_count] = Some(DestroyAssignment {
                draft_id: draft_destroy[di],
                workshop_id: ws_id,
                ability: Ability::Workshop { count: best_count },
            });
            destroy_assign_count += 1;
            reserved_ws.insert(ws_id);
            used_destroy[di] = true;
        }
    }

    // Pass 3: DestroyCards → MixColors
    for di in 0..draft_destroy_count {
        if used_destroy[di] { continue; }
        let mut best_id: Option<u8> = None;
        let mut best_count = 0u32;
        for id in workshop_cards.iter() {
            if reserved_ws.contains(id) { continue; }
            let card = card_lookup[id as usize];
            if let Ability::MixColors { count } = card.ability() {
                if count > best_count {
                    best_count = count;
                    best_id = Some(id);
                }
            }
        }
        if let Some(ws_id) = best_id {
            destroy_assignments[destroy_assign_count] = Some(DestroyAssignment {
                draft_id: draft_destroy[di],
                workshop_id: ws_id,
                ability: Ability::MixColors { count: best_count },
            });
            destroy_assign_count += 1;
            reserved_ws.insert(ws_id);
            used_destroy[di] = true;
        }
    }

    // Step 3: Compute total capacity including virtual abilities from DestroyCards
    let mut total_workshop_slots: u32 = draft_workshop[..draft_workshop_count].iter().map(|(_, c)| c).sum();
    let mut total_mix_slots: u32 = draft_mix[..draft_mix_count].iter().map(|(_, c)| c).sum();
    let mut total_sell_abilities = draft_sell_count;

    for i in 0..destroy_assign_count {
        if let Some(ref a) = destroy_assignments[i] {
            match a.ability {
                Ability::Workshop { count } => total_workshop_slots += count,
                Ability::MixColors { count } => total_mix_slots += count,
                Ability::Sell => total_sell_abilities += 1,
                _ => {}
            }
        }
    }

    if total_sell_abilities == 0 {
        return plan; // No sell abilities (direct or via destroy), nothing to do
    }

    // Step 4: Build projected state — what we'd have after workshopping best cards
    // Exclude reserved workshop cards (they'll be destroyed, not workshopped)
    let mut projected_wheel = player.color_wheel.clone();
    let mut projected_materials = player.materials.clone();

    // Score each workshop card and pick top N
    let mut ws_scored: [(u8, u32); 16] = [(0, 0); 16];
    let mut ws_scored_count = 0usize;
    for id in workshop_cards.iter() {
        if reserved_ws.contains(id) { continue; } // skip cards reserved for destruction
        let card = card_lookup[id as usize];
        let mut score = 0u32;
        for &color in card.colors() {
            for sc in sell_card_display {
                for &cc in sc.sell_card.color_cost() {
                    if cc == color {
                        score += sc.sell_card.ducats() * 5;
                    }
                }
            }
        }
        for &mt in card.material_types() {
            for sc in sell_card_display {
                if sc.sell_card.required_material() == mt {
                    score += sc.sell_card.ducats() * 10;
                }
            }
        }
        ws_scored[ws_scored_count] = (id, score);
        ws_scored_count += 1;
    }

    // Sort descending by score
    for i in 1..ws_scored_count {
        let mut j = i;
        while j > 0 && ws_scored[j].1 > ws_scored[j - 1].1 {
            ws_scored.swap(j, j - 1);
            j -= 1;
        }
    }

    // Project top workshop cards (for resource estimation only)
    let take = (total_workshop_slots as usize).min(ws_scored_count);
    for i in 0..take {
        let id = ws_scored[i].0;
        let card = card_lookup[id as usize];
        for &color in card.colors() {
            projected_wheel.increment(color);
        }
        for &mt in card.material_types() {
            projected_materials.increment(mt);
        }
    }

    // Step 4: Find achievable sell cards
    // Check affordability against post-workshop wheel (before mixes),
    // but account for colors that could be produced by mixes.
    // Sort sell card display indices by ducats descending
    let mut sell_indices: [usize; MAX_SELL_CARD_DISPLAY] = [0; MAX_SELL_CARD_DISPLAY];
    let sell_len = sell_card_display.len();
    for i in 0..sell_len {
        sell_indices[i] = i;
    }
    for i in 1..sell_len {
        let mut j = i;
        while j > 0 && sell_card_display[sell_indices[j]].sell_card.ducats() > sell_card_display[sell_indices[j - 1]].sell_card.ducats() {
            sell_indices.swap(j, j - 1);
            j -= 1;
        }
    }

    let mut sells_planned = 0usize;
    let mut used_sell_draft: [bool; 8] = [false; 8];
    let mut used_destroy_sell: [bool; 8] = [false; 8]; // tracks which destroy assignments provide Sell
    // Track state as we commit sells: simulate on a copy of the projected wheel
    let mut sim_wheel = projected_wheel.clone();
    let mut sim_materials = projected_materials.clone();
    let mut mix_slots_remaining = total_mix_slots;

    let mut target_sells: [(u32, usize); 4] = [(0, 0); 4]; // (instance_id, sell_display_index)
    let mut target_sell_count = 0usize;
    // Track which mixes are needed for each target sell
    let mut sell_mixes: [[(Color, Color); 2]; 4] = [[(Color::Red, Color::Red); 2]; 4];
    let mut sell_mix_counts: [usize; 4] = [0; 4];
    // Track whether each target sell uses a direct Sell card or a DestroyForSell
    let mut sell_is_destroy: [bool; 4] = [false; 4];
    let mut sell_destroy_idx: [usize; 4] = [0; 4]; // index into destroy_assignments

    for si in 0..sell_len {
        if sells_planned >= total_sell_abilities {
            break;
        }
        let idx = sell_indices[si];
        let sc = &sell_card_display[idx];

        // Check material
        let mat = sc.sell_card.required_material();
        if sim_materials.get(mat) == 0 {
            continue;
        }

        // Check colors — try to pay directly, plan mixes for missing colors
        let cost = sc.sell_card.color_cost();
        let mut temp_wheel = sim_wheel.clone();
        let mut mixes_for_this: [(Color, Color); 2] = [(Color::Red, Color::Red); 2];
        let mut mix_count_for_this = 0usize;
        let mut mix_budget = mix_slots_remaining;
        let mut affordable = true;

        for &c in cost {
            if temp_wheel.get(c) > 0 {
                // Have it, consume it
                temp_wheel.decrement(c);
            } else if mix_budget > 0 && mix_count_for_this < 2 {
                // Try to produce via mix
                let mut found_mix = false;
                for &(a, b) in &VALID_MIX_PAIRS {
                    if mix_result(a, b) == c && temp_wheel.get(a) > 0 && temp_wheel.get(b) > 0 {
                        mixes_for_this[mix_count_for_this] = (a, b);
                        mix_count_for_this += 1;
                        mix_budget -= 1;
                        // Apply the mix to temp_wheel so subsequent cost checks
                        // see the consumed inputs and produced output
                        perform_mix_unchecked(&mut temp_wheel, a, b);
                        // Now consume the output for this sell cost
                        temp_wheel.decrement(c);
                        found_mix = true;
                        break;
                    }
                }
                if !found_mix {
                    affordable = false;
                    break;
                }
            } else {
                affordable = false;
                break;
            }
        }

        if !affordable {
            continue;
        }

        // Find a sell ability source: prefer direct Sell drafted cards, then DestroyCards→Sell
        let mut found_source = false;
        let mut is_destroy = false;
        let mut destroy_idx = 0usize;

        for i in 0..draft_sell_count {
            if !used_sell_draft[i] {
                used_sell_draft[i] = true;
                found_source = true;
                break;
            }
        }
        if !found_source {
            // Try DestroyCards→Sell assignments
            for i in 0..destroy_assign_count {
                if used_destroy_sell[i] { continue; }
                if let Some(ref a) = destroy_assignments[i] {
                    if matches!(a.ability, Ability::Sell) {
                        used_destroy_sell[i] = true;
                        found_source = true;
                        is_destroy = true;
                        destroy_idx = i;
                        break;
                    }
                }
            }
        }
        if !found_source {
            break;
        }

        // Commit this sell — update simulation state
        sim_wheel = temp_wheel;
        sim_materials.decrement(mat);
        mix_slots_remaining = mix_budget;
        sell_mixes[target_sell_count] = mixes_for_this;
        sell_mix_counts[target_sell_count] = mix_count_for_this;
        sell_is_destroy[target_sell_count] = is_destroy;
        sell_destroy_idx[target_sell_count] = destroy_idx;
        target_sells[target_sell_count] = (sc.instance_id, idx);
        target_sell_count += 1;
        sells_planned += 1;
    }

    if target_sell_count == 0 {
        return plan; // No achievable sells
    }

    // Step 5: Build the action plan
    // Determine which workshop cards are actually needed for the target sells
    let mut needed_colors = [0u32; 12];
    let mut needed_materials = [0u32; 3];
    for i in 0..target_sell_count {
        let (_, idx) = target_sells[i];
        let sc = &sell_card_display[idx];
        needed_materials[sc.sell_card.required_material() as usize] += 1;
        for &c in sc.sell_card.color_cost() {
            needed_colors[c.index()] += 1;
        }
        // Mixes consume inputs and produce outputs — account for net color needs
        for j in 0..sell_mix_counts[i] {
            let (a, b) = sell_mixes[i][j];
            needed_colors[a.index()] += 1;
            needed_colors[b.index()] += 1;
            let output = mix_result(a, b);
            // The mix output is consumed by the sell cost, which is already counted above,
            // so we don't subtract it here — the sell cost entry covers the output.
            // But we do need the inputs.
            let _ = output;
        }
    }
    // Subtract what we already have
    for i in 0..3 {
        needed_materials[i] = needed_materials[i].saturating_sub(player.materials.counts[i]);
    }
    for i in 0..12 {
        needed_colors[i] = needed_colors[i].saturating_sub(player.color_wheel.counts[i]);
    }

    // Select workshop cards that provide needed resources (in score order)
    let mut ws_for_plan: [(u8, u32); 16] = [(0, 0); 16]; // (id, score) preserving order
    let mut ws_for_plan_count = 0usize;
    let mut ws_plan_slots = 0u32;
    for i in 0..ws_scored_count {
        if ws_plan_slots >= total_workshop_slots {
            break;
        }
        let id = ws_scored[i].0;
        let card = card_lookup[id as usize];
        let mut useful = false;
        for &mt in card.material_types() {
            if needed_materials[mt as usize] > 0 {
                useful = true;
            }
        }
        for &color in card.colors() {
            if needed_colors[color.index()] > 0 {
                useful = true;
            }
        }
        if useful {
            ws_for_plan[ws_for_plan_count] = (id, ws_scored[i].1);
            ws_for_plan_count += 1;
            ws_plan_slots += 1;
            for &mt in card.material_types() {
                needed_materials[mt as usize] = needed_materials[mt as usize].saturating_sub(1);
            }
            for &color in card.colors() {
                needed_colors[color.index()] = needed_colors[color.index()].saturating_sub(1);
            }
        }
    }

    // Collect all planned mixes from the sell plans
    let mut plan_mixes: [(Color, Color); 4] = [(Color::Red, Color::Red); 4];
    let mut plan_mix_count = 0usize;
    for i in 0..target_sell_count {
        for j in 0..sell_mix_counts[i] {
            if plan_mix_count < 4 {
                plan_mixes[plan_mix_count] = sell_mixes[i][j];
                plan_mix_count += 1;
            }
        }
    }

    // Build action sequence:
    // 1. Workshop (from drafted Workshop cards)
    // 2. Destroy → Workshop (DestroyCards targeting Workshop-ability workshop cards)
    // 3. Mix (from drafted MixColors cards)
    // 4. Destroy → MixColors (DestroyCards targeting MixColors-ability workshop cards)
    // 5. Sell (from drafted Sell cards)
    // 6. Destroy → Sell (DestroyCards targeting Sell-ability workshop cards)
    let mut action_idx = 0usize;

    // 1. Add Workshop actions from drafted Workshop cards
    if ws_for_plan_count > 0 {
        let mut ws_assigned = 0usize;
        for i in 0..draft_workshop_count {
            if ws_assigned >= ws_for_plan_count {
                break;
            }
            let (draft_id, count) = draft_workshop[i];
            let mut selected = UnorderedCards::new();
            let take = (count as usize).min(ws_for_plan_count - ws_assigned);
            for j in 0..take {
                selected.insert(ws_for_plan[ws_assigned + j].0);
            }
            ws_assigned += take;
            if selected.is_empty() {
                continue;
            }
            plan.actions[action_idx] = Some(SellPlanAction {
                draft_card_id: draft_id,
                action_type: SellPlanActionType::Workshop { selected_cards: selected },
            });
            plan.consumed_draft_ids[plan.consumed_count] = draft_id;
            plan.consumed_count += 1;
            action_idx += 1;
        }
    }

    // 2. Add Destroy → Workshop actions
    for i in 0..destroy_assign_count {
        if let Some(ref a) = destroy_assignments[i] {
            if matches!(a.ability, Ability::Workshop { .. }) {
                plan.actions[action_idx] = Some(SellPlanAction {
                    draft_card_id: a.draft_id,
                    action_type: SellPlanActionType::Destroy { target_workshop_id: a.workshop_id },
                });
                plan.consumed_draft_ids[plan.consumed_count] = a.draft_id;
                plan.consumed_count += 1;
                action_idx += 1;
            }
        }
    }

    // 3. Add Mix actions from drafted MixColors cards
    if plan_mix_count > 0 {
        let mut mixes_assigned = 0usize;
        for i in 0..draft_mix_count {
            if mixes_assigned >= plan_mix_count {
                break;
            }
            let (draft_id, count) = draft_mix[i];
            let mut mixes = [(Color::Red, Color::Red); 2];
            let mut mc = 0usize;
            for _ in 0..(count as usize).min(2) {
                if mixes_assigned >= plan_mix_count {
                    break;
                }
                mixes[mc] = plan_mixes[mixes_assigned];
                mc += 1;
                mixes_assigned += 1;
            }
            if mc > 0 {
                plan.actions[action_idx] = Some(SellPlanAction {
                    draft_card_id: draft_id,
                    action_type: SellPlanActionType::Mix { mixes, mix_count: mc },
                });
                plan.consumed_draft_ids[plan.consumed_count] = draft_id;
                plan.consumed_count += 1;
                action_idx += 1;
            }
        }
    }

    // 4. Add Destroy → MixColors actions
    for i in 0..destroy_assign_count {
        if let Some(ref a) = destroy_assignments[i] {
            if matches!(a.ability, Ability::MixColors { .. }) {
                plan.actions[action_idx] = Some(SellPlanAction {
                    draft_card_id: a.draft_id,
                    action_type: SellPlanActionType::Destroy { target_workshop_id: a.workshop_id },
                });
                plan.consumed_draft_ids[plan.consumed_count] = a.draft_id;
                plan.consumed_count += 1;
                action_idx += 1;
            }
        }
    }

    // 5. Add Sell actions from drafted Sell cards
    let mut sell_target_idx = 0usize;
    for i in 0..draft_sell_count {
        if sell_target_idx >= target_sell_count {
            break;
        }
        // Skip sells that aren't assigned to direct Sell cards
        if !used_sell_draft[i] {
            continue;
        }
        // Find the next target that uses a direct Sell card
        while sell_target_idx < target_sell_count && sell_is_destroy[sell_target_idx] {
            sell_target_idx += 1;
        }
        if sell_target_idx >= target_sell_count {
            break;
        }
        let (instance_id, _) = target_sells[sell_target_idx];
        plan.actions[action_idx] = Some(SellPlanAction {
            draft_card_id: draft_sell[i],
            action_type: SellPlanActionType::Sell { sell_card_instance_id: instance_id },
        });
        plan.consumed_draft_ids[plan.consumed_count] = draft_sell[i];
        plan.consumed_count += 1;
        action_idx += 1;
        sell_target_idx += 1;
    }

    // 6. Add Destroy → Sell actions
    for i in 0..target_sell_count {
        if !sell_is_destroy[i] {
            continue;
        }
        let (instance_id, _) = target_sells[i];
        let da_idx = sell_destroy_idx[i];
        if let Some(ref a) = destroy_assignments[da_idx] {
            plan.actions[action_idx] = Some(SellPlanAction {
                draft_card_id: a.draft_id,
                action_type: SellPlanActionType::DestroyForSell {
                    target_workshop_id: a.workshop_id,
                    sell_card_instance_id: instance_id,
                },
            });
            plan.consumed_draft_ids[plan.consumed_count] = a.draft_id;
            plan.consumed_count += 1;
            action_idx += 1;
        }
    }

    plan.action_count = action_idx;
    plan
}

/// Resolve pending abilities on the stack using heuristic logic.
/// Loops until the ability stack is empty.
fn resolve_pending_abilities_heuristic(state: &mut GameState, rng: &mut impl Rng) {
    loop {
        let (player_index, ability) = match &state.phase {
            GamePhase::Action { action_state } => {
                match action_state.ability_stack.last() {
                    Some(ability) => (action_state.current_player_index, *ability),
                    None => return,
                }
            }
            _ => return,
        };

        let cache = SellCardCache::new(
            &state.sell_card_display,
            &state.players[player_index].color_wheel,
            &state.players[player_index].materials,
        );

        match ability {
            Ability::Workshop { count } => {
                let workshop = state.players[player_index].workshop_cards;
                if workshop.is_empty() {
                    skip_workshop(state, rng);
                } else {
                    // Score and pick best workshop cards
                    let mut scored: [(u8, u32); 16] = [(0, 0); 16];
                    let mut scored_count = 0usize;
                    for id in workshop.iter() {
                        let card = state.card_lookup[id as usize];
                        let score = workshop_card_score(card, &cache);
                        scored[scored_count] = (id, score);
                        scored_count += 1;
                    }
                    for i in 1..scored_count {
                        let mut j = i;
                        while j > 0 && scored[j].1 > scored[j - 1].1 {
                            scored.swap(j, j - 1);
                            j -= 1;
                        }
                    }
                    let take = (count as usize).min(scored_count);
                    let mut selected = UnorderedCards::new();
                    for i in 0..take {
                        selected.insert(scored[i].0);
                    }
                    resolve_workshop_choice(state, selected, rng);
                }
            }
            Ability::MixColors { count } => {
                let (mixes, mix_count) = two_step_heuristic_mix_seq(
                    &state.players[player_index].color_wheel, count, &cache, rng,
                );
                for i in 0..mix_count {
                    let (a, b) = mixes[i];
                    perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
                }
                if let GamePhase::Action { ref mut action_state } = state.phase {
                    action_state.ability_stack.pop();
                }
                process_ability_stack(state, rng);
            }
            Ability::Sell => {
                let sell_card_id_opt = cache.best_affordable_id;
                match sell_card_id_opt {
                    Some(sell_card_id) => {
                        resolve_select_sell_card(state, sell_card_id, rng);
                    }
                    None => {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    }
                }
            }
            Ability::DestroyCards => {
                let workshop = state.players[player_index].workshop_cards;
                if workshop.is_empty() {
                    resolve_destroy_cards(state, UnorderedCards::new(), rng);
                } else {
                    let mut copy = workshop;
                    let selected = copy.draw_up_to(1, rng);
                    resolve_destroy_cards(state, selected, rng);
                }
            }
            Ability::GainSecondary => {
                let color = pick_best_color(&SECONDARIES, &cache, rng);
                resolve_gain_color(state, color, rng);
            }
            Ability::GainPrimary => {
                let color = pick_best_color(&PRIMARIES, &cache, rng);
                resolve_gain_color(state, color, rng);
            }
            Ability::ChangeTertiary => {
                let player = &state.players[player_index];
                let mut owned_tertiaries = [Color::Red; 6];
                let mut own_count = 0usize;
                for &c in &TERTIARIES {
                    if player.color_wheel.get(c) > 0 {
                        owned_tertiaries[own_count] = c;
                        own_count += 1;
                    }
                }
                if own_count == 0 {
                    if let GamePhase::Action { ref mut action_state } = state.phase {
                        action_state.ability_stack.pop();
                    }
                    process_ability_stack(state, rng);
                } else {
                    let mut best_lose = owned_tertiaries[0];
                    let mut best_lose_score = u32::MAX;
                    for i in 0..own_count {
                        let score = cache.color_demand(owned_tertiaries[i]);
                        if score < best_lose_score {
                            best_lose_score = score;
                            best_lose = owned_tertiaries[i];
                        }
                    }
                    let mut best_gain = TERTIARIES[0];
                    let mut best_gain_score = 0u32;
                    for &c in &TERTIARIES {
                        if c != best_lose {
                            let score = cache.color_demand(c);
                            if score > best_gain_score {
                                best_gain_score = score;
                                best_gain = c;
                            }
                        }
                    }
                    resolve_choose_tertiary_to_lose(state, best_lose);
                    resolve_choose_tertiary_to_gain(state, best_gain, rng);
                }
            }
            Ability::MoveToDrafted => {
                let player = &mut state.players[player_index];
                if player.workshop_cards.is_empty() {
                    if let GamePhase::Action { ref mut action_state } = state.phase {
                        action_state.ability_stack.pop();
                    }
                    process_ability_stack(state, rng);
                } else {
                    let card_id = player.workshop_cards.pick_random(rng).unwrap();
                    player.workshop_cards.remove(card_id);
                    player.drafted_cards.insert(card_id);
                    if let GamePhase::Action { ref mut action_state } = state.phase {
                        action_state.ability_stack.pop();
                    }
                    process_ability_stack(state, rng);
                }
            }
            _ => {
                // DrawCards, GainDucats handled by process_ability_stack
                process_ability_stack(state, rng);
            }
        }
    }
}

fn execute_solver_turn(state: &mut GameState, player_index: usize, heuristic_draft: bool, rng: &mut impl Rng) {
    // Skip the solver in the first 2 rounds — early game is better spent
    // building the engine (colors, materials) rather than selling
    let plan = if state.round <= 2 {
        SellPlan {
            actions: [const { None }; 8],
            action_count: 0,
            consumed_draft_ids: [0; 8],
            consumed_count: 0,
        }
    } else {
        compute_sell_plan(
            &state.players[player_index],
            &state.sell_card_display,
            &state.card_lookup,
            &state.players[player_index].workshop_cards,
        )
    };

    // Phase 1: Execute sell plan actions
    for i in 0..plan.action_count {
        let action = match &plan.actions[i] {
            Some(a) => a,
            None => continue,
        };

        // Verify the drafted card is still available
        if !state.players[player_index].drafted_cards.contains(action.draft_card_id) {
            continue;
        }

        match &action.action_type {
            SellPlanActionType::Workshop { selected_cards } => {
                // Destroy the drafted card to get the Workshop ability
                destroy_drafted_card(state, action.draft_card_id as u32, rng);
                // Now Workshop ability should be on the stack
                if let GamePhase::Action { ref action_state } = state.phase {
                    if let Some(Ability::Workshop { count }) = action_state.ability_stack.last() {
                        let ws_count = *count;
                        // Intersect planned selection with actual workshop cards
                        let actual_available = state.players[player_index].workshop_cards;
                        let valid_selection = selected_cards.intersection(actual_available);
                        if valid_selection.is_empty() && actual_available.is_empty() {
                            skip_workshop(state, rng);
                        } else {
                            // Fill remaining capacity with heuristic-scored cards
                            let remaining_slots = (ws_count - valid_selection.len().min(ws_count)) as usize;
                            let mut full_selection = valid_selection;
                            if remaining_slots > 0 {
                                let pool = actual_available.difference(valid_selection);
                                let cache = SellCardCache::new(
                                    &state.sell_card_display,
                                    &state.players[player_index].color_wheel,
                                    &state.players[player_index].materials,
                                );
                                let mut scored: [(u8, u32); 16] = [(0, 0); 16];
                                let mut scored_count = 0usize;
                                for id in pool.iter() {
                                    let card = state.card_lookup[id as usize];
                                    let score = workshop_card_score(card, &cache);
                                    scored[scored_count] = (id, score);
                                    scored_count += 1;
                                }
                                for ii in 1..scored_count {
                                    let mut jj = ii;
                                    while jj > 0 && scored[jj].1 > scored[jj - 1].1 {
                                        scored.swap(jj, jj - 1);
                                        jj -= 1;
                                    }
                                }
                                let fill = remaining_slots.min(scored_count);
                                for fi in 0..fill {
                                    full_selection.insert(scored[fi].0);
                                }
                            }
                            if full_selection.is_empty() {
                                skip_workshop(state, rng);
                            } else {
                                resolve_workshop_choice(state, full_selection, rng);
                            }
                        }
                    }
                }
                // Resolve any nested abilities from action workshop cards
                resolve_pending_abilities_heuristic(state, rng);
            }
            SellPlanActionType::Mix { mixes, mix_count } => {
                // Destroy the drafted card to get the MixColors ability
                destroy_drafted_card(state, action.draft_card_id as u32, rng);
                // Now MixColors ability should be on the stack
                if let GamePhase::Action { ref action_state } = state.phase {
                    if let Some(Ability::MixColors { count }) = action_state.ability_stack.last() {
                        let total_mixes = *count;
                        // First, perform the planned mixes
                        let mut done = 0usize;
                        for j in 0..*mix_count {
                            let (a, b) = mixes[j];
                            let wheel = &state.players[player_index].color_wheel;
                            if wheel.get(a) > 0 && wheel.get(b) > 0 {
                                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
                                done += 1;
                            }
                        }
                        // Fill remaining mix capacity with heuristic choices
                        let remaining = (total_mixes as usize).saturating_sub(done);
                        if remaining > 0 {
                            let cache = SellCardCache::new(
                                &state.sell_card_display,
                                &state.players[player_index].color_wheel,
                                &state.players[player_index].materials,
                            );
                            let (extra_mixes, extra_count) = heuristic_mix_seq(
                                &state.players[player_index].color_wheel,
                                remaining as u32,
                                &cache,
                                rng,
                            );
                            for j in 0..extra_count {
                                let (a, b) = extra_mixes[j];
                                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
                            }
                        }
                        // Pop the MixColors ability from the stack
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    }
                }
                resolve_pending_abilities_heuristic(state, rng);
            }
            SellPlanActionType::Sell { sell_card_instance_id } => {
                // Destroy the drafted card to get the Sell ability
                destroy_drafted_card(state, action.draft_card_id as u32, rng);
                // Verify the sell card is still affordable and available
                if let GamePhase::Action { ref action_state } = state.phase {
                    if matches!(action_state.ability_stack.last(), Some(Ability::Sell)) {
                        let still_valid = state.sell_card_display.iter().any(|sc| {
                            sc.instance_id == *sell_card_instance_id
                                && can_afford_sell_card(&state.players[player_index], &sc.sell_card)
                        });
                        if still_valid {
                            resolve_select_sell_card(state, *sell_card_instance_id, rng);
                        } else {
                            // Fall through to heuristic sell
                            resolve_pending_abilities_heuristic(state, rng);
                        }
                    }
                }
                resolve_pending_abilities_heuristic(state, rng);
            }
            SellPlanActionType::Destroy { target_workshop_id } => {
                // Destroy the drafted DestroyCards card, then target the planned workshop card
                destroy_drafted_card(state, action.draft_card_id as u32, rng);
                if let GamePhase::Action { ref action_state } = state.phase {
                    if matches!(action_state.ability_stack.last(), Some(Ability::DestroyCards)) {
                        let player = &state.players[player_index];
                        if player.workshop_cards.contains(*target_workshop_id) {
                            let mut selected = UnorderedCards::new();
                            selected.insert(*target_workshop_id);
                            resolve_destroy_cards(state, selected, rng);
                        } else {
                            // Target no longer available, let heuristic pick
                            resolve_pending_abilities_heuristic(state, rng);
                        }
                    }
                }
                // Resolve the resulting ability (Workshop, MixColors, etc.) via heuristic
                resolve_pending_abilities_heuristic(state, rng);
            }
            SellPlanActionType::DestroyForSell { target_workshop_id, sell_card_instance_id } => {
                // Destroy the drafted DestroyCards card
                destroy_drafted_card(state, action.draft_card_id as u32, rng);
                if let GamePhase::Action { ref action_state } = state.phase {
                    if matches!(action_state.ability_stack.last(), Some(Ability::DestroyCards)) {
                        let player = &state.players[player_index];
                        if player.workshop_cards.contains(*target_workshop_id) {
                            let mut selected = UnorderedCards::new();
                            selected.insert(*target_workshop_id);
                            resolve_destroy_cards(state, selected, rng);
                            // Now Sell should be on the stack
                            if let GamePhase::Action { ref action_state } = state.phase {
                                if matches!(action_state.ability_stack.last(), Some(Ability::Sell)) {
                                    let still_valid = state.sell_card_display.iter().any(|sc| {
                                        sc.instance_id == *sell_card_instance_id
                                            && can_afford_sell_card(&state.players[player_index], &sc.sell_card)
                                    });
                                    if still_valid {
                                        resolve_select_sell_card(state, *sell_card_instance_id, rng);
                                    } else {
                                        resolve_pending_abilities_heuristic(state, rng);
                                    }
                                }
                            }
                        } else {
                            resolve_pending_abilities_heuristic(state, rng);
                        }
                    }
                }
                resolve_pending_abilities_heuristic(state, rng);
            }
        }
    }

    // Phase 2: Handle remaining drafted cards with heuristic
    loop {
        match &state.phase {
            GamePhase::Action { action_state } => {
                if !action_state.ability_stack.is_empty() {
                    resolve_pending_abilities_heuristic(state, rng);
                    continue;
                }
                let player_idx = action_state.current_player_index;
                if player_idx != player_index {
                    break; // Turn has ended, moved to next player
                }
                let drafted = state.players[player_index].drafted_cards;
                if drafted.is_empty() {
                    // Try glass then end turn
                    if !try_activate_random_glass(state, player_index, rng) {
                        end_player_turn(state, rng);
                        if matches!(state.phase, GamePhase::Draw) {
                            if heuristic_draft {
                                heuristic_rollout_draw_and_draft(state, rng);
                            } else {
                                rollout_draw_and_draft(state, rng);
                            }
                        }
                    }
                    break;
                }
                // Use heuristic for remaining cards
                let cache = SellCardCache::new(
                    &state.sell_card_display,
                    &state.players[player_index].color_wheel,
                    &state.players[player_index].materials,
                );
                handle_action_no_pending_heuristic(state, player_index, heuristic_draft, &cache, rng);
            }
            _ => break,
        }
    }
}

pub fn apply_solver_rollout_step<R: Rng>(state: &mut GameState, heuristic_draft: bool, rng: &mut R) {
    // Draft phase: same as heuristic
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if heuristic_draft {
            heuristic_draft_loop(state, rng);
        } else {
            loop {
                let card_id = {
                    if let GamePhase::Draft { ref draft_state } = state.phase {
                        let player = draft_state.current_player_index;
                        let hand = draft_state.hands[player];
                        match hand.pick_random(rng) {
                            Some(id) => id as u32,
                            None => break,
                        }
                    } else {
                        break;
                    }
                };
                player_pick(state, card_id, rng);
            }
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player_index = action_state.current_player_index;
            if action_state.ability_stack.is_empty() {
                execute_solver_turn(state, player_index, heuristic_draft, rng);
            } else {
                // Shouldn't normally happen, but fallback
                apply_heuristic_rollout_step(state, heuristic_draft, rng);
            }
        }
        _ => {}
    }
}
