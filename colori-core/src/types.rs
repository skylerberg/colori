use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::deck::Deck;
use crate::fixed_vec::FixedVec;
use crate::game_log::DrawLog;
use crate::unordered_cards::{UnorderedSellCards, UnorderedCards};

pub type AbilityStack = SmallVec<[Ability; 4]>;

/// The most mixes one ability grants, and so the longest sequence a single
/// choice can carry.
pub const MAX_MIX_SEQUENCE: usize = 3;
pub type MixSequence = SmallVec<[(Color, Color); MAX_MIX_SEQUENCE]>;

pub const MAX_PLAYERS: usize = 4;
pub const MAX_SELL_CARD_DISPLAY: usize = 5;

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
    #[serde(rename = "drawCards")]
    DrawCards { count: u32 },
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
    #[serde(rename = "gainPrimary")]
    GainPrimary,
    #[serde(rename = "gainMaterial")]
    GainMaterial,
    #[serde(rename = "gainSilver")]
    GainSilver { count: u32 },
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
    IndigoDye,
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
    /// Resolved in order when the card is destroyed.
    destroy_abilities: &'static [Ability],
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

const fn dye(name: &'static str, destroy_abilities: &'static [Ability], colors: &'static [Color]) -> CardProperties {
    CardProperties { name, kind: CardKind::Dye, destroy_abilities, colors, material_types: &[], workshop_abilities: &[] }
}

const fn basic_dye(name: &'static str, colors: &'static [Color]) -> CardProperties {
    CardProperties { name, kind: CardKind::BasicDye, destroy_abilities: WORKSHOP_2, colors, material_types: &[], workshop_abilities: &[] }
}

const fn material(
    name: &'static str,
    destroy_abilities: &'static [Ability],
    material_types: &'static [MaterialType],
    colors: &'static [Color],
) -> CardProperties {
    CardProperties { name, kind: CardKind::Material, destroy_abilities, colors, material_types, workshop_abilities: &[] }
}

const fn action(name: &'static str, destroy_abilities: &'static [Ability], workshop_abilities: &'static [Ability]) -> CardProperties {
    CardProperties { name, kind: CardKind::Action, destroy_abilities, colors: &[], material_types: &[], workshop_abilities }
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
    dye("Lapis Lazuli", WORKSHOP_3, &[Color::Indigo]),
    dye("Cochineal", WORKSHOP_3, &[Color::Magenta]),
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
    action("Warehouse", MOVE_TO_DRAFT_POOL, &[Ability::GainSilver { count: 2 }]),
    action("Gondola", MOVE_TO_DRAFT_POOL, &[Ability::GainBoats { count: 1 }]),
    action("Chalk", MIX_2, &[Ability::GainSilver { count: 1 }]),
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
    pub fn destroy_abilities(&self) -> &'static [Ability] {
        self.props().destroy_abilities
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
    pub silver: u32,
    /// Wild materials: each covers the material a sale needs. Unspent, each is
    /// worth half a point.
    #[serde(default)]
    pub boats: u32,
    /// Ducats from sell cards and gold. The half points for silver and boats
    /// are not in here; `scoring::score_in_half_points` adds them.
    #[serde(skip)]
    pub cached_ducat_score: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionState {
    pub current_player_index: usize,
    pub ability_stack: AbilityStack,
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
    pub destroyed_pile: UnorderedCards,
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
    6
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
    #[serde(rename = "moveToDraftPool")]
    MoveToDraftPool { card: Option<Card> },
    #[serde(rename = "selectSellCard")]
    SelectSellCard {
        #[serde(rename = "sellCard", alias = "sell_card")]
        sell_card: SellCard,
    },
    #[serde(rename = "gainSecondary")]
    GainSecondary { color: Color },
    #[serde(rename = "gainPrimary")]
    GainPrimary { color: Color },
    #[serde(rename = "gainMaterial")]
    GainMaterial { material: MaterialType },
    #[serde(rename = "mixAll")]
    MixAll { mixes: MixSequence },
    #[serde(rename = "destroyAndMix", alias = "destroyAndMixAll")]
    DestroyAndMix { card: Card, mixes: MixSequence },
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
    #[serde(rename = "destroyAndMoveToDraftPool")]
    DestroyAndMoveToDraftPool {
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
    /// Take the top of the sell card deck unseen, rather than one of the five
    /// face up.
    #[serde(rename = "drawBuyer")]
    DrawBuyer,
}
