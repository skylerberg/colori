use std::collections::HashMap;

use eframe::egui;

use colori_core::game_log::{StructuredGameLog, StructuredLogEntry};
use colori_core::types::{BuyerInstance, CardInstance, ALL_COLORS, ALL_MATERIAL_TYPES};

use crate::analysis::computations::{
    build_buyer_instance_map, build_card_instance_map, buyer_name_from_instance, format_choice,
};

const PLAYER_COLORS: [egui::Color32; 5] = [
    egui::Color32::from_rgb(230, 57, 70),  // red
    egui::Color32::from_rgb(59, 130, 246),  // blue
    egui::Color32::from_rgb(46, 204, 113),  // green
    egui::Color32::from_rgb(244, 162, 97),  // orange
    egui::Color32::from_rgb(168, 85, 247),  // purple
];

struct RoundGroup<'a> {
    round: u32,
    phases: Vec<(String, Vec<&'a StructuredLogEntry>)>,
}

fn phase_name(phase: &str) -> &str {
    match phase {
        "draw" => "Draw",
        "draft" => "Draft",
        "action" => "Action",
        _ => phase,
    }
}

pub struct GameViewerState {
    pub game: Option<StructuredGameLog>,
    pub error: Option<String>,
    pub selected_player: Option<usize>,
    card_map: Option<HashMap<u32, CardInstance>>,
    buyer_map: Option<HashMap<u32, BuyerInstance>>,
}

impl GameViewerState {
    pub fn new() -> Self {
        Self {
            game: None,
            error: None,
            selected_player: None,
            card_map: None,
            buyer_map: None,
        }
    }

    pub fn load_file(&mut self, path: &std::path::Path) {
        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<StructuredGameLog>(&contents) {
                Ok(game) => {
                    let card_map = build_card_instance_map(&game);
                    let buyer_map = build_buyer_instance_map(&game);
                    self.card_map = Some(card_map);
                    self.buyer_map = Some(buyer_map);
                    self.game = Some(game);
                    self.error = None;
                    self.selected_player = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to parse game log: {}", e));
                    self.game = None;
                    self.card_map = None;
                    self.buyer_map = None;
                }
            },
            Err(e) => {
                self.error = Some(format!("Failed to read file: {}", e));
                self.game = None;
                self.card_map = None;
                self.buyer_map = None;
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        if ui.button("Load Game Log...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                self.load_file(&path);
            }
        }

        if let Some(ref error) = self.error {
            ui.colored_label(egui::Color32::RED, error);
        }

        if self.game.is_some() {
            // Reborrow fields individually to avoid borrowing self while game is borrowed.
            let game = self.game.as_ref().unwrap();
            let card_map = self.card_map.as_ref().unwrap();
            let buyer_map = self.buyer_map.as_ref().unwrap();
            let selected_player = &mut self.selected_player;
            render_game(ui, game, card_map, buyer_map, selected_player);
        }
    }
}

fn render_game(
    ui: &mut egui::Ui,
    game: &StructuredGameLog,
    card_map: &HashMap<u32, CardInstance>,
    buyer_map: &HashMap<u32, BuyerInstance>,
    selected_player: &mut Option<usize>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // 1. Summary bar
        render_summary(ui, game);

        ui.add_space(8.0);

        // 2. Player filter
        render_player_filter(ui, game, selected_player);

        ui.add_space(8.0);

        // 3. Final State (collapsible, default closed)
        render_final_state(ui, game, *selected_player);

        ui.add_space(8.0);

        // 4. Round-by-round timeline
        render_timeline(ui, game, card_map, buyer_map, *selected_player);
    });
}

fn render_summary(ui: &mut egui::Ui, game: &StructuredGameLog) {
    ui.horizontal_wrapped(|ui| {
        // Player scores
        if let Some(ref final_scores) = game.final_scores {
            for (i, score) in final_scores.iter().enumerate() {
                let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
                ui.colored_label(color, format!("{}: {}", score.name, score.score));
                ui.separator();
            }
        }

        // Round count
        let max_round = game
            .entries
            .iter()
            .map(|e| e.round)
            .max()
            .unwrap_or(0);
        ui.label(format!("Rounds: {}", max_round));
        ui.separator();

        // Winner(s)
        if let Some(ref final_scores) = game.final_scores {
            let max_score = final_scores.iter().map(|fs| fs.score).max().unwrap_or(0);
            let winners: Vec<&str> = final_scores
                .iter()
                .filter(|fs| fs.score == max_score)
                .map(|fs| fs.name.as_str())
                .collect();
            ui.label(format!("Winner: {}", winners.join(", ")));
            ui.separator();
        }

        // Duration
        if let Some(ms) = game.duration_ms {
            if ms >= 1000 {
                ui.label(format!("Duration: {:.1}s", ms as f64 / 1000.0));
            } else {
                ui.label(format!("Duration: {}ms", ms));
            }
        }
    });
}

fn render_player_filter(
    ui: &mut egui::Ui,
    game: &StructuredGameLog,
    selected_player: &mut Option<usize>,
) {
    let current_label = match *selected_player {
        None => "All players".to_string(),
        Some(idx) => game
            .player_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("Player {}", idx)),
    };

    egui::ComboBox::from_label("Player")
        .selected_text(&current_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected_player, None, "All players");
            for (i, name) in game.player_names.iter().enumerate() {
                ui.selectable_value(selected_player, Some(i), name);
            }
        });
}

fn render_final_state(
    ui: &mut egui::Ui,
    game: &StructuredGameLog,
    selected_player: Option<usize>,
) {
    let final_stats = match &game.final_player_stats {
        Some(stats) => stats,
        None => return,
    };

    egui::CollapsingHeader::new("Final State")
        .default_open(false)
        .show(ui, |ui| {
            for (i, name) in game.player_names.iter().enumerate() {
                // Apply player filter
                if let Some(sel) = selected_player {
                    if i != sel {
                        continue;
                    }
                }

                let stats = match final_stats.iter().find(|s| &s.name == name) {
                    Some(s) => s,
                    None => continue,
                };

                let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
                ui.colored_label(color, name);

                ui.indent(format!("final_state_{}", i), |ui| {
                    // Color wheel
                    for &c in &ALL_COLORS {
                        let count = stats.color_wheel.get(c);
                        if count > 0 {
                            ui.label(format!("{:?}: {}", c, count));
                        }
                    }

                    // Deck size and ducats
                    ui.label(format!("Deck size: {}", stats.deck_size));
                    ui.label(format!("Ducats: {}", stats.ducats));

                    // Materials (non-zero)
                    for &mt in &ALL_MATERIAL_TYPES {
                        let count = stats.materials.get(mt);
                        if count > 0 {
                            ui.label(format!("{:?}: {}", mt, count));
                        }
                    }

                    // Completed buyers
                    if !stats.completed_buyers.is_empty() {
                        ui.label("Completed buyers:");
                        for buyer in &stats.completed_buyers {
                            ui.label(format!("  {}", buyer_name_from_instance(buyer.buyer)));
                        }
                    }
                });

                ui.add_space(4.0);
            }
        });
}

fn build_round_groups<'a>(
    entries: &'a [StructuredLogEntry],
    selected_player: Option<usize>,
) -> Vec<RoundGroup<'a>> {
    let mut groups: Vec<RoundGroup<'a>> = Vec::new();
    let mut current_round: Option<u32> = None;

    for entry in entries {
        // Apply player filter
        if let Some(sel) = selected_player {
            if entry.player_index != sel {
                continue;
            }
        }

        // Start a new round group if needed
        if current_round != Some(entry.round) {
            groups.push(RoundGroup {
                round: entry.round,
                phases: Vec::new(),
            });
            current_round = Some(entry.round);
        }

        let group = groups.last_mut().unwrap();

        // Find or create phase group (preserving order of first appearance)
        let phase_key = entry.phase.clone();
        if let Some(phase_group) = group.phases.iter_mut().find(|(p, _)| *p == phase_key) {
            phase_group.1.push(entry);
        } else {
            group.phases.push((phase_key, vec![entry]));
        }
    }

    groups
}

fn render_timeline(
    ui: &mut egui::Ui,
    game: &StructuredGameLog,
    _card_map: &HashMap<u32, CardInstance>,
    _buyer_map: &HashMap<u32, BuyerInstance>,
    selected_player: Option<usize>,
) {
    let round_groups = build_round_groups(&game.entries, selected_player);

    for group in &round_groups {
        egui::CollapsingHeader::new(format!("Round {}", group.round))
            .default_open(true)
            .show(ui, |ui| {
                for (phase, entries) in &group.phases {
                    ui.strong(phase_name(phase));
                    ui.indent(format!("round_{}_phase_{}", group.round, phase), |ui| {
                        for entry in entries {
                            let player_color =
                                PLAYER_COLORS[entry.player_index % PLAYER_COLORS.len()];
                            let player_name = game
                                .player_names
                                .get(entry.player_index)
                                .map(|s| s.as_str())
                                .unwrap_or("Unknown");
                            let choice_text = format_choice(&entry.choice);

                            ui.horizontal(|ui| {
                                ui.colored_label(player_color, player_name);
                                ui.label(choice_text);
                            });
                        }
                    });
                }
            });
    }
}
