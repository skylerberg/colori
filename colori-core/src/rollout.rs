use crate::action_phase::{
    can_afford_sell_card, destroy_drafted_card, end_player_turn,
    initialize_action_phase,
    process_ability_stack, resolve_choose_tertiary_to_gain, resolve_choose_tertiary_to_lose,
    resolve_destroy_cards, resolve_gain_color, resolve_select_sell_card,
    resolve_workshop_choice,
    skip_workshop,
};
use crate::colors::{
    mix_result, pay_cost, perform_mix_unchecked, PRIMARIES,
    SECONDARIES, TERTIARIES, VALID_MIX_PAIRS,
};
use crate::deck_utils::draw_from_deck;
use crate::draft_phase::player_pick;
use crate::scoring::HeuristicParams;
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

#[inline(always)]
fn handle_action_no_pending(state: &mut GameState, player_index: usize, heuristic_draft: bool, params: &HeuristicParams, rng: &mut impl Rng) {
    let mut copy = state.players[player_index].drafted_cards;
    let sel = copy.draw_up_to(1, rng);
    if sel.is_empty() {
        // No drafted cards left — end turn and advance to next round
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            if heuristic_draft {
                heuristic_rollout_draw_and_draft(state, params, rng);
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
            if let Some(sell_card_id) = pick_random_affordable_sell_card(
                &state.players[player_index],
                &state.sell_card_display,
                rng,
            ) {
                fused_buy(state, player_index, card_id, sell_card_id, rng);
            } else {
                destroy_drafted_card(state, card_id as u32, rng);
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

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, heuristic_draft: bool, params: &HeuristicParams, rng: &mut R) {
    // Fast path: complete entire draft in one step
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if heuristic_draft {
            heuristic_draft_loop(state, params, rng);
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
                    handle_action_no_pending(state, player_index, heuristic_draft, params, rng);
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
                    if let Some(sell_card_id) = pick_random_affordable_sell_card(
                        &state.players[player_index],
                        &state.sell_card_display,
                        rng,
                    ) {
                        resolve_select_sell_card(state, sell_card_id, rng);
                    } else {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
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
    params: &HeuristicParams,
    rng: &mut R,
) -> u8 {
    // Epsilon: random drop
    if rng.random_bool(params.rollout_epsilon) {
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
fn heuristic_rollout_draw_and_draft<R: Rng>(state: &mut GameState, params: &HeuristicParams, rng: &mut R) {
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
            let drop_id = pick_card_to_drop(&dealt[i], &state.card_lookup, params, rng);
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
fn heuristic_draft_loop<R: Rng>(state: &mut GameState, params: &HeuristicParams, rng: &mut R) {
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
                    Some(pick_card_to_drop(&hand, &state.card_lookup, params, rng))
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
    params: &HeuristicParams,
) -> u32 {
    match card.ability() {
        Ability::Sell => {
            if cache.best_affordable_ducats > 0 {
                cache.best_affordable_ducats * params.rollout_sell_affordable_multiplier
            } else {
                params.rollout_sell_base
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
                params.rollout_mix_no_pairs
            } else {
                params.rollout_mix_base + valid_pairs.min(4) * params.rollout_mix_pair_weight + count.min(2) * params.rollout_mix_count_weight
            }
        }
        Ability::Workshop { count } => {
            if player.workshop_cards.is_empty() {
                params.rollout_workshop_empty
            } else {
                params.rollout_workshop_base + count.min(3) * params.rollout_workshop_count_weight
            }
        }
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() {
                params.rollout_destroy_no_targets
            } else {
                params.rollout_destroy_with_targets
            }
        }
        Ability::DrawCards { count } => params.rollout_draw_base + count.min(3) * params.rollout_draw_count_weight,
        _ => params.rollout_other_priority,
    }
}

/// Score a workshop card for selection priority.
#[inline(always)]
fn workshop_card_score(
    card: Card,
    cache: &SellCardCache,
    params: &HeuristicParams,
) -> u32 {
    let mut score = 0u32;

    // Material cards: score by how much their material type is needed
    for &mt in card.material_types() {
        for i in 0..cache.len {
            let entry = &cache.entries[i];
            if entry.required_material == mt {
                let base = entry.ducats * params.rollout_ws_material_base_multiplier;
                if entry.colors_met {
                    score += base * params.rollout_ws_material_colors_met_multiplier;
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
        score += params.rollout_ws_action_bonus;
    }

    score
}

/// Pick the best color from a slice, preferring colors needed for sell cards.
#[inline(always)]
fn pick_best_color<R: Rng>(
    colors: &[Color],
    cache: &SellCardCache,
    params: &HeuristicParams,
    rng: &mut R,
) -> Color {
    if rng.random_bool(params.rollout_epsilon) {
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
    params: &HeuristicParams,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    if rng.random_bool(params.rollout_epsilon) {
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
    params: &HeuristicParams,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    if remaining < 2 {
        return heuristic_mix_seq(wheel, remaining, cache, params, rng);
    }

    if rng.random_bool(params.rollout_epsilon) {
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
fn handle_action_no_pending_heuristic(state: &mut GameState, player_index: usize, heuristic_draft: bool, cache: &SellCardCache, params: &HeuristicParams, rng: &mut impl Rng) {
    let drafted = state.players[player_index].drafted_cards;
    if drafted.is_empty() {
        end_player_turn(state, rng);
        if matches!(state.phase, GamePhase::Draw) {
            if heuristic_draft {
                heuristic_rollout_draw_and_draft(state, params, rng);
            } else {
                rollout_draw_and_draft(state, rng);
            }
        }
        return;
    }

    // Epsilon: random fallback
    if rng.random_bool(params.rollout_epsilon) {
        handle_action_no_pending(state, player_index, heuristic_draft, params, rng);
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
        let priority = destruction_priority(card, &state.players[player_index], &cache, params);
        if priority > best_priority {
            best_priority = priority;
            best_id = Some(id);
        }
    }

    // If best priority is low, chance to just end turn (probability varies by round)
    let end_turn_max = params.rollout_end_turn_max_round.max(2) as f64;
    let t = ((state.round as f64 - 1.0) / (end_turn_max - 1.0)).clamp(0.0, 1.0);
    let end_turn_prob = params.rollout_end_turn_probability_early * (1.0 - t) + params.rollout_end_turn_probability_late * t;
    if best_priority <= params.rollout_end_turn_threshold && rng.random_bool(end_turn_prob) {
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
            let (mixes, mix_count) = two_step_heuristic_mix_seq(&player.color_wheel, count, &cache, params, rng);
            state.players[player_index].drafted_cards.remove(card_id);
            state.destroyed_pile.insert(card_id);
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
            }
        }
        Ability::Sell => {
            if let Some(sell_card_id) = cache.best_affordable_id {
                fused_buy(state, player_index, card_id, sell_card_id, rng);
            } else {
                destroy_drafted_card(state, card_id as u32, rng);
            }
        }
        _ => {
            destroy_drafted_card(state, card_id as u32, rng);
        }
    }
}

pub fn apply_heuristic_rollout_step<R: Rng>(state: &mut GameState, heuristic_draft: bool, params: &HeuristicParams, rng: &mut R) {
    // Draft phase
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if heuristic_draft {
            heuristic_draft_loop(state, params, rng);
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
                    handle_action_no_pending_heuristic(state, player_index, heuristic_draft, &cache, params, rng);
                }
                Some(Ability::Workshop { count }) => {
                    let count = *count;

                    // Epsilon: fall back to random
                    if rng.random_bool(params.rollout_epsilon) {
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
                        let score = workshop_card_score(card, &cache, params);
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
                    if rng.random_bool(params.rollout_epsilon) {
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
                        let score = destruction_priority(card, &state.players[player_index], &cache, params);
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
                    let (mixes, mix_count) = two_step_heuristic_mix_seq(&state.players[player_index].color_wheel, remaining_mixes, &cache, params, rng);
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
                    if rng.random_bool(params.rollout_epsilon) {
                        if let Some(sell_card_id) = pick_random_affordable_sell_card(
                            &state.players[player_index],
                            &state.sell_card_display,
                            rng,
                        ) {
                            resolve_select_sell_card(state, sell_card_id, rng);
                        } else {
                            if let GamePhase::Action { ref mut action_state } = state.phase {
                                action_state.ability_stack.pop();
                            }
                            process_ability_stack(state, rng);
                        }
                        return;
                    }

                    // Heuristic: pick best sell card
                    if let Some(sell_card_id) = cache.best_affordable_id {
                        resolve_select_sell_card(state, sell_card_id, rng);
                    } else {
                        if let GamePhase::Action { ref mut action_state } = state.phase {
                            action_state.ability_stack.pop();
                        }
                        process_ability_stack(state, rng);
                    }
                }
                Some(Ability::GainSecondary) => {
                    let color = pick_best_color(&SECONDARIES, &cache, params, rng);
                    resolve_gain_color(state, color, rng);
                }
                Some(Ability::GainPrimary) => {
                    let color = pick_best_color(&PRIMARIES, &cache, params, rng);
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
                    } else if rng.random_bool(params.rollout_epsilon) {
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
