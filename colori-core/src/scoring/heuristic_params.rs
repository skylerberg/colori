use serde::{Deserialize, Serialize};

/// A set of heuristic weights.
///
/// [`Default`] is the neutral table rather than zeros: every card quality is
/// one, so a card is worth a unit of its kind and no card outranks another.
/// Nothing about stored colours or materials is priced, and the rollout policy
/// stays inert. A trained file is what real play wants; the neutral table
/// exists so that card quality is not silently zero when no file is given.
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
    /// Premium on a wild primary over the stored primary it stands in for.
    pub wild_primary_multiplier: f64,
    /// Premium on a boat over the best stored material it stands in for.
    pub boat_multiplier: f64,
    /// Premium on a ducat still in hand, over the point it scores at game
    /// end: a held ducat is also the option to buy a Workshop, Mix or Sell.
    pub unspent_ducat_multiplier: f64,
    pub heuristic_round_threshold: u32,
    pub heuristic_lookahead: u32,
    pub alum_quality: f64,
    pub gum_arabic_quality: f64,
    pub warehouse_quality: f64,
    pub gondola_quality: f64,
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
    pub rollout_trash_with_targets: u32,
    pub rollout_trash_no_targets: u32,
    pub rollout_end_turn_threshold: u32,
    pub rollout_end_turn_probability_early: f64,
    pub rollout_end_turn_probability_late: f64,
    pub rollout_end_turn_max_round: u32,
    pub rollout_ws_material_base_multiplier: u32,
    pub rollout_ws_material_colors_met_multiplier: u32,
    pub rollout_ws_action_gain_ducats_value: u32,
    pub rollout_ws_action_color_demand_multiplier: u32,
}

/// Raw deserialization helper that supports both the old unified `rolloutWsActionBonus`
/// field and the new per-ability fields. When the old field is present and a new field
/// is absent, the old value is used as the default for each new field.
///
/// The multiplier fields default to 1.0 — the value that leaves the term they
/// scale unchanged — so parameter files written before they existed keep the
/// behaviour they were trained for.
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
    #[serde(default)]
    wild_primary_multiplier: Option<f64>,
    #[serde(default)]
    boat_multiplier: Option<f64>,
    #[serde(default)]
    unspent_ducat_multiplier: Option<f64>,
    heuristic_round_threshold: u32,
    heuristic_lookahead: u32,
    alum_quality: f64,
    gum_arabic_quality: f64,
    #[serde(default)]
    warehouse_quality: f64,
    #[serde(default)]
    gondola_quality: f64,
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
    #[serde(alias = "rolloutDestroyWithTargets")]
    rollout_trash_with_targets: u32,
    #[serde(alias = "rolloutDestroyNoTargets")]
    rollout_trash_no_targets: u32,
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
            wild_primary_multiplier: raw.wild_primary_multiplier.unwrap_or(1.0).max(0.0),
            boat_multiplier: raw.boat_multiplier.unwrap_or(1.0).max(0.0),
            unspent_ducat_multiplier: raw.unspent_ducat_multiplier.unwrap_or(1.0).max(0.0),
            heuristic_round_threshold: raw.heuristic_round_threshold,
            heuristic_lookahead: raw.heuristic_lookahead,
            alum_quality: raw.alum_quality,
            gum_arabic_quality: raw.gum_arabic_quality,
            warehouse_quality: raw.warehouse_quality,
            gondola_quality: raw.gondola_quality,
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
            rollout_trash_with_targets: raw.rollout_trash_with_targets,
            rollout_trash_no_targets: raw.rollout_trash_no_targets,
            rollout_end_turn_threshold: raw.rollout_end_turn_threshold,
            rollout_end_turn_probability_early: raw.rollout_end_turn_probability_early,
            rollout_end_turn_probability_late: raw.rollout_end_turn_probability_late,
            rollout_end_turn_max_round: raw.rollout_end_turn_max_round,
            rollout_ws_material_base_multiplier: raw.rollout_ws_material_base_multiplier,
            rollout_ws_material_colors_met_multiplier: raw.rollout_ws_material_colors_met_multiplier,
            rollout_ws_action_gain_ducats_value: raw.rollout_ws_action_gain_ducats_value.unwrap_or(bonus),
            rollout_ws_action_color_demand_multiplier: raw.rollout_ws_action_color_demand_multiplier.unwrap_or(bonus),
        }
    }
}

impl Default for HeuristicParams {
    fn default() -> Self {
        HeuristicParams {
            // Nothing about the position is priced: these are turned on by
            // training, not by default.
            primary_color_value: 0.0,
            secondary_color_value: 0.0,
            tertiary_color_value: 0.0,
            stored_ceramics_weight: 0.0,
            stored_paintings_weight: 0.0,
            stored_textiles_weight: 0.0,
            deck_thinning_value: 0.0,
            // Every card weighs the same.
            chalk_quality: 1.0,
            basic_dye_quality: 1.0,
            starter_material_quality: 1.0,
            ceramics_material_quality: 1.0,
            paintings_material_quality: 1.0,
            textiles_material_quality: 1.0,
            dual_material_quality: 1.0,
            alum_quality: 1.0,
            gum_arabic_quality: 1.0,
            warehouse_quality: 1.0,
            gondola_quality: 1.0,
            linseed_oil_quality: 1.0,
            primary_dye_quality: 1.0,
            secondary_dye_quality: 1.0,
            tertiary_dye_quality: 1.0,
            sell_card_material_alignment: 0.0,
            sell_card_color_alignment: 0.0,
            // These three scale a term rather than adding one, so one is the
            // value that leaves it alone.
            wild_primary_multiplier: 1.0,
            boat_multiplier: 1.0,
            unspent_ducat_multiplier: 1.0,
            heuristic_round_threshold: 0,
            heuristic_lookahead: 1,
            // The rollout policy is inert until a file supplies these.
            rollout_epsilon: 0.0,
            rollout_sell_affordable_multiplier: 0,
            rollout_sell_base: 0,
            rollout_mix_base: 0,
            rollout_mix_pair_weight: 0,
            rollout_mix_count_weight: 0,
            rollout_mix_no_pairs: 0,
            rollout_workshop_base: 0,
            rollout_workshop_count_weight: 0,
            rollout_workshop_empty: 0,
            rollout_trash_with_targets: 0,
            rollout_trash_no_targets: 0,
            rollout_end_turn_threshold: 0,
            rollout_end_turn_probability_early: 0.0,
            rollout_end_turn_probability_late: 0.0,
            // Must be at least two; the rollout checks it.
            rollout_end_turn_max_round: 2,
            rollout_ws_material_base_multiplier: 0,
            rollout_ws_material_colors_met_multiplier: 0,
            rollout_ws_action_gain_ducats_value: 0,
            rollout_ws_action_color_demand_multiplier: 0,
        }
    }
}
