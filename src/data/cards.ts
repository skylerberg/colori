import type { AnyCard, Color, DyeCard, BasicDyeCard, FabricCard, GarmentCard } from './types';

export function getCardPips(card: AnyCard): Color[] {
  switch (card.kind) {
    case 'dye': return card.colors;
    case 'basicDye': return [card.color];
    case 'fabric': return [];
    case 'garment': return [];
  }
}

export const DYE_CARDS: DyeCard[] = [
  // 1
  {
    kind: 'dye',
    name: 'Kermes',
    colors: ['Red', 'Red', 'Red'],
    ability: { type: 'makeGarment' },
  },
  // 2
  {
    kind: 'dye',
    name: 'Cochineal',
    colors: ['Red', 'Red', 'Magenta'],
    ability: { type: 'drawCards', count: 2 },
  },
  // 3
  {
    kind: 'dye',
    name: 'Madder',
    colors: ['Red', 'Red', 'Vermilion'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  // 4
  {
    kind: 'dye',
    name: 'Brazilwood',
    colors: ['Red', 'Red'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 5
  {
    kind: 'dye',
    name: 'Lac',
    colors: ['Red', 'Red', 'Magenta'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 6
  {
    kind: 'dye',
    name: 'Safflower',
    colors: ['Red', 'Magenta'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 7
  {
    kind: 'dye',
    name: 'Alkanet',
    colors: ['Red', 'Purple'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 8
  {
    kind: 'dye',
    name: "Dragon's Blood",
    colors: ['Red', 'Red', 'Vermilion'],
    ability: { type: 'destroyCards', count: 1 },
  },
  // 9
  {
    kind: 'dye',
    name: 'Venetian Red Earth',
    colors: ['Red', 'Vermilion', 'Amber'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 10
  {
    kind: 'dye',
    name: 'Vermilion (Mineral)',
    colors: ['Red', 'Vermilion', 'Vermilion'],
    ability: { type: 'destroyCards', count: 2 },
  },
  // 11
  {
    kind: 'dye',
    name: 'Woad',
    colors: ['Blue', 'Blue'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  // 12
  {
    kind: 'dye',
    name: 'Indigo',
    colors: ['Blue', 'Blue', 'Indigo'],
    ability: { type: 'makeGarment' },
  },
  // 13
  {
    kind: 'dye',
    name: 'Smalt',
    colors: ['Blue', 'Blue', 'Indigo'],
    ability: { type: 'destroyCards', count: 1 },
  },
  // 14
  {
    kind: 'dye',
    name: 'Azurite',
    colors: ['Blue', 'Blue', 'Teal'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 15
  {
    kind: 'dye',
    name: 'Logwood',
    colors: ['Blue', 'Indigo', 'Purple'],
    ability: { type: 'mixColors', count: 2 },
  },
  // 16
  {
    kind: 'dye',
    name: 'Weld',
    colors: ['Yellow', 'Yellow', 'Yellow'],
    ability: { type: 'makeGarment' },
  },
  // 17
  {
    kind: 'dye',
    name: 'Saffron',
    colors: ['Yellow', 'Yellow', 'Amber'],
    ability: { type: 'drawCards', count: 3 },
  },
  // 18
  {
    kind: 'dye',
    name: 'Turmeric',
    colors: ['Yellow', 'Amber', 'Orange'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 19
  {
    kind: 'dye',
    name: "Dyer's Broom",
    colors: ['Yellow', 'Yellow'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 20
  {
    kind: 'dye',
    name: 'Spanish Broom',
    colors: ['Yellow', 'Yellow'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 21
  {
    kind: 'dye',
    name: 'Old Fustic',
    colors: ['Yellow', 'Yellow', 'Amber'],
    ability: { type: 'makeMaterials', count: 3 },
  },
  // 22
  {
    kind: 'dye',
    name: 'Venetian Sumac',
    colors: ['Yellow', 'Amber', 'Orange'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 23
  {
    kind: 'dye',
    name: 'Persian Berries',
    colors: ['Yellow', 'Chartreuse'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 24
  {
    kind: 'dye',
    name: 'Tyrian Purple',
    colors: ['Purple', 'Purple', 'Magenta'],
    ability: { type: 'makeGarment' },
  },
  // 25
  {
    kind: 'dye',
    name: 'Orchil',
    colors: ['Purple', 'Magenta', 'Red'],
    ability: { type: 'mixColors', count: 2 },
  },
  // 26
  {
    kind: 'dye',
    name: 'Turnsole',
    colors: ['Purple', 'Purple'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 27
  {
    kind: 'dye',
    name: 'Elderberry',
    colors: ['Purple', 'Indigo'],
    ability: { type: 'drawCards', count: 2 },
  },
  // 28
  {
    kind: 'dye',
    name: 'Verdigris',
    colors: ['Green', 'Green', 'Teal'],
    ability: { type: 'destroyCards', count: 1 },
  },
  // 29
  {
    kind: 'dye',
    name: 'Lincoln Green',
    colors: ['Green', 'Teal', 'Blue'],
    ability: { type: 'mixColors', count: 3 },
  },
  // 30
  {
    kind: 'dye',
    name: 'Saxon Green',
    colors: ['Green', 'Green', 'Chartreuse'],
    ability: { type: 'makeMaterials', count: 4 },
  },
  // 31
  {
    kind: 'dye',
    name: 'Gall Nuts',
    colors: ['Yellow', 'Amber'],
    ability: { type: 'destroyCards', count: 1 },
  },
  // 32
  {
    kind: 'dye',
    name: 'Walnut Hulls',
    colors: ['Vermilion', 'Amber', 'Orange'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 33
  {
    kind: 'dye',
    name: 'Oak Bark',
    colors: ['Yellow', 'Amber'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 34
  {
    kind: 'dye',
    name: 'Cutch',
    colors: ['Red', 'Amber', 'Orange'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 35
  {
    kind: 'dye',
    name: 'Chestnut',
    colors: ['Amber', 'Orange'],
    ability: { type: 'drawCards', count: 1 },
  },
  // 36
  {
    kind: 'dye',
    name: 'Alder Bark',
    colors: ['Red', 'Amber'],
    ability: { type: 'makeMaterials', count: 2 },
  },
  // 37
  {
    kind: 'dye',
    name: 'Iron Black',
    colors: ['Blue', 'Purple', 'Indigo'],
    ability: { type: 'destroyCards', count: 2 },
  },
  // 38
  {
    kind: 'dye',
    name: 'Annatto',
    colors: ['Orange', 'Amber', 'Yellow'],
    ability: { type: 'mixColors', count: 1 },
  },
  // 39
  {
    kind: 'dye',
    name: 'Henna',
    colors: ['Orange', 'Vermilion'],
    ability: { type: 'makeMaterials', count: 2 },
  },
];

export const FABRIC_CARDS: FabricCard[] = [
  {
    kind: 'fabric',
    name: 'Wool',
    fabricType: 'Wool',
    ability: { type: 'drawCards', count: 1 },
  },
  {
    kind: 'fabric',
    name: 'Silk',
    fabricType: 'Silk',
    ability: { type: 'drawCards', count: 2 },
  },
  {
    kind: 'fabric',
    name: 'Linen',
    fabricType: 'Linen',
    ability: { type: 'makeMaterials', count: 2 },
  },
  {
    kind: 'fabric',
    name: 'Cotton',
    fabricType: 'Cotton',
    ability: { type: 'mixColors', count: 1 },
  },
];

export const BASIC_DYE_CARDS: BasicDyeCard[] = [
  {
    kind: 'basicDye',
    name: 'Basic Red',
    color: 'Red',
    ability: { type: 'makeMaterials', count: 2 },
  },
  {
    kind: 'basicDye',
    name: 'Basic Yellow',
    color: 'Yellow',
    ability: { type: 'makeMaterials', count: 2 },
  },
  {
    kind: 'basicDye',
    name: 'Basic Blue',
    color: 'Blue',
    ability: { type: 'makeMaterials', count: 2 },
  },
];

export const GARMENT_CARDS: GarmentCard[] = [
  // 1 - Kermes
  {
    kind: 'garment',
    name: 'Kermes Crimson Robe',
    stars: 5,
    requiredFabric: 'Silk',
    colorCost: ['Red', 'Red', 'Red', 'Red', 'Red', 'Red'],
  },
  // 2 - Cochineal
  {
    kind: 'garment',
    name: 'Cochineal Magenta Gown',
    stars: 4,
    requiredFabric: 'Silk',
    colorCost: ['Red', 'Red', 'Red', 'Red', 'Magenta', 'Magenta'],
  },
  // 3 - Madder
  {
    kind: 'garment',
    name: 'Madder Red Doublet',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Red', 'Red', 'Red', 'Red', 'Vermilion', 'Vermilion'],
  },
  // 4 - Brazilwood
  {
    kind: 'garment',
    name: 'Brazilwood Rose Cloak',
    stars: 2,
    requiredFabric: 'Wool',
    colorCost: ['Red', 'Red', 'Red', 'Red'],
  },
  // 5 - Lac
  {
    kind: 'garment',
    name: 'Lac Crimson Sash',
    stars: 4,
    requiredFabric: 'Silk',
    colorCost: ['Red', 'Red', 'Red', 'Red', 'Magenta', 'Magenta'],
  },
  // 6 - Safflower
  {
    kind: 'garment',
    name: 'Safflower Pink Veil',
    stars: 2,
    requiredFabric: 'Silk',
    colorCost: ['Red', 'Red', 'Magenta', 'Magenta'],
  },
  // 7 - Alkanet
  {
    kind: 'garment',
    name: 'Alkanet Violet Bodice',
    stars: 2,
    requiredFabric: 'Linen',
    colorCost: ['Red', 'Red', 'Purple', 'Purple'],
  },
  // 8 - Dragon's Blood
  {
    kind: 'garment',
    name: "Dragon's Blood Scarlet Cape",
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Red', 'Red', 'Red', 'Red', 'Vermilion', 'Vermilion'],
  },
  // 9 - Venetian Red Earth
  {
    kind: 'garment',
    name: 'Venetian Earth Russet Tunic',
    stars: 3,
    requiredFabric: 'Linen',
    colorCost: ['Red', 'Red', 'Vermilion', 'Vermilion', 'Amber', 'Amber'],
  },
  // 10 - Vermilion (Mineral)
  {
    kind: 'garment',
    name: 'Vermilion Ceremonial Stole',
    stars: 4,
    requiredFabric: 'Silk',
    colorCost: ['Red', 'Red', 'Vermilion', 'Vermilion', 'Vermilion', 'Vermilion'],
  },
  // 11 - Woad
  {
    kind: 'garment',
    name: "Woad Blue Workman's Apron",
    stars: 2,
    requiredFabric: 'Linen',
    colorCost: ['Blue', 'Blue', 'Blue', 'Blue'],
  },
  // 12 - Indigo
  {
    kind: 'garment',
    name: "Indigo Merchant's Coat",
    stars: 4,
    requiredFabric: 'Wool',
    colorCost: ['Blue', 'Blue', 'Blue', 'Blue', 'Indigo', 'Indigo'],
  },
  // 13 - Smalt
  {
    kind: 'garment',
    name: 'Smalt Blue Brocade Vest',
    stars: 4,
    requiredFabric: 'Silk',
    colorCost: ['Blue', 'Blue', 'Blue', 'Blue', 'Indigo', 'Indigo'],
  },
  // 14 - Azurite
  {
    kind: 'garment',
    name: 'Azurite Sky-Blue Mantle',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Blue', 'Blue', 'Blue', 'Blue', 'Teal', 'Teal'],
  },
  // 15 - Logwood
  {
    kind: 'garment',
    name: 'Logwood Twilight Cassock',
    stars: 4,
    requiredFabric: 'Wool',
    colorCost: ['Blue', 'Blue', 'Indigo', 'Indigo', 'Purple', 'Purple'],
  },
  // 16 - Weld
  {
    kind: 'garment',
    name: 'Weld Golden Festival Dress',
    stars: 5,
    requiredFabric: 'Linen',
    colorCost: ['Yellow', 'Yellow', 'Yellow', 'Yellow', 'Yellow', 'Yellow'],
  },
  // 17 - Saffron
  {
    kind: 'garment',
    name: 'Saffron Gold Silk Turban',
    stars: 4,
    requiredFabric: 'Silk',
    colorCost: ['Yellow', 'Yellow', 'Yellow', 'Yellow', 'Amber', 'Amber'],
  },
  // 18 - Turmeric
  {
    kind: 'garment',
    name: 'Turmeric Amber Headscarf',
    stars: 3,
    requiredFabric: 'Cotton',
    colorCost: ['Yellow', 'Yellow', 'Amber', 'Amber', 'Orange', 'Orange'],
  },
  // 19 - Dyer's Broom
  {
    kind: 'garment',
    name: "Dyer's Broom Yellow Kirtle",
    stars: 2,
    requiredFabric: 'Wool',
    colorCost: ['Yellow', 'Yellow', 'Yellow', 'Yellow'],
  },
  // 20 - Spanish Broom
  {
    kind: 'garment',
    name: 'Spanish Broom Sunlight Shawl',
    stars: 2,
    requiredFabric: 'Wool',
    colorCost: ['Yellow', 'Yellow', 'Yellow', 'Yellow'],
  },
  // 21 - Old Fustic
  {
    kind: 'garment',
    name: 'Old Fustic Amber Jerkin',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Yellow', 'Yellow', 'Yellow', 'Yellow', 'Amber', 'Amber'],
  },
  // 22 - Venetian Sumac
  {
    kind: 'garment',
    name: 'Venetian Sumac Harvest Skirt',
    stars: 3,
    requiredFabric: 'Linen',
    colorCost: ['Yellow', 'Yellow', 'Amber', 'Amber', 'Orange', 'Orange'],
  },
  // 23 - Persian Berries
  {
    kind: 'garment',
    name: 'Persian Berry Chartreuse Sleeve',
    stars: 3,
    requiredFabric: 'Silk',
    colorCost: ['Yellow', 'Yellow', 'Chartreuse', 'Chartreuse'],
  },
  // 24 - Tyrian Purple
  {
    kind: 'garment',
    name: 'Tyrian Purple Imperial Toga',
    stars: 5,
    requiredFabric: 'Silk',
    colorCost: ['Purple', 'Purple', 'Purple', 'Purple', 'Magenta', 'Magenta'],
  },
  // 25 - Orchil
  {
    kind: 'garment',
    name: 'Orchil Plum Petticoat',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Purple', 'Purple', 'Magenta', 'Magenta', 'Red', 'Red'],
  },
  // 26 - Turnsole
  {
    kind: 'garment',
    name: 'Turnsole Violet Hood',
    stars: 2,
    requiredFabric: 'Wool',
    colorCost: ['Purple', 'Purple', 'Purple', 'Purple'],
  },
  // 27 - Elderberry
  {
    kind: 'garment',
    name: 'Elderberry Dusk Stockings',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Purple', 'Purple', 'Indigo', 'Indigo'],
  },
  // 28 - Verdigris
  {
    kind: 'garment',
    name: 'Verdigris Copper-Green Surcoat',
    stars: 3,
    requiredFabric: 'Linen',
    colorCost: ['Green', 'Green', 'Green', 'Green', 'Teal', 'Teal'],
  },
  // 29 - Lincoln Green
  {
    kind: 'garment',
    name: "Lincoln Green Huntsman's Coat",
    stars: 4,
    requiredFabric: 'Wool',
    colorCost: ['Green', 'Green', 'Teal', 'Teal', 'Blue', 'Blue'],
  },
  // 30 - Saxon Green
  {
    kind: 'garment',
    name: 'Saxon Green Emerald Gown',
    stars: 5,
    requiredFabric: 'Silk',
    colorCost: ['Green', 'Green', 'Green', 'Green', 'Chartreuse', 'Chartreuse'],
  },
  // 31 - Gall Nuts
  {
    kind: 'garment',
    name: 'Gall Nut Tan Breeches',
    stars: 1,
    requiredFabric: 'Linen',
    colorCost: ['Yellow', 'Yellow', 'Amber', 'Amber'],
  },
  // 32 - Walnut Hulls
  {
    kind: 'garment',
    name: "Walnut Brown Traveler's Cloak",
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Vermilion', 'Vermilion', 'Amber', 'Amber', 'Orange', 'Orange'],
  },
  // 33 - Oak Bark
  {
    kind: 'garment',
    name: 'Oak Bark Tawny Coif',
    stars: 1,
    requiredFabric: 'Linen',
    colorCost: ['Yellow', 'Yellow', 'Amber', 'Amber'],
  },
  // 34 - Cutch
  {
    kind: 'garment',
    name: 'Catechu Cinnamon Gloves',
    stars: 3,
    requiredFabric: 'Wool',
    colorCost: ['Red', 'Red', 'Amber', 'Amber', 'Orange', 'Orange'],
  },
  // 35 - Chestnut
  {
    kind: 'garment',
    name: 'Chestnut Autumn Vest',
    stars: 2,
    requiredFabric: 'Cotton',
    colorCost: ['Amber', 'Amber', 'Orange', 'Orange'],
  },
  // 36 - Alder Bark
  {
    kind: 'garment',
    name: 'Alder Bark Russet Apron',
    stars: 2,
    requiredFabric: 'Linen',
    colorCost: ['Red', 'Red', 'Amber', 'Amber'],
  },
  // 37 - Iron Black
  {
    kind: 'garment',
    name: "Iron Black Magistrate's Mantle",
    stars: 5,
    requiredFabric: 'Wool',
    colorCost: ['Blue', 'Blue', 'Purple', 'Purple', 'Indigo', 'Indigo'],
  },
  // 38 - Annatto
  {
    kind: 'garment',
    name: 'Annatto Sunset Bandana',
    stars: 3,
    requiredFabric: 'Cotton',
    colorCost: ['Orange', 'Orange', 'Amber', 'Amber', 'Yellow', 'Yellow'],
  },
  // 39 - Henna
  {
    kind: 'garment',
    name: 'Henna Terra Cotta Sash',
    stars: 2,
    requiredFabric: 'Cotton',
    colorCost: ['Orange', 'Orange', 'Vermilion', 'Vermilion'],
  },
];
