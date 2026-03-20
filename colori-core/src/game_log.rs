use serde::{Deserialize, Serialize};

use crate::scoring::HeuristicParams;
use crate::types::{SellCardInstance, CardInstance, Choice, ColorWheel, Materials};

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
    #[serde(default)]
    pub player_time_ms: Vec<u64>,
    #[serde(default)]
    pub player_iterations: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_variants: Option<Vec<PlayerVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub early_termination_savings: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtree_reuse_savings: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGameState {
    pub players: Vec<LogPlayerState>,
    pub draft_deck: Vec<CardInstance>,
    pub destroyed_pile: Vec<CardInstance>,
    pub sell_card_deck: Vec<SellCardInstance>,
    pub sell_card_display: Vec<SellCardInstance>,
    pub round: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPlayerState {
    pub deck: Vec<CardInstance>,
    pub discard: Vec<CardInstance>,
    #[serde(default)]
    pub workshopped_cards: Vec<CardInstance>,
    pub workshop_cards: Vec<CardInstance>,
    pub drafted_cards: Vec<CardInstance>,
    pub color_wheel: ColorWheel,
    pub materials: Materials,
    pub completed_sell_cards: Vec<SellCardInstance>,
    pub ducats: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalScore {
    pub name: String,
    pub score: u32,
    #[serde(default)]
    pub completed_sell_cards: u32,
    #[serde(default)]
    pub color_wheel_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalPlayerStats {
    pub name: String,
    pub deck_size: usize,
    pub completed_sell_cards: Vec<SellCardInstance>,
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
    pub choice: Choice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerVariant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    pub iterations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_limit_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exploration_constant: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rollout_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic_params: Option<HeuristicParams>,
}
