use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::fixed_vec::FixedVec;
use crate::game_log::DrawLog;
use crate::unordered_cards::{UnorderedSellCards, UnorderedCards};

pub type AbilityStack = SmallVec<[Ability; 4]>;

pub const MAX_PLAYERS: usize = 4;
pub const MAX_SELL_CARD_DISPLAY: usize = 6;
pub const MAX_GLASS_DISPLAY: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Expansions {
    pub glass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlassCard {
    GlassWorkshop,
    GlassDraw,
    GlassMix,
    GlassExchange,
    GlassMoveDrafted,
    GlassUnmix,
    GlassTertiaryDucat,
    GlassReworkshop,
    GlassGainPrimary,
    GlassDestroyClean,
    GlassKeepBoth,
}

impl GlassCard {
    pub fn name(&self) -> &'static str {
        match self {
            GlassCard::GlassWorkshop => "Glass Workshop",
            GlassCard::GlassDraw => "Glass Draw",
            GlassCard::GlassMix => "Glass Mix",
            GlassCard::GlassExchange => "Glass Exchange",
            GlassCard::GlassMoveDrafted => "Glass Move Drafted",
            GlassCard::GlassUnmix => "Glass Unmix",
            GlassCard::GlassTertiaryDucat => "Glass Tertiary Ducat",
            GlassCard::GlassReworkshop => "Glass Reworkshop",
            GlassCard::GlassGainPrimary => "Glass Gain Primary",
            GlassCard::GlassDestroyClean => "Glass Destroy",
            GlassCard::GlassKeepBoth => "Glass Keep Both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassInstance {
    pub instance_id: u32,
    pub card: GlassCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Red,
    Vermilion,
    Orange,
    Amber,
    Yellow,
    Chartreuse,
    Green,
    Teal,
    Blue,
    Indigo,
    Purple,
    Magenta,
}

pub const ALL_COLORS: [Color; 12] = [
    Color::Red,
    Color::Vermilion,
    Color::Orange,
    Color::Amber,
    Color::Yellow,
    Color::Chartreuse,
    Color::Green,
    Color::Teal,
    Color::Blue,
    Color::Indigo,
    Color::Purple,
    Color::Magenta,
];

pub const NUM_COLORS: usize = 12;

impl Color {
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub fn from_index(i: usize) -> Color {
        ALL_COLORS[i]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialType {
    Textiles,
    Ceramics,
    Paintings,
}

pub const ALL_MATERIAL_TYPES: [MaterialType; 3] = [
    MaterialType::Textiles,
    MaterialType::Ceramics,
    MaterialType::Paintings,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Ability {
    #[serde(rename = "workshop")]
    Workshop { count: u32 },
    #[serde(rename = "drawCards")]
    DrawCards { count: u32 },
    #[serde(rename = "mixColors")]
    MixColors { count: u32 },
    #[serde(rename = "destroyCards")]
    DestroyCards,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "gainDucats")]
    GainDucats { count: u32 },
    #[serde(rename = "gainSecondary")]
    GainSecondary,
    #[serde(rename = "gainPrimary")]
    GainPrimary,
    #[serde(rename = "changeTertiary")]
    ChangeTertiary,
    #[serde(rename = "moveToDrafted")]
    MoveToDrafted,
}

// ── CardKind ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardKind {
    #[serde(rename = "dye")]
    Dye,
    #[serde(rename = "basicDye")]
    BasicDye,
    #[serde(rename = "material")]
    Material,
    #[serde(rename = "action")]
    Action,
}

// ── Card enum (46 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Card {
    // Basic dyes (3)
    BasicRed,
    BasicYellow,
    BasicBlue,
    // Primary dyes (9)
    Kermes,
    Weld,
    Woad,
    Lac,
    Brazilwood,
    Pomegranate,
    Sumac,
    Elderberry,
    Turnsole,
    // Secondary dyes (6)
    Madder,
    Turmeric,
    DyersGreenweed,
    Verdigris,
    Orchil,
    Logwood,
    // Tertiary dyes (6)
    VermilionDye,
    Saffron,
    PersianBerries,
    Azurite,
    IndigoDye,
    Cochineal,
    // Starter materials (3)
    StarterCeramics,
    StarterPaintings,
    StarterTextiles,
    // Material+color Ceramics (3)
    TerraCotta,
    OchreWare,
    CobaltWare,
    // Material+color Paintings (3)
    CinnabarCanvas,
    OrpimentCanvas,
    UltramarineCanvas,
    // Material+color Textiles (3)
    AlizarinFabric,
    FusticFabric,
    PastelFabric,
    // Dual materials (3)
    ClayCanvas,
    ClayFabric,
    CanvasFabric,
    // Actions (8)
    Alum,
    CreamOfTartar,
    GumArabic,
    Potash,
    Vinegar,
    Argol,
    Chalk,
    LinseedOil,
    Lye,
}

struct CardProperties {
    name: &'static str,
    kind: CardKind,
    ability: Ability,
    colors: &'static [Color],
    material_types: &'static [MaterialType],
    workshop_abilities: &'static [Ability],
}

const CARD_DATA: [CardProperties; 48] = [
    // BasicRed
    CardProperties { name: "Basic Red", kind: CardKind::BasicDye, ability: Ability::Sell, colors: &[Color::Red], material_types: &[], workshop_abilities: &[] },
    // BasicYellow
    CardProperties { name: "Basic Yellow", kind: CardKind::BasicDye, ability: Ability::Sell, colors: &[Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // BasicBlue
    CardProperties { name: "Basic Blue", kind: CardKind::BasicDye, ability: Ability::Sell, colors: &[Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Kermes
    CardProperties { name: "Kermes", kind: CardKind::Dye, ability: Ability::DestroyCards, colors: &[Color::Red, Color::Red, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Weld
    CardProperties { name: "Weld", kind: CardKind::Dye, ability: Ability::DestroyCards, colors: &[Color::Yellow, Color::Yellow, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Woad
    CardProperties { name: "Woad", kind: CardKind::Dye, ability: Ability::DestroyCards, colors: &[Color::Blue, Color::Blue, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Lac
    CardProperties { name: "Lac", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Red, Color::Red, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Brazilwood
    CardProperties { name: "Brazilwood", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Red, Color::Red, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Pomegranate
    CardProperties { name: "Pomegranate", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Yellow, Color::Yellow, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Sumac
    CardProperties { name: "Sumac", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Yellow, Color::Yellow, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Elderberry
    CardProperties { name: "Elderberry", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Blue, Color::Blue, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Turnsole
    CardProperties { name: "Turnsole", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, colors: &[Color::Blue, Color::Blue, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Madder
    CardProperties { name: "Madder", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Orange, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Turmeric
    CardProperties { name: "Turmeric", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Orange, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // DyersGreenweed
    CardProperties { name: "Dyer's Greenweed", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Green, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Verdigris
    CardProperties { name: "Verdigris", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Green, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Orchil
    CardProperties { name: "Orchil", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Purple, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Logwood
    CardProperties { name: "Logwood", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, colors: &[Color::Purple, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // VermilionDye
    CardProperties { name: "Vermilion", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Vermilion], material_types: &[], workshop_abilities: &[] },
    // Saffron
    CardProperties { name: "Saffron", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Amber], material_types: &[], workshop_abilities: &[] },
    // PersianBerries
    CardProperties { name: "Persian Berries", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Chartreuse], material_types: &[], workshop_abilities: &[] },
    // Azurite
    CardProperties { name: "Azurite", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Teal], material_types: &[], workshop_abilities: &[] },
    // IndigoDye
    CardProperties { name: "Indigo", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Indigo], material_types: &[], workshop_abilities: &[] },
    // Cochineal
    CardProperties { name: "Cochineal", kind: CardKind::Dye, ability: Ability::Sell, colors: &[Color::Magenta], material_types: &[], workshop_abilities: &[] },
    // StarterCeramics
    CardProperties { name: "Ceramics", kind: CardKind::Material, ability: Ability::Workshop { count: 3 }, colors: &[], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // StarterPaintings
    CardProperties { name: "Paintings", kind: CardKind::Material, ability: Ability::Workshop { count: 4 }, colors: &[], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // StarterTextiles
    CardProperties { name: "Textiles", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // TerraCotta
    CardProperties { name: "Terra Cotta", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Red], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // OchreWare
    CardProperties { name: "Ochre Ware", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Yellow], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // CobaltWare
    CardProperties { name: "Cobalt Ware", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Blue], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // CinnabarCanvas
    CardProperties { name: "Cinnabar & Canvas", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Red], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // OrpimentCanvas
    CardProperties { name: "Orpiment & Canvas", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Yellow], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // UltramarineCanvas
    CardProperties { name: "Ultramarine & Canvas", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Blue], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // AlizarinFabric
    CardProperties { name: "Alizarin & Fabric", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Red], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // FusticFabric
    CardProperties { name: "Fustic & Fabric", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Yellow], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // PastelFabric
    CardProperties { name: "Pastel & Fabric", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, colors: &[Color::Blue], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // ClayCanvas
    CardProperties { name: "Clay & Canvas", kind: CardKind::Material, ability: Ability::Sell, colors: &[], material_types: &[MaterialType::Ceramics, MaterialType::Paintings], workshop_abilities: &[] },
    // ClayFabric
    CardProperties { name: "Clay & Fabric", kind: CardKind::Material, ability: Ability::Sell, colors: &[], material_types: &[MaterialType::Ceramics, MaterialType::Textiles], workshop_abilities: &[] },
    // CanvasFabric
    CardProperties { name: "Canvas & Fabric", kind: CardKind::Material, ability: Ability::Sell, colors: &[], material_types: &[MaterialType::Paintings, MaterialType::Textiles], workshop_abilities: &[] },
    // Alum
    CardProperties { name: "Alum", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::GainDucats { count: 1 }] },
    // CreamOfTartar
    CardProperties { name: "Cream of Tartar", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::DrawCards { count: 3 }] },
    // GumArabic
    CardProperties { name: "Gum Arabic", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::GainSecondary] },
    // Potash
    CardProperties { name: "Potash", kind: CardKind::Action, ability: Ability::DrawCards { count: 2 }, colors: &[], material_types: &[], workshop_abilities: &[Ability::Workshop { count: 3 }] },
    // Vinegar
    CardProperties { name: "Vinegar", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::ChangeTertiary] },
    // Argol — not currently used
    CardProperties { name: "Argol", kind: CardKind::Action, ability: Ability::Sell, colors: &[], material_types: &[], workshop_abilities: &[Ability::DrawCards { count: 2 }] },
    // Chalk
    CardProperties { name: "Chalk", kind: CardKind::Action, ability: Ability::Sell, colors: &[], material_types: &[], workshop_abilities: &[Ability::GainPrimary] },
    // LinseedOil
    CardProperties { name: "Linseed Oil", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::MixColors { count: 2 }] },
    // Lye — not currently in draft deck
    CardProperties { name: "Lye", kind: CardKind::Action, ability: Ability::DestroyCards, colors: &[], material_types: &[], workshop_abilities: &[Ability::MoveToDrafted] },
];

impl Card {
    #[inline]
    fn props(&self) -> &'static CardProperties {
        &CARD_DATA[*self as usize]
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.props().name
    }

    #[inline]
    pub fn kind(&self) -> CardKind {
        self.props().kind
    }

    #[inline]
    pub fn ability(&self) -> Ability {
        self.props().ability
    }

    #[inline]
    pub fn colors(&self) -> &'static [Color] {
        self.props().colors
    }

    #[inline]
    pub fn material_types(&self) -> &'static [MaterialType] {
        self.props().material_types
    }

    #[inline]
    pub fn workshop_abilities(&self) -> &'static [Ability] {
        self.props().workshop_abilities
    }

    #[inline]
    pub fn is_action(&self) -> bool {
        self.kind() == CardKind::Action
    }
}

// ── SellCard enum (54 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SellCard {
    // Textiles 2-ducat, single tertiary (6)
    Textiles2Vermilion,
    Textiles2Amber,
    Textiles2Chartreuse,
    Textiles2Teal,
    Textiles2Indigo,
    Textiles2Magenta,
    // Textiles 2-ducat, secondary+primary (9)
    Textiles2OrangeRed,
    Textiles2OrangeYellow,
    Textiles2OrangeBlue,
    Textiles2GreenRed,
    Textiles2GreenYellow,
    Textiles2GreenBlue,
    Textiles2PurpleRed,
    Textiles2PurpleYellow,
    Textiles2PurpleBlue,
    // Textiles 2-ducat, triple primary (3)
    Textiles2RedRedRed,
    Textiles2YellowYellowYellow,
    Textiles2BlueBlueBlue,
    // Ceramics 3-ducat, tertiary+primary (18)
    Ceramics3VermilionRed,
    Ceramics3VermilionYellow,
    Ceramics3VermilionBlue,
    Ceramics3AmberRed,
    Ceramics3AmberYellow,
    Ceramics3AmberBlue,
    Ceramics3ChartreuseRed,
    Ceramics3ChartreuseYellow,
    Ceramics3ChartreuseBlue,
    Ceramics3TealRed,
    Ceramics3TealYellow,
    Ceramics3TealBlue,
    Ceramics3IndigoRed,
    Ceramics3IndigoYellow,
    Ceramics3IndigoBlue,
    Ceramics3MagentaRed,
    Ceramics3MagentaYellow,
    Ceramics3MagentaBlue,
    // Paintings 4-ducat, tertiary+secondary (18)
    Paintings4VermilionOrange,
    Paintings4VermilionGreen,
    Paintings4VermilionPurple,
    Paintings4AmberOrange,
    Paintings4AmberGreen,
    Paintings4AmberPurple,
    Paintings4ChartreuseOrange,
    Paintings4ChartreuseGreen,
    Paintings4ChartreusePurple,
    Paintings4TealOrange,
    Paintings4TealGreen,
    Paintings4TealPurple,
    Paintings4IndigoOrange,
    Paintings4IndigoGreen,
    Paintings4IndigoPurple,
    Paintings4MagentaOrange,
    Paintings4MagentaGreen,
    Paintings4MagentaPurple,
}

struct SellCardProperties {
    ducats: u32,
    required_material: MaterialType,
    color_cost: &'static [Color],
}

const SELL_CARD_DATA: [SellCardProperties; 54] = [
    // Textiles 2-ducat, single tertiary (6)
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Vermilion] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Amber] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Chartreuse] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Teal] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Indigo] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Magenta] },
    // Textiles 2-ducat, secondary+primary (9)
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Red] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Yellow] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Blue] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Red] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Yellow] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Blue] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Red] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Yellow] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Blue] },
    // Textiles 2-ducat, triple primary (3)
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Red, Color::Red, Color::Red] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Yellow, Color::Yellow, Color::Yellow] },
    SellCardProperties { ducats: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Blue, Color::Blue, Color::Blue] },
    // Ceramics 3-ducat, tertiary+primary (18)
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Blue] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Blue] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Blue] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Blue] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Blue] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Red] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Yellow] },
    SellCardProperties { ducats: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Blue] },
    // Paintings 4-ducat, tertiary+secondary (18)
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Purple] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Purple] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Purple] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Purple] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Purple] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Orange] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Green] },
    SellCardProperties { ducats: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Purple] },
];

impl SellCard {
    #[inline]
    fn props(&self) -> &'static SellCardProperties {
        &SELL_CARD_DATA[*self as usize]
    }

    #[inline]
    pub fn ducats(&self) -> u32 {
        self.props().ducats
    }

    #[inline]
    pub fn required_material(&self) -> MaterialType {
        self.props().required_material
    }

    #[inline]
    pub fn color_cost(&self) -> &'static [Color] {
        self.props().color_cost
    }
}

// ── CardInstance / SellCardInstance ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInstance {
    pub instance_id: u32,
    pub card: Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SellCardInstance {
    pub instance_id: u32,
    #[serde(rename = "card")]
    pub sell_card: SellCard,
}

/// Color wheel represented as counts for each of 12 colors.
/// Serialized as a Record<Color, number> in JSON.
#[derive(Debug, Clone)]
pub struct ColorWheel {
    pub counts: [u32; NUM_COLORS],
}

impl ColorWheel {
    pub fn new() -> Self {
        ColorWheel {
            counts: [0; NUM_COLORS],
        }
    }

    #[inline]
    pub fn get(&self, color: Color) -> u32 {
        self.counts[color.index()]
    }

    #[inline]
    pub fn set(&mut self, color: Color, value: u32) {
        self.counts[color.index()] = value;
    }

    #[inline]
    pub fn increment(&mut self, color: Color) {
        self.counts[color.index()] += 1;
    }

    #[inline]
    pub fn decrement(&mut self, color: Color) -> bool {
        let idx = color.index();
        if self.counts[idx] == 0 {
            return false;
        }
        self.counts[idx] -= 1;
        true
    }
}

impl Serialize for ColorWheel {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(NUM_COLORS))?;
        for &color in &ALL_COLORS {
            map.serialize_entry(&color, &self.counts[color.index()])?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ColorWheel {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: std::collections::HashMap<Color, u32> =
            std::collections::HashMap::deserialize(deserializer)?;
        let mut wheel = ColorWheel::new();
        for (&color, &count) in &map {
            wheel.set(color, count);
        }
        Ok(wheel)
    }
}

/// Materials stored as counts for each of 3 material types.
/// Serialized as Record<MaterialType, number> in JSON.
#[derive(Debug, Clone)]
pub struct Materials {
    pub counts: [u32; 3],
}

impl Materials {
    pub fn new() -> Self {
        Materials { counts: [0; 3] }
    }

    #[inline]
    pub fn get(&self, material_type: MaterialType) -> u32 {
        self.counts[material_type as usize]
    }

    #[inline]
    pub fn increment(&mut self, material_type: MaterialType) {
        self.counts[material_type as usize] += 1;
    }

    #[inline]
    pub fn decrement(&mut self, material_type: MaterialType) -> bool {
        let idx = material_type as usize;
        if self.counts[idx] == 0 {
            return false;
        }
        self.counts[idx] -= 1;
        true
    }
}

impl Serialize for Materials {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        for &material_type in &ALL_MATERIAL_TYPES {
            map.serialize_entry(&material_type, &self.counts[material_type as usize])?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Materials {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: std::collections::HashMap<MaterialType, u32> =
            std::collections::HashMap::deserialize(deserializer)?;
        let mut materials = Materials::new();
        for (&material_type, &count) in &map {
            materials.counts[material_type as usize] = count;
        }
        Ok(materials)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub deck: UnorderedCards,
    pub discard: UnorderedCards,
    #[serde(default)]
    pub workshopped_cards: UnorderedCards,
    pub workshop_cards: UnorderedCards,
    pub drafted_cards: UnorderedCards,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    pub completed_sell_cards: SmallVec<[SellCardInstance; 12]>,
    #[serde(default)]
    pub completed_glass: SmallVec<[GlassInstance; 4]>,
    pub ducats: u32,
    #[serde(skip)]
    pub cached_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftState {
    pub pick_number: u32,
    pub current_player_index: usize,
    #[serde(
        serialize_with = "serialize_hands",
        deserialize_with = "deserialize_hands"
    )]
    pub hands: [UnorderedCards; MAX_PLAYERS],
    pub num_hands: usize,
}

fn serialize_hands<S: serde::Serializer>(
    hands: &[UnorderedCards; MAX_PLAYERS],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    // We can't know num_hands here, so serialize all non-empty + trailing empties
    // Actually, we serialize as Vec using the UnorderedCards serde which already works
    // But we need num_hands... let's just serialize all MAX_PLAYERS entries
    // The deserializer will read them back.
    // Actually the original was Vec<Vec<CardInstance>> with variable length.
    // For backward compat, serialize only the active hands.
    // We'll use a helper: serialize as a Vec of UnorderedCards.
    let mut seq = serializer.serialize_seq(Some(MAX_PLAYERS))?;
    for hand in hands.iter() {
        seq.serialize_element(hand)?;
    }
    seq.end()
}

fn deserialize_hands<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[UnorderedCards; MAX_PLAYERS], D::Error> {
    let v = Vec::<UnorderedCards>::deserialize(deserializer)?;
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];
    for (i, h) in v.into_iter().enumerate() {
        if i < MAX_PLAYERS {
            hands[i] = h;
        }
    }
    Ok(hands)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionState {
    pub current_player_index: usize,
    pub ability_stack: AbilityStack,
    #[serde(default)]
    pub used_glass: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GamePhase {
    #[serde(rename = "draw")]
    Draw,
    #[serde(rename = "draft")]
    Draft {
        #[serde(rename = "draftState")]
        draft_state: DraftState,
    },
    #[serde(rename = "action")]
    Action {
        #[serde(rename = "actionState")]
        action_state: ActionState,
    },
    #[serde(rename = "gameOver")]
    GameOver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub players: FixedVec<PlayerState, MAX_PLAYERS>,
    pub draft_deck: UnorderedCards,
    pub destroyed_pile: UnorderedCards,
    pub sell_card_deck: UnorderedSellCards,
    pub sell_card_display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    #[serde(default)]
    pub expansions: Expansions,
    #[serde(default)]
    pub glass_deck: SmallVec<[GlassInstance; 11]>,
    #[serde(default)]
    pub glass_display: FixedVec<GlassInstance, MAX_GLASS_DISPLAY>,
    pub phase: GamePhase,
    pub round: u32,
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
    pub ai_players: FixedVec<bool, MAX_PLAYERS>,
    #[serde(skip, default = "default_card_lookup")]
    pub card_lookup: [Card; 256],
    #[serde(skip, default = "default_sell_card_lookup")]
    pub sell_card_lookup: [SellCard; 256],
    #[serde(skip)]
    pub draw_log: Option<DrawLog>,
    #[serde(skip)]
    pub force_max_workshop: bool,
    #[serde(skip)]
    pub abstract_draft_perspective: Option<usize>,
    #[serde(skip)]
    pub abstract_draft_initial_pick: u32,
}

fn default_max_rounds() -> u32 {
    20
}

fn default_card_lookup() -> [Card; 256] {
    [Card::BasicRed; 256]
}

fn default_sell_card_lookup() -> [SellCard; 256] {
    [SellCard::Textiles2Vermilion; 256]
}

// ── Choice ──

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Choice {
    #[serde(rename = "draftPick")]
    DraftPick { card: Card },
    #[serde(rename = "draftPickAbility")]
    DraftPickAbility { ability: Ability },
    #[serde(rename = "destroyDraftedCard")]
    DestroyDraftedCard { card: Card },
    #[serde(rename = "endTurn")]
    EndTurn,
    #[serde(rename = "workshop")]
    Workshop {
        #[serde(rename = "cardTypes")]
        card_types: SmallVec<[Card; 4]>,
    },
    #[serde(rename = "skipWorkshop")]
    SkipWorkshop,
    #[serde(rename = "destroyDrawnCards")]
    DestroyDrawnCards { card: Option<Card> },
    #[serde(rename = "selectSellCard")]
    SelectSellCard {
        #[serde(rename = "sellCard", alias = "sell_card")]
        sell_card: SellCard,
    },
    #[serde(rename = "gainSecondary")]
    GainSecondary { color: Color },
    #[serde(rename = "gainPrimary")]
    GainPrimary { color: Color },
    #[serde(rename = "mixAll")]
    MixAll {
        mixes: SmallVec<[(Color, Color); 2]>,
    },
    #[serde(rename = "swapTertiary")]
    SwapTertiary {
        #[serde(rename = "loseColor")]
        lose: Color,
        #[serde(rename = "gainColor")]
        gain: Color,
    },
    #[serde(rename = "destroyAndMix", alias = "destroyAndMixAll")]
    DestroyAndMix {
        card: Card,
        mixes: SmallVec<[(Color, Color); 2]>,
    },
    #[serde(rename = "destroyAndSell")]
    DestroyAndSell {
        card: Card,
        #[serde(rename = "sellCard", alias = "sell_card")]
        sell_card: SellCard,
    },
    #[serde(rename = "destroyAndWorkshop")]
    DestroyAndWorkshop {
        card: Card,
        #[serde(rename = "workshopCards")]
        workshop_cards: SmallVec<[Card; 4]>,
    },
    #[serde(rename = "destroyAndDestroyCards")]
    DestroyAndDestroyCards {
        card: Card,
        target: Option<Card>,
    },

    // Glass card acquisition (during Sell)
    #[serde(rename = "selectGlass")]
    SelectGlass {
        glass: GlassCard,
        #[serde(rename = "payColor")]
        pay_color: Color,
    },

    // Glass ability activations (parameterless - push onto ability stack)
    #[serde(rename = "activateGlassWorkshop")]
    ActivateGlassWorkshop,
    #[serde(rename = "activateGlassDraw")]
    ActivateGlassDraw,
    #[serde(rename = "activateGlassMix")]
    ActivateGlassMix,
    #[serde(rename = "activateGlassGainPrimary")]
    ActivateGlassGainPrimary,

    // Glass ability activations (with parameters - resolve immediately)
    #[serde(rename = "activateGlassExchange")]
    ActivateGlassExchange {
        lose: MaterialType,
        gain: MaterialType,
    },
    #[serde(rename = "activateGlassMoveDrafted")]
    ActivateGlassMoveDrafted { card: Card },
    #[serde(rename = "activateGlassUnmix")]
    ActivateGlassUnmix { color: Color },
    #[serde(rename = "activateGlassTertiaryDucat")]
    ActivateGlassTertiaryDucat { color: Color },
    #[serde(rename = "activateGlassReworkshop")]
    ActivateGlassReworkshop { card: Card },
    #[serde(rename = "activateGlassDestroyClean")]
    ActivateGlassDestroyClean { card: Card },

    // Compound: destroy drafted card + select glass
    #[serde(rename = "destroyAndSelectGlass")]
    DestroyAndSelectGlass {
        card: Card,
        glass: GlassCard,
        #[serde(rename = "payColor")]
        pay_color: Color,
    },

    // Compound: workshop a card twice using GlassReworkshop in between
    #[serde(rename = "workshopWithReworkshop")]
    WorkshopWithReworkshop {
        #[serde(rename = "reworkshopCard")]
        reworkshop_card: Card,
        #[serde(rename = "otherCards")]
        other_cards: SmallVec<[Card; 4]>,
    },

    // MoveToDrafted ability (Lye workshop ability)
    #[serde(rename = "selectMoveToDrafted")]
    SelectMoveToDrafted { card: Card },
    #[serde(rename = "skipMoveToDrafted")]
    SkipMoveToDrafted,
}
