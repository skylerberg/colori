use serde::{Deserialize, Serialize};

use crate::types::{BuyerInstance, CardInstance, Color, ColorWheel, Materials};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredGameLog {
    pub version: u32,
    pub game_started_at: String,
    pub game_ended_at: Option<String>,
    pub player_names: Vec<String>,
    pub ai_players: Vec<bool>,
    pub initial_state: LogGameState,
    pub final_scores: Option<Vec<FinalScore>>,
    pub final_player_stats: Option<Vec<FinalPlayerStats>>,
    pub entries: Vec<StructuredLogEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_variants: Option<Vec<PlayerVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGameState {
    pub players: Vec<LogPlayerState>,
    pub draft_deck: Vec<CardInstance>,
    pub destroyed_pile: Vec<CardInstance>,
    pub buyer_deck: Vec<BuyerInstance>,
    pub buyer_display: Vec<BuyerInstance>,
    pub round: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPlayerState {
    pub deck: Vec<CardInstance>,
    pub discard: Vec<CardInstance>,
    #[serde(default)]
    pub used_cards: Vec<CardInstance>,
    pub workshop_cards: Vec<CardInstance>,
    pub drafted_cards: Vec<CardInstance>,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    pub completed_buyers: Vec<BuyerInstance>,
    pub ducats: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalScore {
    pub name: String,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalPlayerStats {
    pub name: String,
    pub deck_size: usize,
    pub completed_buyers: Vec<BuyerInstance>,
    pub ducats: u32,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredLogEntry {
    pub seq: u32,
    pub timestamp: u64,
    pub round: u32,
    pub phase: String,
    pub player_index: usize,
    pub choice: LogChoice,
}

// LogChoice is identical to ColoriChoice but uses Vec<u32> instead of UnorderedCards for card IDs.
// This is needed because ColoriChoice uses UnorderedCards for the card_instance_ids fields,
// and UnorderedCards relies on thread-local registries for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogChoice {
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
    MixAll { mixes: Vec<(Color, Color)> },
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
        mixes: Vec<(Color, Color)>,
    },
    #[serde(rename = "destroyAndSell")]
    DestroyAndSell {
        #[serde(rename = "cardInstanceId")]
        card_instance_id: u32,
        #[serde(rename = "buyerInstanceId")]
        buyer_instance_id: u32,
    },
    #[serde(rename = "keepWorkshopCards")]
    KeepWorkshopCards {
        #[serde(rename = "cardInstanceIds")]
        card_instance_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerVariant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub iterations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exploration_constant: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rollout_steps: Option<u32>,
}
