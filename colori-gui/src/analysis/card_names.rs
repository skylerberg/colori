use colori_core::cards::{
    action_cards, dye_cards, draft_material_cards, ACTION_COPIES, DYE_COPIES, MATERIAL_COPIES,
};
use colori_core::types::{Ability, BuyerCard, Card};

/// Get human-readable display name for a card.
pub fn card_display_name(card: Card) -> &'static str {
    card.name()
}

/// Get human-readable display name for a buyer card.
/// Format: "2-star Textiles [Vermilion]" or "3-star Ceramics [Amber, Red]"
pub fn buyer_display_name(buyer: BuyerCard) -> String {
    let colors = buyer
        .color_cost()
        .iter()
        .map(|c| format!("{:?}", c))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}-star {:?} [{}]",
        buyer.stars(),
        buyer.required_material(),
        colors
    )
}

/// Get the number of copies of a card in the draft deck.
#[allow(dead_code)]
pub fn get_draft_copies(card: Card) -> u32 {
    use colori_core::types::CardKind;
    match card.kind() {
        CardKind::Dye => DYE_COPIES as u32,
        CardKind::Action => ACTION_COPIES as u32,
        CardKind::Material => MATERIAL_COPIES as u32,
        CardKind::BasicDye => 0, // Not in draft
    }
}

/// Get draft copies by card name string (used in normalization).
/// For cards not in the draft deck, returns 1 as default.
pub fn get_draft_copies_by_name(name: &str) -> u32 {
    for card in dye_cards() {
        if card.name() == name {
            return DYE_COPIES as u32;
        }
    }
    for card in draft_material_cards() {
        if card.name() == name {
            return MATERIAL_COPIES as u32;
        }
    }
    for card in action_cards() {
        if card.name() == name {
            return ACTION_COPIES as u32;
        }
    }
    1 // default
}

/// Format an ability as human-readable text.
/// Examples: "Workshop x3", "Draw x2", "Mix x2", "Destroy x1", "Sell"
pub fn format_ability(ability: &Ability) -> String {
    match ability {
        Ability::Workshop { count } => format!("Workshop x{}", count),
        Ability::DrawCards { count } => format!("Draw x{}", count),
        Ability::MixColors { count } => format!("Mix x{}", count),
        Ability::DestroyCards => "Destroy x1".to_string(),
        Ability::Sell => "Sell".to_string(),
        Ability::GainDucats { count } => format!("Gain {} Ducats", count),
        Ability::GainSecondary => "Gain Secondary".to_string(),
        Ability::GainPrimary => "Gain Primary".to_string(),
        Ability::ChangeTertiary => "Change Tertiary".to_string(),
    }
}
