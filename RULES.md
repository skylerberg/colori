# Colori — Rules

## Overview

Colori is a deck-building game for 2–5 players about dyeing fabrics and crafting garments. Over 8 rounds, players draft dye and fabric cards, destroy them to store colors on a color wheel, mix colors, and craft garments worth stars. The player with the most stars at the end wins.

## Components

### Color Wheel

The color wheel has 12 colors arranged in a circle:

**Red** → Vermilion → **Orange** → Amber → **Yellow** → Chartreuse → **Green** → Teal → **Blue** → Indigo → **Purple** → Magenta → (back to Red)

Bold colors are **primary** (Red, Orange is secondary... actually the primaries are Red, Yellow, Blue). The pattern is:

- **Primary colors**: Red, Yellow, Blue
- **Secondary colors**: Orange (between Red and Yellow), Green (between Yellow and Blue), Purple (between Blue and Red)
- **Tertiary colors**: Vermilion (between Red and Orange), Amber (between Orange and Yellow), Chartreuse (between Yellow and Green), Teal (between Green and Blue), Indigo (between Blue and Purple), Magenta (between Purple and Red)

Each player has their own color wheel that tracks how many of each color they have stored.

### Cards

**Basic Dye Cards** (20 of each color, 60 total):
- Basic Red — 1 Red pip | Destroy: Store Colors ×1
- Basic Yellow — 1 Yellow pip | Destroy: Store Colors ×1
- Basic Blue — 1 Blue pip | Destroy: Store Colors ×1

**Dye Cards** (2 copies of each, 78 total): 39 unique dyes, each with color pips and a destroy ability. See the card reference below.

**Fabric Cards** (6 copies of each, 24 total):
- Wool | Destroy: Draw Cards ×1
- Silk | Destroy: Draw Cards ×2
- Linen | Destroy: Store Colors ×1
- Cotton | Destroy: Mix Colors ×1

**Garment Cards** (179 possible, 48 per game): Each garment requires a specific fabric type and a unique subset of colors as its cost. At game start, 12 garments are randomly selected from each of the 4 star tiers. Each garment is worth 2–5 stars.

### Decks

- **Draft Deck**: Contains all dye cards, fabric cards, and basic dyes not in players' starting decks. When the draft deck runs out, shuffle all destroyed cards back in.
- **Garment Deck**: Contains all garment cards. 6 garments are displayed face-up at all times.
- **Personal Deck**: Each player has their own deck that they draw from and build over the course of the game.

## Setup

1. Each player receives a starting personal deck of 8 cards: 2 Basic Red, 2 Basic Yellow, 2 Basic Blue, 1 Wool, 1 Silk. Shuffle it.
2. Place remaining basic dyes, all dye cards, and all fabric cards into the draft deck. Shuffle it.
3. Shuffle the garment deck and reveal 6 garments face-up in the garment display.

## Player Area

Each player has:
- **Personal Deck** (face down)
- **Discard Pile**
- **Drawn Cards** (face up, visible to all players)
- **Drafted Cards** (face up after drafting)
- **Color Wheel** (tracks stored colors)
- **Completed Garments** (scored garments)

## Round Structure

The game is played over **8 rounds**. Each round has three phases:

### 1. Draw Phase

Each player draws 5 cards from their personal deck into their drawn cards area. If the personal deck runs out, shuffle the discard pile to form a new deck, then continue drawing.

Drawn cards are face-up and visible to all players.

### 2. Draft Phase

1. Deal 5 cards from the draft deck to each player as a draft hand (these are private — other players cannot see them).
2. Each player simultaneously picks 1 card from their hand and places it face-down.
3. Players pass their remaining cards to the next player. The direction alternates each round (left on odd rounds, right on even rounds).
4. Repeat steps 2–3 until all 5 cards have been picked.
5. All drafted cards are placed face-up in each player's drafted cards area.

**Local multiplayer note**: Since all players share one device, a "Pass device to [player name]" screen is shown between picks to keep draft hands private.

### 3. Action Phase

Each player takes one turn, in player order. On your turn, you may destroy any number of your **drafted** cards, one at a time. Each destroyed card triggers its destroy ability (see Abilities below). You must fully resolve each ability before destroying another card. When you are done destroying cards (or choose to stop), your turn ends.

At the end of each player's turn:
- Any un-destroyed drafted cards go to the player's discard pile.
- Any remaining drawn cards go to the player's discard pile.

All player information (drawn cards, drafted cards, color wheel) is public during the action phase.

## Abilities

When a card is destroyed, its destroy ability triggers. Abilities are resolved using a queue — if an ability triggers further abilities, they are added to the back of the queue and resolved in order.

### Store Colors ×N

Choose N cards from your **drawn cards** area. For each chosen card, store ALL of its color pips on your color wheel. The chosen cards are discarded.

The destroyed card's own pips are **not** stored — only the chosen drawn cards' pips are stored.

### Draw Cards ×N

Draw N cards from your personal deck into your drawn cards area. If your deck runs out, shuffle your discard pile to form a new deck.

### Mix Colors ×N

Perform N mix operations on your color wheel. For each mix:
1. Choose two colors on your wheel that are exactly **2 positions apart** (e.g., Red and Orange, Yellow and Green, Blue and Purple).
2. Remove one of each from your wheel.
3. Add one of the color **between** them to your wheel.

**Mixing examples**:
- Red + Yellow → Orange
- Red + Orange → Vermilion
- Yellow + Orange → Amber
- Yellow + Green → Chartreuse
- Yellow + Blue → Green
- Blue + Green → Teal
- Blue + Purple → Indigo
- Red + Blue → Purple
- Red + Purple → Magenta (wrapping around the wheel)
- Purple + Magenta → Red... no: Magenta is between Purple and Red, so Purple + Red → Magenta

Complete mixing pairs (two inputs → one output):
| Input 1 | Input 2 | Output |
|---------|---------|--------|
| Red | Orange | Vermilion |
| Orange | Yellow | Amber |
| Yellow | Green | Chartreuse |
| Green | Blue | Teal |
| Blue | Purple | Indigo |
| Purple | Red | Magenta |
| Red | Yellow | Orange |
| Yellow | Blue | Green |
| Blue | Red | Purple |

### Destroy Cards ×N

Choose N cards from your **drawn cards** area and destroy them. Each destroyed card's ability triggers and is added to the ability queue (chain reactions are possible).

### Make Garment

You **must** make a garment if you are able to. If you cannot afford any garment in the display, the ability fizzles (nothing happens).

To make a garment:
1. Choose a garment from the 6 face-up garments in the display.
2. Discard a fabric card of the required type from your **drawn cards**.
3. Pay the color cost by removing the required colors from your color wheel.
4. The garment goes into your **discard pile** (it will cycle through your deck but has no useful effect — it clogs your deck).
5. Immediately refill the garment display from the garment deck.

## End of Game

After 8 rounds, the game ends. Each player totals the stars on all garments they have completed. The player with the most stars wins.

---

## Card Reference

### Dye Cards (39 unique, 2 copies each)

| # | Name | Colors | Destroy Ability |
|---|------|--------|-----------------|
| 1 | Kermes | 3 Red | Make Garment |
| 2 | Cochineal | 2 Red, 1 Magenta | Draw Cards ×2 |
| 3 | Madder | 2 Red, 1 Vermilion | Store Colors ×2 |
| 4 | Brazilwood | 2 Red | Draw Cards ×1 |
| 5 | Lac | 2 Red, 1 Magenta | Mix Colors ×1 |
| 6 | Safflower | 1 Red, 1 Magenta | Draw Cards ×1 |
| 7 | Alkanet | 1 Red, 1 Purple | Mix Colors ×1 |
| 8 | Dragon's Blood | 2 Red, 1 Vermilion | Destroy Cards ×1 |
| 9 | Venetian Red Earth | 1 Red, 1 Vermilion, 1 Amber | Store Colors ×1 |
| 10 | Vermilion (Mineral) | 1 Red, 2 Vermilion | Destroy Cards ×2 |
| 11 | Woad | 2 Blue | Store Colors ×2 |
| 12 | Indigo | 2 Blue, 1 Indigo | Make Garment |
| 13 | Smalt | 2 Blue, 1 Indigo | Destroy Cards ×1 |
| 14 | Azurite | 2 Blue, 1 Teal | Store Colors ×1 |
| 15 | Logwood | 1 Blue, 1 Indigo, 1 Purple | Mix Colors ×2 |
| 16 | Weld | 3 Yellow | Make Garment |
| 17 | Saffron | 2 Yellow, 1 Amber | Draw Cards ×3 |
| 18 | Turmeric | 1 Yellow, 1 Amber, 1 Orange | Mix Colors ×1 |
| 19 | Dyer's Broom | 2 Yellow | Draw Cards ×1 |
| 20 | Spanish Broom | 2 Yellow | Store Colors ×1 |
| 21 | Old Fustic | 2 Yellow, 1 Amber | Store Colors ×2 |
| 22 | Venetian Sumac | 1 Yellow, 1 Amber, 1 Orange | Mix Colors ×1 |
| 23 | Persian Berries | 1 Yellow, 1 Chartreuse | Draw Cards ×1 |
| 24 | Tyrian Purple | 2 Purple, 1 Magenta | Make Garment |
| 25 | Orchil | 1 Purple, 1 Magenta, 1 Red | Mix Colors ×2 |
| 26 | Turnsole | 2 Purple | Store Colors ×1 |
| 27 | Elderberry | 1 Purple, 1 Indigo | Draw Cards ×2 |
| 28 | Verdigris | 2 Green, 1 Teal | Destroy Cards ×1 |
| 29 | Lincoln Green | 1 Green, 1 Teal, 1 Blue | Mix Colors ×3 |
| 30 | Saxon Green | 2 Green, 1 Chartreuse | Store Colors ×3 |
| 31 | Gall Nuts | 1 Yellow, 1 Amber | Destroy Cards ×1 |
| 32 | Walnut Hulls | 1 Vermilion, 1 Amber, 1 Orange | Draw Cards ×1 |
| 33 | Oak Bark | 1 Yellow, 1 Amber | Store Colors ×1 |
| 34 | Cutch | 1 Red, 1 Amber, 1 Orange | Mix Colors ×1 |
| 35 | Chestnut | 1 Amber, 1 Orange | Draw Cards ×1 |
| 36 | Alder Bark | 1 Red, 1 Amber | Store Colors ×1 |
| 37 | Iron Black | 1 Blue, 1 Purple, 1 Indigo | Destroy Cards ×2 |
| 38 | Annatto | 1 Orange, 1 Amber, 1 Yellow | Mix Colors ×1 |
| 39 | Henna | 1 Orange, 1 Vermilion | Store Colors ×1 |

### Fabric Cards (4 types, 6 copies each)

| Name | Destroy Ability |
|------|-----------------|
| Wool | Draw Cards ×1 |
| Silk | Draw Cards ×2 |
| Linen | Store Colors ×1 |
| Cotton | Mix Colors ×1 |

### Basic Dye Cards (3 types, 20 copies each)

| Name | Color | Destroy Ability |
|------|-------|-----------------|
| Basic Red | 1 Red | Store Colors ×1 |
| Basic Yellow | 1 Yellow | Store Colors ×1 |
| Basic Blue | 1 Blue | Store Colors ×1 |

### Garment Cards (179 possible, 48 per game)

Garments do not have names. Each garment's color cost is a unique subset of the 12 colors on the wheel (each color appears at most once). The total "primary value" of the subset determines the garment's tier:

- **Primaries** (Red, Yellow, Blue) = value 1
- **Secondaries** (Orange, Green, Purple) = value 2
- **Tertiaries** (Vermilion, Amber, Chartreuse, Teal, Indigo, Magenta) = value 3

| Total Value | Stars | Fabric | Possible Count |
|-------------|-------|--------|----------------|
| 3           | 2     | Cotton | 16             |
| 4           | 3     | Linen  | 30             |
| 5           | 4     | Wool   | 48             |
| 6           | 5     | Silk   | 85             |

At game start, 12 garments are randomly selected from each star tier (48 total). These are shuffled together to form the garment deck.
