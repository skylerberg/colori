use crate::action_phase::{
    can_afford_glass, can_afford_sell_card, destroy_drafted_card, end_player_turn,
    get_action_state_mut, initialize_action_phase, mark_glass_used,
    process_ability_stack, resolve_choose_tertiary_to_gain, resolve_choose_tertiary_to_lose,
    resolve_destroy_cards, resolve_gain_color, resolve_select_sell_card,
    resolve_select_glass, resolve_workshop_choice, resolve_workshop_with_reworkshop,
    skip_workshop,
};
use crate::colors::{
    is_primary, pay_cost, perform_mix_unchecked, perform_unmix, PRIMARIES, SECONDARIES,
    TERTIARIES, VALID_MIX_PAIRS,
};
use crate::choices::is_glass_ability_available;
use crate::deck_utils::draw_from_deck;
use crate::draft_phase::player_pick;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use rand::RngExt;
use smallvec::SmallVec;

// ── Move tracking trait ──

pub trait MoveTracker {
    fn track(&mut self, player: usize, choice: Choice);
}

pub struct NoTracking;
impl MoveTracker for NoTracking {
    #[inline(always)]
    fn track(&mut self, _: usize, _: Choice) {}
}

impl MoveTracker for Vec<(usize, Choice)> {
    #[inline(always)]
    fn track(&mut self, player: usize, choice: Choice) {
        self.push((player, choice));
    }
}

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
fn activate_glass_in_rollout_impl<T: MoveTracker, R: Rng>(
    state: &mut GameState,
    player_index: usize,
    glass: GlassCard,
    tracker: &mut T,
    rng: &mut R,
) {
    mark_glass_used(state, glass);
    match glass {
        // Stack-pushing abilities
        GlassCard::GlassWorkshop => {
            tracker.track(player_index, Choice::ActivateGlassWorkshop);
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::Workshop { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassDraw => {
            tracker.track(player_index, Choice::ActivateGlassDraw);
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::DrawCards { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassMix => {
            tracker.track(player_index, Choice::ActivateGlassMix);
            get_action_state_mut(state)
                .ability_stack
                .push(Ability::MixColors { count: 1 });
            process_ability_stack(state, rng);
        }
        GlassCard::GlassGainPrimary => {
            tracker.track(player_index, Choice::ActivateGlassGainPrimary);
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
            tracker.track(player_index, Choice::ActivateGlassExchange { lose, gain });
            let player = &mut state.players[player_index];
            player.materials.decrement(lose);
            player.materials.increment(gain);
        }
        GlassCard::GlassMoveDrafted => {
            let player = &mut state.players[player_index];
            let card_id = player.drafted_cards.pick_random(rng).unwrap();
            let card = state.card_lookup[card_id as usize];
            tracker.track(player_index, Choice::ActivateGlassMoveDrafted { card });
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
            tracker.track(player_index, Choice::ActivateGlassUnmix { color });
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
            tracker.track(player_index, Choice::ActivateGlassTertiaryDucat { color });
            let player = &mut state.players[player_index];
            player.color_wheel.decrement(color);
            player.ducats += 1;
            player.cached_score += 1;
        }
        GlassCard::GlassReworkshop => {
            let player = &mut state.players[player_index];
            let card_id = player.workshopped_cards.pick_random(rng).unwrap();
            let card = state.card_lookup[card_id as usize];
            tracker.track(player_index, Choice::ActivateGlassReworkshop { card });
            player.workshopped_cards.remove(card_id);
            player.workshop_cards.insert(card_id);
        }
        GlassCard::GlassDestroyClean => {
            let card_id = state.players[player_index].workshop_cards.pick_random(rng).unwrap();
            let card = state.card_lookup[card_id as usize];
            tracker.track(player_index, Choice::ActivateGlassDestroyClean { card });
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
fn try_activate_random_glass_impl<T: MoveTracker, R: Rng>(
    state: &mut GameState,
    player_index: usize,
    tracker: &mut T,
    rng: &mut R,
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
    activate_glass_in_rollout_impl(state, player_index, available[choice], tracker, rng);
    true
}

#[inline(always)]
fn handle_action_no_pending_impl<T: MoveTracker, R: Rng>(state: &mut GameState, player_index: usize, tracker: &mut T, rng: &mut R) {
    // Try activating a random glass ability before picking a drafted card
    if try_activate_random_glass_impl(state, player_index, tracker, rng) {
        return;
    }

    let mut copy = state.players[player_index].drafted_cards;
    let sel = copy.draw_up_to(1, rng);
    if sel.is_empty() {
        // No drafted cards left — end turn and advance to next round
        tracker.track(player_index, Choice::EndTurn);
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            rollout_draw_and_draft(state, rng);
        }
        return;
    }

    let card_id = sel.lowest_bit().unwrap();
    let card = state.card_lookup[card_id as usize];
    match card.ability() {
        Ability::MixColors { count } => {
            let (mixes, mix_count) =
                random_mix_seq(&state.players[player_index].color_wheel, count, rng);
            let mut mixes_vec = SmallVec::new();
            for i in 0..mix_count {
                mixes_vec.push(mixes[i]);
            }
            tracker.track(player_index, Choice::DestroyAndMix { card, mixes: mixes_vec });
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
                        let sell_card = state.sell_card_display.iter()
                            .find(|c| c.instance_id == sell_card_id).unwrap().sell_card;
                        tracker.track(player_index, Choice::DestroyAndSell { card, sell_card });
                        fused_buy(state, player_index, card_id, sell_card_id, rng);
                    } else {
                        let (glass, pay_color) = fused_glass_acquire(state, player_index, card_id, rng);
                        tracker.track(player_index, Choice::DestroyAndSelectGlass { card, glass, pay_color });
                    }
                }
                (Some(sell_card_id), false) => {
                    let sell_card = state.sell_card_display.iter()
                        .find(|c| c.instance_id == sell_card_id).unwrap().sell_card;
                    tracker.track(player_index, Choice::DestroyAndSell { card, sell_card });
                    fused_buy(state, player_index, card_id, sell_card_id, rng);
                }
                (None, true) => {
                    let (glass, pay_color) = fused_glass_acquire(state, player_index, card_id, rng);
                    tracker.track(player_index, Choice::DestroyAndSelectGlass { card, glass, pay_color });
                }
                (None, false) => {
                    tracker.track(player_index, Choice::DestroyDraftedCard { card });
                    destroy_drafted_card(state, card_id as u32, rng);
                }
            }
        }
        _ => {
            tracker.track(player_index, Choice::DestroyDraftedCard { card });
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
/// Returns (glass_card, pay_color) for tracking.
#[inline(always)]
fn fused_glass_acquire<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    card_id: u8,
    rng: &mut R,
) -> (GlassCard, Color) {
    state.players[player_index].drafted_cards.remove(card_id);
    state.destroyed_pile.insert(card_id);

    // Pick random glass from display
    let glass_idx = rng.random_range(0..state.glass_display.len());
    let glass_instance = state.glass_display.swap_remove(glass_idx);
    let glass_card = glass_instance.card;

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

    (glass_card, pay_color)
}

#[inline(always)]
fn apply_rollout_step_impl<T: MoveTracker, R: Rng>(state: &mut GameState, tracker: &mut T, rng: &mut R) {
    // Fast path: complete entire draft in one step
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        loop {
            let (player_index, card_id) = {
                if let GamePhase::Draft { ref draft_state } = state.phase {
                    let player = draft_state.current_player_index;
                    let hand = draft_state.hands[player];
                    match hand.pick_random(rng) {
                        Some(id) => (player, id as u32),
                        None => break, // Empty hand (e.g., GlassKeepBoth)
                    }
                } else {
                    break;
                }
            };
            let card = state.card_lookup[card_id as usize];
            tracker.track(player_index, Choice::DraftPick { card });
            player_pick(state, card_id);
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player_index = action_state.current_player_index;
            match action_state.ability_stack.last() {
                None => {
                    handle_action_no_pending_impl(state, player_index, tracker, rng);
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
                            tracker.track(player_index, Choice::SkipWorkshop);
                            skip_workshop(state, rng);
                        } else {
                            let reworkshop_id = selected.pick_random(rng).unwrap();
                            let reworkshop_card = state.card_lookup[reworkshop_id as usize];
                            let mut other_cards = SmallVec::new();
                            for id in selected.iter() {
                                if id != reworkshop_id {
                                    other_cards.push(state.card_lookup[id as usize]);
                                }
                            }
                            tracker.track(player_index, Choice::WorkshopWithReworkshop { reworkshop_card, other_cards });
                            mark_glass_used(state, GlassCard::GlassReworkshop);
                            resolve_workshop_with_reworkshop(state, selected, reworkshop_id, rng);
                        }
                    } else {
                        let mut copy = state.players[player_index].workshop_cards;
                        let selected = copy.draw_up_to(count as u8, rng);
                        if selected.is_empty() {
                            tracker.track(player_index, Choice::SkipWorkshop);
                            skip_workshop(state, rng);
                        } else {
                            let mut card_types = SmallVec::new();
                            for id in selected.iter() {
                                card_types.push(state.card_lookup[id as usize]);
                            }
                            tracker.track(player_index, Choice::Workshop { card_types });
                            resolve_workshop_choice(state, selected, rng);
                        }
                    }
                }
                Some(Ability::DestroyCards) => {
                    let mut copy = state.players[player_index].workshop_cards;
                    let selected = copy.draw_up_to(1, rng);
                    let card = selected.iter().next().map(|id| state.card_lookup[id as usize]);
                    tracker.track(player_index, Choice::DestroyDrawnCards { card });
                    resolve_destroy_cards(state, selected, rng);
                }
                Some(Ability::MixColors { count }) => {
                    let remaining_mixes = *count;
                    let (mixes, mix_count) =
                        random_mix_seq(&state.players[player_index].color_wheel, remaining_mixes, rng);
                    let mut mixes_vec = SmallVec::new();
                    for i in 0..mix_count {
                        mixes_vec.push(mixes[i]);
                    }
                    tracker.track(player_index, Choice::MixAll { mixes: mixes_vec });
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
                                let sell_card = state.sell_card_display.iter()
                                    .find(|c| c.instance_id == sell_card_id).unwrap().sell_card;
                                tracker.track(player_index, Choice::SelectSellCard { sell_card });
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
                                tracker.track(player_index, Choice::SelectGlass { glass: glass_card, pay_color });
                                resolve_select_glass(state, glass_card, pay_color, rng);
                            }
                        }
                        (Some(sell_card_id), false) => {
                            let sell_card = state.sell_card_display.iter()
                                .find(|c| c.instance_id == sell_card_id).unwrap().sell_card;
                            tracker.track(player_index, Choice::SelectSellCard { sell_card });
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
                            tracker.track(player_index, Choice::SelectGlass { glass: glass_card, pay_color });
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
                    tracker.track(player_index, Choice::GainSecondary { color });
                    resolve_gain_color(state, color, rng);
                }
                Some(Ability::GainPrimary) => {
                    let color = PRIMARIES[rng.random_range(0..PRIMARIES.len())];
                    tracker.track(player_index, Choice::GainPrimary { color });
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
                        tracker.track(player_index, Choice::SwapTertiary { lose: lose_color, gain: gain_color });
                        resolve_choose_tertiary_to_lose(state, lose_color);
                        resolve_choose_tertiary_to_gain(state, gain_color, rng);
                    }
                }
                // Instant abilities should never be on top waiting — they get processed immediately
                Some(_) => panic!("Unexpected ability on stack top during rollout"),
            }
        }
        _ => panic!("Cannot apply rollout step for current state"),
    }
}

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, rng: &mut R) {
    apply_rollout_step_impl(state, &mut NoTracking, rng);
}

pub fn apply_rollout_step_tracked<R: Rng>(
    state: &mut GameState,
    move_log: &mut Vec<(usize, Choice)>,
    rng: &mut R,
) {
    apply_rollout_step_impl(state, move_log, rng);
}
