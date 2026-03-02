pub struct CardCategory {
    pub label: &'static str,
    pub card_names: Vec<&'static str>,
    pub total_copies: u32,
}

pub fn draft_card_categories() -> Vec<CardCategory> {
    vec![
        CardCategory {
            label: "Pure Primary Dyes",
            card_names: vec!["Kermes", "Weld", "Woad"],
            total_copies: 9,
        },
        CardCategory {
            label: "Primary Dyes",
            card_names: vec![
                "Lac",
                "Brazilwood",
                "Pomegranate",
                "Sumac",
                "Elderberry",
                "Turnsole",
            ],
            total_copies: 18,
        },
        CardCategory {
            label: "Secondary Dyes",
            card_names: vec![
                "Madder",
                "Turmeric",
                "Dyer's Greenweed",
                "Verdigris",
                "Orchil",
                "Logwood",
            ],
            total_copies: 18,
        },
        CardCategory {
            label: "Tertiary Dyes",
            card_names: vec![
                "Vermilion",
                "Saffron",
                "Persian Berries",
                "Azurite",
                "Indigo",
                "Cochineal",
            ],
            total_copies: 18,
        },
        CardCategory {
            label: "Action Cards",
            card_names: vec!["Alum", "Cream of Tartar", "Gum Arabic", "Potash"],
            total_copies: 16,
        },
        CardCategory {
            label: "Double Materials",
            card_names: vec!["Fine Ceramics", "Fine Paintings", "Fine Textiles"],
            total_copies: 3,
        },
        CardCategory {
            label: "Material + Color",
            card_names: vec![
                "Terra Cotta",
                "Ochre Ware",
                "Cobalt Ware",
                "Cinnabar & Canvas",
                "Orpiment & Canvas",
                "Ultramarine & Canvas",
                "Alizarin & Fabric",
                "Fustic & Fabric",
                "Pastel & Fabric",
            ],
            total_copies: 9,
        },
        CardCategory {
            label: "Dual Materials",
            card_names: vec!["Clay & Canvas", "Clay & Fabric", "Canvas & Fabric"],
            total_copies: 3,
        },
    ]
}

pub fn get_starter_card_categories(num_players: usize) -> Vec<CardCategory> {
    vec![
        CardCategory {
            label: "Starter Dyes",
            card_names: vec!["Basic Red", "Basic Yellow", "Basic Blue"],
            total_copies: 3 * num_players as u32,
        },
        CardCategory {
            label: "Starter Materials",
            card_names: vec!["Ceramics", "Paintings", "Textiles"],
            total_copies: 3 * num_players as u32,
        },
        CardCategory {
            label: "Argol",
            card_names: vec!["Argol"],
            total_copies: 1 * num_players as u32,
        },
    ]
}
