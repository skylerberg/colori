use crate::types::{Trade, Card, Buyer, ALL_BUYERS};

pub const PIGMENT_COPIES: usize = 3;
/// A pigment's tier shows in how many colors it carries, and the tertiary pigments are
/// the thinnest: one color each, and two copies of each.
pub const TERTIARY_PIGMENT_COPIES: usize = 2;
pub const ACTION_COPIES: usize = 5;
pub const MATERIAL_COLOR_COPIES: usize = 2;
pub const DUAL_MATERIAL_COPIES: usize = 1;
pub const PRIMARIES_COPIES: usize = 6;

pub fn starter_pigment_cards() -> [Card; 3] {
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

pub fn draft_pigment_cards() -> [Card; 18] {
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

/// The Primaries card: one of each primary when workshopped, Workshop ×2 when
/// trashed.
pub fn primaries_card() -> Card {
    Card::Primaries
}

/// Copies of a pigment in the draft deck: two of a tertiary pigment, three otherwise.
pub fn pigment_copies(card: Card) -> usize {
    if card.colors().len() == 1 { TERTIARY_PIGMENT_COPIES } else { PIGMENT_COPIES }
}

/// Every card in the draft deck with how many copies of it there are.
pub fn draft_deck_cards() -> impl Iterator<Item = (Card, usize)> {
    let pigments = draft_pigment_cards().into_iter().map(|c| (c, pigment_copies(c)));
    let material_colors = material_color_cards().into_iter().map(|c| (c, MATERIAL_COLOR_COPIES));
    let dual_materials = dual_material_cards().into_iter().map(|c| (c, DUAL_MATERIAL_COPIES));
    let actions = action_cards().into_iter().map(|c| (c, ACTION_COPIES));
    let primaries = std::iter::once((primaries_card(), PRIMARIES_COPIES));
    pigments.chain(material_colors).chain(dual_materials).chain(actions).chain(primaries)
}

/// Copies of `card` in the draft deck; zero for the starter cards.
pub fn draft_copies(card: Card) -> usize {
    draft_deck_cards()
        .find(|&(c, _)| c == card)
        .map_or(0, |(_, copies)| copies)
}

/// Copies of `buyer` in the buyer deck.
pub fn buyer_copies(buyer: Buyer) -> usize {
    match buyer.trade() {
        Trade::Dyer | Trade::Potter => 2,
        _ => 1,
    }
}

/// Every buyer with how many copies of it are in the buyer deck.
pub fn buyer_deck_cards() -> impl Iterator<Item = (Buyer, usize)> {
    ALL_BUYERS.into_iter().map(|c| (c, buyer_copies(c)))
}
