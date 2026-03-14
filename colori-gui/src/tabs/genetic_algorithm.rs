use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use colori_core::scoring::HeuristicParams;

const BATCH_COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(230, 57, 70),   // red
    egui::Color32::from_rgb(59, 130, 246),   // blue
    egui::Color32::from_rgb(46, 204, 113),   // green
    egui::Color32::from_rgb(249, 168, 37),   // amber
    egui::Color32::from_rgb(171, 71, 188),   // purple
    egui::Color32::from_rgb(0, 188, 212),    // cyan
    egui::Color32::from_rgb(255, 138, 101),  // coral
    egui::Color32::from_rgb(129, 199, 132),  // light green
];

struct GenerationEntry {
    generation: u32,
    params: HeuristicParams,
}

struct BatchRun {
    batch_id: String,
    generations: Vec<GenerationEntry>,
}

pub struct GeneticAlgorithmState {
    batches: Vec<BatchRun>,
    selected_batches: HashSet<String>, // empty = all
    error: Option<String>,
}

const PARAM_GROUPS: &[(&str, &[&str])] = &[
    ("Pip Weights", &[
        "primary_pip_weight", "secondary_pip_weight", "tertiary_pip_weight",
    ]),
    ("Material Weights", &[
        "stored_material_weight", "chalk_quality", "starter_material_quality",
        "draft_material_quality", "dual_material_quality",
    ]),
    ("Card Type Quality", &[
        "action_quality", "dye_quality", "basic_dye_quality",
    ]),
    ("Buyer & Glass Weights", &[
        "buyer_material_weight", "buyer_color_weight", "glass_weight",
    ]),
    ("Action Card Overrides", &[
        "alum_quality", "cream_of_tartar_quality", "gum_arabic_quality",
        "potash_quality", "vinegar_quality", "argol_quality",
    ]),
    ("Dye Type Overrides", &[
        "pure_primary_dye_quality", "primary_dye_quality",
        "secondary_dye_quality", "tertiary_dye_quality",
    ]),
    ("Coverage Weights", &[
        "primary_color_coverage_weight", "secondary_color_coverage_weight",
    ]),
    ("Deck Weights", &[
        "cards_in_deck_weight", "cards_in_deck_squared_weight",
        "material_type_count_weight", "material_coverage_weight",
    ]),
    ("Heuristic Control", &[
        "heuristic_round_threshold", "heuristic_lookahead", "heuristic_score_threshold",
    ]),
];

fn get_param_value(params: &HeuristicParams, name: &str) -> Option<f64> {
    match name {
        "primary_pip_weight" => Some(params.primary_pip_weight),
        "secondary_pip_weight" => Some(params.secondary_pip_weight),
        "tertiary_pip_weight" => Some(params.tertiary_pip_weight),
        "stored_material_weight" => Some(params.stored_material_weight),
        "chalk_quality" => Some(params.chalk_quality),
        "action_quality" => Some(params.action_quality),
        "dye_quality" => Some(params.dye_quality),
        "basic_dye_quality" => Some(params.basic_dye_quality),
        "starter_material_quality" => Some(params.starter_material_quality),
        "draft_material_quality" => Some(params.draft_material_quality),
        "dual_material_quality" => Some(params.dual_material_quality),
        "buyer_material_weight" => Some(params.buyer_material_weight),
        "buyer_color_weight" => Some(params.buyer_color_weight),
        "glass_weight" => Some(params.glass_weight),
        "heuristic_round_threshold" => Some(params.heuristic_round_threshold as f64),
        "heuristic_lookahead" => Some(params.heuristic_lookahead as f64),
        "alum_quality" => params.alum_quality,
        "cream_of_tartar_quality" => params.cream_of_tartar_quality,
        "gum_arabic_quality" => params.gum_arabic_quality,
        "potash_quality" => params.potash_quality,
        "vinegar_quality" => params.vinegar_quality,
        "argol_quality" => params.argol_quality,
        "pure_primary_dye_quality" => params.pure_primary_dye_quality,
        "primary_dye_quality" => params.primary_dye_quality,
        "secondary_dye_quality" => params.secondary_dye_quality,
        "tertiary_dye_quality" => params.tertiary_dye_quality,
        "primary_color_coverage_weight" => Some(params.primary_color_coverage_weight),
        "secondary_color_coverage_weight" => Some(params.secondary_color_coverage_weight),
        "cards_in_deck_weight" => Some(params.cards_in_deck_weight),
        "cards_in_deck_squared_weight" => Some(params.cards_in_deck_squared_weight),
        "material_type_count_weight" => Some(params.material_type_count_weight),
        "material_coverage_weight" => Some(params.material_coverage_weight),
        "heuristic_score_threshold" => params.heuristic_score_threshold,
        _ => None,
    }
}

fn param_display_name(name: &str) -> String {
    name.replace('_', " ")
}

impl GeneticAlgorithmState {
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            selected_batches: HashSet::new(),
            error: None,
        }
    }

    pub fn load_folder(&mut self, dir: &Path) {
        let re = match Regex::new(r"^batch-([a-z0-9]+)-gen-(\d+)\.json$") {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(format!("Regex error: {}", e));
                return;
            }
        };

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                self.error = Some(format!("Failed to read directory: {}", e));
                return;
            }
        };

        let mut groups: HashMap<String, Vec<GenerationEntry>> = HashMap::new();
        let mut errors = Vec::new();

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if let Some(caps) = re.captures(&name) {
                let batch_id = caps[1].to_string();
                let generation: u32 = match caps[2].parse() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                let path = entry.path();
                match std::fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<HeuristicParams>(&content) {
                        Ok(params) => {
                            groups
                                .entry(batch_id)
                                .or_default()
                                .push(GenerationEntry { generation, params });
                        }
                        Err(e) => {
                            errors.push(format!("{}: {}", name, e));
                        }
                    },
                    Err(e) => {
                        errors.push(format!("{}: {}", name, e));
                    }
                }
            }
        }

        let mut batches: Vec<BatchRun> = groups
            .into_iter()
            .map(|(batch_id, mut gens)| {
                gens.sort_by_key(|g| g.generation);
                BatchRun {
                    batch_id,
                    generations: gens,
                }
            })
            .collect();
        batches.sort_by(|a, b| a.batch_id.cmp(&b.batch_id));

        self.batches = batches;
        self.selected_batches.clear();
        if errors.is_empty() {
            self.error = None;
        } else {
            self.error = Some(format!("Parse errors:\n{}", errors.join("\n")));
        }
    }

    fn filtered_batches(&self) -> Vec<&BatchRun> {
        if self.selected_batches.is_empty() {
            self.batches.iter().collect()
        } else {
            self.batches
                .iter()
                .filter(|b| self.selected_batches.contains(&b.batch_id))
                .collect()
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        if let Some(ref error) = self.error {
            ui.colored_label(egui::Color32::RED, error);
        }

        if self.batches.is_empty() {
            if self.error.is_none() {
                ui.label("No GA batch files found in genetic-algorithm/ directory.");
            }
            return;
        }

        // Batch filter
        if self.batches.len() > 1 {
            ui.horizontal(|ui| {
                ui.label("Batch:");
                let button_text = if self.selected_batches.is_empty() {
                    "All batches".to_string()
                } else if self.selected_batches.len() == 1 {
                    let batch_id = self.selected_batches.iter().next().unwrap();
                    format!("{} ({} gens)", batch_id, self.batches.iter().find(|b| &b.batch_id == batch_id).map_or(0, |b| b.generations.len()))
                } else {
                    format!("{} batches selected", self.selected_batches.len())
                };
                let batch_ids: Vec<String> = self.batches.iter().map(|b| b.batch_id.clone()).collect();
                let batch_labels: Vec<String> = self.batches.iter().map(|b| format!("{} ({} gens)", b.batch_id, b.generations.len())).collect();
                let button = ui.button(format!("{} ▾", button_text));
                egui::Popup::from_toggle_button_response(&button)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        let mut all_selected = self.selected_batches.is_empty();
                        if ui.checkbox(&mut all_selected, "All batches").changed() {
                            if all_selected {
                                self.selected_batches.clear();
                            } else {
                                self.selected_batches = HashSet::from([batch_ids[0].clone()]);
                            }
                        }
                        ui.separator();
                        for (batch_id, label) in batch_ids.iter().zip(batch_labels.iter()) {
                            let mut is_selected = self.selected_batches.contains(batch_id);
                            if ui.checkbox(&mut is_selected, label).changed() {
                                if is_selected {
                                    self.selected_batches.insert(batch_id.clone());
                                    if self.selected_batches.len() == self.batches.len() {
                                        self.selected_batches.clear();
                                    }
                                } else {
                                    self.selected_batches.remove(batch_id);
                                }
                            }
                        }
                    });
            });
        }

        let filtered = self.filtered_batches();
        let batch_count = filtered.len();
        let total_count = self.batches.len();
        if self.selected_batches.is_empty() {
            ui.label(format!("{} batch(es) loaded", total_count));
        } else {
            ui.label(format!("{} of {} batch(es) shown", batch_count, total_count));
        }
        ui.separator();

        if filtered.is_empty() {
            return;
        }

        // Build a stable color map: batch_id -> color index based on position in all batches
        let color_map: HashMap<&str, usize> = self.batches.iter().enumerate().map(|(i, b)| (b.batch_id.as_str(), i)).collect();

        // Parameter graphs
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (group_name, param_names) in PARAM_GROUPS {
                egui::CollapsingHeader::new(*group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        for param_name in *param_names {
                            ui.label(param_display_name(param_name));
                            let plot = Plot::new(format!("ga_plot_{}", param_name))
                                .height(180.0)
                                .legend(Legend::default())
                                .x_axis_label("Generation")
                                .y_axis_label(param_display_name(param_name));
                            plot.show(ui, |plot_ui| {
                                for batch in &filtered {
                                    let points: Vec<[f64; 2]> = batch
                                        .generations
                                        .iter()
                                        .filter_map(|gen| {
                                            get_param_value(&gen.params, param_name)
                                                .map(|v| [gen.generation as f64, v])
                                        })
                                        .collect();
                                    if !points.is_empty() {
                                        let batch_idx = color_map.get(batch.batch_id.as_str()).copied().unwrap_or(0);
                                        let color =
                                            BATCH_COLORS[batch_idx % BATCH_COLORS.len()];
                                        let line = Line::new(
                                            &batch.batch_id,
                                            PlotPoints::new(points),
                                        )
                                            .color(color)
                                            .width(2.0);
                                        plot_ui.line(line);
                                    }
                                }
                            });
                        }
                    });
            }
        });
    }
}
