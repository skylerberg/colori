use crate::types::{Card, SellCard};

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
        Card::Verdigris, Card::Brazilwood, Card::IndigoDye,
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

pub fn generate_all_sell_cards() -> [SellCard; 54] {
    [
        // Textiles 2-star, single tertiary (6)
        SellCard::Textiles2Vermilion, SellCard::Textiles2Amber, SellCard::Textiles2Chartreuse,
        SellCard::Textiles2Teal, SellCard::Textiles2Indigo, SellCard::Textiles2Magenta,
        // Textiles 2-star, secondary+primary (9)
        SellCard::Textiles2OrangeRed, SellCard::Textiles2OrangeYellow, SellCard::Textiles2OrangeBlue,
        SellCard::Textiles2GreenRed, SellCard::Textiles2GreenYellow, SellCard::Textiles2GreenBlue,
        SellCard::Textiles2PurpleRed, SellCard::Textiles2PurpleYellow, SellCard::Textiles2PurpleBlue,
        // Textiles 2-star, triple primary (3)
        SellCard::Textiles2RedRedRed, SellCard::Textiles2YellowYellowYellow, SellCard::Textiles2BlueBlueBlue,
        // Ceramics 3-star, tertiary+primary (18)
        SellCard::Ceramics3VermilionRed, SellCard::Ceramics3VermilionYellow, SellCard::Ceramics3VermilionBlue,
        SellCard::Ceramics3AmberRed, SellCard::Ceramics3AmberYellow, SellCard::Ceramics3AmberBlue,
        SellCard::Ceramics3ChartreuseRed, SellCard::Ceramics3ChartreuseYellow, SellCard::Ceramics3ChartreuseBlue,
        SellCard::Ceramics3TealRed, SellCard::Ceramics3TealYellow, SellCard::Ceramics3TealBlue,
        SellCard::Ceramics3IndigoRed, SellCard::Ceramics3IndigoYellow, SellCard::Ceramics3IndigoBlue,
        SellCard::Ceramics3MagentaRed, SellCard::Ceramics3MagentaYellow, SellCard::Ceramics3MagentaBlue,
        // Paintings 4-star, tertiary+secondary (18)
        SellCard::Paintings4VermilionOrange, SellCard::Paintings4VermilionGreen, SellCard::Paintings4VermilionPurple,
        SellCard::Paintings4AmberOrange, SellCard::Paintings4AmberGreen, SellCard::Paintings4AmberPurple,
        SellCard::Paintings4ChartreuseOrange, SellCard::Paintings4ChartreuseGreen, SellCard::Paintings4ChartreusePurple,
        SellCard::Paintings4TealOrange, SellCard::Paintings4TealGreen, SellCard::Paintings4TealPurple,
        SellCard::Paintings4IndigoOrange, SellCard::Paintings4IndigoGreen, SellCard::Paintings4IndigoPurple,
        SellCard::Paintings4MagentaOrange, SellCard::Paintings4MagentaGreen, SellCard::Paintings4MagentaPurple,
    ]
}
