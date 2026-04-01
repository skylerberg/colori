use crate::types::{Card, SellCard};

pub const DYE_COPIES: usize = 3;
pub const ACTION_COPIES: usize = 4;
pub const MATERIAL_COPIES: usize = 1;

pub fn basic_dye_cards() -> [Card; 3] {
    [Card::BasicRed, Card::BasicYellow, Card::BasicBlue]
}

pub fn starter_material_cards() -> [Card; 3] {
    [Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles]
}

pub fn chalk_card() -> Card {
    Card::Chalk
}

pub fn dye_cards() -> [Card; 21] {
    [
        Card::Kermes, Card::Weld, Card::Woad,
        Card::Lac, Card::Brazilwood, Card::Pomegranate,
        Card::Sumac, Card::Elderberry, Card::Turnsole,
        Card::Madder, Card::Turmeric, Card::DyersGreenweed,
        Card::Verdigris, Card::Orchil, Card::Logwood,
        Card::VermilionDye, Card::Saffron, Card::PersianBerries,
        Card::Azurite, Card::IndigoDye, Card::Cochineal,
    ]
}

pub fn draft_dye_cards() -> [Card; 18] {
    [
        Card::Lac, Card::Brazilwood, Card::Pomegranate,
        Card::Sumac, Card::Elderberry, Card::Turnsole,
        Card::Madder, Card::Turmeric, Card::DyersGreenweed,
        Card::Verdigris, Card::Orchil, Card::Logwood,
        Card::VermilionDye, Card::Saffron, Card::PersianBerries,
        Card::Azurite, Card::IndigoDye, Card::Cochineal,
    ]
}

pub fn draft_material_cards() -> [Card; 12] {
    [
        Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
        Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
        Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
        Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    ]
}

pub fn action_cards() -> [Card; 6] {
    [Card::Alum, Card::CreamOfTartar, Card::GumArabic, Card::Potash, Card::LinseedOil, Card::Vinegar]
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
