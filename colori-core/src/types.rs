use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::deck::Deck;
use crate::fixed_vec::{CopyVec, FixedVec};
use crate::game_log::DrawLog;
use crate::unordered_cards::{UnorderedSellCards, UnorderedCards};

pub type AbilityStack = SmallVec<[Ability; 4]>;

/// The most mixes one ability grants, and so the longest sequence a single
/// choice can carry.
pub const MAX_MIX_SEQUENCE: usize = 3;
pub type MixSequence = CopyVec<(Color, Color), MAX_MIX_SEQUENCE>;
/// The cards a workshop choice takes, by type. A workshop takes at most three.
pub type WorkshopCards = CopyVec<Card, 4>;

pub const MAX_PLAYERS: usize = 4;

/// Rounds in a game. Solo simulations can set their own `GameState::max_rounds`.
pub const NUM_ROUNDS: u32 = 7;

/// Size of the face-up sell card display: the number revealed at setup, and the
/// count each claim is replaced back up to.
pub const MAX_SELL_CARD_DISPLAY: usize = 7;

/// Buyer slots a player can ever hold. See `buyers_phase::buyer_capacity`.
pub const MAX_BUYERS: usize = 3;

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
    Ultramarine,
    Purple,
    Fuchsia,
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
    Color::Ultramarine,
    Color::Purple,
    Color::Fuchsia,
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

/// The three things a ducat buys, each the smallest unit of the matching
/// ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DucatPurchase {
    #[serde(rename = "workshop")]
    Workshop,
    #[serde(rename = "mixColors")]
    MixColors,
    #[serde(rename = "sell")]
    Sell,
}

pub const ALL_DUCAT_PURCHASES: [DucatPurchase; 3] = [
    DucatPurchase::Workshop,
    DucatPurchase::MixColors,
    DucatPurchase::Sell,
];

impl DucatPurchase {
    /// What lands on the ability stack when the ducat is paid.
    pub fn ability(self) -> Ability {
        match self {
            DucatPurchase::Workshop => Ability::Workshop { count: 1 },
            DucatPurchase::MixColors => Ability::MixColors { count: 1 },
            DucatPurchase::Sell => Ability::Sell,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Ability {
    #[serde(rename = "workshop")]
    Workshop { count: u32 },
    #[serde(rename = "mixColors")]
    MixColors { count: u32 },
    #[serde(rename = "moveToDraftPool")]
    MoveToDraftPool,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "gainDucats")]
    GainDucats { count: u32 },
    #[serde(rename = "gainSecondary")]
    GainSecondary,
    #[serde(rename = "gainWildPrimaries")]
    GainWildPrimaries { count: u32 },
    #[serde(rename = "gainBoats")]
    GainBoats { count: u32 },
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

// ── Card enum (42 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Card {
    // Starter dyes (3)
    BasicRed,
    BasicYellow,
    BasicBlue,
    // Primary dyes (6)
    Realgar,
    Kermes,
    Litharge,
    Malachite,
    Woad,
    Logwood,
    // Secondary dyes (6)
    DragonsBlood,
    Orpiment,
    GreenEarth,
    Verdigris,
    Brazilwood,
    Indigo,
    // Tertiary dyes (6)
    Cinnabar,
    Saffron,
    CopperResinate,
    Azurite,
    LapisLazuli,
    Cochineal,
    // Starter materials (3)
    StarterCeramics,
    StarterCanvas,
    StarterFabric,
    // Ceramics + primary (3)
    CeramicsRed,
    CeramicsYellow,
    CeramicsBlue,
    // Canvas + primary (3)
    CanvasRed,
    CanvasYellow,
    CanvasBlue,
    // Fabric + secondary (3)
    FabricOrange,
    FabricGreen,
    FabricPurple,
    // Dual materials (3)
    CanvasCeramics,
    CeramicsFabric,
    CanvasFabric,
    // Actions (6)
    Alum,
    GumArabic,
    LinseedOil,
    Warehouse,
    Gondola,
    Chalk,
}

pub const NUM_CARDS: usize = 42;

struct CardProperties {
    name: &'static str,
    kind: CardKind,
    /// Resolved in order when the card is trashed.
    trash_abilities: &'static [Ability],
    colors: &'static [Color],
    material_types: &'static [MaterialType],
    workshop_abilities: &'static [Ability],
}

const SELL: &[Ability] = &[Ability::Sell];
const WORKSHOP_2: &[Ability] = &[Ability::Workshop { count: 2 }];
const WORKSHOP_3: &[Ability] = &[Ability::Workshop { count: 3 }];
const MIX_2: &[Ability] = &[Ability::MixColors { count: 2 }];
const WORKSHOP_2_MIX_1: &[Ability] = &[Ability::Workshop { count: 2 }, Ability::MixColors { count: 1 }];
const MOVE_TO_DRAFT_POOL: &[Ability] = &[Ability::MoveToDraftPool];

const fn dye(name: &'static str, trash_abilities: &'static [Ability], colors: &'static [Color]) -> CardProperties {
    CardProperties { name, kind: CardKind::Dye, trash_abilities, colors, material_types: &[], workshop_abilities: &[] }
}

const fn basic_dye(name: &'static str, colors: &'static [Color]) -> CardProperties {
    CardProperties { name, kind: CardKind::BasicDye, trash_abilities: WORKSHOP_2, colors, material_types: &[], workshop_abilities: &[] }
}

const fn material(
    name: &'static str,
    trash_abilities: &'static [Ability],
    material_types: &'static [MaterialType],
    colors: &'static [Color],
) -> CardProperties {
    CardProperties { name, kind: CardKind::Material, trash_abilities, colors, material_types, workshop_abilities: &[] }
}

const fn action(name: &'static str, trash_abilities: &'static [Ability], workshop_abilities: &'static [Ability]) -> CardProperties {
    CardProperties { name, kind: CardKind::Action, trash_abilities, colors: &[], material_types: &[], workshop_abilities }
}

const CARD_DATA: [CardProperties; NUM_CARDS] = [
    basic_dye("Red", &[Color::Red]),
    basic_dye("Yellow", &[Color::Yellow]),
    basic_dye("Blue", &[Color::Blue]),
    dye("Realgar", MIX_2, &[Color::Red, Color::Red, Color::Yellow]),
    dye("Kermes", MIX_2, &[Color::Red, Color::Red, Color::Blue]),
    dye("Litharge", MIX_2, &[Color::Yellow, Color::Yellow, Color::Red]),
    dye("Malachite", MIX_2, &[Color::Yellow, Color::Yellow, Color::Blue]),
    dye("Woad", MIX_2, &[Color::Blue, Color::Blue, Color::Red]),
    dye("Logwood", MIX_2, &[Color::Blue, Color::Blue, Color::Yellow]),
    dye("Dragon's Blood", SELL, &[Color::Orange, Color::Red]),
    dye("Orpiment", SELL, &[Color::Orange, Color::Yellow]),
    dye("Green Earth", SELL, &[Color::Green, Color::Yellow]),
    dye("Verdigris", SELL, &[Color::Green, Color::Blue]),
    dye("Brazilwood", SELL, &[Color::Purple, Color::Red]),
    dye("Indigo", SELL, &[Color::Purple, Color::Blue]),
    dye("Cinnabar", WORKSHOP_3, &[Color::Vermilion]),
    dye("Saffron", WORKSHOP_3, &[Color::Amber]),
    dye("Copper Resinate", WORKSHOP_3, &[Color::Chartreuse]),
    dye("Azurite", WORKSHOP_3, &[Color::Teal]),
    dye("Lapis Lazuli", WORKSHOP_3, &[Color::Ultramarine]),
    dye("Cochineal", WORKSHOP_3, &[Color::Fuchsia]),
    material("Ceramics", SELL, &[MaterialType::Ceramics], &[]),
    material("Canvas", SELL, &[MaterialType::Paintings], &[]),
    material("Fabric", SELL, &[MaterialType::Textiles], &[]),
    material("Ceramics & Red", WORKSHOP_2_MIX_1, &[MaterialType::Ceramics], &[Color::Red]),
    material("Ceramics & Yellow", WORKSHOP_2_MIX_1, &[MaterialType::Ceramics], &[Color::Yellow]),
    material("Ceramics & Blue", WORKSHOP_2_MIX_1, &[MaterialType::Ceramics], &[Color::Blue]),
    material("Canvas & Red", WORKSHOP_2_MIX_1, &[MaterialType::Paintings], &[Color::Red]),
    material("Canvas & Yellow", WORKSHOP_2_MIX_1, &[MaterialType::Paintings], &[Color::Yellow]),
    material("Canvas & Blue", WORKSHOP_2_MIX_1, &[MaterialType::Paintings], &[Color::Blue]),
    material("Fabric & Orange", SELL, &[MaterialType::Textiles], &[Color::Orange]),
    material("Fabric & Green", SELL, &[MaterialType::Textiles], &[Color::Green]),
    material("Fabric & Purple", SELL, &[MaterialType::Textiles], &[Color::Purple]),
    material("Canvas & Ceramics", WORKSHOP_2_MIX_1, &[MaterialType::Paintings, MaterialType::Ceramics], &[]),
    material("Ceramics & Fabric", WORKSHOP_2_MIX_1, &[MaterialType::Ceramics, MaterialType::Textiles], &[]),
    material("Canvas & Fabric", WORKSHOP_2_MIX_1, &[MaterialType::Paintings, MaterialType::Textiles], &[]),
    action("Alum", MOVE_TO_DRAFT_POOL, &[Ability::GainDucats { count: 1 }]),
    action("Gum Arabic", MOVE_TO_DRAFT_POOL, &[Ability::GainSecondary]),
    action("Linseed Oil", MOVE_TO_DRAFT_POOL, &[Ability::MixColors { count: 3 }]),
    action("Warehouse", MOVE_TO_DRAFT_POOL, &[Ability::GainWildPrimaries { count: 2 }]),
    action("Gondola", MOVE_TO_DRAFT_POOL, &[Ability::GainBoats { count: 1 }]),
    action("Chalk", MIX_2, &[Ability::GainWildPrimaries { count: 1 }]),
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
    pub fn trash_abilities(&self) -> &'static [Ability] {
        self.props().trash_abilities
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

// ── Buyer ──

/// Who a sell card sells to. The trade sets what the sale is worth and which
/// materials it takes; the colours are what tell two cards of a trade apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Buyer {
    Shipwright,
    Glassmaker,
    Dyer,
    Illuminator,
    Potter,
    Tilemaker,
    Painter,
    MasterPainter,
}

pub const ALL_BUYERS: [Buyer; 8] = [
    Buyer::Shipwright,
    Buyer::Glassmaker,
    Buyer::Dyer,
    Buyer::Illuminator,
    Buyer::Potter,
    Buyer::Tilemaker,
    Buyer::Painter,
    Buyer::MasterPainter,
];

impl Buyer {
    pub fn name(self) -> &'static str {
        match self {
            Buyer::Shipwright => "Shipwright",
            Buyer::Glassmaker => "Glassmaker",
            Buyer::Dyer => "Dyer",
            Buyer::Illuminator => "Illuminator",
            Buyer::Potter => "Potter",
            Buyer::Tilemaker => "Tilemaker",
            Buyer::Painter => "Painter",
            Buyer::MasterPainter => "Master Painter",
        }
    }

    #[inline]
    pub const fn points(self) -> u32 {
        match self {
            Buyer::Dyer | Buyer::Illuminator => 2,
            Buyer::Glassmaker | Buyer::Potter => 3,
            Buyer::Shipwright | Buyer::Tilemaker | Buyer::Painter => 4,
            Buyer::MasterPainter => 5,
        }
    }

    /// Stored materials a sale takes, one entry per unit.
    #[inline]
    pub const fn materials(self) -> &'static [MaterialType] {
        match self {
            Buyer::Glassmaker | Buyer::Illuminator => &[],
            Buyer::Shipwright | Buyer::Dyer => &[MaterialType::Textiles],
            Buyer::Potter => &[MaterialType::Ceramics],
            Buyer::Tilemaker => &[MaterialType::Ceramics, MaterialType::Ceramics],
            Buyer::Painter | Buyer::MasterPainter => &[MaterialType::Paintings],
        }
    }

    /// Boats a sale takes outright, before any standing in for a material.
    #[inline]
    pub const fn boats(self) -> u32 {
        match self {
            Buyer::Shipwright => 1,
            _ => 0,
        }
    }
}

// ── SellCard enum (60 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SellCard {
    // Shipwrights: a boat, Textiles and two secondaries (3)
    ShipwrightPurpleGreen,
    ShipwrightOrangeGreen,
    ShipwrightPurpleOrange,
    // Glassmakers: a tertiary and two of one primary, no material (6)
    GlassmakerVermilionBlue,
    GlassmakerAmberBlue,
    GlassmakerChartreuseRed,
    GlassmakerTealRed,
    GlassmakerUltramarineYellow,
    GlassmakerFuchsiaYellow,
    // Dyers: Textiles and two primaries, or one secondary (9)
    DyerRedRed,
    DyerRedYellow,
    DyerRedBlue,
    DyerYellowYellow,
    DyerBlueYellow,
    DyerBlueBlue,
    DyerOrange,
    DyerGreen,
    DyerPurple,
    // Illuminators: one tertiary, no material (6)
    IlluminatorVermilion,
    IlluminatorAmber,
    IlluminatorChartreuse,
    IlluminatorTeal,
    IlluminatorUltramarine,
    IlluminatorFuchsia,
    // Potters: Ceramics and one tertiary (6)
    PotterVermilion,
    PotterAmber,
    PotterChartreuse,
    PotterTeal,
    PotterUltramarine,
    PotterFuchsia,
    // Tilemakers: two Ceramics and one tertiary (6)
    TilemakerVermilion,
    TilemakerAmber,
    TilemakerChartreuse,
    TilemakerTeal,
    TilemakerUltramarine,
    TilemakerFuchsia,
    // Painters: Paintings, a tertiary and a secondary (18)
    PainterVermilionOrange,
    PainterVermilionGreen,
    PainterVermilionPurple,
    PainterAmberOrange,
    PainterAmberGreen,
    PainterAmberPurple,
    PainterChartreuseOrange,
    PainterChartreuseGreen,
    PainterChartreusePurple,
    PainterTealOrange,
    PainterTealGreen,
    PainterTealPurple,
    PainterUltramarineOrange,
    PainterUltramarineGreen,
    PainterUltramarinePurple,
    PainterFuchsiaOrange,
    PainterFuchsiaGreen,
    PainterFuchsiaPurple,
    // Master painters: Paintings and two tertiaries (6)
    MasterPainterVermilionTeal,
    MasterPainterVermilionFuchsia,
    MasterPainterAmberUltramarine,
    MasterPainterChartreuseAmber,
    MasterPainterChartreuseFuchsia,
    MasterPainterUltramarineTeal,
}

pub const NUM_SELL_CARDS: usize = 60;

pub const ALL_SELL_CARDS: [SellCard; NUM_SELL_CARDS] = [
    SellCard::ShipwrightPurpleGreen, SellCard::ShipwrightOrangeGreen, SellCard::ShipwrightPurpleOrange,
    SellCard::GlassmakerVermilionBlue, SellCard::GlassmakerAmberBlue, SellCard::GlassmakerChartreuseRed, SellCard::GlassmakerTealRed, SellCard::GlassmakerUltramarineYellow, SellCard::GlassmakerFuchsiaYellow,
    SellCard::DyerRedRed, SellCard::DyerRedYellow, SellCard::DyerRedBlue, SellCard::DyerYellowYellow, SellCard::DyerBlueYellow, SellCard::DyerBlueBlue, SellCard::DyerOrange, SellCard::DyerGreen, SellCard::DyerPurple,
    SellCard::IlluminatorVermilion, SellCard::IlluminatorAmber, SellCard::IlluminatorChartreuse, SellCard::IlluminatorTeal, SellCard::IlluminatorUltramarine, SellCard::IlluminatorFuchsia,
    SellCard::PotterVermilion, SellCard::PotterAmber, SellCard::PotterChartreuse, SellCard::PotterTeal, SellCard::PotterUltramarine, SellCard::PotterFuchsia,
    SellCard::TilemakerVermilion, SellCard::TilemakerAmber, SellCard::TilemakerChartreuse, SellCard::TilemakerTeal, SellCard::TilemakerUltramarine, SellCard::TilemakerFuchsia,
    SellCard::PainterVermilionOrange, SellCard::PainterVermilionGreen, SellCard::PainterVermilionPurple, SellCard::PainterAmberOrange, SellCard::PainterAmberGreen, SellCard::PainterAmberPurple, SellCard::PainterChartreuseOrange, SellCard::PainterChartreuseGreen, SellCard::PainterChartreusePurple, SellCard::PainterTealOrange, SellCard::PainterTealGreen, SellCard::PainterTealPurple, SellCard::PainterUltramarineOrange, SellCard::PainterUltramarineGreen, SellCard::PainterUltramarinePurple, SellCard::PainterFuchsiaOrange, SellCard::PainterFuchsiaGreen, SellCard::PainterFuchsiaPurple,
    SellCard::MasterPainterVermilionTeal, SellCard::MasterPainterVermilionFuchsia, SellCard::MasterPainterAmberUltramarine, SellCard::MasterPainterChartreuseAmber, SellCard::MasterPainterChartreuseFuchsia, SellCard::MasterPainterUltramarineTeal,
];

struct SellCardProperties {
    buyer: Buyer,
    color_cost: &'static [Color],
}

const SELL_CARD_DATA: [SellCardProperties; NUM_SELL_CARDS] = [
    SellCardProperties { buyer: Buyer::Shipwright, color_cost: &[Color::Purple, Color::Green] },
    SellCardProperties { buyer: Buyer::Shipwright, color_cost: &[Color::Orange, Color::Green] },
    SellCardProperties { buyer: Buyer::Shipwright, color_cost: &[Color::Purple, Color::Orange] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Vermilion, Color::Blue, Color::Blue] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Amber, Color::Blue, Color::Blue] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Chartreuse, Color::Red, Color::Red] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Teal, Color::Red, Color::Red] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Ultramarine, Color::Yellow, Color::Yellow] },
    SellCardProperties { buyer: Buyer::Glassmaker, color_cost: &[Color::Fuchsia, Color::Yellow, Color::Yellow] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Red, Color::Red] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Red, Color::Yellow] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Red, Color::Blue] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Yellow, Color::Yellow] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Blue, Color::Yellow] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Blue, Color::Blue] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Orange] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Green] },
    SellCardProperties { buyer: Buyer::Dyer, color_cost: &[Color::Purple] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Vermilion] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Amber] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Chartreuse] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Teal] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Ultramarine] },
    SellCardProperties { buyer: Buyer::Illuminator, color_cost: &[Color::Fuchsia] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Vermilion] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Amber] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Chartreuse] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Teal] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Ultramarine] },
    SellCardProperties { buyer: Buyer::Potter, color_cost: &[Color::Fuchsia] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Vermilion] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Amber] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Chartreuse] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Teal] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Ultramarine] },
    SellCardProperties { buyer: Buyer::Tilemaker, color_cost: &[Color::Fuchsia] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Vermilion, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Vermilion, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Vermilion, Color::Purple] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Amber, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Amber, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Amber, Color::Purple] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Chartreuse, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Chartreuse, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Chartreuse, Color::Purple] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Teal, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Teal, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Teal, Color::Purple] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Ultramarine, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Ultramarine, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Ultramarine, Color::Purple] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Fuchsia, Color::Orange] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Fuchsia, Color::Green] },
    SellCardProperties { buyer: Buyer::Painter, color_cost: &[Color::Fuchsia, Color::Purple] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Vermilion, Color::Teal] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Vermilion, Color::Fuchsia] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Amber, Color::Ultramarine] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Chartreuse, Color::Amber] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Chartreuse, Color::Fuchsia] },
    SellCardProperties { buyer: Buyer::MasterPainter, color_cost: &[Color::Ultramarine, Color::Teal] },
];

impl SellCard {
    #[inline]
    fn props(&self) -> &'static SellCardProperties {
        &SELL_CARD_DATA[*self as usize]
    }

    #[inline]
    pub fn buyer(&self) -> Buyer {
        self.props().buyer
    }

    #[inline]
    pub fn points(&self) -> u32 {
        self.buyer().points()
    }

    #[inline]
    pub fn required_materials(&self) -> &'static [MaterialType] {
        self.buyer().materials()
    }

    #[inline]
    pub fn required_boats(&self) -> u32 {
        self.buyer().boats()
    }

    #[inline]
    pub fn color_cost(&self) -> &'static [Color] {
        self.props().color_cost
    }

    #[inline]
    pub fn stats(self) -> &'static SellCardStats {
        &SELL_CARD_STATS[self as usize]
    }
}

/// What a sale takes and pays, worked out once per sell card, so the checks a
/// rollout makes on every step read one record instead of the buyer's tables.
#[derive(Clone, Copy)]
pub struct SellCardStats {
    pub points: u32,
    /// Boats the sale takes outright, before any standing in for a material.
    pub boats: u32,
    /// Units of each material, indexed by `MaterialType`.
    pub materials: [u8; 3],
    pub total_colors: u8,
    colors: [(Color, u8); 4],
    distinct_colors: u8,
}

impl SellCardStats {
    /// Each color the sale takes, once, with how many of it.
    #[inline]
    pub fn colors(&self) -> &[(Color, u8)] {
        &self.colors[..self.distinct_colors as usize]
    }
}

const SELL_CARD_STATS: [SellCardStats; NUM_SELL_CARDS] = {
    let empty = SellCardStats {
        points: 0,
        boats: 0,
        materials: [0; 3],
        total_colors: 0,
        colors: [(Color::Red, 0); 4],
        distinct_colors: 0,
    };
    let mut table = [empty; NUM_SELL_CARDS];
    let mut i = 0;
    while i < NUM_SELL_CARDS {
        let props = &SELL_CARD_DATA[i];
        let mut stats = empty;
        stats.points = props.buyer.points();
        stats.boats = props.buyer.boats();
        let materials = props.buyer.materials();
        let mut m = 0;
        while m < materials.len() {
            stats.materials[materials[m] as usize] += 1;
            m += 1;
        }
        let cost = props.color_cost;
        let mut c = 0;
        while c < cost.len() {
            let mut d = 0;
            while d < stats.distinct_colors as usize && stats.colors[d].0 as usize != cost[c] as usize {
                d += 1;
            }
            if d == stats.distinct_colors as usize {
                stats.colors[d] = (cost[c], 0);
                stats.distinct_colors += 1;
            }
            stats.colors[d].1 += 1;
            c += 1;
        }
        stats.total_colors = cost.len() as u8;
        table[i] = stats;
        i += 1;
    }
    table
};

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
    /// A byte a color: half the width of a mix outcome's key, and of the
    /// copies the mix search makes.
    pub counts: [u8; NUM_COLORS],
}

impl ColorWheel {
    pub fn new() -> Self {
        ColorWheel {
            counts: [0; NUM_COLORS],
        }
    }

    #[inline]
    pub fn get(&self, color: Color) -> u32 {
        self.counts[color.index()] as u32
    }

    #[inline]
    pub fn set(&mut self, color: Color, value: u32) {
        self.counts[color.index()] = u8::try_from(value).expect("a color count fits in a byte");
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
            map.serialize_entry(&color, &(self.counts[color.index()] as u32))?;
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
    pub deck: Deck,
    #[serde(default)]
    pub workshopped_cards: UnorderedCards,
    pub workshop_cards: UnorderedCards,
    pub drafted_cards: UnorderedCards,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    /// Sell cards claimed from the shared display and now sellable only by
    /// this player. A sale empties the slot; the next buyers phase refills it.
    #[serde(default)]
    pub buyers: FixedVec<SellCardInstance, MAX_BUYERS>,
    pub completed_sell_cards: SmallVec<[SellCardInstance; 12]>,
    pub ducats: u32,
    /// Wild primaries: each covers a Red, Yellow or Blue the wheel lacks, in a
    /// mix or a sale. Unspent, each is worth half a point.
    #[serde(default)]
    pub wild_primaries: u32,
    /// Wild materials: each covers the material a sale needs. Unspent, each is
    /// worth half a point.
    #[serde(default)]
    pub boats: u32,
    /// Points from sell cards plus ducats. The half points for wild primaries and boats
    /// are not in here; `scoring::score_in_half_points` adds them.
    #[serde(skip)]
    pub cached_whole_points: u32,
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
pub struct BuyersState {
    pub current_player_index: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTurn {
    pub ability_stack: AbilityStack,
    pub ended: bool,
}

/// Everyone takes their action turn at the same time, so each seat has its own
/// abilities waiting to resolve and ends its own turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionState {
    pub turns: FixedVec<PlayerTurn, MAX_PLAYERS>,
}

impl ActionState {
    /// The lowest seat still taking its turn. Order means nothing here; this
    /// is for drivers that make one decision at a time.
    pub fn first_unended(&self) -> Option<usize> {
        self.turns.iter().position(|turn| !turn.ended)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GamePhase {
    #[serde(rename = "draw")]
    Draw,
    #[serde(rename = "buyers")]
    Buyers {
        #[serde(rename = "buyersState")]
        buyers_state: BuyersState,
    },
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
    #[serde(alias = "destroyedPile")]
    pub trash: UnorderedCards,
    pub sell_card_deck: UnorderedSellCards,
    pub sell_card_display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
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
}

fn default_max_rounds() -> u32 {
    NUM_ROUNDS
}

fn default_card_lookup() -> [Card; 256] {
    [Card::BasicRed; 256]
}

fn default_sell_card_lookup() -> [SellCard; 256] {
    [ALL_SELL_CARDS[0]; 256]
}

// ── Choice ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Choice {
    #[serde(rename = "draftPick")]
    DraftPick { card: Card },
    #[serde(rename = "trashDraftedCard", alias = "destroyDraftedCard")]
    TrashDraftedCard { card: Card },
    #[serde(rename = "endTurn")]
    EndTurn,
    #[serde(rename = "workshop")]
    Workshop {
        #[serde(rename = "cardTypes")]
        card_types: WorkshopCards,
    },
    #[serde(rename = "skipWorkshop")]
    SkipWorkshop,
    #[serde(rename = "moveToDraftPool")]
    MoveToDraftPool { card: Option<Card> },
    #[serde(rename = "selectSellCard")]
    SelectSellCard {
        #[serde(rename = "sellCard", alias = "sell_card")]
        sell_card: SellCard,
    },
    #[serde(rename = "gainSecondary")]
    GainSecondary { color: Color },
    #[serde(rename = "mixAll")]
    MixAll { mixes: MixSequence },
    #[serde(rename = "trashAndMix", alias = "destroyAndMix", alias = "destroyAndMixAll")]
    TrashAndMix { card: Card, mixes: MixSequence },
    #[serde(rename = "trashAndSell", alias = "destroyAndSell")]
    TrashAndSell {
        card: Card,
        #[serde(rename = "sellCard", alias = "sell_card")]
        sell_card: SellCard,
    },
    #[serde(rename = "trashAndWorkshop", alias = "destroyAndWorkshop")]
    TrashAndWorkshop {
        card: Card,
        #[serde(rename = "workshopCards")]
        workshop_cards: WorkshopCards,
    },
    #[serde(rename = "trashAndMoveToDraftPool", alias = "destroyAndMoveToDraftPool")]
    TrashAndMoveToDraftPool {
        card: Card,
        target: Option<Card>,
    },
    /// What a ducat can be exchanged for during your own turn. Deliberately
    /// not `Ability`, which carries counts a purchase must not be able to set.
    #[serde(rename = "spendDucat")]
    SpendDucat { purchase: DucatPurchase },
    #[serde(rename = "takeBuyer")]
    TakeBuyer {
        #[serde(rename = "sellCard")]
        sell_card: SellCard,
    },
    /// Take the top of the sell card deck unseen, rather than one of the
    /// face-up cards on display.
    #[serde(rename = "drawBuyer")]
    DrawBuyer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sell_card_stats_agree_with_the_buyer_tables() {
        for sell_card in ALL_SELL_CARDS {
            let stats = sell_card.stats();
            assert_eq!(stats.points, sell_card.points());
            assert_eq!(stats.boats, sell_card.required_boats());
            for material in ALL_MATERIAL_TYPES {
                let units = sell_card.required_materials().iter().filter(|&&m| m == material).count();
                assert_eq!(stats.materials[material as usize] as usize, units, "{sell_card:?}");
            }
            let cost = sell_card.color_cost();
            assert_eq!(stats.total_colors as usize, cost.len());
            for &color in cost {
                let units = cost.iter().filter(|&&c| c == color).count();
                let listed: Vec<_> = stats.colors().iter().filter(|&&(c, _)| c == color).collect();
                assert_eq!(listed.len(), 1, "{sell_card:?} lists {color:?} once");
                assert_eq!(listed[0].1 as usize, units, "{sell_card:?}");
            }
        }
    }
}
