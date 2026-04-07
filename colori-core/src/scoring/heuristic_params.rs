use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "HeuristicParamsRaw")]
pub struct HeuristicParams {
    pub primary_color_value: f64,
    pub secondary_color_value: f64,
    pub tertiary_color_value: f64,
    pub stored_ceramics_weight: f64,
    pub stored_paintings_weight: f64,
    pub stored_textiles_weight: f64,
    pub deck_thinning_value: f64,
    pub chalk_quality: f64,
    pub basic_dye_quality: f64,
    pub starter_material_quality: f64,
    pub ceramics_material_quality: f64,
    pub paintings_material_quality: f64,
    pub textiles_material_quality: f64,
    pub dual_material_quality: f64,
    pub sell_card_material_alignment: f64,
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

/// Raw deserialization helper that supports both the old unified `rolloutWsActionBonus`
/// field and the new per-ability fields. When the old field is present and a new field
/// is absent, the old value is used as the default for each new field.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeuristicParamsRaw {
    #[serde(alias = "primaryPipWeight")]
    primary_color_value: f64,
    #[serde(alias = "secondaryPipWeight")]
    secondary_color_value: f64,
    #[serde(alias = "tertiaryPipWeight")]
    tertiary_color_value: f64,
    #[serde(default)]
    stored_ceramics_weight: f64,
    #[serde(default)]
    stored_paintings_weight: f64,
    #[serde(default)]
    stored_textiles_weight: f64,
    #[serde(default)]
    deck_thinning_value: f64,
    chalk_quality: f64,
    basic_dye_quality: f64,
    starter_material_quality: f64,
    #[serde(default)]
    ceramics_material_quality: f64,
    #[serde(default)]
    paintings_material_quality: f64,
    #[serde(default)]
    textiles_material_quality: f64,
    dual_material_quality: f64,
    #[serde(alias = "buyerMaterialWeight")]
    sell_card_material_alignment: f64,
    #[serde(alias = "buyerColorWeight")]
    sell_card_color_alignment: f64,
    heuristic_round_threshold: u32,
    heuristic_lookahead: u32,
    alum_quality: f64,
    cream_of_tartar_quality: f64,
    gum_arabic_quality: f64,
    potash_quality: f64,
    vinegar_quality: f64,
    linseed_oil_quality: f64,
    primary_dye_quality: f64,
    secondary_dye_quality: f64,
    tertiary_dye_quality: f64,
    // Rollout policy parameters
    rollout_epsilon: f64,
    rollout_sell_affordable_multiplier: u32,
    rollout_sell_base: u32,
    rollout_mix_base: u32,
    rollout_mix_pair_weight: u32,
    rollout_mix_count_weight: u32,
    rollout_mix_no_pairs: u32,
    rollout_workshop_base: u32,
    rollout_workshop_count_weight: u32,
    rollout_workshop_empty: u32,
    rollout_destroy_with_targets: u32,
    rollout_destroy_no_targets: u32,
    rollout_draw_base: u32,
    rollout_draw_count_weight: u32,
    rollout_other_priority: u32,
    rollout_end_turn_threshold: u32,
    rollout_end_turn_probability_early: f64,
    rollout_end_turn_probability_late: f64,
    rollout_end_turn_max_round: u32,
    rollout_ws_material_base_multiplier: u32,
    rollout_ws_material_colors_met_multiplier: u32,
    // Legacy unified field — used as fallback for the per-ability fields below
    #[serde(default)]
    rollout_ws_action_bonus: Option<u32>,
    #[serde(default)]
    rollout_ws_action_gain_ducats_value: Option<u32>,
    #[serde(default)]
    rollout_ws_action_draw_value: Option<u32>,
    #[serde(default)]
    rollout_ws_action_workshop_per_card: Option<u32>,
    #[serde(default)]
    rollout_ws_action_color_demand_multiplier: Option<u32>,
}

impl From<HeuristicParamsRaw> for HeuristicParams {
    fn from(raw: HeuristicParamsRaw) -> Self {
        let bonus = raw.rollout_ws_action_bonus.unwrap_or(0);
        HeuristicParams {
            primary_color_value: raw.primary_color_value,
            secondary_color_value: raw.secondary_color_value,
            tertiary_color_value: raw.tertiary_color_value,
            stored_ceramics_weight: raw.stored_ceramics_weight,
            stored_paintings_weight: raw.stored_paintings_weight,
            stored_textiles_weight: raw.stored_textiles_weight,
            deck_thinning_value: raw.deck_thinning_value,
            chalk_quality: raw.chalk_quality,
            basic_dye_quality: raw.basic_dye_quality,
            starter_material_quality: raw.starter_material_quality,
            ceramics_material_quality: raw.ceramics_material_quality,
            paintings_material_quality: raw.paintings_material_quality,
            textiles_material_quality: raw.textiles_material_quality,
            dual_material_quality: raw.dual_material_quality,
            sell_card_material_alignment: raw.sell_card_material_alignment,
            sell_card_color_alignment: raw.sell_card_color_alignment,
            heuristic_round_threshold: raw.heuristic_round_threshold,
            heuristic_lookahead: raw.heuristic_lookahead,
            alum_quality: raw.alum_quality,
            cream_of_tartar_quality: raw.cream_of_tartar_quality,
            gum_arabic_quality: raw.gum_arabic_quality,
            potash_quality: raw.potash_quality,
            vinegar_quality: raw.vinegar_quality,
            linseed_oil_quality: raw.linseed_oil_quality,
            primary_dye_quality: raw.primary_dye_quality,
            secondary_dye_quality: raw.secondary_dye_quality,
            tertiary_dye_quality: raw.tertiary_dye_quality,
            rollout_epsilon: raw.rollout_epsilon,
            rollout_sell_affordable_multiplier: raw.rollout_sell_affordable_multiplier,
            rollout_sell_base: raw.rollout_sell_base,
            rollout_mix_base: raw.rollout_mix_base,
            rollout_mix_pair_weight: raw.rollout_mix_pair_weight,
            rollout_mix_count_weight: raw.rollout_mix_count_weight,
            rollout_mix_no_pairs: raw.rollout_mix_no_pairs,
            rollout_workshop_base: raw.rollout_workshop_base,
            rollout_workshop_count_weight: raw.rollout_workshop_count_weight,
            rollout_workshop_empty: raw.rollout_workshop_empty,
            rollout_destroy_with_targets: raw.rollout_destroy_with_targets,
            rollout_destroy_no_targets: raw.rollout_destroy_no_targets,
            rollout_draw_base: raw.rollout_draw_base,
            rollout_draw_count_weight: raw.rollout_draw_count_weight,
            rollout_other_priority: raw.rollout_other_priority,
            rollout_end_turn_threshold: raw.rollout_end_turn_threshold,
            rollout_end_turn_probability_early: raw.rollout_end_turn_probability_early,
            rollout_end_turn_probability_late: raw.rollout_end_turn_probability_late,
            rollout_end_turn_max_round: raw.rollout_end_turn_max_round,
            rollout_ws_material_base_multiplier: raw.rollout_ws_material_base_multiplier,
            rollout_ws_material_colors_met_multiplier: raw.rollout_ws_material_colors_met_multiplier,
            rollout_ws_action_gain_ducats_value: raw.rollout_ws_action_gain_ducats_value.unwrap_or(bonus),
            rollout_ws_action_draw_value: raw.rollout_ws_action_draw_value.unwrap_or(bonus),
            rollout_ws_action_workshop_per_card: raw.rollout_ws_action_workshop_per_card.unwrap_or(bonus),
            rollout_ws_action_color_demand_multiplier: raw.rollout_ws_action_color_demand_multiplier.unwrap_or(bonus),
        }
    }
}
