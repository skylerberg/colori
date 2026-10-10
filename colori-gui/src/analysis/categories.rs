use colori_core::cards::{
    action_cards, basic_dye_cards, chalk_card, draft_copies, draft_dye_cards, dual_material_cards,
    material_color_cards, starter_material_cards,
};
use colori_core::types::Card;

pub struct CardCategory {
    pub label: &'static str,
    pub card_names: Vec<&'static str>,
    pub total_copies: u32,
}

fn draft_category(label: &'static str, cards: impl IntoIterator<Item = Card>) -> CardCategory {
    let cards: Vec<Card> = cards.into_iter().collect();
    CardCategory {
        label,
        card_names: cards.iter().map(|c| c.name()).collect(),
        total_copies: cards.iter().map(|&c| draft_copies(c) as u32).sum(),
    }
}

fn starter_category(
    label: &'static str,
    cards: impl IntoIterator<Item = Card>,
    num_players: usize,
) -> CardCategory {
    let card_names: Vec<&'static str> = cards.into_iter().map(|c| c.name()).collect();
    let total_copies = (card_names.len() * num_players) as u32;
    CardCategory { label, card_names, total_copies }
}

pub fn draft_card_categories() -> Vec<CardCategory> {
    // A dye's tier shows in how many colours it carries.
    let dyes_with =
        |color_count: usize| draft_dye_cards().into_iter().filter(move |c| c.colors().len() == color_count);
    vec![
        draft_category("Primary Dyes", dyes_with(3)),
        draft_category("Secondary Dyes", dyes_with(2)),
        draft_category("Tertiary Dyes", dyes_with(1)),
        draft_category("Action Cards", action_cards()),
        draft_category("Material + Color", material_color_cards()),
        draft_category("Dual Materials", dual_material_cards()),
    ]
}

pub fn get_starter_card_categories(num_players: usize) -> Vec<CardCategory> {
    vec![
        starter_category("Starter Dyes", basic_dye_cards(), num_players),
        starter_category("Starter Materials", starter_material_cards(), num_players),
        starter_category("Chalk", [chalk_card()], num_players),
    ]
}
