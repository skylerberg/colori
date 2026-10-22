use std::collections::BTreeMap;

use eframe::egui;
use colori_core::cards::{
    action_cards, basic_dye_cards, chalk_card, draft_copies, draft_deck_cards, draft_dye_cards,
    draft_material_cards, primaries_card, sell_deck_cards, starter_material_cards,
};
use colori_core::types::{
    Buyer, Card, CardKind, Color, MaterialType, ALL_BUYERS, ALL_MATERIAL_TYPES, ALL_SELL_CARDS,
};

use crate::analysis::card_names::{format_abilities, format_ability};
use crate::analysis::categories::draft_card_categories;

pub fn render_card_reference_tab(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        render_draft_deck_summary(ui);
        ui.add_space(8.0);
        render_ability_distribution(ui);
        ui.add_space(8.0);
        render_sell_card_summary(ui);
        ui.add_space(8.0);
        render_category_details(ui);
        ui.add_space(8.0);
        render_starter_cards(ui);
        ui.add_space(8.0);
        render_sell_cards_by_points(ui);
    });
}

fn render_draft_deck_summary(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("draft_deck_summary");
    egui::CollapsingHeader::new("Draft Deck Summary")
        .id_salt(id)
        .default_open(true)
        .show(ui, |ui| {
            let categories = draft_card_categories();
            let mut total_unique: usize = 0;
            let mut total_copies: u32 = 0;

            egui::Grid::new("draft_deck_summary_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Category");
                    ui.strong("Unique Cards");
                    ui.strong("Copies per Card");
                    ui.strong("Total Copies");
                    ui.end_row();

                    for cat in &categories {
                        let unique = cat.card_names.len();
                        let copies_per = if unique > 0 {
                            cat.total_copies / unique as u32
                        } else {
                            0
                        };
                        total_unique += unique;
                        total_copies += cat.total_copies;

                        ui.label(cat.label);
                        ui.label(unique.to_string());
                        ui.label(copies_per.to_string());
                        ui.label(cat.total_copies.to_string());
                        ui.end_row();
                    }

                    ui.strong("Total");
                    ui.strong(total_unique.to_string());
                    ui.strong("--");
                    ui.strong(total_copies.to_string());
                    ui.end_row();
                });
        });
}

fn render_ability_distribution(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("ability_distribution");
    egui::CollapsingHeader::new("Ability Distribution in Draft Deck")
        .id_salt(id)
        .default_open(true)
        .show(ui, |ui| {
            let mut ability_counts: BTreeMap<String, u32> = BTreeMap::new();

            for (card, copies) in draft_deck_cards() {
                let ability_str = format_abilities(card.trash_abilities());
                *ability_counts.entry(ability_str).or_insert(0) += copies as u32;
            }

            egui::Grid::new("ability_distribution_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Ability");
                    ui.strong("Total Copies");
                    ui.end_row();

                    for (ability, count) in &ability_counts {
                        ui.label(ability);
                        ui.label(count.to_string());
                        ui.end_row();
                    }
                });
        });
}

fn format_sale_materials(buyer: Buyer) -> String {
    let mut parts: Vec<String> = Vec::new();
    if buyer.boats() > 0 {
        parts.push(format!("{} Boat", buyer.boats()));
    }
    for &material in &ALL_MATERIAL_TYPES {
        let count = buyer.materials().iter().filter(|&&m| m == material).count();
        if count > 0 {
            parts.push(format!("{} {:?}", count, material));
        }
    }
    if parts.is_empty() {
        "--".to_string()
    } else {
        parts.join(" + ")
    }
}

fn render_sell_card_summary(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("sell_card_summary");
    egui::CollapsingHeader::new("Sell Card Summary")
        .id_salt(id)
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("sell_card_summary_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Buyer");
                    ui.strong("Points");
                    ui.strong("Materials");
                    ui.strong("Cards");
                    ui.strong("Copies");
                    ui.end_row();

                    let mut total = 0usize;
                    for buyer in ALL_BUYERS {
                        let cards = ALL_SELL_CARDS.iter().filter(|c| c.buyer() == buyer).count();
                        let copies: usize = sell_deck_cards()
                            .filter(|(c, _)| c.buyer() == buyer)
                            .map(|(_, copies)| copies)
                            .sum();
                        total += copies;
                        ui.label(buyer.name());
                        ui.label(buyer.points().to_string());
                        ui.label(format_sale_materials(buyer));
                        ui.label(cards.to_string());
                        ui.label(copies.to_string());
                        ui.end_row();
                    }

                    ui.strong("Total");
                    ui.strong("");
                    ui.strong("");
                    ui.strong(ALL_SELL_CARDS.len().to_string());
                    ui.strong(total.to_string());
                    ui.end_row();
                });
        });
}

fn find_card_by_name(name: &str) -> Option<Card> {
    for card in draft_dye_cards() {
        if card.name() == name {
            return Some(card);
        }
    }
    for card in draft_material_cards() {
        if card.name() == name {
            return Some(card);
        }
    }
    for card in action_cards() {
        if card.name() == name {
            return Some(card);
        }
    }
    for card in [primaries_card()] {
        if card.name() == name {
            return Some(card);
        }
    }
    None
}

fn format_color(color: &Color) -> &'static str {
    match color {
        Color::Red => "Red",
        Color::Vermilion => "Vermilion",
        Color::Orange => "Orange",
        Color::Amber => "Amber",
        Color::Yellow => "Yellow",
        Color::Chartreuse => "Chartreuse",
        Color::Green => "Green",
        Color::Teal => "Teal",
        Color::Blue => "Blue",
        Color::Ultramarine => "Ultramarine",
        Color::Purple => "Purple",
        Color::Fuchsia => "Fuchsia",
    }
}

fn format_colors(colors: &[Color]) -> String {
    colors.iter()
        .map(|c| format_color(c))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_material_types(types: &[MaterialType]) -> String {
    if types.len() == 2 && types[0] == types[1] {
        return format!("2x {:?}", types[0]);
    }
    types
        .iter()
        .map(|t| format!("{:?}", t))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn render_category_details(ui: &mut egui::Ui) {
    let categories = draft_card_categories();

    for cat in &categories {
        let id = ui.make_persistent_id(format!("category_detail_{}", cat.label));
        egui::CollapsingHeader::new(cat.label)
            .id_salt(id)
            .default_open(false)
            .show(ui, |ui| {
                // Determine the kind of cards in this category by looking at the first card
                let first_card = cat
                    .card_names
                    .first()
                    .and_then(|name| find_card_by_name(name));

                let kind = first_card.map(|c| c.kind());

                let grid_id = format!("category_grid_{}", cat.label);
                egui::Grid::new(grid_id)
                    .striped(true)
                    .show(ui, |ui| {
                        match kind {
                            Some(CardKind::Dye) => {
                                ui.strong("Name");
                                ui.strong("Colors");
                                ui.strong("Ability");
                                ui.strong("Copies");
                                ui.end_row();

                                for name in &cat.card_names {
                                    if let Some(card) = find_card_by_name(name) {
                                        ui.label(card.name());
                                        ui.label(format_colors(card.colors()));
                                        ui.label(format_abilities(card.trash_abilities()));
                                        ui.label(draft_copies(card).to_string());
                                        ui.end_row();
                                    }
                                }
                            }
                            Some(CardKind::Material) => {
                                ui.strong("Name");
                                ui.strong("Material Types");
                                ui.strong("Color");
                                ui.strong("Ability");
                                ui.strong("Copies");
                                ui.end_row();

                                for name in &cat.card_names {
                                    if let Some(card) = find_card_by_name(name) {
                                        ui.label(card.name());
                                        ui.label(format_material_types(card.material_types()));
                                        let pip_str = if card.colors().is_empty() {
                                            "--".to_string()
                                        } else {
                                            format_colors(card.colors())
                                        };
                                        ui.label(pip_str);
                                        ui.label(format_abilities(card.trash_abilities()));
                                        ui.label(draft_copies(card).to_string());
                                        ui.end_row();
                                    }
                                }
                            }
                            Some(CardKind::Action) => {
                                ui.strong("Name");
                                ui.strong("Main Ability");
                                ui.strong("Workshop Ability");
                                ui.strong("Copies");
                                ui.end_row();

                                for name in &cat.card_names {
                                    if let Some(card) = find_card_by_name(name) {
                                        ui.label(card.name());
                                        ui.label(format_abilities(card.trash_abilities()));
                                        let workshop_str = card
                                            .workshop_abilities()
                                            .iter()
                                            .map(|a| format_ability(&a))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        let workshop_display = if workshop_str.is_empty() {
                                            "--".to_string()
                                        } else {
                                            workshop_str
                                        };
                                        ui.label(workshop_display);
                                        ui.label(draft_copies(card).to_string());
                                        ui.end_row();
                                    }
                                }
                            }
                            _ => {
                                ui.label("No cards found in this category.");
                                ui.end_row();
                            }
                        }
                    });
            });
    }
}

fn render_starter_cards(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("starter_cards");
    egui::CollapsingHeader::new("Starter Cards")
        .id_salt(id)
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("starter_cards_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Type");
                    ui.strong("Details");
                    ui.strong("Ability");
                    ui.end_row();

                    // Basic dye cards
                    for card in basic_dye_cards() {
                        ui.label(card.name());
                        ui.label("Basic Dye");
                        ui.label(format_colors(card.colors()));
                        ui.label(format_abilities(card.trash_abilities()));
                        ui.end_row();
                    }

                    // Starter material cards
                    for card in starter_material_cards() {
                        ui.label(card.name());
                        ui.label("Material");
                        ui.label(format_material_types(card.material_types()));
                        ui.label(format_abilities(card.trash_abilities()));
                        ui.end_row();
                    }

                    let chalk = chalk_card();
                    ui.label(chalk.name());
                    ui.label("Action");
                    ui.label(
                        chalk
                            .workshop_abilities()
                            .iter()
                            .map(format_ability)
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ui.label(format_abilities(chalk.trash_abilities()));
                    ui.end_row();

                });
        });
}

fn render_sell_cards_by_points(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("sell_cards_by_points");
    egui::CollapsingHeader::new("Sell Cards by Buyer")
        .id_salt(id)
        .default_open(false)
        .show(ui, |ui| {
            for buyer in ALL_BUYERS {
                ui.heading(format!("{} ({} points)", buyer.name(), buyer.points()));
                ui.add_space(4.0);

                egui::Grid::new(format!("sell_cards_{:?}_grid", buyer))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Materials");
                        ui.strong("Color Cost");
                        ui.strong("Copies");
                        ui.end_row();

                        for (sell_card, copies) in sell_deck_cards() {
                            if sell_card.buyer() == buyer {
                                ui.label(format_sale_materials(buyer));
                                ui.label(format_colors(sell_card.color_cost()));
                                ui.label(copies.to_string());
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(8.0);
            }
        });
}
