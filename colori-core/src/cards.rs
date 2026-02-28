use crate::types::{Card, BuyerCard};

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

pub fn draft_material_cards() -> [Card; 15] {
    [
        Card::FineCeramics, Card::FinePaintings, Card::FineTextiles,
        Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
        Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
        Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
        Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    ]
}

pub fn action_cards() -> [Card; 4] {
    // Vinegar is temporarily removed from the draft deck and will be added back
    [Card::Alum, Card::CreamOfTartar, Card::GumArabic, Card::Potash]
}

pub fn generate_all_buyers() -> [BuyerCard; 54] {
    [
        // Textiles 2-star, single tertiary (6)
        BuyerCard::Textiles2Vermilion, BuyerCard::Textiles2Amber, BuyerCard::Textiles2Chartreuse,
        BuyerCard::Textiles2Teal, BuyerCard::Textiles2Indigo, BuyerCard::Textiles2Magenta,
        // Textiles 2-star, secondary+primary (9)
        BuyerCard::Textiles2OrangeRed, BuyerCard::Textiles2OrangeYellow, BuyerCard::Textiles2OrangeBlue,
        BuyerCard::Textiles2GreenRed, BuyerCard::Textiles2GreenYellow, BuyerCard::Textiles2GreenBlue,
        BuyerCard::Textiles2PurpleRed, BuyerCard::Textiles2PurpleYellow, BuyerCard::Textiles2PurpleBlue,
        // Textiles 2-star, triple primary (3)
        BuyerCard::Textiles2RedRedRed, BuyerCard::Textiles2YellowYellowYellow, BuyerCard::Textiles2BlueBlueBlue,
        // Ceramics 3-star, tertiary+primary (18)
        BuyerCard::Ceramics3VermilionRed, BuyerCard::Ceramics3VermilionYellow, BuyerCard::Ceramics3VermilionBlue,
        BuyerCard::Ceramics3AmberRed, BuyerCard::Ceramics3AmberYellow, BuyerCard::Ceramics3AmberBlue,
        BuyerCard::Ceramics3ChartreuseRed, BuyerCard::Ceramics3ChartreuseYellow, BuyerCard::Ceramics3ChartreuseBlue,
        BuyerCard::Ceramics3TealRed, BuyerCard::Ceramics3TealYellow, BuyerCard::Ceramics3TealBlue,
        BuyerCard::Ceramics3IndigoRed, BuyerCard::Ceramics3IndigoYellow, BuyerCard::Ceramics3IndigoBlue,
        BuyerCard::Ceramics3MagentaRed, BuyerCard::Ceramics3MagentaYellow, BuyerCard::Ceramics3MagentaBlue,
        // Paintings 4-star, tertiary+secondary (18)
        BuyerCard::Paintings4VermilionOrange, BuyerCard::Paintings4VermilionGreen, BuyerCard::Paintings4VermilionPurple,
        BuyerCard::Paintings4AmberOrange, BuyerCard::Paintings4AmberGreen, BuyerCard::Paintings4AmberPurple,
        BuyerCard::Paintings4ChartreuseOrange, BuyerCard::Paintings4ChartreuseGreen, BuyerCard::Paintings4ChartreusePurple,
        BuyerCard::Paintings4TealOrange, BuyerCard::Paintings4TealGreen, BuyerCard::Paintings4TealPurple,
        BuyerCard::Paintings4IndigoOrange, BuyerCard::Paintings4IndigoGreen, BuyerCard::Paintings4IndigoPurple,
        BuyerCard::Paintings4MagentaOrange, BuyerCard::Paintings4MagentaGreen, BuyerCard::Paintings4MagentaPurple,
    ]
}
