use crate::types::{Ability, AnyCard, Color, MaterialType};

const PRIMARIES_LIST: [Color; 3] = [Color::Red, Color::Yellow, Color::Blue];
const SECONDARIES_LIST: [Color; 3] = [Color::Orange, Color::Green, Color::Purple];
const TERTIARIES_LIST: [Color; 6] = [
    Color::Vermilion,
    Color::Amber,
    Color::Chartreuse,
    Color::Teal,
    Color::Indigo,
    Color::Magenta,
];

pub fn basic_dye_cards() -> Vec<AnyCard> {
    vec![
        AnyCard::BasicDye {
            name: "Basic Red".to_string(),
            color: Color::Red,
            ability: Ability::Sell,
        },
        AnyCard::BasicDye {
            name: "Basic Yellow".to_string(),
            color: Color::Yellow,
            ability: Ability::Sell,
        },
        AnyCard::BasicDye {
            name: "Basic Blue".to_string(),
            color: Color::Blue,
            ability: Ability::Sell,
        },
    ]
}

pub fn dye_cards() -> Vec<AnyCard> {
    vec![
        // Primary (3) — sell
        AnyCard::Dye {
            name: "Kermes".to_string(),
            colors: vec![Color::Red, Color::Red, Color::Red],
            ability: Ability::Sell,
        },
        AnyCard::Dye {
            name: "Weld".to_string(),
            colors: vec![Color::Yellow, Color::Yellow, Color::Yellow],
            ability: Ability::Sell,
        },
        AnyCard::Dye {
            name: "Woad".to_string(),
            colors: vec![Color::Blue, Color::Blue, Color::Blue],
            ability: Ability::Sell,
        },
        // Secondary (6) — workshop x3
        AnyCard::Dye {
            name: "Madder".to_string(),
            colors: vec![Color::Orange, Color::Red],
            ability: Ability::Workshop { count: 3 },
        },
        AnyCard::Dye {
            name: "Turmeric".to_string(),
            colors: vec![Color::Orange, Color::Yellow],
            ability: Ability::Workshop { count: 3 },
        },
        AnyCard::Dye {
            name: "Dyer's Greenweed".to_string(),
            colors: vec![Color::Green, Color::Yellow],
            ability: Ability::Workshop { count: 3 },
        },
        AnyCard::Dye {
            name: "Verdigris".to_string(),
            colors: vec![Color::Green, Color::Blue],
            ability: Ability::Workshop { count: 3 },
        },
        AnyCard::Dye {
            name: "Orchil".to_string(),
            colors: vec![Color::Purple, Color::Red],
            ability: Ability::Workshop { count: 3 },
        },
        AnyCard::Dye {
            name: "Logwood".to_string(),
            colors: vec![Color::Purple, Color::Blue],
            ability: Ability::Workshop { count: 3 },
        },
        // Tertiary (6) — mixColors x2
        AnyCard::Dye {
            name: "Vermilion".to_string(),
            colors: vec![Color::Vermilion],
            ability: Ability::MixColors { count: 2 },
        },
        AnyCard::Dye {
            name: "Saffron".to_string(),
            colors: vec![Color::Amber],
            ability: Ability::MixColors { count: 2 },
        },
        AnyCard::Dye {
            name: "Persian Berries".to_string(),
            colors: vec![Color::Chartreuse],
            ability: Ability::MixColors { count: 2 },
        },
        AnyCard::Dye {
            name: "Azurite".to_string(),
            colors: vec![Color::Teal],
            ability: Ability::MixColors { count: 2 },
        },
        AnyCard::Dye {
            name: "Indigo".to_string(),
            colors: vec![Color::Indigo],
            ability: Ability::MixColors { count: 2 },
        },
        AnyCard::Dye {
            name: "Cochineal".to_string(),
            colors: vec![Color::Magenta],
            ability: Ability::MixColors { count: 2 },
        },
    ]
}

pub fn starter_material_cards() -> Vec<AnyCard> {
    vec![
        AnyCard::Material {
            name: "Ceramics".to_string(),
            material_types: vec![MaterialType::Ceramics],
            color_pip: None,
            ability: Ability::Workshop { count: 2 },
        },
        AnyCard::Material {
            name: "Paintings".to_string(),
            material_types: vec![MaterialType::Paintings],
            color_pip: None,
            ability: Ability::Sell,
        },
        AnyCard::Material {
            name: "Textiles".to_string(),
            material_types: vec![MaterialType::Textiles],
            color_pip: None,
            ability: Ability::Workshop { count: 2 },
        },
    ]
}

pub fn draft_material_cards() -> Vec<AnyCard> {
    vec![
        // Double material cards
        AnyCard::Material {
            name: "Fine Ceramics".to_string(),
            material_types: vec![MaterialType::Ceramics, MaterialType::Ceramics],
            color_pip: None,
            ability: Ability::Workshop { count: 2 },
        },
        AnyCard::Material {
            name: "Fine Paintings".to_string(),
            material_types: vec![MaterialType::Paintings, MaterialType::Paintings],
            color_pip: None,
            ability: Ability::Sell,
        },
        AnyCard::Material {
            name: "Fine Textiles".to_string(),
            material_types: vec![MaterialType::Textiles, MaterialType::Textiles],
            color_pip: None,
            ability: Ability::DrawCards { count: 2 },
        },
        // Material + color pip cards (Ceramics)
        AnyCard::Material {
            name: "Terra Cotta".to_string(),
            material_types: vec![MaterialType::Ceramics],
            color_pip: Some(Color::Red),
            ability: Ability::Workshop { count: 2 },
        },
        AnyCard::Material {
            name: "Ochre Ware".to_string(),
            material_types: vec![MaterialType::Ceramics],
            color_pip: Some(Color::Yellow),
            ability: Ability::Workshop { count: 2 },
        },
        AnyCard::Material {
            name: "Cobalt Ware".to_string(),
            material_types: vec![MaterialType::Ceramics],
            color_pip: Some(Color::Blue),
            ability: Ability::Workshop { count: 2 },
        },
        // Material + color pip cards (Paintings)
        AnyCard::Material {
            name: "Cinnabar & Canvas".to_string(),
            material_types: vec![MaterialType::Paintings],
            color_pip: Some(Color::Red),
            ability: Ability::Sell,
        },
        AnyCard::Material {
            name: "Orpiment & Canvas".to_string(),
            material_types: vec![MaterialType::Paintings],
            color_pip: Some(Color::Yellow),
            ability: Ability::Sell,
        },
        AnyCard::Material {
            name: "Ultramarine & Canvas".to_string(),
            material_types: vec![MaterialType::Paintings],
            color_pip: Some(Color::Blue),
            ability: Ability::Sell,
        },
        // Material + color pip cards (Textiles)
        AnyCard::Material {
            name: "Alizarin & Fabric".to_string(),
            material_types: vec![MaterialType::Textiles],
            color_pip: Some(Color::Red),
            ability: Ability::DrawCards { count: 2 },
        },
        AnyCard::Material {
            name: "Fustic & Fabric".to_string(),
            material_types: vec![MaterialType::Textiles],
            color_pip: Some(Color::Yellow),
            ability: Ability::DrawCards { count: 2 },
        },
        AnyCard::Material {
            name: "Pastel & Fabric".to_string(),
            material_types: vec![MaterialType::Textiles],
            color_pip: Some(Color::Blue),
            ability: Ability::DrawCards { count: 2 },
        },
        // Dual material cards
        AnyCard::Material {
            name: "Clay & Canvas".to_string(),
            material_types: vec![MaterialType::Ceramics, MaterialType::Paintings],
            color_pip: None,
            ability: Ability::DestroyCards { count: 1 },
        },
        AnyCard::Material {
            name: "Clay & Fabric".to_string(),
            material_types: vec![MaterialType::Ceramics, MaterialType::Textiles],
            color_pip: None,
            ability: Ability::DestroyCards { count: 1 },
        },
        AnyCard::Material {
            name: "Canvas & Fabric".to_string(),
            material_types: vec![MaterialType::Paintings, MaterialType::Textiles],
            color_pip: None,
            ability: Ability::DestroyCards { count: 1 },
        },
    ]
}

pub fn action_cards() -> Vec<AnyCard> {
    vec![
        AnyCard::Action {
            name: "Alum".to_string(),
            ability: Ability::DestroyCards { count: 1 },
            workshop_abilities: vec![Ability::GainDucats { count: 1 }],
        },
        AnyCard::Action {
            name: "Cream of Tartar".to_string(),
            ability: Ability::DestroyCards { count: 1 },
            workshop_abilities: vec![Ability::DrawCards { count: 3 }],
        },
        AnyCard::Action {
            name: "Gum Arabic".to_string(),
            ability: Ability::DestroyCards { count: 1 },
            workshop_abilities: vec![Ability::GainSecondary],
        },
        AnyCard::Action {
            name: "Potash".to_string(),
            ability: Ability::DestroyCards { count: 1 },
            workshop_abilities: vec![Ability::Workshop { count: 3 }],
        },
        AnyCard::Action {
            name: "Vinegar".to_string(),
            ability: Ability::DestroyCards { count: 1 },
            workshop_abilities: vec![Ability::ChangeTertiary],
        },
    ]
}

pub fn chalk_card() -> AnyCard {
    AnyCard::Action {
        name: "Chalk".to_string(),
        ability: Ability::Sell,
        workshop_abilities: vec![Ability::GainPrimary],
    }
}

pub fn generate_all_buyers() -> Vec<AnyCard> {
    let mut buyers = Vec::new();

    // Textiles (2pt): one tertiary
    for &t in &TERTIARIES_LIST {
        buyers.push(AnyCard::Buyer {
            stars: 2,
            required_material: MaterialType::Textiles,
            color_cost: vec![t],
        });
    }

    // Textiles (2pt): one secondary + one primary
    for &s in &SECONDARIES_LIST {
        for &p in &PRIMARIES_LIST {
            buyers.push(AnyCard::Buyer {
                stars: 2,
                required_material: MaterialType::Textiles,
                color_cost: vec![s, p],
            });
        }
    }

    // Ceramics (3pt): one tertiary + one primary
    for &t in &TERTIARIES_LIST {
        for &p in &PRIMARIES_LIST {
            buyers.push(AnyCard::Buyer {
                stars: 3,
                required_material: MaterialType::Ceramics,
                color_cost: vec![t, p],
            });
        }
    }

    // Paintings (4pt): one tertiary + one secondary
    for &t in &TERTIARIES_LIST {
        for &s in &SECONDARIES_LIST {
            buyers.push(AnyCard::Buyer {
                stars: 4,
                required_material: MaterialType::Paintings,
                color_cost: vec![t, s],
            });
        }
    }

    buyers
}
