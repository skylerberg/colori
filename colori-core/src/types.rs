use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::fixed_vec::FixedVec;
use crate::unordered_cards::{UnorderedBuyers, UnorderedCards};

pub const MAX_PLAYERS: usize = 4;
pub const MAX_BUYER_DISPLAY: usize = 6;

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
    DestroyCards { count: u32 },
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
}

// ── CardKind ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Dye,
    BasicDye,
    Material,
    Action,
}

// ── Card enum (42 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Card {
    // Basic dyes (3)
    BasicRed,
    BasicYellow,
    BasicBlue,
    // Primary dyes (3)
    Kermes,
    Weld,
    Woad,
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
    // Double materials (3)
    FineCeramics,
    FinePaintings,
    FineTextiles,
    // Material+pip Ceramics (3)
    TerraCotta,
    OchreWare,
    CobaltWare,
    // Material+pip Paintings (3)
    CinnabarCanvas,
    OrpimentCanvas,
    UltramarineCanvas,
    // Material+pip Textiles (3)
    AlizarinFabric,
    FusticFabric,
    PastelFabric,
    // Dual materials (3)
    ClayCanvas,
    ClayFabric,
    CanvasFabric,
    // Actions (6)
    Alum,
    CreamOfTartar,
    GumArabic,
    Potash,
    Vinegar,
    Chalk,
}

struct CardProperties {
    name: &'static str,
    kind: CardKind,
    ability: Ability,
    pips: &'static [Color],
    material_types: &'static [MaterialType],
    workshop_abilities: &'static [Ability],
}

const CARD_DATA: [CardProperties; 42] = [
    // BasicRed
    CardProperties { name: "Basic Red", kind: CardKind::BasicDye, ability: Ability::Sell, pips: &[Color::Red], material_types: &[], workshop_abilities: &[] },
    // BasicYellow
    CardProperties { name: "Basic Yellow", kind: CardKind::BasicDye, ability: Ability::Sell, pips: &[Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // BasicBlue
    CardProperties { name: "Basic Blue", kind: CardKind::BasicDye, ability: Ability::Sell, pips: &[Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Kermes
    CardProperties { name: "Kermes", kind: CardKind::Dye, ability: Ability::Sell, pips: &[Color::Red, Color::Red, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Weld
    CardProperties { name: "Weld", kind: CardKind::Dye, ability: Ability::Sell, pips: &[Color::Yellow, Color::Yellow, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Woad
    CardProperties { name: "Woad", kind: CardKind::Dye, ability: Ability::Sell, pips: &[Color::Blue, Color::Blue, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Madder
    CardProperties { name: "Madder", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Orange, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Turmeric
    CardProperties { name: "Turmeric", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Orange, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // DyersGreenweed
    CardProperties { name: "Dyer's Greenweed", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Green, Color::Yellow], material_types: &[], workshop_abilities: &[] },
    // Verdigris
    CardProperties { name: "Verdigris", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Green, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // Orchil
    CardProperties { name: "Orchil", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Purple, Color::Red], material_types: &[], workshop_abilities: &[] },
    // Logwood
    CardProperties { name: "Logwood", kind: CardKind::Dye, ability: Ability::Workshop { count: 3 }, pips: &[Color::Purple, Color::Blue], material_types: &[], workshop_abilities: &[] },
    // VermilionDye
    CardProperties { name: "Vermilion", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Vermilion], material_types: &[], workshop_abilities: &[] },
    // Saffron
    CardProperties { name: "Saffron", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Amber], material_types: &[], workshop_abilities: &[] },
    // PersianBerries
    CardProperties { name: "Persian Berries", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Chartreuse], material_types: &[], workshop_abilities: &[] },
    // Azurite
    CardProperties { name: "Azurite", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Teal], material_types: &[], workshop_abilities: &[] },
    // IndigoDye
    CardProperties { name: "Indigo", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Indigo], material_types: &[], workshop_abilities: &[] },
    // Cochineal
    CardProperties { name: "Cochineal", kind: CardKind::Dye, ability: Ability::MixColors { count: 2 }, pips: &[Color::Magenta], material_types: &[], workshop_abilities: &[] },
    // StarterCeramics
    CardProperties { name: "Ceramics", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // StarterPaintings
    CardProperties { name: "Paintings", kind: CardKind::Material, ability: Ability::Sell, pips: &[], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // StarterTextiles
    CardProperties { name: "Textiles", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // FineCeramics
    CardProperties { name: "Fine Ceramics", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[], material_types: &[MaterialType::Ceramics, MaterialType::Ceramics], workshop_abilities: &[] },
    // FinePaintings
    CardProperties { name: "Fine Paintings", kind: CardKind::Material, ability: Ability::Sell, pips: &[], material_types: &[MaterialType::Paintings, MaterialType::Paintings], workshop_abilities: &[] },
    // FineTextiles
    CardProperties { name: "Fine Textiles", kind: CardKind::Material, ability: Ability::DrawCards { count: 2 }, pips: &[], material_types: &[MaterialType::Textiles, MaterialType::Textiles], workshop_abilities: &[] },
    // TerraCotta
    CardProperties { name: "Terra Cotta", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[Color::Red], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // OchreWare
    CardProperties { name: "Ochre Ware", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[Color::Yellow], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // CobaltWare
    CardProperties { name: "Cobalt Ware", kind: CardKind::Material, ability: Ability::Workshop { count: 2 }, pips: &[Color::Blue], material_types: &[MaterialType::Ceramics], workshop_abilities: &[] },
    // CinnabarCanvas
    CardProperties { name: "Cinnabar & Canvas", kind: CardKind::Material, ability: Ability::Sell, pips: &[Color::Red], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // OrpimentCanvas
    CardProperties { name: "Orpiment & Canvas", kind: CardKind::Material, ability: Ability::Sell, pips: &[Color::Yellow], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // UltramarineCanvas
    CardProperties { name: "Ultramarine & Canvas", kind: CardKind::Material, ability: Ability::Sell, pips: &[Color::Blue], material_types: &[MaterialType::Paintings], workshop_abilities: &[] },
    // AlizarinFabric
    CardProperties { name: "Alizarin & Fabric", kind: CardKind::Material, ability: Ability::DrawCards { count: 2 }, pips: &[Color::Red], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // FusticFabric
    CardProperties { name: "Fustic & Fabric", kind: CardKind::Material, ability: Ability::DrawCards { count: 2 }, pips: &[Color::Yellow], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // PastelFabric
    CardProperties { name: "Pastel & Fabric", kind: CardKind::Material, ability: Ability::DrawCards { count: 2 }, pips: &[Color::Blue], material_types: &[MaterialType::Textiles], workshop_abilities: &[] },
    // ClayCanvas
    CardProperties { name: "Clay & Canvas", kind: CardKind::Material, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[MaterialType::Ceramics, MaterialType::Paintings], workshop_abilities: &[] },
    // ClayFabric
    CardProperties { name: "Clay & Fabric", kind: CardKind::Material, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[MaterialType::Ceramics, MaterialType::Textiles], workshop_abilities: &[] },
    // CanvasFabric
    CardProperties { name: "Canvas & Fabric", kind: CardKind::Material, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[MaterialType::Paintings, MaterialType::Textiles], workshop_abilities: &[] },
    // Alum
    CardProperties { name: "Alum", kind: CardKind::Action, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[], workshop_abilities: &[Ability::GainDucats { count: 1 }] },
    // CreamOfTartar
    CardProperties { name: "Cream of Tartar", kind: CardKind::Action, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[], workshop_abilities: &[Ability::DrawCards { count: 3 }] },
    // GumArabic
    CardProperties { name: "Gum Arabic", kind: CardKind::Action, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[], workshop_abilities: &[Ability::GainSecondary] },
    // Potash
    CardProperties { name: "Potash", kind: CardKind::Action, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[], workshop_abilities: &[Ability::Workshop { count: 3 }] },
    // Vinegar
    CardProperties { name: "Vinegar", kind: CardKind::Action, ability: Ability::DestroyCards { count: 1 }, pips: &[], material_types: &[], workshop_abilities: &[Ability::ChangeTertiary] },
    // Chalk
    CardProperties { name: "Chalk", kind: CardKind::Action, ability: Ability::Sell, pips: &[], material_types: &[], workshop_abilities: &[Ability::GainPrimary] },
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
    pub fn pips(&self) -> &'static [Color] {
        self.props().pips
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

// ── BuyerCard enum (51 variants) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuyerCard {
    // Textiles 2-star, single tertiary (6)
    Textiles2Vermilion,
    Textiles2Amber,
    Textiles2Chartreuse,
    Textiles2Teal,
    Textiles2Indigo,
    Textiles2Magenta,
    // Textiles 2-star, secondary+primary (9)
    Textiles2OrangeRed,
    Textiles2OrangeYellow,
    Textiles2OrangeBlue,
    Textiles2GreenRed,
    Textiles2GreenYellow,
    Textiles2GreenBlue,
    Textiles2PurpleRed,
    Textiles2PurpleYellow,
    Textiles2PurpleBlue,
    // Ceramics 3-star, tertiary+primary (18)
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
    // Paintings 4-star, tertiary+secondary (18)
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

struct BuyerProperties {
    stars: u32,
    required_material: MaterialType,
    color_cost: &'static [Color],
}

const BUYER_DATA: [BuyerProperties; 51] = [
    // Textiles 2-star, single tertiary (6)
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Vermilion] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Amber] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Chartreuse] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Teal] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Indigo] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Magenta] },
    // Textiles 2-star, secondary+primary (9)
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Red] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Yellow] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Orange, Color::Blue] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Red] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Yellow] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Green, Color::Blue] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Red] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Yellow] },
    BuyerProperties { stars: 2, required_material: MaterialType::Textiles, color_cost: &[Color::Purple, Color::Blue] },
    // Ceramics 3-star, tertiary+primary (18)
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Vermilion, Color::Blue] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Amber, Color::Blue] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Chartreuse, Color::Blue] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Teal, Color::Blue] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Indigo, Color::Blue] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Red] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Yellow] },
    BuyerProperties { stars: 3, required_material: MaterialType::Ceramics, color_cost: &[Color::Magenta, Color::Blue] },
    // Paintings 4-star, tertiary+secondary (18)
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Vermilion, Color::Purple] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Amber, Color::Purple] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Chartreuse, Color::Purple] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Teal, Color::Purple] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Indigo, Color::Purple] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Orange] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Green] },
    BuyerProperties { stars: 4, required_material: MaterialType::Paintings, color_cost: &[Color::Magenta, Color::Purple] },
];

impl BuyerCard {
    #[inline]
    fn props(&self) -> &'static BuyerProperties {
        &BUYER_DATA[*self as usize]
    }

    #[inline]
    pub fn stars(&self) -> u32 {
        self.props().stars
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

// ── CardInstance / BuyerInstance ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInstance {
    pub instance_id: u32,
    pub card: Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyerInstance {
    pub instance_id: u32,
    #[serde(rename = "card")]
    pub buyer: BuyerCard,
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
    pub fn get(&self, mt: MaterialType) -> u32 {
        self.counts[mt as usize]
    }

    #[inline]
    pub fn increment(&mut self, mt: MaterialType) {
        self.counts[mt as usize] += 1;
    }

    #[inline]
    pub fn decrement(&mut self, mt: MaterialType) -> bool {
        let idx = mt as usize;
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
        for &mt in &ALL_MATERIAL_TYPES {
            map.serialize_entry(&mt, &self.counts[mt as usize])?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Materials {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: std::collections::HashMap<MaterialType, u32> =
            std::collections::HashMap::deserialize(deserializer)?;
        let mut materials = Materials::new();
        for (&mt, &count) in &map {
            materials.counts[mt as usize] = count;
        }
        Ok(materials)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub deck: UnorderedCards,
    pub discard: UnorderedCards,
    pub workshop_cards: UnorderedCards,
    pub drafted_cards: UnorderedCards,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    pub completed_buyers: Vec<BuyerInstance>,
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
    pub direction: i32,
    pub waiting_for_pass: bool,
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
#[serde(tag = "type")]
pub enum PendingChoice {
    #[serde(rename = "chooseCardsForWorkshop")]
    ChooseCardsForWorkshop { count: u32 },
    #[serde(rename = "chooseCardsToDestroy")]
    ChooseCardsToDestroy { count: u32 },
    #[serde(rename = "chooseMix")]
    ChooseMix { remaining: u32 },
    #[serde(rename = "chooseBuyer")]
    ChooseBuyer,
    #[serde(rename = "chooseSecondaryColor")]
    ChooseSecondaryColor,
    #[serde(rename = "choosePrimaryColor")]
    ChoosePrimaryColor,
    #[serde(rename = "chooseTertiaryToLose")]
    ChooseTertiaryToLose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionState {
    pub current_player_index: usize,
    pub ability_stack: Vec<Ability>,
    pub pending_choice: Option<PendingChoice>,
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
    pub buyer_deck: UnorderedBuyers,
    pub buyer_display: FixedVec<BuyerInstance, MAX_BUYER_DISPLAY>,
    pub phase: GamePhase,
    pub round: u32,
    pub ai_players: FixedVec<bool, MAX_PLAYERS>,
    #[serde(skip, default = "default_card_lookup")]
    pub card_lookup: [Card; 128],
    #[serde(skip, default = "default_buyer_lookup")]
    pub buyer_lookup: [BuyerCard; 128],
}

fn default_card_lookup() -> [Card; 128] {
    [Card::BasicRed; 128]
}

fn default_buyer_lookup() -> [BuyerCard; 128] {
    [BuyerCard::Textiles2Vermilion; 128]
}

// ── Serde helpers for UnorderedCards as Vec<u32> ──

fn serialize_ids<S: serde::Serializer>(cards: &UnorderedCards, s: S) -> Result<S::Ok, S::Error> {
    let ids: Vec<u32> = cards.iter().map(|id| id as u32).collect();
    ids.serialize(s)
}

fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<UnorderedCards, D::Error> {
    let ids = Vec::<u32>::deserialize(d)?;
    let mut cards = UnorderedCards::new();
    for id in ids {
        cards.insert(id as u8);
    }
    Ok(cards)
}

// ── ColoriChoice ──

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColoriChoice {
    #[serde(rename = "draftPick")]
    DraftPick {
        #[serde(rename = "cardInstanceId")]
        card_instance_id: u32,
    },
    #[serde(rename = "destroyDraftedCard")]
    DestroyDraftedCard {
        #[serde(rename = "cardInstanceId")]
        card_instance_id: u32,
    },
    #[serde(rename = "endTurn")]
    EndTurn,
    #[serde(rename = "workshop")]
    Workshop {
        #[serde(
            rename = "cardInstanceIds",
            serialize_with = "serialize_ids",
            deserialize_with = "deserialize_ids"
        )]
        card_instance_ids: UnorderedCards,
    },
    #[serde(rename = "skipWorkshop")]
    SkipWorkshop,
    #[serde(rename = "destroyDrawnCards")]
    DestroyDrawnCards {
        #[serde(
            rename = "cardInstanceIds",
            serialize_with = "serialize_ids",
            deserialize_with = "deserialize_ids"
        )]
        card_instance_ids: UnorderedCards,
    },
    #[serde(rename = "selectBuyer")]
    SelectBuyer {
        #[serde(rename = "buyerInstanceId")]
        buyer_instance_id: u32,
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
    #[serde(rename = "destroyAndMixAll")]
    DestroyAndMixAll {
        #[serde(rename = "cardInstanceId")]
        card_instance_id: u32,
        mixes: SmallVec<[(Color, Color); 2]>,
    },
    #[serde(rename = "destroyAndSell")]
    DestroyAndSell {
        #[serde(rename = "cardInstanceId")]
        card_instance_id: u32,
        #[serde(rename = "buyerInstanceId")]
        buyer_instance_id: u32,
    },
}
