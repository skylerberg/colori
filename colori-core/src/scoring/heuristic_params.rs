use serde::{Deserialize, Serialize};

fn default_exploration_constant() -> f64 {
    std::f64::consts::SQRT_2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicParams {
    #[serde(default = "default_exploration_constant")]
    pub exploration_constant: f64,
    #[serde(default)]
    pub rollout_destroy_worst: bool,
    #[serde(alias = "primaryPipWeight")]
    pub primary_color_value: f64,
    #[serde(alias = "secondaryPipWeight")]
    pub secondary_color_value: f64,
    #[serde(alias = "tertiaryPipWeight")]
    pub tertiary_color_value: f64,
    #[serde(default)]
    pub stored_ceramics_weight: f64,
    #[serde(default)]
    pub stored_paintings_weight: f64,
    #[serde(default)]
    pub stored_textiles_weight: f64,
    #[serde(default)]
    pub deck_thinning_value: f64,
    pub chalk_quality: f64,
    pub basic_dye_quality: f64,
    pub starter_material_quality: f64,
    #[serde(default)]
    pub ceramics_material_quality: f64,
    #[serde(default)]
    pub paintings_material_quality: f64,
    #[serde(default)]
    pub textiles_material_quality: f64,
    pub dual_material_quality: f64,
    #[serde(alias = "buyerMaterialWeight")]
    pub sell_card_material_alignment: f64,
    #[serde(alias = "buyerColorWeight")]
    pub sell_card_color_alignment: f64,
    pub heuristic_round_threshold: u32,
    pub heuristic_lookahead: u32,
    pub alum_quality: f64,
    pub cream_of_tartar_quality: f64,
    pub gum_arabic_quality: f64,
    pub potash_quality: f64,
    pub vinegar_quality: f64,
    pub linseed_oil_quality: f64,
    pub primary_dye_quality: f64,
    pub secondary_dye_quality: f64,
    pub tertiary_dye_quality: f64,
    // Rollout policy parameters
    pub rollout_epsilon: f64,
    pub rollout_sell_affordable_multiplier: u32,
    pub rollout_sell_base: u32,
    pub rollout_mix_base: u32,
    pub rollout_mix_pair_weight: u32,
    pub rollout_mix_count_weight: u32,
    pub rollout_mix_no_pairs: u32,
    pub rollout_workshop_base: u32,
    pub rollout_workshop_count_weight: u32,
    pub rollout_workshop_empty: u32,
    pub rollout_destroy_with_targets: u32,
    pub rollout_destroy_no_targets: u32,
    pub rollout_draw_base: u32,
    pub rollout_draw_count_weight: u32,
    pub rollout_other_priority: u32,
    pub rollout_end_turn_threshold: u32,
    pub rollout_end_turn_probability_early: f64,
    pub rollout_end_turn_probability_late: f64,
    pub rollout_end_turn_max_round: u32,
    pub rollout_ws_material_base_multiplier: u32,
    pub rollout_ws_material_colors_met_multiplier: u32,
    pub rollout_ws_action_gain_ducats_value: u32,
    pub rollout_ws_action_draw_value: u32,
    pub rollout_ws_action_workshop_per_card: u32,
    pub rollout_ws_action_color_demand_multiplier: u32,
}
