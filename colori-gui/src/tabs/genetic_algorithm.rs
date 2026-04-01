use eframe::egui;
use egui_plot::{HLine, Legend, Line, LineStyle, Plot, PlotPoints, Points};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use colori_core::scoring::HeuristicParams;

static BASELINE_PARAMS: std::sync::LazyLock<HeuristicParams> = std::sync::LazyLock::new(|| {
    const JSON: &str = include_str!("../../../genetic-algorithm/batch-nocdm1-gen-7.json");
    serde_json::from_str(JSON).expect("Failed to parse baseline params")
});

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
    selected_batch: String, // "all" or a batch ID
    loaded_path: Option<std::path::PathBuf>,
    error: Option<String>,
}

const PARAM_GROUPS: &[(&str, &[&str])] = &[
    ("Color Value Weights", &[
        "primary_color_value", "secondary_color_value", "tertiary_color_value",
    ]),
    ("Material Weights", &[
        "stored_material_weight", "chalk_quality", "starter_material_quality",
        "draft_material_quality", "dual_material_quality",
    ]),
    ("Card Type Quality", &[
        "basic_dye_quality",
    ]),
    ("Sell Card Weights", &[
        "sell_card_material_alignment", "sell_card_color_alignment",
    ]),
    ("Action Card Overrides", &[
        "alum_quality", "cream_of_tartar_quality", "gum_arabic_quality",
        "potash_quality", "vinegar_quality",
        "linseed_oil_quality",
    ]),
    ("Dye Type Overrides", &[
        "primary_dye_quality",
        "secondary_dye_quality", "tertiary_dye_quality",
    ]),
    ("Heuristic Control", &[
        "heuristic_round_threshold", "heuristic_lookahead",
    ]),
    ("Rollout General", &[
        "rollout_epsilon", "rollout_end_turn_threshold",
        "rollout_end_turn_probability_early", "rollout_end_turn_probability_late",
        "rollout_end_turn_max_round", "rollout_other_priority",
    ]),
    ("Rollout Sell", &[
        "rollout_sell_affordable_multiplier", "rollout_sell_base",
    ]),
    ("Rollout Mix", &[
        "rollout_mix_base", "rollout_mix_pair_weight",
        "rollout_mix_count_weight", "rollout_mix_no_pairs",
    ]),
    ("Rollout Workshop", &[
        "rollout_workshop_base", "rollout_workshop_count_weight",
        "rollout_workshop_empty", "rollout_ws_material_base_multiplier",
        "rollout_ws_material_colors_met_multiplier", "rollout_ws_action_bonus",
    ]),
    ("Rollout Destroy & Draw", &[
        "rollout_destroy_with_targets", "rollout_destroy_no_targets",
        "rollout_draw_base", "rollout_draw_count_weight",
    ]),
];

fn get_param_value(params: &HeuristicParams, name: &str) -> Option<f64> {
    match name {
        "primary_color_value" => Some(params.primary_color_value),
        "secondary_color_value" => Some(params.secondary_color_value),
        "tertiary_color_value" => Some(params.tertiary_color_value),
        "stored_material_weight" => Some(params.stored_material_weight),
        "chalk_quality" => Some(params.chalk_quality),
        "basic_dye_quality" => Some(params.basic_dye_quality),
        "starter_material_quality" => Some(params.starter_material_quality),
        "draft_material_quality" => Some(params.draft_material_quality),
        "dual_material_quality" => Some(params.dual_material_quality),
        "sell_card_material_alignment" => Some(params.sell_card_material_alignment),
        "sell_card_color_alignment" => Some(params.sell_card_color_alignment),
        "heuristic_round_threshold" => Some(params.heuristic_round_threshold as f64),
        "heuristic_lookahead" => Some(params.heuristic_lookahead as f64),
        "alum_quality" => params.alum_quality,
        "cream_of_tartar_quality" => params.cream_of_tartar_quality,
        "gum_arabic_quality" => params.gum_arabic_quality,
        "potash_quality" => params.potash_quality,
        "vinegar_quality" => params.vinegar_quality,
        "linseed_oil_quality" => Some(params.linseed_oil_quality),
        "primary_dye_quality" => params.primary_dye_quality,
        "secondary_dye_quality" => params.secondary_dye_quality,
        "tertiary_dye_quality" => params.tertiary_dye_quality,
        "rollout_epsilon" => Some(params.rollout_epsilon),
        "rollout_sell_affordable_multiplier" => Some(params.rollout_sell_affordable_multiplier as f64),
        "rollout_sell_base" => Some(params.rollout_sell_base as f64),
        "rollout_mix_base" => Some(params.rollout_mix_base as f64),
        "rollout_mix_pair_weight" => Some(params.rollout_mix_pair_weight as f64),
        "rollout_mix_count_weight" => Some(params.rollout_mix_count_weight as f64),
        "rollout_mix_no_pairs" => Some(params.rollout_mix_no_pairs as f64),
        "rollout_workshop_base" => Some(params.rollout_workshop_base as f64),
        "rollout_workshop_count_weight" => Some(params.rollout_workshop_count_weight as f64),
        "rollout_workshop_empty" => Some(params.rollout_workshop_empty as f64),
        "rollout_destroy_with_targets" => Some(params.rollout_destroy_with_targets as f64),
        "rollout_destroy_no_targets" => Some(params.rollout_destroy_no_targets as f64),
        "rollout_draw_base" => Some(params.rollout_draw_base as f64),
        "rollout_draw_count_weight" => Some(params.rollout_draw_count_weight as f64),
        "rollout_other_priority" => Some(params.rollout_other_priority as f64),
        "rollout_end_turn_threshold" => Some(params.rollout_end_turn_threshold as f64),
        "rollout_end_turn_probability_early" => Some(params.rollout_end_turn_probability_early),
        "rollout_end_turn_probability_late" => Some(params.rollout_end_turn_probability_late),
        "rollout_end_turn_max_round" => Some(params.rollout_end_turn_max_round as f64),
        "rollout_ws_material_base_multiplier" => Some(params.rollout_ws_material_base_multiplier as f64),
        "rollout_ws_material_colors_met_multiplier" => Some(params.rollout_ws_material_colors_met_multiplier as f64),
        "rollout_ws_action_bonus" => Some(params.rollout_ws_action_bonus as f64),
        _ => None,
    }
}

fn baseline_fallback(param_name: &str) -> Option<f64> {
    match param_name {
        "alum_quality" | "cream_of_tartar_quality" | "gum_arabic_quality"
        | "potash_quality" | "vinegar_quality" => {
            None
        }
        "primary_dye_quality"
        | "secondary_dye_quality" | "tertiary_dye_quality" => {
            None
        }
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
            selected_batch: "all".to_string(),
            loaded_path: None,
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
        let mut batch_latest_modified: HashMap<String, SystemTime> = HashMap::new();
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
                let modified = std::fs::metadata(&path)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let latest = batch_latest_modified.entry(batch_id.clone()).or_insert(SystemTime::UNIX_EPOCH);
                if modified > *latest {
                    *latest = modified;
                }
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
        // Sort by most recently modified first
        batches.sort_by(|a, b| {
            let ta = batch_latest_modified.get(&a.batch_id).copied().unwrap_or(SystemTime::UNIX_EPOCH);
            let tb = batch_latest_modified.get(&b.batch_id).copied().unwrap_or(SystemTime::UNIX_EPOCH);
            tb.cmp(&ta)
        });

        self.batches = batches;
        self.selected_batch = self.batches.first()
            .map(|b| b.batch_id.clone())
            .unwrap_or_else(|| "all".to_string());
        self.loaded_path = Some(dir.to_path_buf());
        if errors.is_empty() {
            self.error = None;
        } else {
            self.error = Some(format!("Parse errors:\n{}", errors.join("\n")));
        }
    }

    fn filtered_batches(&self) -> Vec<&BatchRun> {
        if self.selected_batch == "all" {
            self.batches.iter().collect()
        } else {
            self.batches
                .iter()
                .filter(|b| b.batch_id == self.selected_batch)
                .collect()
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = self.loaded_path.clone() {
            if ui.button("Refresh").clicked() {
                self.load_folder(&path);
            }
        }

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
                let selected_text = if self.selected_batch == "all" {
                    "All batches".to_string()
                } else {
                    let gen_count = self.batches.iter()
                        .find(|b| b.batch_id == self.selected_batch)
                        .map_or(0, |b| b.generations.len());
                    format!("{} ({} gens)", self.selected_batch, gen_count)
                };
                egui::ComboBox::from_id_salt("ga_batch_filter")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_batch, "all".to_string(), "All batches");
                        for batch in &self.batches {
                            let label = format!("{} ({} gens)", batch.batch_id, batch.generations.len());
                            ui.selectable_value(&mut self.selected_batch, batch.batch_id.clone(), label);
                        }
                    });
            });
        }

        let filtered = self.filtered_batches();
        let batch_count = filtered.len();
        let total_count = self.batches.len();
        if self.selected_batch == "all" {
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

        // Parameter graphs in 2-column layout
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (group_name, param_names) in PARAM_GROUPS {
                egui::CollapsingHeader::new(*group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        for chunk in param_names.chunks(2) {
                            ui.columns(2, |columns| {
                                for (col_idx, param_name) in chunk.iter().enumerate() {
                                    let ui = &mut columns[col_idx];
                                    ui.label(param_display_name(param_name));
                                    let mut plot = Plot::new(format!("ga_plot_{}", param_name))
                                        .height(180.0)
                                        .allow_zoom(false)
                                        .allow_scroll(false)
                                        .allow_drag(false)
                                        .allow_boxed_zoom(false);
                                    if filtered.len() > 1 {
                                        plot = plot.legend(Legend::default());
                                    }
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
                                                    PlotPoints::new(points.clone()),
                                                )
                                                    .color(color)
                                                    .width(2.0)
                                                    .allow_hover(false);
                                                plot_ui.line(line);
                                                let markers = Points::new(
                                                    &batch.batch_id,
                                                    PlotPoints::new(points),
                                                )
                                                    .color(color)
                                                    .radius(3.0);
                                                plot_ui.points(markers);
                                            }
                                        }
                                        let baseline_val = get_param_value(&BASELINE_PARAMS, param_name)
                                            .or_else(|| baseline_fallback(param_name));
                                        if let Some(baseline_val) = baseline_val {
                                            let hline = HLine::new("Baseline", baseline_val)
                                                .color(egui::Color32::from_rgb(200, 200, 200))
                                                .width(1.5)
                                                .style(LineStyle::dashed_dense());
                                            plot_ui.hline(hline);
                                        }
                                    });
                                }
                            });
                        }
                    });
            }
        });
    }
}
