use std::collections::HashMap;

use eframe::egui;
use colori_core::game_log::StructuredGameLog;

use crate::analysis::categories::{draft_card_categories, get_starter_card_categories, CardCategory};
use crate::analysis::computations::*;
use crate::analysis::stats::wilson_confidence_interval;
use crate::widgets::bar_table::{bar_table, win_rate_table};
use crate::widgets::stat_grid::{stat_grid, StatCard};

/// Cached analysis results. Recomputed when filter changes.
#[allow(dead_code)]
pub struct CachedAnalysis {
    pub filter_key: String, // "batch:variant" to detect changes
    pub action_dist: HashMap<String, usize>,
    pub draft_freq: HashMap<String, usize>,
    pub draft_freq_normalized: HashMap<String, f64>,
    pub cards_added: HashMap<String, usize>,
    pub cards_added_normalized: HashMap<String, f64>,
    pub destroyed_draft: HashMap<String, usize>,
    pub destroyed_workshop: HashMap<String, usize>,
    pub buyer_acq: BuyerAcquisitions,
    pub deck_stats: DeckSizeStats,
    pub score_dist: std::collections::BTreeMap<u32, usize>,
    pub win_rate_position: HashMap<usize, WinRateEntry>,
    pub game_length: GameLengthStats,
    pub color_stats: HashMap<String, f64>,
    pub duration_stats: Option<DurationStats>,
    pub variant_win_rate: Option<HashMap<String, WinRateEntry>>,
    pub penultimate_deck_sizes: std::collections::BTreeMap<u32, usize>,
    pub round_count_dist: std::collections::BTreeMap<u32, usize>,
    pub card_win_rate: HashMap<String, WinRateEntry>,
    pub draft_win_rate: HashMap<String, WinRateEntry>,
    // Derived
    pub draft_freq_categories: Vec<CategoryStat>,
    pub cards_added_categories: Vec<CategoryStat>,
    pub card_win_rate_categories: Vec<WinRateCategoryStat>,
    pub draft_win_rate_categories: Vec<WinRateCategoryStat>,
    pub destroy_rate: HashMap<String, f64>,
    pub destroy_rate_categories: Vec<CategoryStat>,
    pub destroy_rate_cat_normalized: Vec<CategoryStat>,
    pub winner_buyer_breakdown: WinnerBuyerBreakdown,
}

impl CachedAnalysis {
    pub fn compute(
        logs: &[StructuredGameLog],
        _filter: Option<&PlayerFilter>,
        variant_label: Option<&str>,
    ) -> Self {
        let filter = variant_label.map(|vl| compute_player_filter(logs, vl));
        let filter_ref = filter.as_ref();

        let action_dist = compute_action_distribution(logs, filter_ref);
        let draft_freq = compute_draft_frequency(logs, filter_ref);
        let draft_freq_normalized = normalize_by_draft_copies(&draft_freq);
        let cards_added = compute_cards_added_to_deck(logs, filter_ref);
        let cards_added_normalized = normalize_by_draft_copies(&cards_added);
        let destroyed_draft = compute_destroyed_from_draft(logs, filter_ref);
        let destroyed_workshop = compute_destroyed_from_workshop(logs, filter_ref);
        let buyer_acq = compute_buyer_acquisitions(logs, filter_ref);
        let deck_stats = compute_deck_size_stats(logs, filter_ref);
        let score_dist = compute_score_distribution(logs, filter_ref);
        let win_rate_position = compute_win_rate_by_position(logs, filter_ref);
        let game_length = compute_average_game_length(logs);
        let color_stats = compute_color_wheel_stats(logs, filter_ref);
        let duration_stats = compute_duration_stats(logs);
        let variant_win_rate = compute_win_rate_by_variant(logs);
        let penultimate_deck_sizes = compute_penultimate_round_deck_sizes(logs, filter_ref);
        let round_count_dist = compute_round_count_distribution(logs);
        let card_win_rate = compute_win_rate_by_card(logs, filter_ref);
        let draft_win_rate = compute_win_rate_if_drafted(logs, filter_ref);
        let winner_buyer_breakdown = compute_winner_buyer_breakdown(logs, filter_ref);

        let num_players = if !logs.is_empty() {
            logs[0].player_names.len()
        } else {
            2
        };
        let draft_categories = draft_card_categories();
        let starter_categories = get_starter_card_categories(num_players);
        let mut all_categories: Vec<CardCategory> = draft_card_categories();
        all_categories.extend(starter_categories);

        let draft_freq_categories = compute_category_stats(&draft_freq, &draft_categories);
        let cards_added_categories = compute_category_stats(&cards_added, &all_categories);
        let card_win_rate_categories =
            compute_win_rate_category_stats(&card_win_rate, &all_categories);
        let draft_win_rate_categories =
            compute_win_rate_category_stats(&draft_win_rate, &all_categories);

        let destroy_rate = compute_destroy_rate(&destroyed_draft, &draft_freq);
        let destroy_rate_categories =
            compute_category_stats(&destroyed_draft, &draft_categories);

        // Compute normalized destroy rate by category (destroyed / drafted per category)
        let destroy_rate_cat_normalized: Vec<CategoryStat> = destroy_rate_categories
            .iter()
            .zip(draft_freq_categories.iter())
            .map(|(dc, dfc)| {
                let normalized_rate = if dfc.raw_total > 0.0 {
                    dc.raw_total / dfc.raw_total
                } else {
                    0.0
                };
                CategoryStat {
                    label: dc.label.clone(),
                    raw_total: dc.raw_total,
                    total_copies: dc.total_copies,
                    normalized_rate,
                }
            })
            .collect();

        let filter_key = format!("{}:{}", "computed", variant_label.unwrap_or("all"));

        CachedAnalysis {
            filter_key,
            action_dist,
            draft_freq,
            draft_freq_normalized,
            cards_added,
            cards_added_normalized,
            destroyed_draft,
            destroyed_workshop,
            buyer_acq,
            deck_stats,
            score_dist,
            win_rate_position,
            game_length,
            color_stats,
            duration_stats,
            variant_win_rate,
            penultimate_deck_sizes,
            round_count_dist,
            card_win_rate,
            draft_win_rate,
            draft_freq_categories,
            cards_added_categories,
            card_win_rate_categories,
            draft_win_rate_categories,
            destroy_rate,
            destroy_rate_categories,
            destroy_rate_cat_normalized,
            winner_buyer_breakdown,
        }
    }
}

/// Render the Analysis tab content.
pub fn render_analysis_tab(ui: &mut egui::Ui, analysis: &CachedAnalysis, num_games: usize) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // 1. Game Overview
        let id = ui.make_persistent_id("game_overview");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.strong("Game Overview");
            })
            .body(|ui| {
                let mut cards = vec![
                    StatCard {
                        value: format!("{}", num_games),
                        label: "Games".into(),
                    },
                    StatCard {
                        value: format!("{:.1}", analysis.game_length.avg_rounds),
                        label: "Avg Rounds".into(),
                    },
                    StatCard {
                        value: format!("{:.1}", analysis.deck_stats.mean),
                        label: "Avg Deck Size".into(),
                    },
                    StatCard {
                        value: format!("{:.0}", analysis.deck_stats.median),
                        label: "Median Deck Size".into(),
                    },
                    StatCard {
                        value: format!(
                            "{} - {}",
                            analysis.deck_stats.min, analysis.deck_stats.max
                        ),
                        label: "Deck Size Range".into(),
                    },
                ];
                if let Some(ref ds) = analysis.duration_stats {
                    cards.push(StatCard {
                        value: format!("{:.1}s", ds.avg_ms / 1000.0),
                        label: "Avg Duration".into(),
                    });
                    cards.push(StatCard {
                        value: format!("{:.1}s", ds.median_ms / 1000.0),
                        label: "Median Duration".into(),
                    });
                }
                stat_grid(ui, &cards);
            });

        // 2-4. Score, Game Length, and Deck Size distributions side by side
        ui.columns(3, |columns| {
            render_btree_bar_section(
                &mut columns[0],
                "Score Distribution",
                "Score",
                "Count",
                &analysis.score_dist,
                true,
            );
            render_btree_bar_section(
                &mut columns[1],
                "Game Length (Rounds) Distribution",
                "Rounds",
                "Count",
                &analysis.round_count_dist,
                true,
            );
            render_btree_bar_section(
                &mut columns[2],
                "Deck Size Distribution (Penultimate Round)",
                "Deck Size",
                "Count",
                &analysis.penultimate_deck_sizes,
                true,
            );
        });

        // 5. Win Rate by Position
        let id = ui.make_persistent_id("win_rate_position");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.strong("Win Rate by Position");
            })
            .body(|ui| {
                let mut positions: Vec<usize> =
                    analysis.win_rate_position.keys().cloned().collect();
                positions.sort();
                let rows: Vec<_> = positions
                    .iter()
                    .map(|&pos| {
                        let entry = &analysis.win_rate_position[&pos];
                        let ci = wilson_confidence_interval(entry.wins, entry.games);
                        (format!("Player {}", pos + 1), entry.wins, entry.games, ci)
                    })
                    .collect();
                win_rate_table(ui, "Position", "Games", &rows);
            });

        // 6. Win Rate by Variant (conditional)
        if let Some(ref variant_wr) = analysis.variant_win_rate {
            let id = ui.make_persistent_id("win_rate_variant");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    ui.strong("Win Rate by Variant");
                })
                .body(|ui| {
                    let mut entries: Vec<_> = variant_wr.iter().collect();
                    entries.sort_by(|a, b| {
                        let rate_a = if a.1.games > 0.0 {
                            a.1.wins / a.1.games
                        } else {
                            0.0
                        };
                        let rate_b = if b.1.games > 0.0 {
                            b.1.wins / b.1.games
                        } else {
                            0.0
                        };
                        rate_b.partial_cmp(&rate_a).unwrap()
                    });
                    let rows: Vec<_> = entries
                        .iter()
                        .map(|(label, entry)| {
                            let ci = wilson_confidence_interval(entry.wins, entry.games);
                            (label.to_string(), entry.wins, entry.games, ci)
                        })
                        .collect();
                    win_rate_table(ui, "Variant", "Games", &rows);
                });
        }

        // --- Draft Analysis ---
        section_group_heading(ui, "Draft Analysis");

        let id = ui.make_persistent_id("draft_frequency");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Draft Frequency");
            })
            .body(|ui| {
                ui.strong("By Category");
                render_category_bar_rows(ui, &analysis.draft_freq_categories, "Drafts per Copy");
                ui.add_space(8.0);
                ui.strong("By Card");
                render_f64_hashmap_bar_rows(
                    ui,
                    &analysis.draft_freq_normalized,
                    "Drafts per Copy",
                    2,
                );
            });

        // 9. Cards Added to Deck
        let id = ui.make_persistent_id("cards_added_deck");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Cards Added to Deck");
            })
            .body(|ui| {
                ui.strong("By Category");
                render_category_bar_rows(
                    ui,
                    &analysis.cards_added_categories,
                    "Added per Copy",
                );
                ui.add_space(8.0);
                ui.strong("By Card");
                render_f64_hashmap_bar_rows(
                    ui,
                    &analysis.cards_added_normalized,
                    "Added per Copy",
                    2,
                );
            });

        // --- Win Rates ---
        section_group_heading(ui, "Win Rates");

        render_win_rate_card_section(
            ui,
            "Win Rate by Card in Deck",
            "Times Taken",
            &analysis.card_win_rate,
            &analysis.card_win_rate_categories,
        );

        // 11. Win Rate if Drafted
        render_win_rate_card_section(
            ui,
            "Win Rate if Drafted",
            "Times Drafted",
            &analysis.draft_win_rate,
            &analysis.draft_win_rate_categories,
        );

        // --- Actions & Buyers ---
        section_group_heading(ui, "Actions & Buyers");

        render_hashmap_bar_section(
            ui,
            "Action Distribution",
            "Action",
            "Count",
            &analysis.action_dist,
            false,
        );

        // Destroyed from Draft
        let id = ui.make_persistent_id("destroyed_draft");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Destroyed from Draft");
            })
            .body(|ui| {
                ui.strong("By Category");
                render_category_bar_rows_pct(
                    ui,
                    &analysis.destroy_rate_cat_normalized,
                    "Destroy Rate",
                );
                ui.add_space(8.0);
                ui.strong("By Card");
                render_f64_hashmap_bar_rows_pct(ui, &analysis.destroy_rate, "Destroy Rate");
            });

        // 13. Destroyed from Workshop
        render_hashmap_bar_section(
            ui,
            "Destroyed from Workshop",
            "Card",
            "Count",
            &analysis.destroyed_workshop,
            false,
        );

        // 14. Buyer Acquisitions
        let id = ui.make_persistent_id("buyer_acquisitions");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Buyer Acquisitions");
            })
            .body(|ui| {
                ui.strong("By Buyer");
                render_usize_hashmap_bar_rows(ui, &analysis.buyer_acq.by_buyer, "Count");
                ui.add_space(8.0);
                ui.strong("By Stars");
                render_u32_hashmap_bar_rows(
                    ui,
                    &analysis.buyer_acq.by_stars,
                    "Stars",
                    "Count",
                );
                ui.add_space(8.0);
                ui.strong("By Material");
                render_usize_hashmap_bar_rows(ui, &analysis.buyer_acq.by_material, "Count");
            });

        // --- End State ---
        section_group_heading(ui, "End State");

        // Color Wheel at End
        let id = ui.make_persistent_id("color_wheel_end");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Color Wheel at End");
            })
            .body(|ui| {
                render_f64_hashmap_bar_rows(ui, &analysis.color_stats, "Average Count", 2);
            });

        // Winner Buyer Breakdown
        let id = ui.make_persistent_id("winner_buyer_breakdown");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Winner Buyer Breakdown");
            })
            .body(|ui| {
                let wb = &analysis.winner_buyer_breakdown;
                let cards = vec![
                    StatCard {
                        value: format!("{:.1}", wb.avg_textiles),
                        label: "Avg Textiles".into(),
                    },
                    StatCard {
                        value: format!("{:.1}", wb.avg_ceramics),
                        label: "Avg Ceramics".into(),
                    },
                    StatCard {
                        value: format!("{:.1}", wb.avg_paintings),
                        label: "Avg Paintings".into(),
                    },
                ];
                stat_grid(ui, &cards);
            });
    });
}

// -- Helper rendering functions --

/// Render a section group heading with a separator line above it.
fn section_group_heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(title)
            .size(14.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(2.0);
}

fn render_btree_bar_section(
    ui: &mut egui::Ui,
    title: &str,
    key_header: &str,
    val_header: &str,
    data: &std::collections::BTreeMap<u32, usize>,
    default_open: bool,
) {
    let id = ui.make_persistent_id(title);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
        .show_header(ui, |ui| {
            ui.strong(title);
        })
        .body(|ui| {
            if data.is_empty() {
                ui.label("No data available.");
                return;
            }
            let max_val = *data.values().max().unwrap_or(&1) as f32;
            let rows: Vec<_> = data
                .iter()
                .map(|(&k, &v)| (format!("{}", k), format!("{}", v), v as f32 / max_val))
                .collect();
            bar_table(ui, &[key_header, val_header], &rows);
        });
}

fn render_hashmap_bar_section(
    ui: &mut egui::Ui,
    title: &str,
    key_header: &str,
    val_header: &str,
    data: &HashMap<String, usize>,
    default_open: bool,
) {
    let id = ui.make_persistent_id(title);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
        .show_header(ui, |ui| {
            ui.strong(title);
        })
        .body(|ui| {
            if data.is_empty() {
                ui.label("No data available.");
                return;
            }
            let mut sorted: Vec<_> = data.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            let max_val = *sorted.first().map(|(_, v)| *v).unwrap_or(&1) as f32;
            let rows: Vec<_> = sorted
                .iter()
                .map(|(k, &v)| (k.to_string(), format!("{}", v), v as f32 / max_val))
                .collect();
            bar_table(ui, &[key_header, val_header], &rows);
        });
}

fn render_category_bar_rows(ui: &mut egui::Ui, categories: &[CategoryStat], val_header: &str) {
    if categories.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = categories.iter().collect();
    sorted.sort_by(|a, b| b.normalized_rate.partial_cmp(&a.normalized_rate).unwrap());
    let max_val = sorted
        .iter()
        .map(|c| c.normalized_rate)
        .fold(0.0_f64, f64::max) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|c| {
            let frac = if max_val > 0.0 {
                c.normalized_rate as f32 / max_val
            } else {
                0.0
            };
            (c.label.clone(), format!("{:.2}", c.normalized_rate), frac)
        })
        .collect();
    bar_table(ui, &["Category", val_header], &rows);
}

fn render_f64_hashmap_bar_rows(
    ui: &mut egui::Ui,
    data: &HashMap<String, f64>,
    val_header: &str,
    decimals: usize,
) {
    if data.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = data.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    let max_val = sorted.first().map(|(_, v)| **v).unwrap_or(1.0) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|(k, &v)| {
            let frac = if max_val > 0.0 {
                v as f32 / max_val
            } else {
                0.0
            };
            (k.to_string(), format!("{:.prec$}", v, prec = decimals), frac)
        })
        .collect();
    bar_table(ui, &["Card", val_header], &rows);
}

fn render_f64_hashmap_bar_rows_pct(
    ui: &mut egui::Ui,
    data: &HashMap<String, f64>,
    val_header: &str,
) {
    if data.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = data.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    let max_val = sorted.first().map(|(_, v)| **v).unwrap_or(1.0) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|(k, &v)| {
            let frac = if max_val > 0.0 {
                v as f32 / max_val
            } else {
                0.0
            };
            (k.to_string(), format!("{:.1}%", v * 100.0), frac)
        })
        .collect();
    bar_table(ui, &["Card", val_header], &rows);
}

fn render_category_bar_rows_pct(
    ui: &mut egui::Ui,
    categories: &[CategoryStat],
    val_header: &str,
) {
    if categories.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = categories.iter().collect();
    sorted.sort_by(|a, b| b.normalized_rate.partial_cmp(&a.normalized_rate).unwrap());
    let max_val = sorted
        .iter()
        .map(|c| c.normalized_rate)
        .fold(0.0_f64, f64::max) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|c| {
            let frac = if max_val > 0.0 {
                c.normalized_rate as f32 / max_val
            } else {
                0.0
            };
            (
                c.label.clone(),
                format!("{:.1}%", c.normalized_rate * 100.0),
                frac,
            )
        })
        .collect();
    bar_table(ui, &["Category", val_header], &rows);
}

fn render_usize_hashmap_bar_rows(
    ui: &mut egui::Ui,
    data: &HashMap<String, usize>,
    val_header: &str,
) {
    if data.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = data.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    let max_val = *sorted.first().map(|(_, v)| *v).unwrap_or(&1) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|(k, &v)| (k.to_string(), format!("{}", v), v as f32 / max_val))
        .collect();
    bar_table(ui, &["Name", val_header], &rows);
}

fn render_u32_hashmap_bar_rows(
    ui: &mut egui::Ui,
    data: &HashMap<u32, usize>,
    key_header: &str,
    val_header: &str,
) {
    if data.is_empty() {
        return;
    }
    let mut sorted: Vec<_> = data.iter().collect();
    sorted.sort_by_key(|(&k, _)| k);
    let max_val = sorted.iter().map(|(_, &v)| v).max().unwrap_or(1) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|(&k, &v)| (format!("{}", k), format!("{}", v), v as f32 / max_val))
        .collect();
    bar_table(ui, &[key_header, val_header], &rows);
}

fn render_win_rate_card_section(
    ui: &mut egui::Ui,
    title: &str,
    games_header: &str,
    card_win_rate: &HashMap<String, WinRateEntry>,
    category_stats: &[WinRateCategoryStat],
) {
    let id = ui.make_persistent_id(title);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.strong(title);
        })
        .body(|ui| {
            if card_win_rate.is_empty() {
                ui.label("No data available.");
                return;
            }

            // All summary
            ui.strong("All");
            let all_wins: f64 = card_win_rate.values().map(|e| e.wins).sum();
            let all_games: f64 = card_win_rate.values().map(|e| e.games).sum();
            let all_ci = wilson_confidence_interval(all_wins, all_games);
            win_rate_table(
                ui,
                "Label",
                games_header,
                &[("All Cards".into(), all_wins, all_games, all_ci)],
            );

            ui.add_space(8.0);

            // By Category
            ui.strong("By Category");
            let mut cat_sorted: Vec<_> = category_stats.iter().collect();
            cat_sorted.sort_by(|a, b| {
                let rate_a = if a.games > 0.0 {
                    a.wins / a.games
                } else {
                    0.0
                };
                let rate_b = if b.games > 0.0 {
                    b.wins / b.games
                } else {
                    0.0
                };
                rate_b.partial_cmp(&rate_a).unwrap()
            });
            let cat_rows: Vec<_> = cat_sorted
                .iter()
                .map(|c| {
                    let ci = wilson_confidence_interval(c.wins, c.games);
                    (c.label.clone(), c.wins, c.games, ci)
                })
                .collect();
            win_rate_table(ui, "Category", games_header, &cat_rows);

            ui.add_space(8.0);

            // By Card
            ui.strong("By Card");
            let mut card_sorted: Vec<_> = card_win_rate.iter().collect();
            card_sorted.sort_by(|a, b| {
                let rate_a = if a.1.games > 0.0 {
                    a.1.wins / a.1.games
                } else {
                    0.0
                };
                let rate_b = if b.1.games > 0.0 {
                    b.1.wins / b.1.games
                } else {
                    0.0
                };
                rate_b.partial_cmp(&rate_a).unwrap()
            });
            let card_rows: Vec<_> = card_sorted
                .iter()
                .map(|(name, entry)| {
                    let ci = wilson_confidence_interval(entry.wins, entry.games);
                    (name.to_string(), entry.wins, entry.games, ci)
                })
                .collect();
            win_rate_table(ui, "Card", games_header, &card_rows);
        });
}
