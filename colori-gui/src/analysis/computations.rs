use std::collections::{BTreeMap, HashMap, HashSet};

use colori_core::game_log::{FinalScore, PlayerVariant, StructuredGameLog};
use colori_core::types::{SellCard, SellCardInstance, CardInstance, Choice, GlassCard, MaterialType, ALL_COLORS};

use super::card_names::{sell_card_display_name, card_display_name, get_draft_copies_by_name};
use super::categories::CardCategory;

// ── Types ──

/// Maps log index (position in the slice) to filtered player indices.
pub type PlayerFilter = HashMap<usize, HashSet<usize>>;

pub struct WinRateEntry {
    pub wins: f64,
    pub games: f64,
}

pub struct CategoryStat {
    pub label: String,
    pub raw_total: f64,
    pub total_copies: u32,
    pub normalized_rate: f64,
}

pub struct WinRateCategoryStat {
    pub label: String,
    pub wins: f64,
    pub games: f64,
}

pub struct DeckSizeStats {
    pub mean: f64,
    pub median: f64,
    pub min: u32,
    pub max: u32,
}

#[allow(dead_code)]
pub struct GameLengthStats {
    pub avg_rounds: f64,
    pub avg_choices: f64,
}

#[allow(dead_code)]
pub struct DurationStats {
    pub avg_ms: f64,
    pub median_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
}

pub struct SellCardAcquisitions {
    pub by_sell_card: HashMap<String, usize>,
    pub by_ducats: HashMap<u32, usize>,
    pub by_material: HashMap<String, usize>,
}

#[allow(dead_code)]
pub struct WinnerSellCardBreakdown {
    pub avg_textiles: f64,
    pub avg_ceramics: f64,
    pub avg_paintings: f64,
    pub avg_ducats: f64,
    pub num_games: usize,
}

// ── Helper functions ──

/// Build a map from card instance ID to CardInstance from a log's initial state.
pub fn build_card_instance_map(log: &StructuredGameLog) -> HashMap<u32, CardInstance> {
    let mut map = HashMap::new();
    let state = &log.initial_state;

    let add_cards = |map: &mut HashMap<u32, CardInstance>, cards: &[CardInstance]| {
        for c in cards {
            map.insert(c.instance_id, *c);
        }
    };

    for p in &state.players {
        add_cards(&mut map, &p.deck);
        add_cards(&mut map, &p.discard);
        add_cards(&mut map, &p.workshopped_cards);
        add_cards(&mut map, &p.workshop_cards);
        add_cards(&mut map, &p.drafted_cards);
    }
    add_cards(&mut map, &state.draft_deck);
    add_cards(&mut map, &state.destroyed_pile);

    map
}

/// Build a map from sell card instance ID to SellCardInstance from a log's initial state.
pub fn build_sell_card_instance_map(log: &StructuredGameLog) -> HashMap<u32, SellCardInstance> {
    let mut map = HashMap::new();
    let state = &log.initial_state;

    let add_sell_cards = |map: &mut HashMap<u32, SellCardInstance>, sell_cards: &[SellCardInstance]| {
        for b in sell_cards {
            map.insert(b.instance_id, *b);
        }
    };

    for p in &state.players {
        add_sell_cards(&mut map, &p.completed_sell_cards);
    }
    add_sell_cards(&mut map, &state.sell_card_deck);
    add_sell_cards(&mut map, &state.sell_card_display);

    map
}

/// Get the display name for a card.
pub fn card_name_from_instance(card: colori_core::types::Card) -> String {
    card_display_name(card).to_string()
}

/// Get the display name for a sell card.
pub fn sell_card_name_from_instance(sell_card: SellCard) -> String {
    sell_card_display_name(sell_card)
}

/// Look up a card instance ID and return its display name.
#[allow(dead_code)]
pub fn get_card_name(card_map: &HashMap<u32, CardInstance>, id: u32) -> String {
    match card_map.get(&id) {
        Some(inst) => card_name_from_instance(inst.card),
        None => format!("Card #{}", id),
    }
}

/// Look up a sell card instance ID and return its display name.
#[allow(dead_code)]
pub fn get_sell_card_name(sell_card_map: &HashMap<u32, SellCardInstance>, id: u32) -> String {
    match sell_card_map.get(&id) {
        Some(inst) => sell_card_name_from_instance(inst.sell_card),
        None => format!("Sell Card #{}", id),
    }
}

pub fn final_score_ranking(fs: &FinalScore) -> (u32, u32, u32) {
    (fs.score, fs.completed_sell_cards, fs.color_wheel_total)
}

fn compute_winners(final_scores: &[FinalScore]) -> (Box<dyn Fn(&str) -> bool + '_>, usize) {
    if final_scores.len() == 1 {
        // Solo mode: win = score >= 16
        let won = final_scores[0].score >= 16;
        let num_winners = if won { 1 } else { 0 };
        return (Box::new(move |_name: &str| won), num_winners);
    }
    let best = final_scores.iter().map(|fs| final_score_ranking(fs)).max().unwrap_or((0, 0, 0));
    let num_winners = final_scores.iter().filter(|fs| final_score_ranking(fs) == best).count();
    let is_winner = move |name: &str| {
        final_scores.iter().any(|fs| fs.name == name && final_score_ranking(fs) == best)
    };
    (Box::new(is_winner), num_winners)
}

// ── Player filtering ──

/// Compute a player filter that selects only players matching a given variant label.
pub fn compute_player_filter(
    logs: &[StructuredGameLog],
    variant_label: &str,
) -> PlayerFilter {
    let mut filter = PlayerFilter::new();
    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(ref variants) = log.player_variants {
            let mut indices = HashSet::new();
            for (i, variant) in variants.iter().enumerate() {
                if format_variant_label(variant, Some(variants)) == variant_label {
                    indices.insert(i);
                }
            }
            if !indices.is_empty() {
                filter.insert(log_idx, indices);
            }
        }
    }
    filter
}

fn format_iterations_short(iters: u32) -> String {
    if iters >= 1000 && iters % 1000 == 0 {
        format!("{}k", iters / 1000)
    } else {
        format!("{}", iters)
    }
}

/// Format a player variant as a human-readable label.
pub fn format_variant_label(
    variant: &PlayerVariant,
    all_variants: Option<&[PlayerVariant]>,
) -> String {
    if let Some(ref name) = variant.name {
        return name.clone();
    }

    let mut differing_algorithm = false;
    let mut differing_iterations = false;
    let mut differing_exploration = false;
    let mut differing_rollout = false;
    if let Some(all) = all_variants {
        if all.len() > 1 {
            let first = &all[0];
            differing_algorithm = all.iter().any(|v| v.algorithm != first.algorithm);
            differing_iterations = all.iter().any(|v| v.iterations != first.iterations);
            differing_exploration = all
                .iter()
                .any(|v| v.exploration_constant != first.exploration_constant);
            differing_rollout = all
                .iter()
                .any(|v| v.max_rollout_steps != first.max_rollout_steps);
        }
    }

    let mut parts = Vec::new();

    if differing_algorithm {
        parts.push(variant.algorithm.clone().unwrap_or_else(|| "ucb".to_string()));
    }
    if differing_iterations || (!differing_algorithm && !differing_exploration && !differing_rollout) {
        parts.push(format_iterations_short(variant.iterations));
    }
    if differing_exploration {
        let c = variant.exploration_constant.unwrap_or(std::f64::consts::SQRT_2);
        parts.push(format!("c={:.2}", c));
    }
    if differing_rollout {
        let rollout = variant.max_rollout_steps.unwrap_or(1000);
        parts.push(format!("rollout={}", rollout));
    }
    parts.join(", ")
}

// ── Format choice ──

/// Format a choice as a human-readable string.
pub fn format_choice(choice: &Choice) -> String {
    let card_name = |card: &colori_core::types::Card| card_name_from_instance(*card);
    let sell_card_name = |sell_card: &SellCard| sell_card_name_from_instance(*sell_card);
    let card_names = |cards: &[colori_core::types::Card]| {
        cards
            .iter()
            .map(|c| card_name(c))
            .collect::<Vec<_>>()
            .join(", ")
    };

    match choice {
        Choice::DraftPick { card } => {
            format!("Drafted {}", card_name(card))
        }
        Choice::DraftPickAbility { ability } => {
            format!("Drafted by ability {:?}", ability)
        }
        Choice::DestroyDraftedCard { card } => {
            format!("Destroyed {} from draft", card_name(card))
        }
        Choice::EndTurn => "Ended turn".to_string(),
        Choice::Workshop { card_types } => {
            if card_types.is_empty() {
                "Workshopped nothing".to_string()
            } else {
                format!("Workshopped {}", card_names(card_types))
            }
        }
        Choice::SkipWorkshop => "Skipped workshop".to_string(),
        Choice::DestroyDrawnCards { card } => match card {
            Some(c) => format!("Destroyed {} from workshop", card_name(c)),
            None => "Destroyed nothing from workshop".to_string(),
        }
        Choice::SelectSellCard { sell_card } => {
            format!("Sold to {}", sell_card_name(sell_card))
        }
        Choice::GainSecondary { color } => {
            format!("Gained {:?} (secondary)", color)
        }
        Choice::GainPrimary { color } => {
            format!("Gained {:?} (primary)", color)
        }
        Choice::MixAll { mixes } => {
            if mixes.is_empty() {
                "Skipped mixing".to_string()
            } else {
                let mix_strs: Vec<String> =
                    mixes.iter().map(|(a, b)| format!("{:?}+{:?}", a, b)).collect();
                format!("Mixed {}", mix_strs.join(", "))
            }
        }
        Choice::SwapTertiary { lose, gain } => {
            format!("Swapped {:?} for {:?}", lose, gain)
        }
        Choice::DestroyAndMix {
            card,
            mixes,
        } => {
            if mixes.is_empty() {
                format!(
                    "Destroyed {} and skipped mixing",
                    card_name(card)
                )
            } else {
                let mix_strs: Vec<String> =
                    mixes.iter().map(|(a, b)| format!("{:?}+{:?}", a, b)).collect();
                format!(
                    "Destroyed {} and mixed {}",
                    card_name(card),
                    mix_strs.join(", ")
                )
            }
        }
        Choice::DestroyAndSell {
            card,
            sell_card,
        } => {
            format!(
                "Destroyed {} and sold to {}",
                card_name(card),
                sell_card_name(sell_card)
            )
        }
        Choice::DestroyAndWorkshop {
            card,
            workshop_cards,
        } => {
            if workshop_cards.is_empty() {
                format!("Destroyed {} and skipped workshop", card_name(card))
            } else {
                format!(
                    "Destroyed {} and workshopped {}",
                    card_name(card),
                    card_names(workshop_cards)
                )
            }
        }
        Choice::DestroyAndDestroyCards {
            card,
            target,
        } => {
            match target {
                Some(t) => format!(
                    "Destroyed {} and destroyed {} from workshop",
                    card_name(card),
                    card_name(t)
                ),
                None => format!(
                    "Destroyed {} and destroyed nothing from workshop",
                    card_name(card)
                ),
            }
        }
        Choice::SelectGlass { glass, pay_color } => {
            format!("Acquired {:?} (paid 4 {:?})", glass, pay_color)
        }
        Choice::ActivateGlassWorkshop => "Activated Glass Workshop".to_string(),
        Choice::ActivateGlassDraw => "Activated Glass Draw".to_string(),
        Choice::ActivateGlassMix => "Activated Glass Mix".to_string(),
        Choice::ActivateGlassGainPrimary => "Activated Glass Gain Primary".to_string(),
        Choice::ActivateGlassExchange { lose, gain } => {
            format!("Activated Glass Exchange: {:?} for {:?}", lose, gain)
        }
        Choice::ActivateGlassMoveDrafted { card } => {
            format!("Activated Glass Move Drafted: {}", card_name(card))
        }
        Choice::ActivateGlassUnmix { color } => {
            format!("Activated Glass Unmix: {:?}", color)
        }
        Choice::ActivateGlassTertiaryDucat { color } => {
            format!("Activated Glass Tertiary Ducat: {:?}", color)
        }
        Choice::ActivateGlassReworkshop { card } => {
            format!("Activated Glass Reworkshop: {}", card_name(card))
        }
        Choice::ActivateGlassDestroyClean { card } => {
            format!("Activated Glass Destroy: {}", card_name(card))
        }
        Choice::DestroyAndSelectGlass { card, glass, pay_color } => {
            format!("Destroyed {} and acquired {:?} (paid 4 {:?})", card_name(card), glass, pay_color)
        }
        Choice::WorkshopWithReworkshop { reworkshop_card, other_cards } => {
            if other_cards.is_empty() {
                format!("Workshopped {} x2 (Glass Reworkshop)", card_name(reworkshop_card))
            } else {
                format!(
                    "Workshopped {} x2, {} (Glass Reworkshop)",
                    card_name(reworkshop_card),
                    card_names(other_cards)
                )
            }
        }
        Choice::SelectMoveToDrafted { card } => {
            format!("Moved {} to drafted", card_name(card))
        }
        Choice::SkipMoveToDrafted => "Skipped move to drafted".to_string(),
    }
}

// ── Action analysis ──

/// Count the frequency of each choice type across all log entries.
pub fn compute_action_distribution(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            let type_name = choice_type_name(&entry.choice);
            *counts.entry(type_name).or_insert(0) += 1;
        }
    }
    counts
}

fn choice_type_name(choice: &Choice) -> String {
    match choice {
        Choice::DraftPick { .. } => "draftPick".to_string(),
        Choice::DraftPickAbility { .. } => "draftPickAbility".to_string(),
        Choice::DestroyDraftedCard { .. } => "destroyDraftedCard".to_string(),
        Choice::EndTurn => "endTurn".to_string(),
        Choice::Workshop { .. } => "workshop".to_string(),
        Choice::SkipWorkshop => "skipWorkshop".to_string(),
        Choice::DestroyDrawnCards { .. } => "destroyDrawnCards".to_string(),
        Choice::SelectSellCard { .. } => "selectSellCard".to_string(),
        Choice::GainSecondary { .. } => "gainSecondary".to_string(),
        Choice::GainPrimary { .. } => "gainPrimary".to_string(),
        Choice::MixAll { .. } => "mixAll".to_string(),
        Choice::SwapTertiary { .. } => "swapTertiary".to_string(),
        Choice::DestroyAndMix { .. } => "destroyAndMix".to_string(),
        Choice::DestroyAndSell { .. } => "destroyAndSell".to_string(),
        Choice::DestroyAndWorkshop { .. } => "destroyAndWorkshop".to_string(),
        Choice::DestroyAndDestroyCards { .. } => "destroyAndDestroyCards".to_string(),
        Choice::SelectGlass { .. } => "selectGlass".to_string(),
        Choice::ActivateGlassWorkshop => "activateGlassWorkshop".to_string(),
        Choice::ActivateGlassDraw => "activateGlassDraw".to_string(),
        Choice::ActivateGlassMix => "activateGlassMix".to_string(),
        Choice::ActivateGlassGainPrimary => "activateGlassGainPrimary".to_string(),
        Choice::ActivateGlassExchange { .. } => "activateGlassExchange".to_string(),
        Choice::ActivateGlassMoveDrafted { .. } => "activateGlassMoveDrafted".to_string(),
        Choice::ActivateGlassUnmix { .. } => "activateGlassUnmix".to_string(),
        Choice::ActivateGlassTertiaryDucat { .. } => "activateGlassTertiaryDucat".to_string(),
        Choice::ActivateGlassReworkshop { .. } => "activateGlassReworkshop".to_string(),
        Choice::ActivateGlassDestroyClean { .. } => "activateGlassDestroyClean".to_string(),
        Choice::DestroyAndSelectGlass { .. } => "destroyAndSelectGlass".to_string(),
        Choice::WorkshopWithReworkshop { .. } => "workshopWithReworkshop".to_string(),
        Choice::SelectMoveToDrafted { .. } => "selectMoveToDrafted".to_string(),
        Choice::SkipMoveToDrafted => "skipMoveToDrafted".to_string(),
    }
}

// ── Draft analysis ──

/// Count how many times each card was drafted (by display name).
pub fn compute_draft_frequency(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            if let Choice::DraftPick { card } = &entry.choice {
                let name = card_name_from_instance(*card);
                *counts.entry(name).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Count cards that were drafted and not subsequently destroyed (added to final deck).
pub fn compute_cards_added_to_deck(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        // Track per-player drafted and destroyed card names with counts
        let mut player_drafted: HashMap<usize, HashMap<String, usize>> = HashMap::new();
        let mut player_destroyed: HashMap<usize, HashMap<String, usize>> = HashMap::new();

        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            let pi = entry.player_index;
            match &entry.choice {
                Choice::DraftPick { card } => {
                    let name = card_name_from_instance(*card);
                    *player_drafted.entry(pi).or_default().entry(name).or_insert(0) += 1;
                }
                Choice::DestroyDraftedCard { card }
                | Choice::DestroyAndMix { card, .. }
                | Choice::DestroyAndSell { card, .. }
                | Choice::DestroyAndWorkshop { card, .. }
                | Choice::DestroyAndDestroyCards { card, .. }
                | Choice::DestroyAndSelectGlass { card, .. } => {
                    let name = card_name_from_instance(*card);
                    *player_destroyed.entry(pi).or_default().entry(name).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        for (_pi, drafted) in &player_drafted {
            let destroyed = player_destroyed.get(_pi);
            for (name, &drafted_count) in drafted {
                let destroyed_count = destroyed
                    .and_then(|d| d.get(name))
                    .copied()
                    .unwrap_or(0);
                let kept = drafted_count.saturating_sub(destroyed_count);
                if kept > 0 {
                    *counts.entry(name.clone()).or_insert(0) += kept;
                }
            }
        }
    }
    counts
}

/// Normalize raw counts by draft copies for each card name.
pub fn normalize_by_draft_copies(counts: &HashMap<String, usize>) -> HashMap<String, f64> {
    let mut normalized = HashMap::new();
    for (name, &count) in counts {
        let copies = get_draft_copies_by_name(name);
        normalized.insert(name.clone(), count as f64 / copies as f64);
    }
    normalized
}

/// Compute aggregate stats per category.
pub fn compute_category_stats(
    counts: &HashMap<String, usize>,
    categories: &[CardCategory],
) -> Vec<CategoryStat> {
    categories
        .iter()
        .map(|cat| {
            let mut raw_total = 0.0;
            for name in &cat.card_names {
                raw_total += *counts.get(*name).unwrap_or(&0) as f64;
            }
            let normalized_rate = if cat.total_copies > 0 {
                raw_total / cat.total_copies as f64
            } else {
                0.0
            };
            CategoryStat {
                label: cat.label.to_string(),
                raw_total,
                total_copies: cat.total_copies,
                normalized_rate,
            }
        })
        .collect()
}

// ── Destroy analysis ──

/// Count cards destroyed from the draft phase (destroyDraftedCard, destroyAndMixAll, destroyAndSell).
pub fn compute_destroyed_from_draft(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            let card = match &entry.choice {
                Choice::DestroyDraftedCard { card }
                | Choice::DestroyAndMix { card, .. }
                | Choice::DestroyAndSell { card, .. }
                | Choice::DestroyAndWorkshop { card, .. }
                | Choice::DestroyAndDestroyCards { card, .. }
                | Choice::DestroyAndSelectGlass { card, .. } => Some(card),
                _ => None,
            };
            if let Some(card) = card {
                let name = card_name_from_instance(*card);
                *counts.entry(name).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Count cards destroyed from workshop (destroyDrawnCards).
pub fn compute_destroyed_from_workshop(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            if let Choice::DestroyDrawnCards { card: Some(card) } = &entry.choice {
                let name = card_name_from_instance(*card);
                *counts.entry(name).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Compute destroy rate: destroyed / drafted for each card name.
pub fn compute_destroy_rate(
    destroyed: &HashMap<String, usize>,
    drafted: &HashMap<String, usize>,
) -> HashMap<String, f64> {
    let mut rates = HashMap::new();
    for (name, &destroyed_count) in destroyed {
        let drafted_count = *drafted.get(name).unwrap_or(&0);
        let rate = if drafted_count > 0 {
            destroyed_count as f64 / drafted_count as f64
        } else {
            0.0
        };
        rates.insert(name.clone(), rate);
    }
    rates
}

// ── Win rate analysis ──

/// Compute win rate by card name (cards in final deck).
pub fn compute_win_rate_by_card(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, WinRateEntry> {
    let mut stats: HashMap<String, WinRateEntry> = HashMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };
        let allowed = filter.and_then(|f| f.get(&log_idx));

        // Track per-player drafted and destroyed card names with counts
        let mut player_drafted: HashMap<usize, HashMap<String, usize>> = HashMap::new();
        let mut player_destroyed: HashMap<usize, HashMap<String, usize>> = HashMap::new();

        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            let pi = entry.player_index;
            match &entry.choice {
                Choice::DraftPick { card } => {
                    let name = card_name_from_instance(*card);
                    *player_drafted.entry(pi).or_default().entry(name).or_insert(0) += 1;
                }
                Choice::DestroyDraftedCard { card }
                | Choice::DestroyAndMix { card, .. }
                | Choice::DestroyAndSell { card, .. }
                | Choice::DestroyAndWorkshop { card, .. }
                | Choice::DestroyAndDestroyCards { card, .. }
                | Choice::DestroyAndSelectGlass { card, .. } => {
                    let name = card_name_from_instance(*card);
                    *player_destroyed.entry(pi).or_default().entry(name).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        // Compute winners
        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        // For each player, determine final deck cards and tally
        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let drafted = player_drafted.get(&i);
            let destroyed = player_destroyed.get(&i);
            let player_name = &log.player_names[i];
            let is_winner = is_winner_fn(player_name);

            let mut deck_card_names: HashSet<String> = HashSet::new();
            if let Some(drafted) = drafted {
                for (name, &drafted_count) in drafted {
                    let destroyed_count = destroyed
                        .and_then(|d| d.get(name))
                        .copied()
                        .unwrap_or(0);
                    if drafted_count > destroyed_count {
                        deck_card_names.insert(name.clone());
                    }
                }
            }

            for name in &deck_card_names {
                let entry = stats
                    .entry(name.clone())
                    .or_insert(WinRateEntry { wins: 0.0, games: 0.0 });
                entry.games += 1.0;
                if is_winner {
                    entry.wins += 1.0 / num_winners as f64;
                }
            }
        }
    }

    stats
}

/// Compute win rate by card name for cards that were drafted (regardless of whether they were destroyed).
pub fn compute_win_rate_if_drafted(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, WinRateEntry> {
    let mut stats: HashMap<String, WinRateEntry> = HashMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };
        let allowed = filter.and_then(|f| f.get(&log_idx));

        // Track per-player drafted card names (no destruction filtering)
        let mut player_drafted: HashMap<usize, HashSet<String>> = HashMap::new();

        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            if let Choice::DraftPick { card } = &entry.choice {
                let name = card_name_from_instance(*card);
                player_drafted
                    .entry(entry.player_index)
                    .or_default()
                    .insert(name);
            }
        }

        // Compute winners
        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        // For each player, tally win rate by drafted card names
        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let drafted_names = player_drafted.get(&i);
            let player_name = &log.player_names[i];
            let is_winner = is_winner_fn(player_name);

            if let Some(drafted_names) = drafted_names {
                for name in drafted_names {
                    let entry = stats
                        .entry(name.clone())
                        .or_insert(WinRateEntry { wins: 0.0, games: 0.0 });
                    entry.games += 1.0;
                    if is_winner {
                        entry.wins += 1.0 / num_winners as f64;
                    }
                }
            }
        }
    }

    stats
}

/// Compute win rate by player position (seat index).
pub fn compute_win_rate_by_position(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<usize, WinRateEntry> {
    let mut stats: HashMap<usize, WinRateEntry> = HashMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };
        let allowed = filter.and_then(|f| f.get(&log_idx));
        let num_players = final_scores.len();

        // Initialize positions
        for i in 0..num_players {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            stats
                .entry(i)
                .or_insert(WinRateEntry { wins: 0.0, games: 0.0 })
                .games += 1.0;
        }

        // Find winners using tiebreaker ranking
        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        // Match final scores back to player indices by name
        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let player_name = &log.player_names[i];
            let is_winner = is_winner_fn(player_name);
            if is_winner {
                if let Some(entry) = stats.get_mut(&i) {
                    entry.wins += 1.0 / num_winners as f64;
                }
            }
        }
    }

    stats
}

/// Compute win rate by variant label. Returns None if no logs have player variants.
pub fn compute_win_rate_by_variant(
    logs: &[StructuredGameLog],
) -> Option<HashMap<String, WinRateEntry>> {
    let has_variants = logs.iter().any(|log| log.player_variants.is_some());
    if !has_variants {
        return None;
    }

    let mut stats: HashMap<String, WinRateEntry> = HashMap::new();

    for log in logs {
        let variants = match &log.player_variants {
            Some(v) => v,
            None => continue,
        };
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };

        // Find winners using tiebreaker ranking
        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        for (i, variant) in variants.iter().enumerate() {
            let label = format_variant_label(variant, Some(variants));
            let entry = stats
                .entry(label)
                .or_insert(WinRateEntry { wins: 0.0, games: 0.0 });
            entry.games += 1.0;

            if i < log.player_names.len() {
                let player_name = &log.player_names[i];
                if is_winner_fn(player_name) {
                    entry.wins += 1.0 / num_winners as f64;
                }
            }
        }
    }

    if stats.is_empty() {
        None
    } else {
        Some(stats)
    }
}

/// Compute win rate by variant, grouped by game length (round count).
/// Returns a map from round count to (variant label -> win rate entry).
pub fn compute_variant_win_rate_by_game_length(
    logs: &[StructuredGameLog],
) -> Option<BTreeMap<u32, HashMap<String, WinRateEntry>>> {
    let has_variants = logs.iter().any(|log| log.player_variants.is_some());
    if !has_variants {
        return None;
    }

    let mut stats: BTreeMap<u32, HashMap<String, WinRateEntry>> = BTreeMap::new();

    for log in logs {
        let variants = match &log.player_variants {
            Some(v) => v,
            None => continue,
        };
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };

        let mut max_round: u32 = 0;
        for entry in &log.entries {
            if entry.round > max_round {
                max_round = entry.round;
            }
        }
        if max_round == 0 {
            continue;
        }

        let (is_winner_fn, num_winners) = compute_winners(final_scores);
        let round_stats = stats.entry(max_round).or_default();

        for (i, variant) in variants.iter().enumerate() {
            let label = format_variant_label(variant, Some(variants));
            let entry = round_stats
                .entry(label)
                .or_insert(WinRateEntry { wins: 0.0, games: 0.0 });
            entry.games += 1.0;

            if i < log.player_names.len() {
                let player_name = &log.player_names[i];
                if is_winner_fn(player_name) {
                    entry.wins += 1.0 / num_winners as f64;
                }
            }
        }
    }

    if stats.is_empty() {
        None
    } else {
        Some(stats)
    }
}

/// Skill vs chance statistics derived from calibrated Elo ratings.
/// Based on Duersch, Lambrecht, and Oechssler (2018).
pub struct SkillChanceStats {
    /// Optimal K-factor that minimizes prediction error.
    pub optimal_k: f64,
    /// Standard deviation of final Elo ratings (higher = more skill).
    pub elo_std_dev: f64,
    /// Win probability of a player +1 std dev above opponent.
    pub p_sd: f64,
    /// Games needed for better player to most likely be ahead (>50% wins with >75% prob).
    pub repetitions: u64,
    /// Final Elo ratings per variant (centered at 1500).
    pub elos: HashMap<String, f64>,
}

/// Run Elo with a given k-factor. Returns (ratings, mean_squared_error, total_pairs).
/// Ratings start at 0 (paper convention). MSE measures prediction accuracy.
fn compute_elo_with_mse(
    logs: &[StructuredGameLog],
    k: f64,
) -> (HashMap<String, f64>, f64) {
    let mut elos: HashMap<String, f64> = HashMap::new();
    let mut total_sq_error = 0.0;
    let mut total_pairs: usize = 0;

    for log in logs {
        let variants = match &log.player_variants {
            Some(v) => v,
            None => continue,
        };
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };

        let n = variants.len();
        if n < 2 {
            continue;
        }

        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        let labels: Vec<String> = variants
            .iter()
            .map(|v| format_variant_label(v, Some(variants)))
            .collect();

        let current_elos: Vec<f64> = labels
            .iter()
            .map(|l| *elos.get(l).unwrap_or(&0.0))
            .collect();

        let actual_scores: Vec<f64> = (0..n)
            .map(|i| {
                if i < log.player_names.len() && is_winner_fn(&log.player_names[i]) {
                    1.0 / num_winners as f64
                } else {
                    0.0
                }
            })
            .collect();

        let k_adj = k / (n - 1) as f64;

        for i in 0..n {
            let mut delta = 0.0;
            for j in 0..n {
                if i == j {
                    continue;
                }
                let expected =
                    1.0 / (1.0 + 10.0_f64.powf((current_elos[j] - current_elos[i]) / 400.0));
                let actual = if actual_scores[i] > actual_scores[j] {
                    1.0
                } else if (actual_scores[i] - actual_scores[j]).abs() < f64::EPSILON {
                    0.5
                } else {
                    0.0
                };
                delta += k_adj * (actual - expected);
                total_sq_error += (actual - expected) * (actual - expected);
                total_pairs += 1;
            }
            let rating = elos.entry(labels[i].clone()).or_insert(0.0);
            *rating += delta;
        }
    }

    let mse = if total_pairs > 0 {
        total_sq_error / total_pairs as f64
    } else {
        0.5 // baseline: predicting 0.5 for everything
    };
    (elos, mse)
}

/// Calibrate K-factor via grid search (Duersch et al. Appendix 6.2).
/// Finds k* that minimizes mean squared prediction error.
fn calibrate_k(logs: &[StructuredGameLog]) -> f64 {
    let mut grid = vec![0.0, 20.0, 40.0, 60.0, 80.0];

    for _ in 0..25 {
        let losses: Vec<f64> = grid.iter().map(|&k| compute_elo_with_mse(logs, k).1).collect();

        let best_idx = losses
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let best_k = grid[best_idx];
        let step = if grid.len() >= 2 {
            (grid[1] - grid[0]) / 2.0
        } else {
            break;
        };

        if step < 0.01 {
            return best_k;
        }

        // Check convergence: (L(k+) - L(k*)) + (L(k-) - L(k*)) relative to L(0) - L(k*)
        let loss_best = losses[best_idx];
        let loss_at_zero = compute_elo_with_mse(logs, 0.0).1;
        let denom = loss_at_zero - loss_best;
        if denom > 0.0 {
            let loss_above = if best_idx + 1 < losses.len() {
                losses[best_idx + 1]
            } else {
                loss_best
            };
            let loss_below = if best_idx > 0 {
                losses[best_idx - 1]
            } else {
                loss_best
            };
            let precision =
                ((loss_above - loss_best) + (loss_below - loss_best)) / denom;
            if precision < 1e-6 {
                return best_k;
            }
        }

        // Halve grid around best
        grid = vec![
            best_k - 2.0 * step,
            best_k - step,
            best_k,
            best_k + step,
            best_k + 2.0 * step,
        ];
        grid.retain(|&k| k >= 0.0);
    }

    grid[grid.len() / 2]
}

/// Compute skill vs chance statistics using calibrated Elo ratings.
pub fn compute_skill_chance_stats(
    logs: &[StructuredGameLog],
) -> Option<SkillChanceStats> {
    let has_variants = logs.iter().any(|log| log.player_variants.is_some());
    if !has_variants {
        return None;
    }

    let optimal_k = calibrate_k(logs);
    let (elos, _mse) = compute_elo_with_mse(logs, optimal_k);

    if elos.is_empty() {
        return None;
    }

    // Compute standard deviation of ratings
    let values: Vec<f64> = elos.values().copied().collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64;
    let elo_std_dev = variance.sqrt();

    // Win probability at +1 standard deviation: p^sd = 1 / (1 + 10^(-σ/400))
    let p_sd = 1.0 / (1.0 + 10.0_f64.powf(-elo_std_dev / 400.0));

    // Games needed for skill to show (normal approximation)
    // P(X > n/2) > 0.75 where X ~ Binomial(n, p_sd)
    // n > z^2 * p*(1-p) / (p - 0.5)^2, z = 0.6745 for 75%
    let repetitions = if p_sd > 0.5 {
        let z = 0.6745_f64;
        let n = z * z * p_sd * (1.0 - p_sd) / ((p_sd - 0.5) * (p_sd - 0.5));
        (n.ceil() as u64).max(1)
    } else {
        u64::MAX
    };

    // Shift ratings to center at 1500 for display
    let display_elos: HashMap<String, f64> = elos
        .into_iter()
        .map(|(label, rating)| (label, rating - mean + 1500.0))
        .collect();

    Some(SkillChanceStats {
        optimal_k,
        elo_std_dev,
        p_sd,
        repetitions,
        elos: display_elos,
    })
}

/// Compute aggregate win rate stats per category.
pub fn compute_win_rate_category_stats(
    win_rates: &HashMap<String, WinRateEntry>,
    categories: &[CardCategory],
) -> Vec<WinRateCategoryStat> {
    categories
        .iter()
        .map(|cat| {
            let mut wins = 0.0;
            let mut games = 0.0;
            for name in &cat.card_names {
                if let Some(entry) = win_rates.get(*name) {
                    wins += entry.wins;
                    games += entry.games;
                }
            }
            WinRateCategoryStat {
                label: cat.label.to_string(),
                wins,
                games,
            }
        })
        .collect()
}

// ── Score / game stats ──

/// Compute score distribution across all players.
pub fn compute_score_distribution(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> BTreeMap<u32, usize> {
    let mut counts = BTreeMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(ref final_scores) = log.final_scores {
            let allowed = filter.and_then(|f| f.get(&log_idx));
            for i in 0..log.player_names.len() {
                if let Some(allowed) = allowed {
                    if !allowed.contains(&i) {
                        continue;
                    }
                }
                let player_name = &log.player_names[i];
                if let Some(score_entry) = final_scores.iter().find(|fs| &fs.name == player_name) {
                    *counts.entry(score_entry.score).or_insert(0) += 1;
                }
            }
        }
    }
    counts
}

/// Compute the distribution of game round counts.
pub fn compute_round_count_distribution(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> BTreeMap<u32, usize> {
    let mut counts = BTreeMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(f) = filter {
            if !f.contains_key(&log_idx) {
                continue;
            }
        }
        let mut max_round: u32 = 0;
        for entry in &log.entries {
            if entry.round > max_round {
                max_round = entry.round;
            }
        }
        if max_round > 0 {
            *counts.entry(max_round).or_insert(0) += 1;
        }
    }
    counts
}

/// Compute deck size statistics across all players at end of game.
pub fn compute_deck_size_stats(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> DeckSizeStats {
    let mut sizes: Vec<u32> = Vec::new();
    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(ref final_player_stats) = log.final_player_stats {
            let allowed = filter.and_then(|f| f.get(&log_idx));
            for i in 0..log.player_names.len() {
                if let Some(allowed) = allowed {
                    if !allowed.contains(&i) {
                        continue;
                    }
                }
                let player_name = &log.player_names[i];
                if let Some(ps) = final_player_stats.iter().find(|p| &p.name == player_name) {
                    sizes.push(ps.deck_size as u32);
                }
            }
        }
    }

    if sizes.is_empty() {
        return DeckSizeStats {
            mean: 0.0,
            median: 0.0,
            min: 0,
            max: 0,
        };
    }

    sizes.sort();
    let sum: u32 = sizes.iter().sum();
    let mean = sum as f64 / sizes.len() as f64;
    let mid = sizes.len() / 2;
    let median = if sizes.len() % 2 == 0 {
        (sizes[mid - 1] as f64 + sizes[mid] as f64) / 2.0
    } else {
        sizes[mid] as f64
    };

    DeckSizeStats {
        mean,
        median,
        min: sizes[0],
        max: *sizes.last().unwrap(),
    }
}

/// Compute average game length in rounds and choices.
pub fn compute_average_game_length(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> GameLengthStats {
    let mut total_rounds: u32 = 0;
    let mut total_choices: usize = 0;
    let mut game_count: usize = 0;

    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(f) = filter {
            if !f.contains_key(&log_idx) {
                continue;
            }
        }
        let mut max_round: u32 = 0;
        for entry in &log.entries {
            if entry.round > max_round {
                max_round = entry.round;
            }
        }
        total_rounds += max_round;
        total_choices += log.entries.len();
        game_count += 1;
    }

    if game_count == 0 {
        return GameLengthStats {
            avg_rounds: 0.0,
            avg_choices: 0.0,
        };
    }

    GameLengthStats {
        avg_rounds: total_rounds as f64 / game_count as f64,
        avg_choices: total_choices as f64 / game_count as f64,
    }
}

/// Compute duration statistics. Returns None if no logs have duration_ms.
pub fn compute_duration_stats(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> Option<DurationStats> {
    let mut durations: Vec<u64> = Vec::new();
    for (log_idx, log) in logs.iter().enumerate() {
        if let Some(f) = filter {
            if !f.contains_key(&log_idx) {
                continue;
            }
        }
        if let Some(ms) = log.duration_ms {
            durations.push(ms);
        }
    }

    if durations.is_empty() {
        return None;
    }

    durations.sort();
    let sum: u64 = durations.iter().sum();
    let avg_ms = sum as f64 / durations.len() as f64;
    let mid = durations.len() / 2;
    let median_ms = if durations.len() % 2 == 0 {
        (durations[mid - 1] as f64 + durations[mid] as f64) / 2.0
    } else {
        durations[mid] as f64
    };

    Some(DurationStats {
        avg_ms,
        median_ms,
        min_ms: durations[0],
        max_ms: *durations.last().unwrap(),
    })
}

/// Compute deck sizes at the end of the penultimate round.
pub fn compute_penultimate_round_deck_sizes(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> BTreeMap<u32, usize> {
    let mut counts = BTreeMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let mut max_round: u32 = 0;
        for entry in &log.entries {
            if entry.round > max_round {
                max_round = entry.round;
            }
        }
        if max_round < 2 {
            continue;
        }
        let penultimate_round = max_round - 1;
        let allowed = filter.and_then(|f| f.get(&log_idx));

        let mut player_deck_sizes: Vec<i32> = log
            .initial_state
            .players
            .iter()
            .map(|p| {
                (p.deck.len()
                    + p.discard.len()
                    + p.workshopped_cards.len()
                    + p.workshop_cards.len()
                    + p.drafted_cards.len()) as i32
            })
            .collect();

        for entry in &log.entries {
            if entry.round > penultimate_round {
                break;
            }
            let pi = entry.player_index;
            match &entry.choice {
                Choice::DraftPick { .. } => {
                    player_deck_sizes[pi] += 1;
                }
                Choice::DestroyDraftedCard { .. }
                | Choice::DestroyAndMix { .. }
                | Choice::DestroyAndSell { .. }
                | Choice::DestroyAndWorkshop { .. }
                | Choice::DestroyAndDestroyCards { .. }
                | Choice::DestroyAndSelectGlass { .. } => {
                    player_deck_sizes[pi] -= 1;
                }
                Choice::DestroyDrawnCards { card } => {
                    if card.is_some() {
                        player_deck_sizes[pi] -= 1;
                    }
                }
                _ => {}
            }
        }

        for (i, &size) in player_deck_sizes.iter().enumerate() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let size = size.max(0) as u32;
            *counts.entry(size).or_insert(0) += 1;
        }
    }

    counts
}

// ── Sell card analysis ──

/// Compute sell card acquisition counts by sell card name, star count, and material type.
pub fn compute_sell_card_acquisitions(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> SellCardAcquisitions {
    let mut by_sell_card = HashMap::new();
    let mut by_ducats = HashMap::new();
    let mut by_material = HashMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));

        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            let sell_card = match &entry.choice {
                Choice::SelectSellCard { sell_card } => Some(sell_card),
                Choice::DestroyAndSell { sell_card, .. } => Some(sell_card),
                _ => None,
            };
            if let Some(sell_card) = sell_card {
                let name = sell_card_name_from_instance(*sell_card);
                *by_sell_card.entry(name).or_insert(0) += 1;
                *by_ducats.entry(sell_card.ducats()).or_insert(0) += 1;
                let material_name = format!("{:?}", sell_card.required_material());
                *by_material.entry(material_name).or_insert(0) += 1;
            }
        }
    }

    SellCardAcquisitions {
        by_sell_card,
        by_ducats,
        by_material,
    }
}

// ── Color analysis ──

/// Compute average sell card breakdown (Textiles/Ceramics/Paintings) for game winners.
pub fn compute_winner_sell_card_breakdown(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> WinnerSellCardBreakdown {
    let mut total_textiles: f64 = 0.0;
    let mut total_ceramics: f64 = 0.0;
    let mut total_paintings: f64 = 0.0;
    let mut total_ducats: f64 = 0.0;
    let mut num_games: usize = 0;

    for (log_idx, log) in logs.iter().enumerate() {
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };

        let (is_winner_fn, num_winners) = compute_winners(final_scores);
        let weight = 1.0 / num_winners as f64;

        // Track sell card acquisitions per player
        let mut player_textiles: HashMap<usize, u32> = HashMap::new();
        let mut player_ceramics: HashMap<usize, u32> = HashMap::new();
        let mut player_paintings: HashMap<usize, u32> = HashMap::new();

        let allowed = filter.and_then(|f| f.get(&log_idx));

        for entry in &log.entries {
            let sell_card = match &entry.choice {
                Choice::SelectSellCard { sell_card } => Some(sell_card),
                Choice::DestroyAndSell { sell_card, .. } => Some(sell_card),
                _ => None,
            };
            if let Some(sell_card) = sell_card {
                match sell_card.required_material() {
                    MaterialType::Textiles => {
                        *player_textiles.entry(entry.player_index).or_insert(0) += 1;
                    }
                    MaterialType::Ceramics => {
                        *player_ceramics.entry(entry.player_index).or_insert(0) += 1;
                    }
                    MaterialType::Paintings => {
                        *player_paintings.entry(entry.player_index).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut found_filtered_winner = false;

        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let player_name = &log.player_names[i];
            if is_winner_fn(player_name) {
                found_filtered_winner = true;
                total_textiles += *player_textiles.get(&i).unwrap_or(&0) as f64 * weight;
                total_ceramics += *player_ceramics.get(&i).unwrap_or(&0) as f64 * weight;
                total_paintings += *player_paintings.get(&i).unwrap_or(&0) as f64 * weight;
                if let Some(ref final_player_stats) = log.final_player_stats {
                    if let Some(stats) = final_player_stats.iter().find(|p| &p.name == player_name) {
                        total_ducats += stats.ducats as f64 * weight;
                    }
                }
            }
        }

        if found_filtered_winner {
            num_games += 1;
        }
    }

    let divisor = if num_games > 0 { num_games as f64 } else { 1.0 };
    WinnerSellCardBreakdown {
        avg_textiles: total_textiles / divisor,
        avg_ceramics: total_ceramics / divisor,
        avg_paintings: total_paintings / divisor,
        avg_ducats: total_ducats / divisor,
        num_games,
    }
}

/// Compute average color wheel values across all players.
pub fn compute_color_wheel_stats(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, f64> {
    let mut totals: HashMap<String, f64> = HashMap::new();
    let mut player_count: usize = 0;

    for (log_idx, log) in logs.iter().enumerate() {
        let final_player_stats = match &log.final_player_stats {
            Some(fps) => fps,
            None => continue,
        };
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let player_name = &log.player_names[i];
            let ps = match final_player_stats.iter().find(|p| &p.name == player_name) {
                Some(ps) => ps,
                None => continue,
            };
            player_count += 1;
            for &color in &ALL_COLORS {
                let color_name = format!("{:?}", color);
                let count = ps.color_wheel.get(color);
                *totals.entry(color_name).or_insert(0.0) += count as f64;
            }
        }
    }

    let mut averages = HashMap::new();
    if player_count > 0 {
        for (color, total) in &totals {
            averages.insert(color.clone(), total / player_count as f64);
        }
    }

    averages
}

// ── Glass analysis ──

fn extract_glass(choice: &Choice) -> Option<GlassCard> {
    match choice {
        Choice::SelectGlass { glass, .. } | Choice::DestroyAndSelectGlass { glass, .. } => {
            Some(*glass)
        }
        _ => None,
    }
}

/// Count how many times each glass card was acquired.
pub fn compute_glass_acquisitions(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (log_idx, log) in logs.iter().enumerate() {
        let allowed = filter.and_then(|f| f.get(&log_idx));
        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            if let Some(glass) = extract_glass(&entry.choice) {
                *counts.entry(glass.name().to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Compute win rate for players who acquired each glass card.
pub fn compute_glass_win_rate(
    logs: &[StructuredGameLog],
    filter: Option<&PlayerFilter>,
) -> HashMap<String, WinRateEntry> {
    let mut stats: HashMap<String, WinRateEntry> = HashMap::new();

    for (log_idx, log) in logs.iter().enumerate() {
        let final_scores = match &log.final_scores {
            Some(fs) => fs,
            None => continue,
        };
        let allowed = filter.and_then(|f| f.get(&log_idx));

        // Track per-player glass card acquisitions (unique names)
        let mut player_glass: HashMap<usize, HashSet<String>> = HashMap::new();

        for entry in &log.entries {
            if let Some(allowed) = allowed {
                if !allowed.contains(&entry.player_index) {
                    continue;
                }
            }
            if let Some(glass) = extract_glass(&entry.choice) {
                player_glass
                    .entry(entry.player_index)
                    .or_default()
                    .insert(glass.name().to_string());
            }
        }

        let (is_winner_fn, num_winners) = compute_winners(final_scores);

        for i in 0..log.player_names.len() {
            if let Some(allowed) = allowed {
                if !allowed.contains(&i) {
                    continue;
                }
            }
            let glass_names = match player_glass.get(&i) {
                Some(names) => names,
                None => continue,
            };
            let player_name = &log.player_names[i];
            let is_winner = is_winner_fn(player_name);

            for name in glass_names {
                let entry = stats
                    .entry(name.clone())
                    .or_insert(WinRateEntry { wins: 0.0, games: 0.0 });
                entry.games += 1.0;
                if is_winner {
                    entry.wins += 1.0 / num_winners as f64;
                }
            }
        }
    }

    stats
}
