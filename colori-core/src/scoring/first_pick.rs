use serde::{Deserialize, Serialize};

use crate::colors::{is_primary, is_tertiary, SECONDARIES};
use crate::fixed_vec::FixedVec;
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct FirstPickParams {
    pub is_tertiary_dye: f64,
    pub is_secondary_dye: f64,
    pub is_primary_dye: f64,
    pub is_pure_primary_dye: f64,
    pub is_alum: f64,
    pub is_gum_arabic: f64,
    pub is_cream_of_tartar: f64,
    pub is_potash: f64,
    pub is_dual_material: f64,
    pub is_material_plus_color: f64,
    pub matching_tertiary_colors: f64,
    pub matching_secondary_colors: f64,
    pub matching_primary_colors: f64,
    pub matching_materials: f64,
}

impl Default for FirstPickParams {
    fn default() -> Self {
        FirstPickParams {
            is_tertiary_dye: 0.0,
            is_secondary_dye: 0.0,
            is_primary_dye: 0.0,
            is_pure_primary_dye: 0.0,
            is_alum: 0.0,
            is_gum_arabic: 0.0,
            is_cream_of_tartar: 0.0,
            is_potash: 0.0,
            is_dual_material: 0.0,
            is_material_plus_color: 0.0,
            matching_tertiary_colors: 0.0,
            matching_secondary_colors: 0.0,
            matching_primary_colors: 0.0,
            matching_materials: 0.0,
        }
    }
}

impl FirstPickParams {
    pub fn score_card(
        &self,
        card: Card,
        sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    ) -> f64 {
        let mut score = 0.0;

        // Card type features
        match card.kind() {
            CardKind::Dye => {
                let colors = card.colors();
                if colors.len() == 1 && is_tertiary(colors[0]) {
                    score += self.is_tertiary_dye;
                } else if colors.len() == 2 && SECONDARIES.contains(&colors[0]) {
                    score += self.is_secondary_dye;
                } else if colors.len() == 3 && colors.iter().all(|c| is_primary(*c)) {
                    if colors[0] == colors[1] && colors[1] == colors[2] {
                        score += self.is_pure_primary_dye;
                    } else {
                        score += self.is_primary_dye;
                    }
                }
            }
            CardKind::Action => {
                match card {
                    Card::Alum => score += self.is_alum,
                    Card::GumArabic => score += self.is_gum_arabic,
                    Card::CreamOfTartar => score += self.is_cream_of_tartar,
                    Card::Potash => score += self.is_potash,
                    _ => {}
                }
            }
            CardKind::Material => {
                let colors = card.colors();
                let mat_types = card.material_types();
                if mat_types.len() == 2 {
                    score += self.is_dual_material;
                } else if !colors.is_empty() && mat_types.len() == 1 {
                    score += self.is_material_plus_color;
                }
            }
            CardKind::BasicDye => {}
        }

        // Sell card display matching features
        let card_colors = card.colors();
        let card_materials = card.material_types();

        for sc_inst in sell_card_display.iter() {
            let sc = sc_inst.sell_card;
            let cost = sc.color_cost();

            for &card_color in card_colors {
                for &cost_color in cost {
                    if card_color == cost_color {
                        if is_tertiary(card_color) {
                            score += self.matching_tertiary_colors;
                        } else if SECONDARIES.contains(&card_color) {
                            score += self.matching_secondary_colors;
                        } else if is_primary(card_color) {
                            score += self.matching_primary_colors;
                        }
                    }
                }
            }

            for &card_mat in card_materials {
                if card_mat == sc.required_material() {
                    score += self.matching_materials;
                }
            }
        }

        score
    }
}
