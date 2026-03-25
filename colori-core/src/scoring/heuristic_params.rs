use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct HeuristicParams {
    #[serde(alias = "primaryPipWeight")]
    pub primary_color_value: f64,
    #[serde(alias = "secondaryPipWeight")]
    pub secondary_color_value: f64,
    #[serde(alias = "tertiaryPipWeight")]
    pub tertiary_color_value: f64,
    pub stored_material_weight: f64,
    pub chalk_quality: f64,
    pub action_quality: f64,
    pub dye_quality: f64,
    pub basic_dye_quality: f64,
    pub starter_material_quality: f64,
    pub draft_material_quality: f64,
    pub dual_material_quality: f64,
    #[serde(alias = "buyerMaterialWeight")]
    pub sell_card_material_alignment: f64,
    #[serde(alias = "buyerColorWeight")]
    pub sell_card_color_alignment: f64,
    pub glass_weight: f64,
    pub heuristic_round_threshold: u32,
    pub heuristic_lookahead: u32,
    // Per-action-card quality overrides
    pub alum_quality: Option<f64>,
    pub cream_of_tartar_quality: Option<f64>,
    pub gum_arabic_quality: Option<f64>,
    pub potash_quality: Option<f64>,
    pub vinegar_quality: Option<f64>,
    pub argol_quality: Option<f64>,
    // Per-dye-type quality overrides
    pub pure_primary_dye_quality: Option<f64>,
    pub primary_dye_quality: Option<f64>,
    pub secondary_dye_quality: Option<f64>,
    pub tertiary_dye_quality: Option<f64>,
    // New scoring terms
    pub primary_color_coverage_weight: f64,
    pub secondary_color_coverage_weight: f64,
    pub cards_in_deck_weight: f64,
    pub cards_in_deck_squared_weight: f64,
    pub material_type_count_weight: f64,
    pub material_coverage_weight: f64,
    // Score-based heuristic threshold
    pub heuristic_score_threshold: Option<f64>,
}

impl Default for HeuristicParams {
    /// Trained values from genetic-algorithm/batch-rqo1vv-gen-18.json
    fn default() -> Self {
        HeuristicParams {
            primary_color_value: 0.058344015021050195,
            secondary_color_value: 0.16009515188384604,
            tertiary_color_value: 0.039995983373095643,
            stored_material_weight: 0.0852665976284885,
            chalk_quality: 0.3252105643620574,
            action_quality: 2.128632339864752,
            dye_quality: 1.6849083982631334,
            basic_dye_quality: 0.03440411806299383,
            starter_material_quality: 0.35646705410956914,
            draft_material_quality: 0.22841758712894503,
            dual_material_quality: 0.42737233013877646,
            sell_card_material_alignment: 0.5103003108821427,
            sell_card_color_alignment: 1.5172566875413194,
            glass_weight: 1.0,
            heuristic_round_threshold: 6,
            heuristic_lookahead: 9,
            alum_quality: None,
            cream_of_tartar_quality: None,
            gum_arabic_quality: None,
            potash_quality: None,
            vinegar_quality: None,
            argol_quality: None,
            pure_primary_dye_quality: None,
            primary_dye_quality: None,
            secondary_dye_quality: None,
            tertiary_dye_quality: None,
            primary_color_coverage_weight: 0.0,
            secondary_color_coverage_weight: 0.0,
            cards_in_deck_weight: 0.0,
            cards_in_deck_squared_weight: 0.0,
            material_type_count_weight: 0.0,
            material_coverage_weight: 0.0,
            heuristic_score_threshold: None,
        }
    }
}
