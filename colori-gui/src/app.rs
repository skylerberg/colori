use eframe::egui;

use crate::analysis::computations::format_variant_label;
use crate::analysis::log_loader::{LogLoader, LoadResult, TaggedGameLog};
use crate::tabs::analysis::{CachedAnalysis, render_analysis_tab};
use crate::tabs::card_reference::render_card_reference_tab;
use crate::tabs::game_viewer::GameViewerState;

use colori_core::game_log::StructuredGameLog;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Analysis,
    CardReference,
    GameViewer,
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
    selected_batch: String, // "all" or batch ID
    selected_variant: String, // "all" or variant label
    previous_batch: String,

    // Cached analysis
    cached_analysis: Option<CachedAnalysis>,
    cache_key: String,

    // Game viewer
    game_viewer: GameViewerState,
}

impl ColoriGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        let mut app = Self {
            active_tab: Tab::Analysis,
            loader: LogLoader::new(),
            tagged_logs: Vec::new(),
            load_error: None,
            selected_batch: "all".to_string(),
            selected_variant: "all".to_string(),
            previous_batch: "all".to_string(),
            cached_analysis: None,
            cache_key: String::new(),
            game_viewer: GameViewerState::new(),
        };

        // Auto-load game-logs directory if it exists
        if let Ok(cwd) = std::env::current_dir() {
            let game_logs_path = cwd.join("game-logs");
            if game_logs_path.is_dir() {
                app.loader.start_loading(&game_logs_path);
            }
        }

        app
    }

    fn filtered_logs(&self) -> Vec<&StructuredGameLog> {
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
        let key = format!("{}:{}:{}", self.selected_batch, self.selected_variant, self.tagged_logs.len());
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
                    self.selected_batch = batches[0].clone();
                } else {
                    self.selected_batch = "all".to_string();
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

            // Folder picker
            ui.horizontal(|ui| {
                if ui.button("Select game logs folder...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.loader.start_loading(&path);
                    }
                }
                if self.loader.is_loading() {
                    ui.spinner();
                    ui.label("Loading...");
                }
            });

            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Analysis, "Game Analysis");
                ui.selectable_value(&mut self.active_tab, Tab::CardReference, "Card Reference");
                ui.selectable_value(&mut self.active_tab, Tab::GameViewer, "Game Viewer");
            });
            ui.separator();

            match self.active_tab {
                Tab::Analysis => {
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
                                egui::ComboBox::from_id_salt("batch_filter")
                                    .selected_text(if self.selected_batch == "all" {
                                        "All batches".to_string()
                                    } else {
                                        self.batch_label(&self.selected_batch, &batch_info)
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_batch, "all".to_string(), "All batches");
                                        for batch in &batches {
                                            let label = self.batch_label(batch, &batch_info);
                                            ui.selectable_value(&mut self.selected_batch, batch.clone(), label);
                                        }
                                    });
                            });
                        }

                        // Reset variant when batch changes
                        if self.selected_batch != self.previous_batch {
                            self.selected_variant = "all".to_string();
                            self.previous_batch = self.selected_batch.clone();
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
            }
        });
    }
}
