use eframe::egui;

pub struct StatCard {
    pub value: String,
    pub label: String,
}

/// Render a horizontal grid of stat cards.
pub fn stat_grid(ui: &mut egui::Ui, cards: &[StatCard]) {
    ui.horizontal_wrapped(|ui| {
        for card in cards {
            ui.group(|ui| {
                ui.set_min_width(120.0);
                ui.set_max_width(150.0);
                ui.vertical_centered(|ui| {
                    ui.heading(&card.value);
                    ui.label(&card.label);
                });
            });
        }
    });
}
