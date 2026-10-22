use crate::action_phase::{
    ability_stack, ability_stack_mut, boats_to_sell, can_afford_buyer, can_buy_with_gold_coin,
    end_player_turn, process_ability_stack, resolve_gain_color, resolve_keep_card,
    resolve_lift_card, resolve_sell_to_buyer, resolve_spend_gold_coin, resolve_workshop_choice,
    sell_to_buyer, skip_workshop, trash_top_card,
};
use crate::colors::{
    can_supply_mix, is_primary, mix_result, perform_mix_unchecked, PRIMARIES, SECONDARIES,
    VALID_MIX_PAIRS,
};
use crate::draft_phase::player_pick;
use crate::buyers_phase::{draw_buyer, take_buyer};
use crate::scoring::{buyer_alignment, HeuristicParams};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use rand::RngExt;


// ── Buyers phase ──

enum BuyerClaim {
    Display(u32),
    Deck,
}

/// What `player_index` should claim, or None when both piles are dry.
///
/// With `params`, take the face-up card best aligned with what the player has
/// stored, falling back to the deck only when nothing is face up — except
/// `rollout_epsilon` of the time, which drops to the uniform policy. Without
/// them, pick uniformly among everything on offer, the deck included.
fn choose_buyer_claim<R: Rng>(
    state: &GameState,
    player_index: usize,
    params: Option<&HeuristicParams>,
    rng: &mut R,
) -> Option<BuyerClaim> {
    let display = &state.buyer_display;
    let deck_available = !state.buyer_deck.is_empty();
    let params = params.filter(|p| !rng.random_bool(p.rollout_epsilon));

    if let Some(params) = params {
        if display.is_empty() {
            return deck_available.then_some(BuyerClaim::Deck);
        }
        let player = &state.players[player_index];
        let mut best_id = display[0].instance_id;
        let mut best = f64::NEG_INFINITY;
        for instance in display.iter() {
            let score = buyer_alignment(player, instance.buyer, params);
            if score > best {
                best = score;
                best_id = instance.instance_id;
            }
        }
        return Some(BuyerClaim::Display(best_id));
    }

    let options = display.len() + usize::from(deck_available);
    if options == 0 {
        return None;
    }
    let pick = rng.random_range(0..options);
    Some(if pick < display.len() {
        BuyerClaim::Display(display[pick].instance_id)
    } else {
        BuyerClaim::Deck
    })
}

/// Play out an in-progress buyers phase, letting the phase machinery decide
/// who is next. Reached when a leaf lands mid-phase rather than at a turn
/// boundary, where the shortcut above does not apply.
fn rollout_resolve_buyers_phase<R: Rng>(
    state: &mut GameState,
    params: Option<&HeuristicParams>,
    rng: &mut R,
) {
    while let GamePhase::Buyers { ref buyers_state } = state.phase {
        let player_index = buyers_state.current_player_index;
        match choose_buyer_claim(state, player_index, params, rng) {
            Some(BuyerClaim::Display(id)) => take_buyer(state, id, rng),
            Some(BuyerClaim::Deck) => draw_buyer(state, rng),
            // `seat_first_claimant` will not seat anyone with both piles dry,
            // so this is unreachable; breaking keeps it from spinning if that
            // ever stops holding.
            None => break,
        }
    }
}

// ── Rollout draw+draft shortcut ──

/// Stand-in for a round's draw, buyers and draft phases. Every player keeps
/// the whole hand dealt to them: a draft only shares the dealt cards out, so
/// with nobody choosing, dealing straight to the players is the same thing.
/// `buyer_params` claims buyers by the heuristic instead of at random.
// ── Fused rollout step ──

/// The mix pairs the stock can supply, with bowls covering missing ones.
#[inline(always)]
fn suppliable_mix_pairs(wheel: &ColorWheel, bowls: u32) -> ([(Color, Color); 9], usize) {
    let mut pairs = [(Color::Red, Color::Red); 9];
    let mut count = 0usize;
    for &(a, b) in &VALID_MIX_PAIRS {
        if can_supply_mix(wheel, bowls, a, b) {
            pairs[count] = (a, b);
            count += 1;
        }
    }
    (pairs, count)
}

#[inline(always)]
fn apply_mixes(player: &mut PlayerState, mixes: &MixSequence) {
    for &(a, b) in mixes {
        perform_mix_unchecked(&mut player.color_wheel, &mut player.bowls, a, b);
    }
}

#[inline(always)]
fn random_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    bowls: u32,
    remaining: u32,
    rng: &mut R,
) -> MixSequence {
    let mut mixes = MixSequence::new();
    let mut sim_wheel = wheel.clone();
    let mut sim_bowls = bowls;
    for _ in 0..remaining.min(MAX_MIX_SEQUENCE as u32) {
        let (pairs, pair_count) = suppliable_mix_pairs(&sim_wheel, sim_bowls);
        if pair_count == 0 {
            break;
        }
        let target = rng.random_range(0..pair_count + 1);
        if target == pair_count {
            break;
        }
        let (a, b) = pairs[target];
        mixes.push((a, b));
        perform_mix_unchecked(&mut sim_wheel, &mut sim_bowls, a, b);
    }
    mixes
}

#[inline(always)]
fn pick_random_affordable_buyer<R: Rng>(player: &PlayerState, rng: &mut R) -> Option<u32> {
    let mut affordable = [0u32; MAX_BUYERS];
    let mut count = 0usize;
    for buyer in player.buyers.iter() {
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


/// Legal gold_coin purchases for the player, for the rollout policies.
fn legal_gold_coin_purchases(player: &PlayerState) -> ([GoldCoinPurchase; 3], usize) {
    let mut out = [GoldCoinPurchase::Workshop; 3];
    let mut len = 0;
    for purchase in ALL_GOLD_COIN_PURCHASES {
        if can_buy_with_gold_coin(player, purchase) {
            out[len] = purchase;
            len += 1;
        }
    }
    (out, len)
}

#[inline(always)]
fn handle_action_no_pending(state: &mut GameState, player_index: usize, rng: &mut impl Rng) {
    // Spending a gold_coin is one of the things a turn can do, so a rollout that
    // never did it would leave the search blind to what gold_coins are worth.
    let (purchases, purchase_count) = legal_gold_coin_purchases(&state.players[player_index]);
    if purchase_count > 0 {
        let drafted = state.players[player_index].top_cards.len() as usize;
        let pick = rng.random_range(0..(purchase_count + drafted));
        if pick < purchase_count {
            resolve_spend_gold_coin(state, player_index, purchases[pick]);
            return;
        }
    }

    let drafted = state.players[player_index].top_cards;
    if drafted.is_empty() {
        // No top cards left — end turn, and the round with it if this
        // was the last player still acting
        end_player_turn(state, player_index, true, rng);
        return;
    }

    // Every card in the pool is decided, so the policy picks a card and then
    // whether to use it or keep it.
    let card_id = drafted.pick_random(rng).expect("a card in the pool");
    if rng.random_bool(0.5) {
        resolve_keep_card(state, player_index, card_id as u32);
        return;
    }

    // Trash the chosen card, activating its abilities. A lone Mix or Sell is
    // fused inline as an optimization; everything else goes through
    // trash_top_card, which pushes onto the stack for resolution on the
    // next step.
    let card = state.card_lookup[card_id as usize];
    match card.top_actions() {
        &[Ability::MixColors { count }] => {
            let player = &mut state.players[player_index];
            let mixes = random_mix_seq(&player.color_wheel, player.bowls, count, rng);
            player.top_cards.remove(card_id);
            apply_mixes(player, &mixes);
            state.trash.insert(card_id);
        }
        [Ability::Sell] => {
            if let Some(buyer_id) =
                pick_random_affordable_buyer(&state.players[player_index], rng)
            {
                fused_buy(state, player_index, card_id, buyer_id);
            } else {
                trash_top_card(state, player_index, card_id as u32);
            }
        }
        _ => {
            trash_top_card(state, player_index, card_id as u32);
        }
    }
}

/// Fused buyer purchase (no ability stack involvement).
#[inline(always)]
fn fused_buy(state: &mut GameState, player_index: usize, card_id: u8, buyer_id: u32) {
    state.players[player_index].top_cards.remove(card_id);
    state.trash.insert(card_id);
    let player = &mut state.players[player_index];
    let buyer_index = player
        .buyers
        .iter()
        .position(|c| c.instance_id == buyer_id)
        .unwrap();
    sell_to_buyer(player, buyer_index);
}

pub fn apply_rollout_step<R: Rng>(
    state: &mut GameState,
    heuristic_draft: bool,
    params: &HeuristicParams,
    rng: &mut R,
) {
    // Fast path: finish the whole buyers phase in one step
    if matches!(&state.phase, GamePhase::Buyers { .. }) {
        rollout_resolve_buyers_phase(state, None, rng);
        return;
    }

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
            let player_index = action_state
                .first_unended()
                .expect("The round ends as the last turn does");
            random_action_step(state, player_index, rng);
        }
        _ => panic!("Cannot apply rollout step for current state"),
    }
}

/// One step of `player_index`'s action turn, chosen at random.
fn random_action_step<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    match ability_stack(state, player_index).last() {
        None => {
            handle_action_no_pending(state, player_index, rng);
        }
        Some(Ability::Workshop { count }) => {
            let count = *count;
            let mut copy = state.players[player_index].workshop_cards;
            let selected = copy.draw_up_to(count as u8, rng);
            if selected.is_empty() {
                skip_workshop(state, player_index);
            } else {
                resolve_workshop_choice(state, player_index, selected);
            }
        }
        Some(Ability::LiftCard) => {
            let target = state.players[player_index].workshop_cards.pick_random(rng);
            resolve_lift_card(state, player_index, target);
        }
        Some(Ability::MixColors { count }) => {
            let remaining_mixes = *count;
            let player = &mut state.players[player_index];
            let mixes =
                random_mix_seq(&player.color_wheel, player.bowls, remaining_mixes, rng);
            apply_mixes(player, &mixes);
            ability_stack_mut(state, player_index).pop();
            process_ability_stack(state, player_index);
        }
        Some(Ability::Sell) => {
            if let Some(buyer_id) =
                pick_random_affordable_buyer(&state.players[player_index], rng)
            {
                resolve_sell_to_buyer(state, player_index, buyer_id);
            } else {
                ability_stack_mut(state, player_index).pop();
                process_ability_stack(state, player_index);
            }
        }
        Some(Ability::GainSecondary) => {
            let color = SECONDARIES[rng.random_range(0..SECONDARIES.len())];
            resolve_gain_color(state, player_index, color);
        }
        // Instant abilities should never be on top waiting — they get processed immediately
        Some(_) => panic!("Unexpected ability on stack top during rollout"),
    }
}

/// Finish `player_index`'s action turn by a rollout policy.
///
/// For playing out an opponent's turn while the searching player is still
/// taking theirs, so this is never the turn that ends the round.
pub(crate) fn play_out_action_turn<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    heuristic: bool,
    params: &HeuristicParams,
    rng: &mut R,
) {
    while matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player_index].ended) {
        if heuristic {
            heuristic_action_step(state, player_index, params, rng);
        } else {
            random_action_step(state, player_index, rng);
        }
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
        Ability::LiftCard => (3, 0),
        _ => (4, 0),
    }
}

/// Given a draft hand, pick the instance ID of the card to pass on: the most
/// redundant one (most other cards share its ability type). Among equally
/// redundant Workshop cards, it is the one with the lowest count.
#[inline(always)]
fn pick_card_to_pass<R: Rng>(
    hand: &UnorderedCards,
    card_lookup: &[Card; 256],
    params: &HeuristicParams,
    rng: &mut R,
) -> u8 {
    // Epsilon: random pass
    if rng.random_bool(params.rollout_epsilon) {
        return hand.pick_random(rng).unwrap();
    }

    // Collect card info: (instance_id, category, workshop_count)
    let mut cards: [(u8, u8, u32); 8] = [(0, 0, 0); 8];
    let mut count = 0usize;
    for id in hand.iter() {
        let card = card_lookup[id as usize];
        let (cat, wc) = ability_category(card.top_actions()[0]);
        cards[count] = (id, cat, wc);
        count += 1;
    }

    // Count how many cards share each category
    let mut cat_counts = [0u32; 5];
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

    // Among cards with max redundancy, pick the worst to pass
    let mut best_pass: Option<u8> = None;
    let mut best_pass_wc = u32::MAX; // for Workshop: lower = worse = pass first
    let mut candidates = 0u32;

    for i in 0..count {
        let redundancy = cat_counts[cards[i].1 as usize] - 1;
        if redundancy != max_redundancy {
            continue;
        }
        let (id, cat, wc) = cards[i];
        if cat == 0 {
            // Workshop: prefer passing the lowest workshop count
            if wc < best_pass_wc {
                best_pass_wc = wc;
                best_pass = Some(id);
                candidates = 1;
            } else if wc == best_pass_wc {
                candidates += 1;
                if rng.random_range(0..candidates) == 0 {
                    best_pass = Some(id);
                }
            }
        } else {
            // Non-workshop: pick randomly among equal redundancy
            candidates += 1;
            if best_pass.is_none() || rng.random_range(0..candidates) == 0 {
                best_pass = Some(id);
            }
        }
    }

    best_pass.unwrap()
}

/// Heuristic draft loop: each player passes on the card the heuristic rates
/// most redundant and takes another, until the draft ends.
fn heuristic_draft_loop<R: Rng>(state: &mut GameState, params: &HeuristicParams, rng: &mut R) {
    while let GamePhase::Draft { ref draft_state } = state.phase {
        let hand = draft_state.hands[draft_state.current_player_index];
        let pass = if hand.len() > 1 {
            Some(pick_card_to_pass(&hand, &state.card_lookup, params, rng))
        } else {
            None
        };
        let Some(card_id) = hand.iter().find(|&id| Some(id) != pass) else {
            break;
        };
        player_pick(state, card_id as u32, rng);
    }
}

// ── Buyer card cache for heuristic rollout performance ──

#[derive(Clone, Copy)]
struct BuyerCacheEntry {
    points: u32,
    /// Units of each material, indexed by `MaterialType`.
    materials: [u8; 3],
    colors_met: bool,
    have_colors: u32,
    total_colors: u32,
}

struct BuyerCache {
    flat_demand: [u32; 12],
    proximity_demand: [u32; 12],
    entries: [BuyerCacheEntry; MAX_BUYERS],
    len: usize,
}

impl BuyerCache {
    #[inline(always)]
    fn new(player: &PlayerState) -> Self {
        let buyers = &player.buyers;
        let wheel = &player.color_wheel;
        let mut flat_demand = [0u32; 12];
        let mut proximity_demand = [0u32; 12];
        let len = buyers.len();
        const EMPTY_ENTRY: BuyerCacheEntry = BuyerCacheEntry { points: 0, materials: [0; 3], colors_met: false, have_colors: 0, total_colors: 0 };
        let mut entries: [BuyerCacheEntry; MAX_BUYERS] = [EMPTY_ENTRY; MAX_BUYERS];

        for (i, sc) in buyers.iter().enumerate() {
            let stats = sc.buyer.stats();
            let points = stats.points;
            let total_colors = stats.total_colors as u32;
            let has_material = boats_to_sell(player, sc.buyer) <= player.boats;

            let mut have = 0u32;
            let mut primaries_short = 0u32;
            for &(c, needed) in stats.colors() {
                let stored = wheel.get(c).min(needed as u32);
                have += stored;
                if is_primary(c) {
                    primaries_short += needed as u32 - stored;
                }
            }
            have += primaries_short.min(player.bowls);
            let missing = total_colors - have;

            let colors_met = missing == 0;

            // Build flat and proximity demand — iterate cost directly (max 4 entries)
            let total_missing = missing + if has_material { 0 } else { 2 };
            let prox_weight = points * 10 / (total_missing + 1);
            for &(c, needed) in stats.colors() {
                let idx = c.index();
                flat_demand[idx] += points * needed as u32;
                proximity_demand[idx] += prox_weight * needed as u32;
            }

            entries[i] = BuyerCacheEntry {
                points,
                materials: stats.material_units,
                colors_met,
                have_colors: have,
                total_colors,
            };
        }

        BuyerCache {
            flat_demand,
            proximity_demand,
            entries,
            len,
        }
    }

    #[inline(always)]
    fn color_demand(&self, color: Color) -> u32 {
        self.proximity_demand[color.index()]
    }

    /// How much one stored unit of a material type is worth, summed over the
    /// buyers on display that require it.
    #[inline(always)]
    fn material_demand(&self, material: MaterialType, params: &HeuristicParams) -> u32 {
        let mut score = 0u32;
        for i in 0..self.len {
            let entry = &self.entries[i];
            if entry.materials[material as usize] > 0 {
                let base = entry.points * params.rollout_ws_material_base_multiplier;
                if entry.colors_met {
                    score += base * params.rollout_ws_material_colors_met_multiplier;
                } else {
                    score += base + base * entry.have_colors / (entry.total_colors + 1);
                }
            }
        }
        score
    }

}

/// The buyer the player can sell to now for the most points, the first in slot
/// order on a tie. The Sell and trash-priority decisions read only this, so
/// they skip the demand tables [`BuyerCache`] builds.
#[derive(Clone, Copy)]
struct BestSale {
    points: u32,
    id: Option<u32>,
    /// Whether any buyer on the board is affordable at all, which is a
    /// different question from which one pays best: a gold_coin buys the first,
    /// the Sell priority reads the second.
    affordable: bool,
}

impl BestSale {
    #[inline(always)]
    fn of(player: &PlayerState) -> Self {
        let mut best = BestSale {
            points: 0,
            id: None,
            affordable: false,
        };
        for sc in player.buyers.iter() {
            let points = sc.buyer.stats().points;
            if points > best.points {
                if can_afford_buyer(player, &sc.buyer) {
                    best = BestSale {
                        points,
                        id: Some(sc.instance_id),
                        affordable: true,
                    };
                }
            } else if !best.affordable && can_afford_buyer(player, &sc.buyer) {
                best.affordable = true;
            }
        }
        best
    }
}

/// What scoring every card in one hand would otherwise work out again for
/// each of them.
///
/// The player does not change while a hand is scored, so the best sale, the
/// mix pairs the stock can supply, and whether the workshop is empty are the
/// same for every card. The first two are worked out on first use, so a hand
/// with no Buyer card never looks at the buyers and one with no Mix card never
/// scans the mix pairs.
struct PriorityMemo {
    sale: Option<BestSale>,
    mix_pairs: Option<u32>,
    workshop_empty: bool,
}

impl PriorityMemo {
    #[inline(always)]
    fn new(player: &PlayerState) -> Self {
        PriorityMemo {
            sale: None,
            mix_pairs: None,
            workshop_empty: player.workshop_cards.is_empty(),
        }
    }

    #[inline(always)]
    fn sale_points(&mut self, player: &PlayerState) -> u32 {
        self.sale.get_or_insert_with(|| BestSale::of(player)).points
    }

    #[inline(always)]
    fn any_affordable_sale(&mut self, player: &PlayerState) -> bool {
        self.sale
            .get_or_insert_with(|| BestSale::of(player))
            .affordable
    }

    #[inline(always)]
    fn mix_pairs(&mut self, player: &PlayerState) -> u32 {
        *self.mix_pairs.get_or_insert_with(|| {
            suppliable_mix_pairs(&player.color_wheel, player.bowls).1 as u32
        })
    }
}

/// Score how valuable it is to trash a card, i.e. how useful its abilities
/// are to activate right now.
#[inline(always)]
fn destruction_priority(
    card: Card,
    player: &PlayerState,
    memo: &mut PriorityMemo,
    params: &HeuristicParams,
) -> u32 {
    card.top_actions()
        .iter()
        .map(|&ability| ability_priority(ability, player, memo, params))
        .sum()
}

#[inline(always)]
fn ability_priority(
    ability: Ability,
    player: &PlayerState,
    memo: &mut PriorityMemo,
    params: &HeuristicParams,
) -> u32 {
    match ability {
        Ability::Sell => {
            let points = memo.sale_points(player);
            if points > 0 {
                points * params.rollout_sell_affordable_multiplier
            } else {
                params.rollout_sell_base
            }
        }
        Ability::MixColors { count } => {
            let valid_pairs = memo.mix_pairs(player);
            if valid_pairs == 0 {
                params.rollout_mix_no_pairs
            } else {
                params.rollout_mix_base + valid_pairs.min(4) * params.rollout_mix_pair_weight + count.min(2) * params.rollout_mix_count_weight
            }
        }
        Ability::Workshop { count } => {
            if memo.workshop_empty {
                params.rollout_workshop_empty
            } else {
                params.rollout_workshop_base + count.min(3) * params.rollout_workshop_count_weight
            }
        }
        Ability::LiftCard => {
            if memo.workshop_empty {
                params.rollout_trash_no_targets
            } else {
                params.rollout_trash_with_targets
            }
        }
        // Every trash ability in the game is a Workshop, Mix, Sell or Move
        // to Draft Pool, all handled above. A future one that is not gets no
        // priority rather than a panic mid-rollout.
        _ => 0,
    }
}

/// Score an action card's workshop abilities based on game state.
#[inline(always)]
fn action_workshop_value(
    card: Card,
    player: &PlayerState,
    cache: &BuyerCache,
    params: &HeuristicParams,
) -> u32 {
    let mult = params.rollout_ws_action_color_demand_multiplier;
    for &ability in card.workshop_abilities() {
        match ability {
            Ability::GainGoldCoins { count } => {
                return count * params.rollout_ws_action_gain_gold_coins_value;
            }
            Ability::GainSecondary => {
                let mut best = 0u32;
                for &c in &SECONDARIES {
                    best = best.max(cache.color_demand(c));
                }
                return best * mult / 10;
            }
            Ability::MixColors { count } => {
                // Score based on best mix output demands for valid pairs
                let mut best_total = 0u32;
                for &(a, b) in &VALID_MIX_PAIRS {
                    if can_supply_mix(&player.color_wheel, player.bowls, a, b) {
                        let output = mix_result(a, b);
                        best_total = best_total.max(cache.color_demand(output) * count);
                    }
                }
                return best_total * mult / 10;
            }
            // Bowls and boats are half a point each, plus whatever the
            // primary or material they can stand in for is worth.
            Ability::GainBowls { count } => {
                let mut best = 0u32;
                for &c in &PRIMARIES {
                    best = best.max(cache.color_demand(c));
                }
                return count * (params.rollout_ws_action_gain_gold_coins_value / 2 + best * mult / 10);
            }
            Ability::GainBoats { count } => {
                let mut best = 0u32;
                for &m in &ALL_MATERIAL_TYPES {
                    best = best.max(cache.material_demand(m, params));
                }
                return count * (params.rollout_ws_action_gain_gold_coins_value / 2 + best);
            }
            _ => {}
        }
    }
    0
}

/// Score a workshop card for selection priority.
#[inline(always)]
fn workshop_card_score(
    card: Card,
    player: &PlayerState,
    cache: &BuyerCache,
    params: &HeuristicParams,
) -> u32 {
    let mut score = 0u32;

    // Material cards: score by how much their material type is needed
    for &mt in card.material_types() {
        score += cache.material_demand(mt, params);
    }

    // Color cards: score by how much their colors are needed
    for &color in card.colors() {
        score += cache.color_demand(color);
    }

    // Action cards: score based on their specific workshop abilities
    if card.is_special() {
        score += action_workshop_value(card, player, cache, params);
    }

    score
}

/// Pick the best color from a slice, preferring colors needed for buyers.
#[inline(always)]
fn pick_best_color<R: Rng>(
    colors: &[Color],
    cache: &BuyerCache,
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

/// The suppliable mix whose output the player's buyers want most, if any want
/// one at all.
#[inline(always)]
fn most_demanded_mix(wheel: &ColorWheel, bowls: u32, cache: &BuyerCache) -> Option<(Color, Color)> {
    let mut best_pair = None;
    let mut best_score = 0u32;
    for &(a, b) in &VALID_MIX_PAIRS {
        if can_supply_mix(wheel, bowls, a, b) {
            let score = cache.color_demand(mix_result(a, b));
            if score > best_score {
                best_score = score;
                best_pair = Some((a, b));
            }
        }
    }
    best_pair
}

/// Heuristic mix sequence: prefer mixes whose output is useful for buyers.
#[inline(always)]
fn heuristic_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    bowls: u32,
    remaining: u32,
    cache: &BuyerCache,
    params: &HeuristicParams,
    rng: &mut R,
) -> MixSequence {
    if rng.random_bool(params.rollout_epsilon) {
        return random_mix_seq(wheel, bowls, remaining, rng);
    }

    let mut mixes = MixSequence::new();
    let mut sim_wheel = wheel.clone();
    let mut sim_bowls = bowls;
    for _ in 0..remaining.min(MAX_MIX_SEQUENCE as u32) {
        let (pairs, pair_count) = suppliable_mix_pairs(&sim_wheel, sim_bowls);
        if pair_count == 0 {
            break;
        }
        let (a, b) = match most_demanded_mix(&sim_wheel, sim_bowls, cache) {
            Some(pair) => pair,
            // If no mix is useful, skip mixing (50% chance) or do random
            None => {
                if rng.random_bool(0.5) {
                    break;
                }
                pairs[rng.random_range(0..pair_count)]
            }
        };
        mixes.push((a, b));
        perform_mix_unchecked(&mut sim_wheel, &mut sim_bowls, a, b);
    }
    mixes
}

/// Two-step mix lookahead: evaluate all pair combinations when 2 or more mixes
/// are available, then spend any mixes beyond the second greedily.
#[inline(always)]
fn two_step_heuristic_mix_seq<R: Rng>(
    wheel: &ColorWheel,
    bowls: u32,
    remaining: u32,
    cache: &BuyerCache,
    params: &HeuristicParams,
    rng: &mut R,
) -> MixSequence {
    if remaining < 2 {
        return heuristic_mix_seq(wheel, bowls, remaining, cache, params, rng);
    }

    if rng.random_bool(params.rollout_epsilon) {
        return random_mix_seq(wheel, bowls, remaining, rng);
    }

    let (pairs1, pair1_count) = suppliable_mix_pairs(wheel, bowls);
    if pair1_count == 0 {
        return MixSequence::new();
    }

    let mut best_score = 0u32;
    let mut best_combo = MixSequence::new();

    for p1 in 0..pair1_count {
        let (a1, b1) = pairs1[p1];
        let output1 = mix_result(a1, b1);
        let score1 = cache.color_demand(output1);

        // Consider doing only the first mix
        if score1 > best_score {
            best_score = score1;
            best_combo = MixSequence::from_slice(&[(a1, b1)]);
        }

        // Simulate first mix and evaluate second using flat demand (avoids recomputing proximity)
        let mut sim_wheel = wheel.clone();
        let mut sim_bowls = bowls;
        perform_mix_unchecked(&mut sim_wheel, &mut sim_bowls, a1, b1);

        for &(a2, b2) in &VALID_MIX_PAIRS {
            if can_supply_mix(&sim_wheel, sim_bowls, a2, b2) {
                let output2 = mix_result(a2, b2);
                let score2 = cache.flat_demand[output2.index()];
                let total = score1 + score2;
                if total > best_score {
                    best_score = total;
                    best_combo = MixSequence::from_slice(&[(a1, b1), (a2, b2)]);
                }
            }
        }
    }

    // If no useful mixes, 50% chance to skip or fall back to random
    if best_score == 0 {
        if rng.random_bool(0.5) {
            return MixSequence::new();
        }
        return random_mix_seq(wheel, bowls, remaining, rng);
    }

    if best_combo.len() == 2 && remaining > 2 {
        let mut sim_wheel = wheel.clone();
        let mut sim_bowls = bowls;
        for &(a, b) in &best_combo {
            perform_mix_unchecked(&mut sim_wheel, &mut sim_bowls, a, b);
        }
        while (best_combo.len() as u32) < remaining.min(MAX_MIX_SEQUENCE as u32) {
            let Some((a, b)) = most_demanded_mix(&sim_wheel, sim_bowls, cache) else {
                break;
            };
            best_combo.push((a, b));
            perform_mix_unchecked(&mut sim_wheel, &mut sim_bowls, a, b);
        }
    }

    best_combo
}

#[inline(always)]
fn handle_action_no_pending_heuristic(
    state: &mut GameState,
    player_index: usize,
    params: &HeuristicParams,
    rng: &mut impl Rng,
) {
    let drafted = state.players[player_index].top_cards;
    if drafted.is_empty() {
        end_player_turn(state, player_index, true, rng);
        return;
    }

    // Epsilon: random fallback
    if rng.random_bool(params.rollout_epsilon) {
        handle_action_no_pending(state, player_index, rng);
        return;
    }

    // Buy a sale only when the player cannot already trigger one for free. A
    // bought sale spends the same material and colours a drafted Buyer card
    // would have, so paying for it while holding one just burns a gold_coin.
    //
    // "Can a gold_coin buy a sale here?" is the same walk of the buyers the Sell
    // priority below needs, so it is made once and both read it.
    let mut memo = PriorityMemo::new(&state.players[player_index]);
    if state.players[player_index].gold_coins > 0
        && memo.any_affordable_sale(&state.players[player_index])
        && !drafted.iter().any(|id| {
            state.card_lookup[id as usize]
                .top_actions()
                .contains(&Ability::Sell)
        })
    {
        resolve_spend_gold_coin(state, player_index, GoldCoinPurchase::Sell);
        return;
    }

    // Score each top card and pick the best to trash
    let mut best_id: Option<u8> = None;
    let mut best_priority = 0u32;
    let mut seen: u64 = 0;
    for id in drafted.iter() {
        let card = state.card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit != 0 { continue; }
        seen |= bit;
        let priority = destruction_priority(card, &state.players[player_index], &mut memo, params);
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
        // What "stop here" used to be is "keep this one and decide the rest".
        if let Some(id) = best_id {
            resolve_keep_card(state, player_index, id as u32);
            return;
        }
    }

    // As in the random policy, a lone Mix or Sell is fused inline. A card the
    // priority table scored at nothing still has to be decided.
    let card_id = best_id.or_else(|| drafted.lowest_bit()).expect("a card in the pool");
    let card = state.card_lookup[card_id as usize];
    match card.top_actions() {
        &[Ability::MixColors { count }] => {
            let cache = BuyerCache::new(&state.players[player_index]);
            let player = &mut state.players[player_index];
            let mixes = two_step_heuristic_mix_seq(&player.color_wheel, player.bowls, count, &cache, params, rng);
            player.top_cards.remove(card_id);
            apply_mixes(player, &mixes);
            state.trash.insert(card_id);
        }
        [Ability::Sell] => {
            let sale = memo
                .sale
                .unwrap_or_else(|| BestSale::of(&state.players[player_index]));
            if let Some(buyer_id) = sale.id {
                fused_buy(state, player_index, card_id, buyer_id);
            } else {
                trash_top_card(state, player_index, card_id as u32);
            }
        }
        _ => {
            trash_top_card(state, player_index, card_id as u32);
        }
    }
}

pub fn apply_heuristic_rollout_step<R: Rng>(
    state: &mut GameState,
    heuristic_draft: bool,
    params: &HeuristicParams,
    rng: &mut R,
) {
    // Buyers phase
    if matches!(&state.phase, GamePhase::Buyers { .. }) {
        rollout_resolve_buyers_phase(state, Some(params), rng);
        return;
    }

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
            let player_index = action_state
                .first_unended()
                .expect("The round ends as the last turn does");
            heuristic_action_step(state, player_index, params, rng);
        }
        _ => panic!("Cannot apply heuristic rollout step for current state"),
    }
}

/// One step of `player_index`'s action turn, chosen by the heuristic policy.
fn heuristic_action_step<R: Rng>(
    state: &mut GameState,
    player_index: usize,
    params: &HeuristicParams,
    rng: &mut R,
) {
    // Each branch works out only what it reads, the best sale or the demand
    // tables, and only once it is past the returns that read neither.
    match ability_stack(state, player_index).last() {
        None => {
            handle_action_no_pending_heuristic(state, player_index, params, rng);
        }
        Some(Ability::Workshop { count }) => {
            let count = *count;

            // Epsilon: fall back to random
            if rng.random_bool(params.rollout_epsilon) {
                let mut copy = state.players[player_index].workshop_cards;
                let selected = copy.draw_up_to(count as u8, rng);
                if selected.is_empty() {
                    skip_workshop(state, player_index);
                } else {
                    resolve_workshop_choice(state, player_index, selected);
                }
                return;
            }

            // Score each workshop card and pick the top N
            let cache = BuyerCache::new(&state.players[player_index]);
            let workshop = state.players[player_index].workshop_cards;
            let mut scored: [(u8, u32); 16] = [(0, 0); 16];
            let mut scored_count = 0usize;
            for id in workshop.iter() {
                let card = state.card_lookup[id as usize];
                let score = workshop_card_score(card, &state.players[player_index], &cache, params);
                scored[scored_count] = (id, score);
                scored_count += 1;
            }

            if scored_count == 0 {
                skip_workshop(state, player_index);
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
            resolve_workshop_choice(state, player_index, selected);
        }
        Some(Ability::LiftCard) => {
            let workshop = state.players[player_index].workshop_cards;
            if workshop.is_empty() {
                resolve_lift_card(state, player_index, None);
                return;
            }

            // Epsilon: random
            if rng.random_bool(params.rollout_epsilon) {
                resolve_lift_card(state, player_index, workshop.pick_random(rng));
                return;
            }

            // Move the card whose ability is most worth triggering. The
            // move no longer triggers it — the rollout has to trash
            // the card from the pool afterwards — but it is still the
            // card most worth having there.
            let mut memo = PriorityMemo::new(&state.players[player_index]);
            let mut best_id: Option<u8> = None;
            let mut best_score = 0u32;
            for id in workshop.iter() {
                let card = state.card_lookup[id as usize];
                let score = destruction_priority(card, &state.players[player_index], &mut memo, params);
                if score > best_score || best_id.is_none() {
                    best_score = score;
                    best_id = Some(id);
                }
            }

            resolve_lift_card(state, player_index, best_id);
        }
        Some(Ability::MixColors { count }) => {
            let remaining_mixes = *count;
            let cache = BuyerCache::new(&state.players[player_index]);
            let player = &mut state.players[player_index];
            let mixes = two_step_heuristic_mix_seq(&player.color_wheel, player.bowls, remaining_mixes, &cache, params, rng);
            apply_mixes(player, &mixes);
            ability_stack_mut(state, player_index).pop();
            process_ability_stack(state, player_index);
        }
        Some(Ability::Sell) => {
            // Epsilon: random
            if rng.random_bool(params.rollout_epsilon) {
                if let Some(buyer_id) =
                    pick_random_affordable_buyer(&state.players[player_index], rng)
                {
                    resolve_sell_to_buyer(state, player_index, buyer_id);
                } else {
                    ability_stack_mut(state, player_index).pop();
                    process_ability_stack(state, player_index);
                }
                return;
            }

            // Heuristic: pick best buyer
            if let Some(buyer_id) = BestSale::of(&state.players[player_index]).id {
                resolve_sell_to_buyer(state, player_index, buyer_id);
            } else {
                ability_stack_mut(state, player_index).pop();
                process_ability_stack(state, player_index);
            }
        }
        Some(Ability::GainSecondary) => {
            let color = pick_best_color(&SECONDARIES, &BuyerCache::new(&state.players[player_index]), params, rng);
            resolve_gain_color(state, player_index, color);
        }
        Some(_) => panic!("Unexpected ability on stack top during rollout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_choice::apply_choice;
    use crate::fixed_vec::FixedVec;
    use crate::setup::create_initial_game_state;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn greedy_params() -> HeuristicParams {
        const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
        let mut params: HeuristicParams = serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse");
        params.rollout_epsilon = 0.0;
        params
    }

    /// Round 1 of a two-player game, reached by random play, with seat 1's
    /// turn over so that the round ends with seat 0's.
    fn seat_zero_ends_round_one(seed: u64) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        while !matches!(state.phase, GamePhase::Action { .. }) {
            let player = match &state.phase {
                GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
                GamePhase::Draft { draft_state } => draft_state.current_player_index,
                _ => unreachable!("round 1 reaches its action phase"),
            };
            let choices = crate::choices::enumerate_choices(&state, player);
            let choice = choices[rng.random_range(0..choices.len())].clone();
            apply_choice(&mut state, player, &choice, &mut rng);
        }
        end_player_turn(&mut state, 1, true, &mut rng);
        state
    }

    /// The claim seat 1's first heuristic step should make: the best-aligned
    /// face-up card, from the display as round 1 left it.
    fn expected_claim(state: &GameState, params: &HeuristicParams) -> u32 {
        let seat_one = &state.players[1];
        let mut expected = None;
        let mut best = f64::NEG_INFINITY;
        for instance in state.buyer_display.iter() {
            let score = buyer_alignment(seat_one, instance.buyer, params);
            if score > best {
                best = score;
                expected = Some(instance.instance_id);
            }
        }
        expected.expect("round 1 leaves the display stocked")
    }

    /// Ends the round with a heuristic step, then claims with the next, and
    /// checks round 2 opened with the claim it should.
    fn assert_round_two_opens_with_heuristic_claim(mut state: GameState, params: &HeuristicParams) {
        let expected = expected_claim(&state, params);
        apply_heuristic_rollout_step(&mut state, false, params, &mut WyRand::seed_from_u64(0));
        assert_eq!(state.round, 2, "seat 0's step should have ended the round");
        assert!(
            matches!(state.phase, GamePhase::Buyers { .. }),
            "opening round 2 lands in its buyers phase"
        );
        apply_heuristic_rollout_step(&mut state, false, params, &mut WyRand::seed_from_u64(0));
        assert!(
            state.players[1].buyers.iter().any(|b| b.instance_id == expected),
            "seat 1 should have claimed the best-aligned face-up card"
        );
    }

    #[test]
    fn heuristic_rollouts_claim_by_alignment_after_a_turn_runs_out_of_cards() {
        let params = greedy_params();
        for seed in 0..20 {
            let mut state = seat_zero_ends_round_one(seed);
            state.players[0].top_cards = UnorderedCards::new();
            assert_round_two_opens_with_heuristic_claim(state, &params);
        }
    }

    /// A turn that keeps everything still ends — one card at a time — and the
    /// round rolls over into an aligned claim.
    #[test]
    fn heuristic_rollouts_claim_by_alignment_after_a_turn_keeps_its_cards() {
        let mut params = greedy_params();
        // Every top card falls under the threshold, so the heuristic keeps
        // each one rather than using it.
        params.rollout_end_turn_threshold = u32::MAX;
        params.rollout_end_turn_probability_early = 1.0;
        for seed in 0..20 {
            let mut state = seat_zero_ends_round_one(seed);
            // With no buyers there is no sale to buy, which is tried first.
            state.players[0].buyers = FixedVec::new();
            let drafted = state.players[0].top_cards;
            assert!(!drafted.is_empty());
            let expected = expected_claim(&state, &params);

            // Keeping a card never ends the turn while another is still undecided.
            for _ in 0..drafted.len() {
                apply_heuristic_rollout_step(&mut state, false, &params, &mut WyRand::seed_from_u64(0));
                assert_eq!(state.round, 1, "the turn cannot end with cards left to decide");
            }
            apply_heuristic_rollout_step(&mut state, false, &params, &mut WyRand::seed_from_u64(0));
            assert_eq!(state.round, 2, "the turn ends once every card is decided");
            apply_heuristic_rollout_step(&mut state, false, &params, &mut WyRand::seed_from_u64(0));
            assert!(
                state.players[1].buyers.iter().any(|b| b.instance_id == expected),
                "seat 1 should have claimed the best-aligned face-up card"
            );
        }
    }
}
