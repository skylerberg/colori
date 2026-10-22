use std::collections::HashMap;

use eframe::egui;
use colori_core::draft_phase::DRAFT_HAND_SIZE;
use colori_core::game_log::StructuredGameLog;
use colori_core::scoring::format_half_points;

use crate::analysis::categories::{draft_card_categories, get_starter_card_categories, CardCategory};
use crate::analysis::computations::*;
use crate::analysis::stats::wilson_confidence_interval;
use crate::widgets::bar_table::{bar_table, win_rate_color, win_rate_table};
use crate::widgets::stat_grid::{stat_grid, StatCard};

/// Cached analysis results. Recomputed when filter changes.
#[allow(dead_code)]
pub struct CachedAnalysis {
    pub filter_key: String, // "batch:variant" to detect changes
    pub action_dist: HashMap<String, usize>,
    pub pick_positions: HashMap<String, PickPosition>,
    pub cards_added: HashMap<String, usize>,
    pub cards_added_normalized: HashMap<String, f64>,
    pub trashed_draft: HashMap<String, usize>,
    pub draft_pool_moves: DraftPoolMoves,
    pub buyer_acq: BuyerAcquisitions,
    pub deck_stats: DeckSizeStats,
    /// Keyed by half points.
    pub score_dist: std::collections::BTreeMap<u32, usize>,
    pub win_rate_position: HashMap<usize, WinRateEntry>,
    pub avg_plies: f64,
    pub unseen_buyer_share: Option<f64>,
    pub color_stats: HashMap<String, f64>,
    pub duration_stats: Option<DurationStats>,
    pub variant_win_rate: Option<HashMap<String, WinRateEntry>>,
    pub skill_stats: Option<SkillChanceStats>,
    pub penultimate_deck_sizes: std::collections::BTreeMap<u32, usize>,
    pub card_win_rate: HashMap<String, WinRateEntry>,
    pub buyer_type_win_rates: Vec<BuyerTypeWinRates>,
    pub draft_win_rate: HashMap<String, WinRateEntry>,
    // Derived
    pub cards_added_categories: Vec<CategoryStat>,
    pub card_win_rate_categories: Vec<WinRateCategoryStat>,
    pub draft_win_rate_categories: Vec<WinRateCategoryStat>,
    pub trash_rate: HashMap<String, f64>,
    pub trash_rate_categories: Vec<CategoryStat>,
    pub trash_rate_cat_normalized: Vec<CategoryStat>,
    pub winner_buyer_breakdown: WinnerBuyerBreakdown,
    pub num_games: usize,
    pub num_players: usize,
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
        let pick_positions = compute_pick_positions(logs, filter_ref);
        let cards_added = compute_cards_added_to_deck(logs, filter_ref);
        let cards_added_normalized = normalize_by_draft_copies(&cards_added);
        let trashed_draft = compute_trashed_from_draft(logs, filter_ref);
        let draft_pool_moves = compute_draft_pool_moves(logs, filter_ref);
        let buyer_acq = compute_buyer_acquisitions(logs, filter_ref);
        let deck_stats = compute_deck_size_stats(logs, filter_ref);
        let score_dist = compute_score_distribution(logs, filter_ref);
        let win_rate_position = compute_win_rate_by_position(logs, filter_ref);
        let avg_plies = compute_average_plies(logs, filter_ref);
        let unseen_buyer_share = compute_unseen_buyer_share(logs, filter_ref);
        let color_stats = compute_color_wheel_stats(logs, filter_ref);
        let duration_stats = compute_duration_stats(logs, filter_ref);
        let variant_win_rate = compute_win_rate_by_variant(logs);
        let skill_stats = compute_skill_chance_stats(logs);
        let penultimate_deck_sizes = compute_penultimate_round_deck_sizes(logs, filter_ref);
        let card_win_rate = compute_win_rate_by_card(logs, filter_ref);
        let buyer_type_win_rates = compute_win_rate_by_buyer_claims(logs, filter_ref);
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

        let trash_rate = compute_trash_rate(&trashed_draft, &draft_freq);
        let trash_rate_categories =
            compute_category_stats(&trashed_draft, &draft_categories);

        // Compute normalized trash rate by category (trashed / drafted per category)
        let trash_rate_cat_normalized: Vec<CategoryStat> = trash_rate_categories
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

        let num_games = match filter_ref {
            Some(f) => f.len(),
            None => logs.len(),
        };

        let filter_key = format!("{}:{}", "computed", variant_label.unwrap_or("all"));

        CachedAnalysis {
            filter_key,
            action_dist,
            pick_positions,
            cards_added,
            cards_added_normalized,
            trashed_draft,
            draft_pool_moves,
            buyer_acq,
            deck_stats,
            score_dist,
            win_rate_position,
            avg_plies,
            unseen_buyer_share,
            color_stats,
            duration_stats,
            variant_win_rate,
            skill_stats,
            penultimate_deck_sizes,
            card_win_rate,
            buyer_type_win_rates,
            draft_win_rate,
            cards_added_categories,
            card_win_rate_categories,
            draft_win_rate_categories,
            trash_rate,
            trash_rate_categories,
            trash_rate_cat_normalized,
            winner_buyer_breakdown,
            num_games,
            num_players,
        }
    }
}

/// Render the Analysis tab content.
pub fn render_analysis_tab(ui: &mut egui::Ui, analysis: &CachedAnalysis) {
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
                        value: format!("{}", analysis.num_games),
                        label: "Games".into(),
                    },
                    StatCard {
                        value: format!("{:.1}", average_score(&analysis.score_dist)),
                        label: "Avg Score".into(),
                    },
                    StatCard {
                        value: format!("{:.0}", analysis.avg_plies),
                        label: "Avg Plies".into(),
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
                if let Some(share) = analysis.unseen_buyer_share {
                    cards.push(StatCard {
                        value: format!("{:.1}%", share * 100.0),
                        label: "Buyers Drawn Unseen".into(),
                    });
                }
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

        // 2-3. Score and Deck Size distributions side by side
        ui.columns(2, |columns| {
            render_btree_bar_section(
                &mut columns[0],
                "Score Distribution",
                "Score",
                "Count",
                &analysis.score_dist,
                format_half_points,
                true,
            );
            render_btree_bar_section(
                &mut columns[1],
                "Deck Size Distribution (Penultimate Round)",
                "Deck Size",
                "Count",
                &analysis.penultimate_deck_sizes,
                |size| size.to_string(),
                true,
            );
        });

        // 5. Win Rate by Position (skip for solo)
        if analysis.num_players == 1 {
            // Solo: show win rate (score >= 16)
            if let Some(entry) = analysis.win_rate_position.get(&0) {
                let id = ui.make_persistent_id("solo_win_rate");
                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                    .show_header(ui, |ui| {
                        ui.strong("Solo Win Rate");
                    })
                    .body(|ui| {
                        let pct = if entry.games > 0.0 { entry.wins / entry.games * 100.0 } else { 0.0 };
                        if let Some((lo, hi)) = wilson_confidence_interval(entry.wins, entry.games) {
                            ui.label(format!(
                                "{:.0} / {:.0} wins ({:.1}%, 95% CI: {:.1}%-{:.1}%)",
                                entry.wins, entry.games, pct, lo, hi
                            ));
                        } else {
                            ui.label(format!("{:.0} / {:.0} wins ({:.1}%)", entry.wins, entry.games, pct));
                        }
                    });
            }
        } else {
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
        }

        // 6. Win Rate by Variant (conditional)
        if let Some(ref variant_wr) = analysis.variant_win_rate {
            let id = ui.make_persistent_id("win_rate_variant");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    ui.strong("Win Rate by Variant");
                })
                .body(|ui| {
                    let elo_map = analysis.skill_stats.as_ref().map(|s| &s.elos);
                    let mut entries: Vec<_> = variant_wr.iter().collect();
                    // Sort by Elo descending
                    entries.sort_by(|a, b| {
                        let elo_a = elo_map.and_then(|m| m.get(a.0.as_str())).copied().unwrap_or(1500.0);
                        let elo_b = elo_map.and_then(|m| m.get(b.0.as_str())).copied().unwrap_or(1500.0);
                        elo_b.partial_cmp(&elo_a).unwrap()
                    });

                    egui::Grid::new(ui.next_auto_id())
                        .striped(true)
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.strong("Variant");
                            ui.strong("Wins");
                            ui.strong("Games");
                            ui.strong("Win %");
                            ui.strong("95% CI");
                            ui.strong("Elo");
                            ui.end_row();

                            for (label, entry) in &entries {
                                ui.label(label.as_str());
                                if (entry.wins - entry.wins.round()).abs() < 0.01 {
                                    ui.label(format!("{}", entry.wins as i64));
                                } else {
                                    ui.label(format!("{:.1}", entry.wins));
                                }
                                ui.label(format!("{}", entry.games as i64));
                                let pct = if entry.games > 0.0 {
                                    entry.wins / entry.games * 100.0
                                } else {
                                    0.0
                                };
                                let bar_width = 60.0;
                                let bar_height = 16.0;
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(bar_width, bar_height),
                                        egui::Sense::hover(),
                                    );
                                    if ui.is_rect_visible(rect) {
                                        let fraction = (pct / 100.0).clamp(0.0, 1.0) as f32;
                                        let bar_rect = egui::Rect::from_min_size(
                                            rect.min,
                                            egui::vec2(rect.width() * fraction, rect.height()),
                                        );
                                        let color = win_rate_color(fraction);
                                        ui.painter().rect_filled(bar_rect, 2.0, color);
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            format!("{:.1}%", pct),
                                            egui::FontId::default(),
                                            ui.visuals().text_color(),
                                        );
                                    }
                                });
                                let ci = wilson_confidence_interval(entry.wins, entry.games);
                                match ci {
                                    Some((lower, upper)) => {
                                        ui.label(format!("[{:.1}%, {:.1}%]", lower, upper))
                                    }
                                    None => ui.label("\u{2013}"),
                                };
                                let elo = elo_map.and_then(|m| m.get(label.as_str())).copied().unwrap_or(1500.0);
                                ui.label(format!("{:.0}", elo));
                                ui.end_row();
                            }
                        });

                    // Skill vs Chance metrics
                    if let Some(ref stats) = analysis.skill_stats {
                        ui.add_space(8.0);
                        let rep_text = if stats.repetitions == u64::MAX {
                            "\u{221e}".to_string()
                        } else {
                            format!("{}", stats.repetitions)
                        };
                        let cards = vec![
                            StatCard {
                                value: format!("{:.1}", stats.optimal_k),
                                label: "K-factor".into(),
                            },
                            StatCard {
                                value: format!("{:.1}", stats.elo_std_dev),
                                label: "Elo Std Dev".into(),
                            },
                            StatCard {
                                value: format!("{:.1}%", stats.p_sd * 100.0),
                                label: "Win Prob at +1 SD".into(),
                            },
                            StatCard {
                                value: rep_text,
                                label: "Games for Skill to Show".into(),
                            },
                        ];
                        stat_grid(ui, &cards);
                    }
                });
        }

        // --- Draft Analysis ---
        section_group_heading(ui, "Draft Analysis");

        let id = ui.make_persistent_id("pick_position");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Pick Position");
            })
            .body(|ui| render_pick_positions(ui, &analysis.pick_positions));

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

        render_buyer_type_win_rates(ui, &analysis.buyer_type_win_rates);

        // --- Actions & Sell Cards ---
        section_group_heading(ui, "Actions & Sell Cards");

        render_hashmap_bar_section(
            ui,
            "Action Distribution",
            "Action",
            "Count",
            &analysis.action_dist,
            false,
        );

        // Trashed from Draft
        let id = ui.make_persistent_id("trashed_draft");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Trashed from Draft");
            })
            .body(|ui| {
                ui.strong("By Category");
                render_category_bar_rows_pct(
                    ui,
                    &analysis.trash_rate_cat_normalized,
                    "Trash Rate",
                );
                ui.add_space(8.0);
                ui.strong("By Card");
                render_f64_hashmap_bar_rows_pct(ui, &analysis.trash_rate, "Trash Rate");
            });

        // 13. Cards raised to the top cards
        render_draft_pool_moves(ui, &analysis.draft_pool_moves);

        // 14. Sell Card Acquisitions
        let id = ui.make_persistent_id("buyer_acquisitions");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Sell Card Acquisitions");
            })
            .body(|ui| {
                ui.strong("By Sell Card");
                render_usize_hashmap_bar_rows(ui, &analysis.buyer_acq.by_buyer, "Count");
                ui.add_space(8.0);
                ui.strong("By Points");
                render_u32_hashmap_bar_rows(
                    ui,
                    &analysis.buyer_acq.by_points,
                    "Points",
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

        // Winner Sell Card Breakdown
        let id = ui.make_persistent_id("winner_buyer_breakdown");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.strong("Winner Sell Card Breakdown");
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
                    StatCard {
                        value: format!("{:.1}", wb.avg_gold_coins),
                        label: "Avg GoldCoins".into(),
                    },
                ];
                stat_grid(ui, &cards);
            });
    });
}

// -- Helper rendering functions --

/// Render a section group heading with a separator line above it.
/// The mean score, in points, of a distribution keyed by half points.
fn average_score(score_dist: &std::collections::BTreeMap<u32, usize>) -> f64 {
    let games: usize = score_dist.values().sum();
    if games == 0 {
        return 0.0;
    }
    let half_points: usize = score_dist.iter().map(|(&score, &count)| score as usize * count).sum();
    half_points as f64 / games as f64 / 2.0
}

/// How much claiming a buyer type moves a player's win rate: the rate of
/// players who claimed at least one against those who claimed none, then the
/// rate by how many they claimed.
fn render_buyer_type_win_rates(ui: &mut egui::Ui, rates: &[BuyerTypeWinRates]) {
    let id = ui.make_persistent_id("buyer_type_win_rates");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.strong("Win Rate by Trade Type Claimed");
        })
        .body(|ui| {
            ui.label("Claims are the cards a player took in the buyers phases, off the display or unseen off the deck. The difference is claimed minus never claimed, \u{b1} its 95% margin.");
            let pct = |entry: &WinRateEntry| entry.wins / entry.games * 100.0;
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    for header in ["Trade", "Claimed: Games", "Win %", "Never: Games", "Win %", "Difference"] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for rate in rates {
                        let never = &rate.by_claims[0];
                        let claimed = WinRateEntry {
                            wins: rate.by_claims[1..].iter().map(|e| e.wins).sum(),
                            games: rate.by_claims[1..].iter().map(|e| e.games).sum(),
                        };
                        ui.label(rate.trade.name());
                        ui.label(format!("{}", claimed.games as i64));
                        ui.label(if claimed.games > 0.0 { format!("{:.1}%", pct(&claimed)) } else { "\u{2013}".into() });
                        ui.label(format!("{}", never.games as i64));
                        ui.label(if never.games > 0.0 { format!("{:.1}%", pct(never)) } else { "\u{2013}".into() });
                        if claimed.games > 0.0 && never.games > 0.0 {
                            let (p1, p2) = (claimed.wins / claimed.games, never.wins / never.games);
                            let margin = 1.96
                                * (p1 * (1.0 - p1) / claimed.games + p2 * (1.0 - p2) / never.games).sqrt();
                            ui.label(format!("{:+.1} \u{b1} {:.1} pts", (p1 - p2) * 100.0, margin * 100.0));
                        } else {
                            ui.label("\u{2013}");
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
            ui.strong("Win % by number claimed");
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Trade");
                    for claims in 0..CLAIM_COUNT_BUCKETS {
                        let plus = if claims == CLAIM_COUNT_BUCKETS - 1 { "+" } else { "" };
                        ui.strong(format!("{}{}", claims, plus));
                    }
                    ui.end_row();
                    for rate in rates {
                        ui.label(rate.trade.name());
                        for entry in &rate.by_claims {
                            ui.label(if entry.games > 0.0 {
                                format!("{:.1}% ({})", pct(entry), entry.games as i64)
                            } else {
                                "\u{2013}".into()
                            });
                        }
                        ui.end_row();
                    }
                });
        });
}

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
    key_label: fn(u32) -> String,
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
                .map(|(&k, &v)| (key_label(k), format!("{}", v), v as f32 / max_val))
                .collect();
            bar_table(ui, &[key_header, val_header], &rows);
        });
}

/// Which cards the Move to Draft Pool abilities raise, and which trashed card
/// raised them.
fn render_draft_pool_moves(ui: &mut egui::Ui, moves: &DraftPoolMoves) {
    let id = ui.make_persistent_id("draft_pool_moves");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.strong("Cards Raised to the Draft Pool");
        })
        .body(|ui| {
            if moves.raised_by.is_empty() && moves.fizzled.is_empty() {
                ui.label("No data available.");
                return;
            }
            ui.label("A card in the workshop moved to the top cards by the ability of a card that was trashed.");

            let source_total = |source: &String| -> usize {
                moves.raised_by.get(source).map_or(0, |t| t.values().sum())
            };
            let mut sources: Vec<&String> = moves.raised_by.keys().chain(moves.fizzled.keys()).collect();
            sources.sort();
            sources.dedup();
            sources.sort_by_key(|s| std::cmp::Reverse(source_total(s)));

            ui.strong("By trashed card");
            egui::Grid::new(ui.next_auto_id()).striped(true).min_col_width(70.0).show(ui, |ui| {
                for header in ["Trashed card", "Cards raised", "Nothing to raise"] {
                    ui.strong(header);
                }
                ui.end_row();
                for source in &sources {
                    ui.label(source.as_str());
                    ui.label(format!("{}", source_total(source)));
                    ui.label(format!("{}", moves.fizzled.get(*source).copied().unwrap_or(0)));
                    ui.end_row();
                }
            });

            let mut raised: HashMap<&String, usize> = HashMap::new();
            for targets in moves.raised_by.values() {
                for (card, count) in targets {
                    *raised.entry(card).or_insert(0) += count;
                }
            }
            let mut cards: Vec<_> = raised.into_iter().collect();
            cards.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

            ui.add_space(8.0);
            ui.strong("By raised card");
            egui::Grid::new(ui.next_auto_id()).striped(true).min_col_width(70.0).show(ui, |ui| {
                ui.strong("Raised card");
                ui.strong("Total");
                for source in &sources {
                    ui.strong(source.as_str());
                }
                ui.end_row();
                for (card, total) in &cards {
                    ui.label(card.as_str());
                    ui.label(format!("{}", total));
                    for source in &sources {
                        let count = moves.raised_by.get(*source).and_then(|t| t.get(*card)).copied().unwrap_or(0);
                        ui.label(if count == 0 { "\u{2013}".to_string() } else { format!("{}", count) });
                    }
                    ui.end_row();
                }
            });
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

/// Cards ordered by how early they are picked; a longer bar is an earlier pick.
fn render_pick_positions(ui: &mut egui::Ui, positions: &HashMap<String, PickPosition>) {
    if positions.is_empty() {
        ui.label("No data available.");
        return;
    }
    ui.label("Average pick number within a round's draft (1 = first pick). A hand's last card is not picked, so it is not counted.");
    let mut sorted: Vec<_> = positions.iter().collect();
    sorted.sort_by(|a, b| a.1.average().total_cmp(&b.1.average()));
    let last_pick = (DRAFT_HAND_SIZE - 1) as f32;
    let rows: Vec<_> = sorted
        .iter()
        .map(|(name, position)| {
            let average = position.average();
            let earliness = (last_pick - average as f32) / (last_pick - 1.0);
            (name.to_string(), format!("{:.2} ({} picks)", average, position.picks), earliness)
        })
        .collect();
    bar_table(ui, &["Card", "Avg Pick"], &rows);
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
