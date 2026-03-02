use std::collections::BTreeMap;

use eframe::egui;
use colori_core::cards::{
    action_cards, argol_card, basic_dye_cards, draft_material_cards, dye_cards,
    generate_all_buyers, starter_material_cards, ACTION_COPIES, DYE_COPIES, MATERIAL_COPIES,
};
use colori_core::types::{Card, CardKind, Color, MaterialType};

use crate::analysis::card_names::format_ability;
use crate::analysis::categories::draft_card_categories;

pub fn render_card_reference_tab(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        render_draft_deck_summary(ui);
        ui.add_space(8.0);
        render_ability_distribution(ui);
        ui.add_space(8.0);
        render_buyer_summary(ui);
        ui.add_space(8.0);
        render_category_details(ui);
        ui.add_space(8.0);
        render_starter_cards(ui);
        ui.add_space(8.0);
        render_buyers_by_stars(ui);
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

            for card in dye_cards() {
                let ability_str = format_ability(&card.ability());
                let copies = DYE_COPIES as u32;
                *ability_counts.entry(ability_str).or_insert(0) += copies;
            }

            for card in draft_material_cards() {
                let ability_str = format_ability(&card.ability());
                let copies = MATERIAL_COPIES as u32;
                *ability_counts.entry(ability_str).or_insert(0) += copies;
            }

            for card in action_cards() {
                let ability_str = format_ability(&card.ability());
                let copies = ACTION_COPIES as u32;
                *ability_counts.entry(ability_str).or_insert(0) += copies;
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

fn render_buyer_summary(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("buyer_summary");
    egui::CollapsingHeader::new("Buyer Summary")
        .id_salt(id)
        .default_open(true)
        .show(ui, |ui| {
            let buyers = generate_all_buyers();

            // Count by groups
            let mut tertiary_1_count = 0u32;
            let mut secondary_primary_count = 0u32;
            let mut triple_primary_count = 0u32;
            let mut ceramics_3_count = 0u32;
            let mut paintings_4_count = 0u32;

            for buyer in &buyers {
                match buyer.stars() {
                    2 => {
                        let num_colors = buyer.color_cost().len();
                        match num_colors {
                            1 => tertiary_1_count += 1,
                            2 => secondary_primary_count += 1,
                            3 => triple_primary_count += 1,
                            _ => {}
                        }
                    }
                    3 => ceramics_3_count += 1,
                    4 => paintings_4_count += 1,
                    _ => {}
                }
            }

            egui::Grid::new("buyer_summary_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Stars");
                    ui.strong("Material");
                    ui.strong("Color Pattern");
                    ui.strong("Count");
                    ui.end_row();

                    ui.label("2");
                    ui.label("Textiles");
                    ui.label("1 tertiary");
                    ui.label(tertiary_1_count.to_string());
                    ui.end_row();

                    ui.label("2");
                    ui.label("Textiles");
                    ui.label("1 secondary + 1 primary");
                    ui.label(secondary_primary_count.to_string());
                    ui.end_row();

                    ui.label("2");
                    ui.label("Textiles");
                    ui.label("3x same primary");
                    ui.label(triple_primary_count.to_string());
                    ui.end_row();

                    ui.label("3");
                    ui.label("Ceramics");
                    ui.label("1 tertiary + 1 primary");
                    ui.label(ceramics_3_count.to_string());
                    ui.end_row();

                    ui.label("4");
                    ui.label("Paintings");
                    ui.label("1 tertiary + 1 secondary");
                    ui.label(paintings_4_count.to_string());
                    ui.end_row();

                    ui.strong("Total");
                    ui.strong("");
                    ui.strong("");
                    let total = tertiary_1_count
                        + secondary_primary_count
                        + triple_primary_count
                        + ceramics_3_count
                        + paintings_4_count;
                    ui.strong(total.to_string());
                    ui.end_row();
                });
        });
}

fn find_card_by_name(name: &str) -> Option<Card> {
    for card in dye_cards() {
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
        Color::Indigo => "Indigo",
        Color::Purple => "Purple",
        Color::Magenta => "Magenta",
    }
}

fn format_pips(pips: &[Color]) -> String {
    pips.iter()
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

fn copies_for_kind(kind: CardKind) -> u32 {
    match kind {
        CardKind::Dye => DYE_COPIES as u32,
        CardKind::Material => MATERIAL_COPIES as u32,
        CardKind::Action => ACTION_COPIES as u32,
        CardKind::BasicDye => 0,
    }
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
                                ui.strong("Color Pips");
                                ui.strong("Ability");
                                ui.strong("Copies");
                                ui.end_row();

                                for name in &cat.card_names {
                                    if let Some(card) = find_card_by_name(name) {
                                        ui.label(card.name());
                                        ui.label(format_pips(card.pips()));
                                        ui.label(format_ability(&card.ability()));
                                        ui.label(copies_for_kind(card.kind()).to_string());
                                        ui.end_row();
                                    }
                                }
                            }
                            Some(CardKind::Material) => {
                                ui.strong("Name");
                                ui.strong("Material Types");
                                ui.strong("Color Pip");
                                ui.strong("Ability");
                                ui.strong("Copies");
                                ui.end_row();

                                for name in &cat.card_names {
                                    if let Some(card) = find_card_by_name(name) {
                                        ui.label(card.name());
                                        ui.label(format_material_types(card.material_types()));
                                        let pip_str = if card.pips().is_empty() {
                                            "--".to_string()
                                        } else {
                                            format_pips(card.pips())
                                        };
                                        ui.label(pip_str);
                                        ui.label(format_ability(&card.ability()));
                                        ui.label(copies_for_kind(card.kind()).to_string());
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
                                        ui.label(format_ability(&card.ability()));
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
                                        ui.label(copies_for_kind(card.kind()).to_string());
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
                        ui.label(format_pips(card.pips()));
                        ui.label(format_ability(&card.ability()));
                        ui.end_row();
                    }

                    // Starter material cards
                    for card in starter_material_cards() {
                        ui.label(card.name());
                        ui.label("Material");
                        ui.label(format_material_types(card.material_types()));
                        ui.label(format_ability(&card.ability()));
                        ui.end_row();
                    }

                    // Argol
                    let argol = argol_card();
                    let workshop_str = argol
                        .workshop_abilities()
                        .iter()
                        .map(|a| format_ability(&a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let argol_details = if workshop_str.is_empty() {
                        "--".to_string()
                    } else {
                        format!("Workshop: {}", workshop_str)
                    };
                    ui.label(argol.name());
                    ui.label("Action");
                    ui.label(argol_details);
                    ui.label(format_ability(&argol.ability()));
                    ui.end_row();
                });
        });
}

fn render_buyers_by_stars(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("buyers_by_stars");
    egui::CollapsingHeader::new("Buyers by Star Rating")
        .id_salt(id)
        .default_open(false)
        .show(ui, |ui| {
            let buyers = generate_all_buyers();

            for star_level in [2u32, 3, 4] {
                ui.heading(format!("{}-Star Buyers", star_level));
                ui.add_space(4.0);

                let grid_id = format!("buyers_{}star_grid", star_level);
                egui::Grid::new(grid_id)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Required Material");
                        ui.strong("Color Cost");
                        ui.end_row();

                        for buyer in &buyers {
                            if buyer.stars() == star_level {
                                ui.label(format!("{:?}", buyer.required_material()));
                                ui.label(format_pips(buyer.color_cost()));
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(8.0);
            }
        });
}
