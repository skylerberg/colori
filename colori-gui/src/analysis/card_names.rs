use colori_core::cards::draft_deck_cards;
use colori_core::types::{Ability, SellCard, Card};

/// Get human-readable display name for a card.
pub fn card_display_name(card: Card) -> &'static str {
    card.name()
}

/// Get human-readable display name for a sell card.
/// Format: "Potter [Amber]" or "Painter [Teal, Green]"
pub fn sell_card_display_name(sell_card: SellCard) -> String {
    let colors = sell_card
        .color_cost()
        .iter()
        .map(|c| format!("{:?}", c))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} [{}]", sell_card.buyer().name(), colors)
}

/// Get draft copies by card name string (used in normalization).
/// For cards not in the draft deck, returns 1 as default.
pub fn get_draft_copies_by_name(name: &str) -> u32 {
    draft_deck_cards()
        .find(|(card, _)| card.name() == name)
        .map_or(1, |(_, copies)| copies as u32)
}

/// Format an ability as human-readable text.
/// Examples: "Workshop x3", "Mix x2", "Sell"
pub fn format_ability(ability: &Ability) -> String {
    match ability {
        Ability::Workshop { count } => format!("Workshop x{}", count),
        Ability::MixColors { count } => format!("Mix x{}", count),
        Ability::MoveToDraftPool => "Move to Draft Pool".to_string(),
        Ability::Sell => "Sell".to_string(),
        Ability::GainDucats { count } => format!("Gain {} Ducats", count),
        Ability::GainSecondary => "Gain Secondary".to_string(),
        Ability::GainWildPrimaries { count: 1 } => "Gain 1 Wild Primary".to_string(),
        Ability::GainWildPrimaries { count } => format!("Gain {} Wild Primaries", count),
        Ability::GainPrimaries => "Gain One of Each Primary".to_string(),
        Ability::GainBoats { count } => format!("Gain {} Boat(s)", count),
    }
}

/// A card's trash abilities in the order they resolve.
pub fn format_abilities(abilities: &[Ability]) -> String {
    abilities.iter().map(format_ability).collect::<Vec<_>>().join(", then ")
}
