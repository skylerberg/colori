use eframe::egui;

use crate::analysis::computations::format_variant_label;
use crate::analysis::log_loader::{LogLoader, LoadResult, TaggedGameLog};
use crate::tabs::analysis::{CachedAnalysis, render_analysis_tab};
use crate::tabs::card_reference::render_card_reference_tab;
use crate::tabs::game_viewer::GameViewerState;
use crate::tabs::genetic_algorithm::GeneticAlgorithmState;

use colori_core::game_log::StructuredGameLog;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Analysis,
    CardReference,
    GameViewer,
    GeneticAlgorithm,
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
    tagged_logs: Vec<TaggedGameLog>,
    load_error: Option<String>,

    // Batch/variant filtering
    selected_batches: HashSet<String>, // set of selected batch IDs; empty = all
    selected_variant: String, // "all" or variant label
    previous_batches: HashSet<String>,

    // Cached analysis
    cached_analysis: Option<CachedAnalysis>,
    cache_key: String,

    // Game viewer
    game_viewer: GameViewerState,

    // Genetic algorithm
    ga_state: GeneticAlgorithmState,
}

impl ColoriGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::WHITE);
        cc.egui_ctx.set_visuals(visuals);

        Self {
            active_tab: Tab::Analysis,
            loader: LogLoader::new(),
            tagged_logs: Vec::new(),
            load_error: None,
            selected_batches: HashSet::new(),
            selected_variant: "all".to_string(),
            previous_batches: HashSet::new(),
            cached_analysis: None,
            cache_key: String::new(),
            game_viewer: GameViewerState::new(),
            ga_state: GeneticAlgorithmState::new(),
        }
    }

    fn filtered_logs(&self) -> Vec<&StructuredGameLog> {
        if self.selected_batches.is_empty() {
            self.tagged_logs.iter().map(|t| &t.log).collect()
        } else {
            self.tagged_logs
                .iter()
                .filter(|t| self.selected_batches.contains(&t.batch_id))
                .map(|t| &t.log)
                .collect()
        }
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
        let mut batch_key: Vec<&String> = self.selected_batches.iter().collect();
        batch_key.sort();
        let key = format!("{}:{}:{}", batch_key.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(","), self.selected_variant, self.tagged_logs.len());
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
                let batches = self.available_batches();
                if batches.len() > 1 {
                    self.selected_batches = HashSet::from([batches[0].clone()]);
                } else {
                    self.selected_batches.clear();
                }
                self.selected_variant = "all".to_string();
                self.cache_key.clear();
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
                        if ui.button("Load Game Logs...").clicked() && !self.loader.is_loading() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.loader.start_loading(&path);
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

                    if self.tagged_logs.is_empty() && !self.loader.is_loading() && self.load_error.is_none() {
                        ui.label("No game logs loaded. Click 'Load Game Logs...' to select a folder.");
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
                                let button_text = if self.selected_batches.is_empty() {
                                    "All batches".to_string()
                                } else if self.selected_batches.len() == 1 {
                                    let batch_id = self.selected_batches.iter().next().unwrap();
                                    self.batch_label(batch_id, &batch_info)
                                } else {
                                    format!("{} batches selected", self.selected_batches.len())
                                };
                                let button = ui.button(format!("{} ▾", button_text));
                                egui::Popup::from_toggle_button_response(&button)
                                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                    .show(|ui| {
                                    // "All batches" checkbox
                                    let mut all_selected = self.selected_batches.is_empty();
                                    if ui.checkbox(&mut all_selected, "All batches").changed() {
                                        if all_selected {
                                            self.selected_batches.clear();
                                        } else {
                                            // Deselecting "all" → select just the most recent batch
                                            self.selected_batches = HashSet::from([batches[0].clone()]);
                                        }
                                    }
                                    ui.separator();
                                    // Individual batch checkboxes
                                    for batch in &batches {
                                        let label = self.batch_label(batch, &batch_info);
                                        let mut is_selected = self.selected_batches.contains(batch);
                                        if ui.checkbox(&mut is_selected, label).changed() {
                                            if is_selected {
                                                self.selected_batches.insert(batch.clone());
                                                // If all batches are now selected, clear to mean "all"
                                                if self.selected_batches.len() == batches.len() {
                                                    self.selected_batches.clear();
                                                }
                                            } else {
                                                // Prevent empty selection
                                                if self.selected_batches.len() > 1 {
                                                    self.selected_batches.remove(batch);
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                        }

                        // Reset variant when batch selection changes
                        if self.selected_batches != self.previous_batches {
                            self.selected_variant = "all".to_string();
                            self.previous_batches = self.selected_batches.clone();
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
                        if self.selected_batches.is_empty() {
                            ui.label(format!("{} games loaded", total_count));
                        } else {
                            ui.label(format!("{} of {} games shown ({} {})", filtered_count, total_count, self.selected_batches.len(), if self.selected_batches.len() == 1 { "batch" } else { "batches" }));
                            // Show per-batch detail only when a single batch is selected
                            if self.selected_batches.len() == 1 {
                                let batch_id = self.selected_batches.iter().next().unwrap();
                                let batch_info = self.compute_batch_info();
                                if let Some(info) = batch_info.get(batch_id) {
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
                        }

                        // Compute and render analysis
                        self.ensure_analysis_cached();
                        if let Some(ref analysis) = self.cached_analysis {
                            render_analysis_tab(ui, analysis, filtered_count);
                        }
                    }
                }
                Tab::CardReference => {
                    render_card_reference_tab(ui);
                }
                Tab::GameViewer => {
                    self.game_viewer.render(ui);
                }
                Tab::GeneticAlgorithm => {
                    self.ga_state.render(ui);
                }
            }
        });
    }
}
