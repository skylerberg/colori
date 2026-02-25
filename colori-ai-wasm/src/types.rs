use serde::{Deserialize, Serialize};

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
    pub fn index(self) -> usize {
        self as usize
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AnyCard {
    #[serde(rename = "dye")]
    Dye {
        name: String,
        colors: Vec<Color>,
        ability: Ability,
    },
    #[serde(rename = "basicDye")]
    BasicDye {
        name: String,
        color: Color,
        ability: Ability,
    },
    #[serde(rename = "material")]
    Material {
        name: String,
        #[serde(rename = "materialTypes")]
        material_types: Vec<MaterialType>,
        #[serde(rename = "colorPip", skip_serializing_if = "Option::is_none", default)]
        color_pip: Option<Color>,
        ability: Ability,
    },
    #[serde(rename = "action")]
    Action {
        name: String,
        ability: Ability,
        #[serde(rename = "workshopAbilities")]
        workshop_abilities: Vec<Ability>,
    },
    #[serde(rename = "buyer")]
    Buyer {
        stars: u32,
        #[serde(rename = "requiredMaterial")]
        required_material: MaterialType,
        #[serde(rename = "colorCost")]
        color_cost: Vec<Color>,
    },
}

impl AnyCard {
    pub fn get_pips(&self) -> Vec<Color> {
        match self {
            AnyCard::Dye { colors, .. } => colors.clone(),
            AnyCard::BasicDye { color, .. } => vec![*color],
            AnyCard::Material { color_pip, .. } => {
                color_pip.map(|c| vec![c]).unwrap_or_default()
            }
            AnyCard::Action { .. } => vec![],
            AnyCard::Buyer { .. } => vec![],
        }
    }

    pub fn get_ability(&self) -> &Ability {
        match self {
            AnyCard::Dye { ability, .. } => ability,
            AnyCard::BasicDye { ability, .. } => ability,
            AnyCard::Material { ability, .. } => ability,
            AnyCard::Action { ability, .. } => ability,
            AnyCard::Buyer { .. } => panic!("Buyer cards do not have abilities"),
        }
    }

    #[allow(dead_code)]
    pub fn kind_str(&self) -> &str {
        match self {
            AnyCard::Dye { .. } => "dye",
            AnyCard::BasicDye { .. } => "basicDye",
            AnyCard::Material { .. } => "material",
            AnyCard::Action { .. } => "action",
            AnyCard::Buyer { .. } => "buyer",
        }
    }

    #[allow(dead_code)]
    pub fn is_buyer(&self) -> bool {
        matches!(self, AnyCard::Buyer { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardInstance {
    pub instance_id: u32,
    pub card: AnyCard,
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

    pub fn get(&self, color: Color) -> u32 {
        self.counts[color.index()]
    }

    pub fn set(&mut self, color: Color, value: u32) {
        self.counts[color.index()] = value;
    }

    pub fn increment(&mut self, color: Color) {
        self.counts[color.index()] += 1;
    }

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

    pub fn get(&self, mt: MaterialType) -> u32 {
        self.counts[mt as usize]
    }

    pub fn increment(&mut self, mt: MaterialType) {
        self.counts[mt as usize] += 1;
    }

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
    pub name: String,
    pub deck: Vec<CardInstance>,
    pub discard: Vec<CardInstance>,
    pub workshop_cards: Vec<CardInstance>,
    pub drafted_cards: Vec<CardInstance>,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    pub completed_buyers: Vec<CardInstance>,
    pub ducats: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftState {
    pub pick_number: u32,
    pub current_player_index: usize,
    pub hands: Vec<Vec<CardInstance>>,
    pub direction: i32,
    pub waiting_for_pass: bool,
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
    #[serde(rename = "chooseTertiaryToGain")]
    ChooseTertiaryToGain {
        #[serde(rename = "lostColor")]
        lost_color: Color,
    },
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
    pub players: Vec<PlayerState>,
    pub draft_deck: Vec<CardInstance>,
    pub destroyed_pile: Vec<CardInstance>,
    pub buyer_deck: Vec<CardInstance>,
    pub buyer_display: Vec<CardInstance>,
    pub phase: GamePhase,
    pub round: u32,
    pub ai_players: Vec<bool>,
}

// ── ColoriChoice ──

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        #[serde(rename = "cardInstanceIds")]
        card_instance_ids: Vec<u32>,
    },
    #[serde(rename = "skipWorkshop")]
    SkipWorkshop,
    #[serde(rename = "destroyDrawnCards")]
    DestroyDrawnCards {
        #[serde(rename = "cardInstanceIds")]
        card_instance_ids: Vec<u32>,
    },
    #[serde(rename = "mix")]
    Mix {
        #[serde(rename = "colorA")]
        color_a: Color,
        #[serde(rename = "colorB")]
        color_b: Color,
    },
    #[serde(rename = "skipMix")]
    SkipMix,
    #[serde(rename = "selectBuyer")]
    SelectBuyer {
        #[serde(rename = "buyerInstanceId")]
        buyer_instance_id: u32,
    },
    #[serde(rename = "gainSecondary")]
    GainSecondary { color: Color },
    #[serde(rename = "gainPrimary")]
    GainPrimary { color: Color },
    #[serde(rename = "chooseTertiaryToLose")]
    ChooseTertiaryToLose { color: Color },
    #[serde(rename = "chooseTertiaryToGain")]
    ChooseTertiaryToGain { color: Color },
}
