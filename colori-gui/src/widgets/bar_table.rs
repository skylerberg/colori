use eframe::egui;

/// Render a table with string keys and numeric values, with horizontal bar visualization.
/// `data` is sorted by the caller. Each entry is (label, value_text, bar_fraction).
pub fn bar_table(ui: &mut egui::Ui, headers: &[&str], rows: &[(String, String, f32)]) {
    // Reserve space for label column (~100px) and value text (~50px + gap)
    let bar_width = (ui.available_width() - 160.0).clamp(80.0, 600.0);

    egui::Grid::new(ui.next_auto_id())
        .striped(true)
        .min_col_width(80.0)
        .show(ui, |ui| {
            // Header row
            for h in headers {
                ui.strong(*h);
            }
            ui.end_row();

            // Data rows
            for (label, value_text, bar_fraction) in rows {
                ui.label(label.as_str());
                // Bar + value laid out horizontally
                ui.horizontal(|ui| {
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::vec2(bar_width, 18.0),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(rect) {
                        let bar_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(
                                rect.width() * bar_fraction.clamp(0.0, 1.0),
                                rect.height(),
                            ),
                        );
                        ui.painter().rect_filled(
                            bar_rect,
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(74, 158, 255, 160),
                        );
                    }
                    ui.add_space(4.0);
                    ui.label(value_text.as_str());
                });
                ui.end_row();
            }
        });
}

/// Render a win rate table with columns: Label, Wins, Games, Win %, 95% CI
pub fn win_rate_table(
    ui: &mut egui::Ui,
    label_header: &str,
    games_header: &str,
    rows: &[(String, f64, f64, Option<(f64, f64)>)], // (label, wins, games, ci)
) {
    egui::Grid::new(ui.next_auto_id())
        .striped(true)
        .min_col_width(60.0)
        .show(ui, |ui| {
            ui.strong(label_header);
            ui.strong("Wins");
            ui.strong(games_header);
            ui.strong("Win %");
            ui.strong("95% CI");
            ui.end_row();

            for (label, wins, games, ci) in rows {
                ui.label(label.as_str());
                // Format wins
                if (*wins - wins.round()).abs() < 0.01 {
                    ui.label(format!("{}", *wins as i64));
                } else {
                    ui.label(format!("{:.1}", wins));
                }
                ui.label(format!("{}", *games as i64));
                let pct = if *games > 0.0 {
                    wins / games * 100.0
                } else {
                    0.0
                };
                // Win % with inline bar indicator
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
                        // Color: lerp from red (low) through yellow to green (high)
                        let color = win_rate_color(fraction);
                        ui.painter().rect_filled(bar_rect, 2.0, color);
                        // Text centered on the full cell
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.1}%", pct),
                            egui::FontId::default(),
                            ui.visuals().text_color(),
                        );
                    }
                });
                match ci {
                    Some((lower, upper)) => {
                        ui.label(format!("[{:.1}%, {:.1}%]", lower, upper))
                    }
                    None => ui.label("\u{2013}"),
                };
                ui.end_row();
            }
        });
}

/// Returns a color for win rate: red (0%) → yellow (50%) → green (100%).
fn win_rate_color(fraction: f32) -> egui::Color32 {
    let f = fraction.clamp(0.0, 1.0);
    let (r, g) = if f < 0.5 {
        // Red to yellow
        let t = f * 2.0;
        (200, (120.0 * t) as u8)
    } else {
        // Yellow to green
        let t = (f - 0.5) * 2.0;
        ((200.0 * (1.0 - t)) as u8, 140)
    };
    egui::Color32::from_rgba_unmultiplied(r, g, 40, 140)
}
