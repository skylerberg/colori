use eframe::egui;

pub struct StatCard {
    pub value: String,
    pub label: String,
}

/// Render a horizontal grid of stat cards with accent styling.
pub fn stat_grid(ui: &mut egui::Ui, cards: &[StatCard]) {
    let accent_color = egui::Color32::from_rgb(74, 158, 255);
    let bg_color = egui::Color32::from_rgba_unmultiplied(74, 158, 255, 20);

    ui.horizontal_wrapped(|ui| {
        for card in cards {
            let group_response = ui.group(|ui| {
                ui.set_min_width(120.0);
                ui.set_max_width(150.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(&card.value)
                            .size(22.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(&card.label)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(160, 160, 180)),
                    );
                });
            });
            let rect = group_response.response.rect;
            // Subtle background fill
            ui.painter().rect_filled(rect, 4.0, bg_color);
            // Top accent border
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(rect.width(), 2.0),
                ),
                egui::CornerRadius {
                    nw: 4,
                    ne: 4,
                    sw: 0,
                    se: 0,
                },
                accent_color,
            );
        }
    });
}
