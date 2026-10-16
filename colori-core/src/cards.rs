use crate::types::{Buyer, Card, SellCard, ALL_SELL_CARDS};

pub const DYE_COPIES: usize = 3;
pub const ACTION_COPIES: usize = 5;
pub const MATERIAL_COLOR_COPIES: usize = 2;
pub const DUAL_MATERIAL_COPIES: usize = 1;

pub fn basic_dye_cards() -> [Card; 3] {
    [Card::BasicRed, Card::BasicYellow, Card::BasicBlue]
}

pub fn starter_material_cards() -> [Card; 3] {
    [Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric]
}

pub fn chalk_card() -> Card {
    Card::Chalk
}

/// One player's starting deck. The physical game prints six of these, one per
/// character, but they differ only in the portrait.
pub fn starter_deck() -> [Card; 7] {
    [
        Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
        Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric,
        Card::Chalk,
    ]
}

pub fn draft_dye_cards() -> [Card; 18] {
    [
        Card::Realgar, Card::Kermes, Card::Litharge,
        Card::Malachite, Card::Woad, Card::Logwood,
        Card::DragonsBlood, Card::Orpiment, Card::GreenEarth,
        Card::Verdigris, Card::Brazilwood, Card::Indigo,
        Card::Cinnabar, Card::Saffron, Card::CopperResinate,
        Card::Azurite, Card::LapisLazuli, Card::Cochineal,
    ]
}

pub fn material_color_cards() -> [Card; 9] {
    [
        Card::CeramicsRed, Card::CeramicsYellow, Card::CeramicsBlue,
        Card::CanvasRed, Card::CanvasYellow, Card::CanvasBlue,
        Card::FabricOrange, Card::FabricGreen, Card::FabricPurple,
    ]
}

pub fn dual_material_cards() -> [Card; 3] {
    [Card::CanvasCeramics, Card::CeramicsFabric, Card::CanvasFabric]
}

pub fn draft_material_cards() -> impl Iterator<Item = Card> {
    material_color_cards().into_iter().chain(dual_material_cards())
}

pub fn action_cards() -> [Card; 5] {
    [Card::Alum, Card::GumArabic, Card::LinseedOil, Card::Warehouse, Card::Gondola]
}

/// Every card in the draft deck with how many copies of it there are.
pub fn draft_deck_cards() -> impl Iterator<Item = (Card, usize)> {
    let dyes = draft_dye_cards().into_iter().map(|c| (c, DYE_COPIES));
    let material_colors = material_color_cards().into_iter().map(|c| (c, MATERIAL_COLOR_COPIES));
    let dual_materials = dual_material_cards().into_iter().map(|c| (c, DUAL_MATERIAL_COPIES));
    let actions = action_cards().into_iter().map(|c| (c, ACTION_COPIES));
    dyes.chain(material_colors).chain(dual_materials).chain(actions)
}

/// Copies of `card` in the draft deck; zero for the starter cards.
pub fn draft_copies(card: Card) -> usize {
    draft_deck_cards()
        .find(|&(c, _)| c == card)
        .map_or(0, |(_, copies)| copies)
}

/// Copies of `sell_card` in the sell card deck.
pub fn sell_card_copies(sell_card: SellCard) -> usize {
    match sell_card.buyer() {
        Buyer::Dyer | Buyer::Potter => 2,
        _ => 1,
    }
}

/// Every sell card with how many copies of it are in the sell card deck.
pub fn sell_deck_cards() -> impl Iterator<Item = (SellCard, usize)> {
    ALL_SELL_CARDS.into_iter().map(|c| (c, sell_card_copies(c)))
}
