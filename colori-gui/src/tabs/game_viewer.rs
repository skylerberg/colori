use std::collections::HashMap;
use std::sync::mpsc;

use eframe::egui;
use rand::SeedableRng;
use wyrand::WyRand;

use colori_core::colori_game::enumerate_choices;
use colori_core::game_log::{StructuredGameLog, StructuredLogEntry};
use colori_core::ismcts::{ismcts, MctsConfig, MctsNode, TreeStats};
use colori_core::replay::replay_to;
use colori_core::scoring::{calculate_score, HeuristicParams};
use colori_core::types::{
    CardInstance, GamePhase, GameState, SellCardInstance, ALL_COLORS, ALL_MATERIAL_TYPES,
};

use crate::analysis::computations::{
    build_card_instance_map, build_sell_card_instance_map, final_score_ranking, format_choice,
    sell_card_name_from_instance,
};

const PLAYER_COLORS: [egui::Color32; 5] = [
    egui::Color32::from_rgb(230, 57, 70),
    egui::Color32::from_rgb(59, 130, 246),
    egui::Color32::from_rgb(46, 204, 113),
    egui::Color32::from_rgb(244, 162, 97),
    egui::Color32::from_rgb(168, 85, 247),
];

struct RoundGroup<'a> {
    round: u32,
    phases: Vec<(String, Vec<(usize, &'a StructuredLogEntry)>)>,
}

fn phase_name(phase: &str) -> &str {
    match phase {
        "draw" => "Draw",
        "draft" => "Draft",
        "action" => "Action",
        _ => phase,
    }
}

// ── MCTS analysis types ──

pub struct MctsAnalysisResult {
    pub iterations_used: u32,
    pub tree_stats: TreeStats,
    pub root: MctsNode,
}

// ── Main state ──

pub struct GameViewerState {
    pub game: Option<StructuredGameLog>,
    pub loaded_path: Option<String>,
    pub error: Option<String>,
    pub selected_player: Option<usize>,
    card_map: Option<HashMap<u32, CardInstance>>,
    sell_card_map: Option<HashMap<u32, SellCardInstance>>,
    // Raw JSON initial state for replay
    initial_state_json: Option<serde_json::Value>,
    // Replay state
    selected_entry_index: Option<usize>,
    replayed_state: Option<GameState>,
    // MCTS analysis
    mcts_config: MctsGuiConfig,
    mcts_receiver: Option<mpsc::Receiver<MctsAnalysisResult>>,
    mcts_result: Option<MctsAnalysisResult>,
}

struct MctsGuiConfig {
    iterations: u32,
    exploration_constant: f64,
    max_rollout_steps: u32,
    use_heuristic_eval: bool,
    heuristic_rollout: bool,
    early_termination: bool,
    heuristic_params_path: String,
}

impl Default for MctsGuiConfig {
    fn default() -> Self {
        let defaults = MctsConfig::default();
        Self {
            iterations: 10_000,
            exploration_constant: defaults.exploration_constant,
            max_rollout_steps: defaults.max_rollout_steps,
            use_heuristic_eval: defaults.use_heuristic_eval,
            heuristic_rollout: defaults.heuristic_rollout,
            early_termination: defaults.early_termination,
            heuristic_params_path: "genetic-algorithm/batch-rqo1vv-gen-18.json".to_string(),
        }
    }
}

impl MctsGuiConfig {
    fn to_mcts_config(&self) -> MctsConfig {
        let heuristic_params = if !self.heuristic_params_path.is_empty() {
            match std::fs::read_to_string(&self.heuristic_params_path) {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(_) => HeuristicParams::default(),
            }
        } else {
            HeuristicParams::default()
        };
        MctsConfig {
            iterations: self.iterations,
            exploration_constant: self.exploration_constant,
            max_rollout_steps: self.max_rollout_steps,
            use_heuristic_eval: self.use_heuristic_eval,
            heuristic_rollout: self.heuristic_rollout,
            early_termination: self.early_termination,
            subtree_reuse: true, // Always true so we get the tree back
            heuristic_params,
            ..MctsConfig::default()
        }
    }
}

impl GameViewerState {
    pub fn new() -> Self {
        Self {
            game: None,
            loaded_path: None,
            error: None,
            selected_player: None,
            card_map: None,
            sell_card_map: None,
            initial_state_json: None,
            selected_entry_index: None,
            replayed_state: None,
            mcts_config: MctsGuiConfig::default(),
            mcts_receiver: None,
            mcts_result: None,
        }
    }

    pub fn load_file(&mut self, path: &std::path::Path) {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                // Parse raw JSON to extract initialState for replay
                let raw: Result<serde_json::Value, _> = serde_json::from_str(&contents);
                let initial_state_json = raw
                    .as_ref()
                    .ok()
                    .and_then(|v| v.get("initialState").cloned());

                match serde_json::from_str::<StructuredGameLog>(&contents) {
                    Ok(game) => {
                        let card_map = build_card_instance_map(&game);
                        let sell_card_map = build_sell_card_instance_map(&game);
                        self.card_map = Some(card_map);
                        self.sell_card_map = Some(sell_card_map);
                        self.initial_state_json = initial_state_json;
                        self.game = Some(game);
                        self.loaded_path = Some(path.display().to_string());
                        self.error = None;
                        self.selected_player = None;
                        self.selected_entry_index = None;
                        self.replayed_state = None;
                        self.mcts_result = None;
                        self.mcts_receiver = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to parse game log: {}", e));
                        self.game = None;
                        self.card_map = None;
                        self.sell_card_map = None;
                        self.initial_state_json = None;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to read file: {}", e));
                self.game = None;
                self.card_map = None;
                self.sell_card_map = None;
                self.initial_state_json = None;
            }
        }
    }

    fn replay_to_entry(&mut self, entry_index: usize) {
        let game = match &self.game {
            Some(g) => g,
            None => return,
        };
        let initial_state_json = match &self.initial_state_json {
            Some(j) => j,
            None => {
                self.error = Some("No initial state JSON available for replay".to_string());
                return;
            }
        };

        let state = replay_to(
            initial_state_json,
            &game.initial_draws,
            &game.entries,
            entry_index,
        );
        self.replayed_state = Some(state);
        self.selected_entry_index = Some(entry_index);
        self.mcts_result = None;
        self.mcts_receiver = None;
    }

    fn start_mcts_analysis(&mut self) {
        let state = match &self.replayed_state {
            Some(s) => s.clone(),
            None => return,
        };

        let player_index = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            _ => return,
        };

        let config = self.mcts_config.to_mcts_config();
        let max_rollout_round = std::cmp::max(8, state.round + 2);

        let (tx, rx) = mpsc::channel();
        self.mcts_receiver = Some(rx);
        self.mcts_result = None;

        std::thread::spawn(move || {
            let mut rng = WyRand::seed_from_u64(42);
            let result = ismcts(
                &state,
                player_index,
                &config,
                &None,
                Some(max_rollout_round),
                None,
                &mut rng,
            );
            if let Some(root) = result.tree {
                let tree_stats = root.tree_stats();
                let iterations_used = root.visit_count();
                let _ = tx.send(MctsAnalysisResult {
                    iterations_used,
                    tree_stats,
                    root,
                });
            }
        });
    }

    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Poll for MCTS completion
        if let Some(rx) = &self.mcts_receiver {
            if let Ok(result) = rx.try_recv() {
                self.mcts_result = Some(result);
                self.mcts_receiver = None;
            } else {
                ctx.request_repaint();
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Load Game Log...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    self.load_file(&path);
                }
            }
            if let Some(ref path) = self.loaded_path {
                ui.add(egui::Label::new(egui::RichText::new(path).monospace().small()).selectable(true));
            }
        });

        if let Some(ref error) = self.error {
            ui.colored_label(egui::Color32::RED, error);
        }

        if self.game.is_none() {
            return;
        }

        // We need to collect click events during rendering, then process after
        let mut clicked_entry: Option<usize> = None;
        let mut run_mcts = false;

        let game = self.game.as_ref().unwrap();
        let card_map = self.card_map.as_ref().unwrap();
        let sell_card_map = self.sell_card_map.as_ref().unwrap();
        let selected_player = self.selected_player;
        let selected_entry = self.selected_entry_index;
        let replayed_state = self.replayed_state.as_ref();
        let mcts_receiver = &self.mcts_receiver;
        let mcts_result = self.mcts_result.as_ref();
        let mcts_config = &mut self.mcts_config;

        // Split view: left = timeline, right = state + analysis
        let has_selection = replayed_state.is_some();

        ui.horizontal(|ui| {
            // Summary bar
            render_summary(ui, game);
        });
        ui.add_space(4.0);
        render_player_filter(ui, game, &mut self.selected_player);
        ui.add_space(4.0);

        let panel_width = if has_selection {
            ui.available_width() * 0.5
        } else {
            ui.available_width()
        };

        ui.horizontal_top(|ui| {
            // Left panel: timeline
            ui.vertical(|ui| {
                ui.set_width(panel_width);
                egui::ScrollArea::vertical()
                    .id_salt("timeline_scroll")
                    .show(ui, |ui| {
                        render_final_state(ui, game, selected_player);
                        ui.add_space(8.0);
                        render_timeline(
                            ui,
                            game,
                            card_map,
                            sell_card_map,
                            selected_player,
                            selected_entry,
                            &mut clicked_entry,
                        );
                    });
            });

            // Right panel: state display + MCTS analysis
            if has_selection {
                ui.separator();
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("state_scroll")
                        .show(ui, |ui| {
                            if let Some(state) = replayed_state {
                                render_replayed_state(ui, state, game);
                                ui.add_space(12.0);
                                render_mcts_section(
                                    ui,
                                    mcts_config,
                                    mcts_receiver.is_some(),
                                    mcts_result,
                                    &mut run_mcts,
                                );
                            }
                        });
                });
            }
        });

        // Process deferred actions
        if let Some(idx) = clicked_entry {
            self.replay_to_entry(idx);
        }
        if run_mcts {
            self.start_mcts_analysis();
        }
    }
}

// ── Rendering helpers ──

fn render_summary(ui: &mut egui::Ui, game: &StructuredGameLog) {
    ui.horizontal_wrapped(|ui| {
        if let Some(ref final_scores) = game.final_scores {
            for (i, score) in final_scores.iter().enumerate() {
                let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
                ui.colored_label(color, format!("{}: {}", score.name, score.score));
                ui.separator();
            }
        }

        let max_round = game.entries.iter().map(|e| e.round).max().unwrap_or(0);
        ui.label(format!("Rounds: {}", max_round));
        ui.separator();

        if let Some(ref final_scores) = game.final_scores {
            let best = final_scores
                .iter()
                .map(|fs| final_score_ranking(fs))
                .max()
                .unwrap_or((0, 0, 0));
            let winners: Vec<&str> = final_scores
                .iter()
                .filter(|fs| final_score_ranking(fs) == best)
                .map(|fs| fs.name.as_str())
                .collect();
            ui.label(format!("Winner: {}", winners.join(", ")));
            ui.separator();
        }

        ui.label(format!("Plies: {}", game.entries.len()));
        ui.separator();

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
                    for &c in &ALL_COLORS {
                        let count = stats.color_wheel.get(c);
                        if count > 0 {
                            ui.label(format!("{:?}: {}", c, count));
                        }
                    }
                    ui.label(format!("Deck size: {}", stats.deck_size));
                    ui.label(format!("Ducats: {}", stats.ducats));
                    for &mt in &ALL_MATERIAL_TYPES {
                        let count = stats.materials.get(mt);
                        if count > 0 {
                            ui.label(format!("{:?}: {}", mt, count));
                        }
                    }
                    if !stats.completed_sell_cards.is_empty() {
                        ui.label("Completed sell cards:");
                        for sell_card in &stats.completed_sell_cards {
                            ui.label(format!(
                                "  {}",
                                sell_card_name_from_instance(sell_card.sell_card)
                            ));
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

    for (global_idx, entry) in entries.iter().enumerate() {
        if let Some(sel) = selected_player {
            if entry.player_index != sel {
                continue;
            }
        }

        if current_round != Some(entry.round) {
            groups.push(RoundGroup {
                round: entry.round,
                phases: Vec::new(),
            });
            current_round = Some(entry.round);
        }

        let group = groups.last_mut().unwrap();
        let phase_key = entry.phase.clone();
        if let Some(phase_group) = group.phases.iter_mut().find(|(p, _)| *p == phase_key) {
            phase_group.1.push((global_idx, entry));
        } else {
            group.phases.push((phase_key, vec![(global_idx, entry)]));
        }
    }

    groups
}

fn render_timeline(
    ui: &mut egui::Ui,
    game: &StructuredGameLog,
    _card_map: &HashMap<u32, CardInstance>,
    _sell_card_map: &HashMap<u32, SellCardInstance>,
    selected_player: Option<usize>,
    selected_entry: Option<usize>,
    clicked_entry: &mut Option<usize>,
) {
    let round_groups = build_round_groups(&game.entries, selected_player);

    for group in &round_groups {
        egui::CollapsingHeader::new(format!("Round {}", group.round))
            .default_open(true)
            .show(ui, |ui| {
                for (phase, entries) in &group.phases {
                    ui.strong(phase_name(phase));
                    ui.indent(format!("round_{}_phase_{}", group.round, phase), |ui| {
                        for &(global_idx, entry) in entries {
                            let player_color =
                                PLAYER_COLORS[entry.player_index % PLAYER_COLORS.len()];
                            let player_name = game
                                .player_names
                                .get(entry.player_index)
                                .map(|s| s.as_str())
                                .unwrap_or("Unknown");
                            let choice_text = format_choice(&entry.choice);

                            let is_selected = selected_entry == Some(global_idx);

                            ui.horizontal(|ui| {
                                ui.colored_label(player_color, player_name);
                                let label = egui::Label::new(
                                    egui::RichText::new(&choice_text).background_color(
                                        if is_selected {
                                            egui::Color32::from_rgb(60, 60, 90)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        },
                                    ),
                                )
                                .sense(egui::Sense::click());
                                if ui.add(label).clicked() {
                                    *clicked_entry = Some(global_idx);
                                }
                            });
                        }
                    });
                }
            });
    }
}

// ── Replayed state display ──

fn render_replayed_state(ui: &mut egui::Ui, state: &GameState, game: &StructuredGameLog) {
    ui.heading("Game State");

    // Phase info
    let phase_text = match &state.phase {
        GamePhase::Draw => "Draw".to_string(),
        GamePhase::Draft { draft_state } => {
            let name = game
                .player_names
                .get(draft_state.current_player_index)
                .cloned()
                .unwrap_or_else(|| format!("Player {}", draft_state.current_player_index));
            format!("Draft (pick {}, {}'s turn)", draft_state.pick_number + 1, name)
        }
        GamePhase::Action { action_state } => {
            let name = game
                .player_names
                .get(action_state.current_player_index)
                .cloned()
                .unwrap_or_else(|| format!("Player {}", action_state.current_player_index));
            format!("Action ({}'s turn)", name)
        }
        GamePhase::GameOver => "Game Over".to_string(),
    };
    ui.label(format!("Round {} — {}", state.round, phase_text));
    ui.add_space(4.0);

    // Sell card display
    if !state.sell_card_display.is_empty() {
        egui::CollapsingHeader::new("Sell Card Display")
            .default_open(false)
            .show(ui, |ui| {
                for sc in state.sell_card_display.iter() {
                    ui.label(sell_card_name_from_instance(sc.sell_card));
                }
            });
        ui.add_space(4.0);
    }

    // Per-player state
    for (i, player) in state.players.iter().enumerate() {
        let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
        let name = game
            .player_names
            .get(i)
            .cloned()
            .unwrap_or_else(|| format!("Player {}", i));
        let score = calculate_score(player);

        egui::CollapsingHeader::new(
            egui::RichText::new(format!("{} ({})", name, score)).color(color),
        )
        .default_open(true)
        .show(ui, |ui| {
            ui.label(format!("Ducats: {}", player.ducats));

            // Card counts
            ui.label(format!(
                "Deck: {} | Discard: {} | Workshop: {} | Drafted: {}",
                player.deck.len(),
                player.discard.len(),
                player.workshop_cards.len(),
                player.drafted_cards.len()
            ));

            // Workshop cards (by name)
            if !player.workshop_cards.is_empty() {
                let names: Vec<String> = player
                    .workshop_cards
                    .iter()
                    .map(|id| format!("{:?}", state.card_lookup[id as usize]))
                    .collect();
                ui.label(format!("  Workshop: {}", names.join(", ")));
            }

            // Drafted cards (by name)
            if !player.drafted_cards.is_empty() {
                let names: Vec<String> = player
                    .drafted_cards
                    .iter()
                    .map(|id| format!("{:?}", state.card_lookup[id as usize]))
                    .collect();
                ui.label(format!("  Drafted: {}", names.join(", ")));
            }

            // Color wheel (non-zero)
            let colors: Vec<String> = ALL_COLORS
                .iter()
                .filter(|&&c| player.color_wheel.get(c) > 0)
                .map(|&c| format!("{:?}: {}", c, player.color_wheel.get(c)))
                .collect();
            if !colors.is_empty() {
                ui.label(format!("Colors: {}", colors.join(", ")));
            }

            // Materials (non-zero)
            let mats: Vec<String> = ALL_MATERIAL_TYPES
                .iter()
                .filter(|&&mt| player.materials.get(mt) > 0)
                .map(|&mt| format!("{:?}: {}", mt, player.materials.get(mt)))
                .collect();
            if !mats.is_empty() {
                ui.label(format!("Materials: {}", mats.join(", ")));
            }

            // Completed sell cards
            if !player.completed_sell_cards.is_empty() {
                let names: Vec<String> = player
                    .completed_sell_cards
                    .iter()
                    .map(|sc| sell_card_name_from_instance(sc.sell_card))
                    .collect();
                ui.label(format!("Sell cards: {}", names.join(", ")));
            }
        });
    }

    // Draft hands (if in draft phase)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        ui.add_space(4.0);
        egui::CollapsingHeader::new("Draft Hands")
            .default_open(true)
            .show(ui, |ui| {
                for i in 0..state.players.len() {
                    let hand = &draft_state.hands[i];
                    if hand.is_empty() {
                        continue;
                    }
                    let color = PLAYER_COLORS[i % PLAYER_COLORS.len()];
                    let name = game
                        .player_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("Player {}", i));
                    let cards: Vec<String> = hand
                        .iter()
                        .map(|id| format!("{:?}", state.card_lookup[id as usize]))
                        .collect();
                    ui.colored_label(color, format!("{}: {}", name, cards.join(", ")));
                }
            });
    }

    // Available choices
    let choices = enumerate_choices(state);
    if !choices.is_empty() {
        ui.add_space(4.0);
        egui::CollapsingHeader::new(format!("Available Choices ({})", choices.len()))
            .default_open(false)
            .show(ui, |ui| {
                for choice in &choices {
                    ui.label(format_choice(choice));
                }
            });
    }
}

// ── MCTS analysis section ──

fn render_mcts_section(
    ui: &mut egui::Ui,
    config: &mut MctsGuiConfig,
    is_running: bool,
    result: Option<&MctsAnalysisResult>,
    run_mcts: &mut bool,
) {
    ui.separator();
    ui.heading("MCTS Analysis");

    egui::CollapsingHeader::new("Configuration")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Iterations:");
                ui.add(egui::DragValue::new(&mut config.iterations).range(100..=1_000_000));
            });
            ui.horizontal(|ui| {
                ui.label("Exploration constant:");
                ui.add(
                    egui::DragValue::new(&mut config.exploration_constant)
                        .speed(0.01)
                        .range(0.0..=10.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Max rollout steps:");
                ui.add(egui::DragValue::new(&mut config.max_rollout_steps).range(1..=10000));
            });
            ui.checkbox(&mut config.use_heuristic_eval, "Heuristic eval");
            ui.checkbox(&mut config.heuristic_rollout, "Heuristic rollout");
            ui.checkbox(&mut config.early_termination, "Early termination");
            ui.horizontal(|ui| {
                ui.label("Heuristic params file:");
                ui.text_edit_singleline(&mut config.heuristic_params_path);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        config.heuristic_params_path = path.display().to_string();
                    }
                }
            });
        });

    ui.add_space(4.0);

    if is_running {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Running MCTS analysis...");
        });
    } else if ui.button("Run MCTS").clicked() {
        *run_mcts = true;
    }

    // Results
    if let Some(result) = result {
        ui.add_space(8.0);
        ui.label(format!(
            "{} iterations | {} nodes | max depth {} | avg branching {:.1}",
            result.iterations_used,
            result.tree_stats.total_nodes,
            result.tree_stats.max_depth,
            result.tree_stats.avg_branching_factor,
        ));

        ui.add_space(4.0);
        render_mcts_children(ui, &result.root, 0);
    }
}

fn render_mcts_children(ui: &mut egui::Ui, node: &MctsNode, depth: usize) {
    let mut children: Vec<&MctsNode> = node
        .children()
        .iter()
        .filter(|c| c.visit_count() > 0)
        .collect();
    children.sort_by(|a, b| b.visit_count().cmp(&a.visit_count()));

    if depth == 0 {
        // Header row
        egui::Grid::new("mcts_header")
            .min_col_width(60.0)
            .show(ui, |ui| {
                ui.strong("Choice");
                ui.strong("Visits");
                ui.strong("Avg Reward");
                ui.strong("Max Depth");
                ui.strong("Avg Branch");
                ui.end_row();
            });
    }

    for (i, child) in children.iter().enumerate() {
        let choice_text = child
            .choice()
            .map(|ch| format_choice(ch))
            .unwrap_or_else(|| "?".to_string());
        let stats = child.tree_stats();
        let has_children = !child.children().is_empty();

        if has_children {
            egui::CollapsingHeader::new(
                egui::RichText::new(&choice_text).monospace(),
            )
            .id_salt(format!("mcts_d{}_c{}", depth, i))
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Visits: {}", child.visit_count()));
                    ui.separator();
                    ui.label(format!("Avg reward: {:.3}", child.average_reward()));
                    ui.separator();
                    ui.label(format!("Max depth: {}", stats.max_depth));
                    ui.separator();
                    ui.label(format!("Avg branch: {:.1}", stats.avg_branching_factor));
                });
                ui.add_space(4.0);
                render_mcts_children(ui, child, depth + 1);
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&choice_text).monospace());
                ui.label(format!("{}", child.visit_count()));
                ui.label(format!("{:.3}", child.average_reward()));
            });
        }
    }
}
