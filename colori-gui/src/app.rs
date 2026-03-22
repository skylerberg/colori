use eframe::egui;

use crate::analysis::computations::format_variant_label;
use crate::analysis::log_loader::{LogLoader, LoadResult, TaggedGameLog};
use crate::tabs::analysis::{CachedAnalysis, render_analysis_tab};
use crate::tabs::card_reference::render_card_reference_tab;
use crate::tabs::game_viewer::GameViewerState;
use crate::tabs::diff_eval_viewer::DiffEvalViewerState;
use crate::tabs::genetic_algorithm::GeneticAlgorithmState;

use colori_core::game_log::StructuredGameLog;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Analysis,
    CardReference,
    GameViewer,
    GeneticAlgorithm,
    DiffEval,
}

struct BatchInfo {
    count: usize,
    iterations: Option<u32>,
    variants: Option<String>,
    note: Option<String>,
    earliest_timestamp: String,
}

pub struct ColoriGuiApp {
    active_tab: Tab,
    loader: LogLoader,
    game_logs_path: Option<PathBuf>,
    tagged_logs: Vec<TaggedGameLog>,
    load_error: Option<String>,

    // Batch/variant filtering
    selected_batch: String, // "all" or a batch ID
    selected_variant: String, // "all" or variant label
    previous_batch: String,
    excluded_variants: HashSet<String>,

    // Cached analysis
    cached_analysis: Option<CachedAnalysis>,
    cache_key: String,

    // Game viewer
    game_viewer: GameViewerState,

    // Genetic algorithm
    ga_state: GeneticAlgorithmState,

    // Diff eval viewer
    diff_eval_state: DiffEvalViewerState,
}

impl ColoriGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::WHITE);
        cc.egui_ctx.set_visuals(visuals);

        let mut app = Self {
            active_tab: Tab::Analysis,
            loader: LogLoader::new(),
            game_logs_path: None,
            tagged_logs: Vec::new(),
            load_error: None,
            selected_batch: "all".to_string(),
            selected_variant: "all".to_string(),
            previous_batch: "all".to_string(),
            excluded_variants: HashSet::new(),
            cached_analysis: None,
            cache_key: String::new(),
            game_viewer: GameViewerState::new(),
            ga_state: GeneticAlgorithmState::new(),
            diff_eval_state: DiffEvalViewerState::new(),
        };

        if let Ok(cwd) = std::env::current_dir() {
            let game_logs_path = cwd.join("game-logs");
            if game_logs_path.is_dir() {
                app.loader.start_loading(&game_logs_path);
                app.game_logs_path = Some(game_logs_path);
            }
            let ga_path = cwd.join("genetic-algorithm");
            if ga_path.is_dir() {
                app.ga_state.load_folder(&ga_path);
            }
            app.diff_eval_state.try_auto_load();
        }

        app
    }

    fn batch_filtered_logs(&self) -> Vec<&StructuredGameLog> {
        if self.selected_batch == "all" {
            self.tagged_logs.iter().map(|t| &t.log).collect()
        } else {
            self.tagged_logs
                .iter()
                .filter(|t| t.batch_id == self.selected_batch)
                .map(|t| &t.log)
                .collect()
        }
    }

    fn filtered_logs(&self) -> Vec<&StructuredGameLog> {
        let batch_filtered = self.batch_filtered_logs();
        if self.excluded_variants.is_empty() {
            return batch_filtered;
        }
        batch_filtered
            .into_iter()
            .filter(|log| {
                if let Some(ref pvs) = log.player_variants {
                    !pvs.iter().any(|v| {
                        self.excluded_variants
                            .contains(&format_variant_label(v, Some(pvs)))
                    })
                } else {
                    true
                }
            })
            .collect()
    }

    /// All variant labels in the current batch (ignoring exclusions).
    /// Used by the exclusion UI so excluded variants remain visible.
    fn all_batch_variants(&self) -> Vec<String> {
        let filtered = self.batch_filtered_logs();
        let mut labels = HashSet::new();
        for log in &filtered {
            if let Some(ref pvs) = log.player_variants {
                for v in pvs {
                    labels.insert(format_variant_label(v, Some(pvs)));
                }
            }
        }
        let mut sorted: Vec<String> = labels.into_iter().collect();
        sorted.sort();
        sorted
    }

    fn available_batches(&self) -> Vec<String> {
        let mut batches: Vec<String> = self
            .tagged_logs
            .iter()
            .map(|t| t.batch_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let batch_info = self.compute_batch_info();
        batches.sort_by(|a, b| {
            let ts_a = batch_info.get(a).map(|i| &i.earliest_timestamp).cloned().unwrap_or_default();
            let ts_b = batch_info.get(b).map(|i| &i.earliest_timestamp).cloned().unwrap_or_default();
            ts_b.cmp(&ts_a)
        });
        batches
    }

    fn compute_batch_info(&self) -> HashMap<String, BatchInfo> {
        let mut map: HashMap<String, BatchInfo> = HashMap::new();
        for t in &self.tagged_logs {
            if let Some(existing) = map.get_mut(&t.batch_id) {
                existing.count += 1;
                if t.log.game_started_at < existing.earliest_timestamp {
                    existing.earliest_timestamp = t.log.game_started_at.clone();
                }
            } else {
                let variants = t.log.player_variants.as_ref().map(|pvs| {
                    pvs.iter()
                        .map(|v| format_variant_label(v, Some(pvs)))
                        .collect::<Vec<_>>()
                        .join(" vs ")
                });
                map.insert(
                    t.batch_id.clone(),
                    BatchInfo {
                        count: 1,
                        iterations: t.log.iterations,
                        variants,
                        note: t.log.note.clone(),
                        earliest_timestamp: t.log.game_started_at.clone(),
                    },
                );
            }
        }
        map
    }

    fn batch_label(&self, batch_id: &str, batch_info: &HashMap<String, BatchInfo>) -> String {
        let info = match batch_info.get(batch_id) {
            Some(i) => i,
            None => return batch_id.to_string(),
        };
        let mut label = batch_id.to_string();
        if let Some(ref variants) = info.variants {
            label += &format!(" (variants: {})", variants);
        } else if let Some(iters) = info.iterations {
            label += &format!(" (iters: {})", iters);
        }
        if let Some(ref note) = info.note {
            label += &format!(" - {}", note);
        }
        label += &format!(" ({} games)", info.count);
        label
    }

    fn available_variants(&self) -> Vec<String> {
        let filtered = self.filtered_logs();
        let mut labels = HashSet::new();
        for log in &filtered {
            if let Some(ref pvs) = log.player_variants {
                for v in pvs {
                    labels.insert(format_variant_label(v, Some(pvs)));
                }
            }
        }
        let mut sorted: Vec<String> = labels.into_iter().collect();
        sorted.sort();
        sorted
    }

    fn ensure_analysis_cached(&mut self) {
        let variant_label = if self.selected_variant == "all" {
            None
        } else {
            Some(self.selected_variant.as_str())
        };
        let mut excluded_sorted: Vec<&str> = self.excluded_variants.iter().map(|s| s.as_str()).collect();
        excluded_sorted.sort();
        let key = format!("{}:{}:{}:{}", self.selected_batch, self.selected_variant, self.tagged_logs.len(), excluded_sorted.join(","));
        if self.cache_key != key {
            let filtered: Vec<StructuredGameLog> = self.filtered_logs().into_iter().cloned().collect();
            self.cached_analysis = Some(CachedAnalysis::compute(&filtered, None, variant_label));
            self.cache_key = key;
        }
    }
}

impl eframe::App for ColoriGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll loader
        match self.loader.poll() {
            LoadResult::Done(logs) => {
                self.tagged_logs = logs;
                self.load_error = None;
                self.selected_variant = "all".to_string();
                self.excluded_variants.clear();
                self.cache_key.clear();
                // Select only the latest batch by default
                let batches = self.available_batches();
                self.selected_batch = batches.into_iter().next().unwrap_or_else(|| "all".to_string());
            }
            LoadResult::Error(e) => {
                self.load_error = Some(e);
                self.tagged_logs.clear();
                self.cache_key.clear();
            }
            LoadResult::Loading => {
                ctx.request_repaint();
            }
            LoadResult::Idle => {}
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Colori Game Analysis");

            // Tab bar
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (Tab::Analysis, "Game Analysis"),
                    (Tab::CardReference, "Card Reference"),
                    (Tab::GameViewer, "Game Viewer"),
                    (Tab::GeneticAlgorithm, "Genetic Algorithm"),
                    (Tab::DiffEval, "Diff Eval"),
                ] {
                    let is_active = self.active_tab == tab;
                    let response = ui.selectable_value(&mut self.active_tab, tab, label);
                    if is_active {
                        let rect = response.rect;
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(rect.min.x, rect.max.y - 2.0),
                                egui::vec2(rect.width(), 2.0),
                            ),
                            0.0,
                            egui::Color32::from_rgb(74, 158, 255),
                        );
                    }
                }
            });
            ui.separator();

            match self.active_tab {
                Tab::Analysis => {
                    ui.horizontal(|ui| {
                        if let Some(ref path) = self.game_logs_path {
                            if ui.button("Refresh").clicked() && !self.loader.is_loading() {
                                self.loader.start_loading(path);
                            }
                        }
                        if self.loader.is_loading() {
                            ui.spinner();
                            ui.label("Loading...");
                        }
                    });

                    if let Some(ref error) = self.load_error {
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    if !self.tagged_logs.is_empty() {
                        let filtered_count = self.filtered_logs().len();
                        let total_count = self.tagged_logs.len();

                        // Batch filter
                        let batches = self.available_batches();
                        if batches.len() > 1 {
                            let batch_info = self.compute_batch_info();
                            ui.horizontal(|ui| {
                                ui.label("Batch:");
                                let selected_text = if self.selected_batch == "all" {
                                    "All batches".to_string()
                                } else {
                                    self.batch_label(&self.selected_batch, &batch_info)
                                };
                                egui::ComboBox::from_id_salt("batch_filter")
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_batch, "all".to_string(), "All batches");
                                        for batch in &batches {
                                            let label = self.batch_label(batch, &batch_info);
                                            ui.selectable_value(&mut self.selected_batch, batch.clone(), label);
                                        }
                                    });
                            });
                        }

                        // Reset variant when batch selection changes
                        if self.selected_batch != self.previous_batch {
                            self.selected_variant = "all".to_string();
                            self.excluded_variants.clear();
                            self.previous_batch = self.selected_batch.clone();
                        }

                        // Variant exclusion
                        let all_variants = self.all_batch_variants();
                        if all_variants.len() > 1 {
                            ui.horizontal(|ui| {
                                ui.label("Exclude:");
                                for variant in &all_variants {
                                    let mut excluded = self.excluded_variants.contains(variant);
                                    if ui.checkbox(&mut excluded, variant.as_str()).changed() {
                                        if excluded {
                                            self.excluded_variants.insert(variant.clone());
                                        } else {
                                            self.excluded_variants.remove(variant);
                                        }
                                    }
                                }
                            });
                            // Reset selected variant if it was excluded
                            if self.excluded_variants.contains(&self.selected_variant) {
                                self.selected_variant = "all".to_string();
                            }
                        }

                        // Variant filter
                        let variants = self.available_variants();
                        if variants.len() > 1 {
                            ui.horizontal(|ui| {
                                ui.label("Variant:");
                                egui::ComboBox::from_id_salt("variant_filter")
                                    .selected_text(if self.selected_variant == "all" {
                                        "All variants".to_string()
                                    } else {
                                        self.selected_variant.clone()
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_variant, "all".to_string(), "All variants");
                                        for variant in &variants {
                                            ui.selectable_value(&mut self.selected_variant, variant.clone(), variant.as_str());
                                        }
                                    });
                            });
                        }

                        // Batch info display
                        if self.selected_batch == "all" {
                            ui.label(format!("{} games loaded", total_count));
                        } else {
                            ui.label(format!("{} of {} games shown", filtered_count, total_count));
                            let batch_info = self.compute_batch_info();
                            if let Some(info) = batch_info.get(&self.selected_batch) {
                                ui.horizontal(|ui| {
                                    if let Some(ref variants) = info.variants {
                                        ui.label(format!("Variants: {}", variants));
                                    } else if let Some(iters) = info.iterations {
                                        ui.label(format!("Iterations: {}", iters));
                                    }
                                    if let Some(ref note) = info.note {
                                        ui.label(format!("Note: {}", note));
                                    }
                                });
                            }
                        }

                        // Compute and render analysis
                        self.ensure_analysis_cached();
                        if let Some(ref analysis) = self.cached_analysis {
                            render_analysis_tab(ui, analysis);
                        }
                    }
                }
                Tab::CardReference => {
                    render_card_reference_tab(ui);
                }
                Tab::GameViewer => {
                    self.game_viewer.render(ui, ctx);
                }
                Tab::GeneticAlgorithm => {
                    self.ga_state.render(ui);
                }
                Tab::DiffEval => {
                    self.diff_eval_state.render(ui);
                }
            }
        });
    }
}
