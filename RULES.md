# Colori — Rules

## Overview

Colori is a deck-building game for 2–5 players about dyeing materials and selling to buyers. Players draft dye, material, and action cards, destroy them to store colors on a color wheel, mix colors, and sell to buyers worth stars. The game lasts up to 10 rounds, or until any player reaches 15 points. The player with the most points (buyer stars + ducats) wins.

## Components

### Color Wheel

The color wheel has 12 colors arranged in a circle:

**Red** → Vermilion → **Orange** → Amber → **Yellow** → Chartreuse → **Green** → Teal → **Blue** → Indigo → **Purple** → Magenta → (back to Red)

- **Primary colors**: Red, Yellow, Blue
- **Secondary colors**: Orange (between Red and Yellow), Green (between Yellow and Blue), Purple (between Blue and Red)
- **Tertiary colors**: Vermilion (between Red and Orange), Amber (between Orange and Yellow), Chartreuse (between Yellow and Green), Teal (between Green and Blue), Indigo (between Blue and Purple), Magenta (between Purple and Red)

Each player has their own color wheel that tracks how many of each color they have stored.

### Cards

**Basic Dye Cards** (3 types, 1 per player in starting decks only):
- Basic Red — 1 Red pip | Destroy: Sell
- Basic Yellow — 1 Yellow pip | Destroy: Sell
- Basic Blue — 1 Blue pip | Destroy: Sell

**Dye Cards** (15 unique, 4 copies each, 60 total in draft deck):
- 3 Primary dyes — 3 pips of one primary color | Destroy: Sell
- 6 Secondary dyes — 2 pips of two adjacent primary/secondary colors | Destroy: Workshop ×3
- 6 Tertiary dyes — 1 pip of a tertiary color | Destroy: Mix Colors ×2

**Material Cards** (3 types in starting deck, 1 per player; 15 unique draft material cards in draft deck):
Starting deck materials gain 1 material when workshopped. Draft material cards offer enhanced gains: double materials, material + color pip, or dual materials.
- Ceramics | Destroy: Workshop ×2
- Paintings | Destroy: Sell
- Textiles | Destroy: Draw Cards ×2

**Action Cards** (5 unique in draft deck, 3 copies each, 15 total; plus Chalk as a starter card):
Each action card has Destroy Cards ×1 as its destroy ability, plus workshop abilities that trigger when the card is chosen during Workshop resolution. See the card reference below.

**Buyer Cards** (51 total, single deck):
Each buyer requires 2 of a specific material type and a set of colors as its cost. All 51 buyers are shuffled into a single deck. Each buyer is worth 2–4 stars.

### Decks

- **Draft Deck**: Contains 60 dye cards + 15 draft material cards + 15 action cards (90 total). When the draft deck runs out, shuffle all destroyed cards back in.
- **Buyer Deck**: Contains all buyer cards. 6 buyers are displayed face-up at all times.
- **Personal Deck**: Each player has their own deck that they draw from and build over the course of the game.

## Setup

1. Each player receives a starting personal deck of 7 cards: 1 Basic Red, 1 Basic Yellow, 1 Basic Blue, 1 Ceramics, 1 Paintings, 1 Textiles, 1 Chalk. Shuffle it.
2. Each player starts with 1 Red, 1 Yellow, and 1 Blue already stored on their color wheel.
3. Each player starts with 1 of each stored material (1 Ceramics, 1 Paintings, 1 Textiles).
4. Place all dye cards (4 copies each), 15 unique draft material cards, and action cards (3 copies each) into the draft deck. Shuffle it.
5. Shuffle the buyer deck and reveal 6 buyers face-up in the buyer display.

## Player Area

Each player has:
- **Personal Deck** (face down)
- **Discard Pile**
- **Drawn Cards** (face up, visible to all players)
- **Drafted Cards** (face up after drafting)
- **Color Wheel** (tracks stored colors)
- **Stored Materials** (tracks Textiles, Ceramics, and Paintings counts)
- **Ducats** (count toward final score)
- **Completed Buyers** (scored buyers, kept in a separate area)

## Round Structure

The game is played over up to **10 rounds**. Each round has three phases. The starting player rotates each round: player `(round - 1) % numPlayers` goes first.

### 1. Draw Phase

Each player draws 5 cards from their personal deck into their drawn cards area. If the personal deck runs out, shuffle the discard pile to form a new deck, then continue drawing.

Drawn cards are face-up and visible to all players.

### 2. Draft Phase

1. Deal 5 cards from the draft deck to each player as a draft hand (these are private — other players cannot see them).
2. Each player simultaneously picks 1 card from their hand and places it face-down.
3. Players pass their remaining cards to the next player. The direction alternates each round (left on odd rounds, right on even rounds).
4. Repeat steps 2–3 until 4 cards have been picked.
5. The 5th remaining card in each hand is destroyed (no ability triggers).
6. All drafted cards are placed face-up in each player's drafted cards area.

**Local multiplayer note**: Since all players share one device, a "Pass device to [player name]" screen is shown between picks to keep draft hands private.

### 3. Action Phase

Each player takes one turn, starting with the starting player for the round. On your turn, you may destroy any number of your **drafted** cards, one at a time. Each destroyed card triggers its destroy ability (see Abilities below). You must fully resolve each ability before destroying another card. When you are done destroying cards (or choose to stop), your turn ends.

At the end of each player's turn:
- Any un-destroyed drafted cards go to the player's discard pile.
- Any remaining drawn cards go to the player's discard pile.

All player information (drawn cards, drafted cards, color wheel, stored materials, ducats) is public during the action phase.

## Abilities

When a card is destroyed, its destroy ability triggers. Abilities are resolved using a **stack (LIFO)** — if an ability triggers further abilities, they are pushed onto the stack and the most recently pushed ability resolves next.

### Workshop ×N

Choose up to N cards from your **drawn cards** area. You may skip Workshop entirely (choosing 0 cards).

For each chosen card:
- **Material card**: Store all materials listed on the card (e.g., 2x Ceramics, or 1 Ceramics + 1 Paintings). If the card has a color pip, also store that color on your color wheel. The card is discarded.
- **Dye or basic dye card**: Store ALL of its color pips on your color wheel. The card is discarded.
- **Action card**: Consumes 1 pick. The action card's workshop abilities are pushed onto the ability stack, and any remaining Workshop picks carry over (pushed below the workshop abilities on the stack). The card is discarded.

### Draw Cards ×N

Draw N cards from your personal deck into your drawn cards area. If your deck runs out, shuffle your discard pile to form a new deck.

### Mix Colors ×N

Perform up to N mix operations on your color wheel. You may stop early. For each mix:
1. Choose two colors on your wheel that are exactly **2 positions apart** (e.g., Red and Orange, Yellow and Green).
2. Remove one of each from your wheel.
3. Add one of the color **between** them to your wheel.

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

Choose N cards from your **drawn cards** area and destroy them. Each destroyed card's ability triggers and is pushed onto the ability stack (chain reactions are possible).

### Sell

You **must** sell to a buyer if you are able to. If you cannot afford any buyer in the display, the ability fizzles (nothing happens).

To sell to a buyer:
1. Choose a buyer from the 6 face-up buyers in the display.
2. Spend 2 stored materials of the required type.
3. Pay the color cost by removing the required colors from your color wheel.
4. The buyer goes to your **completed buyers** area.
5. Immediately refill the buyer display from the buyer deck.

### Gain Ducats ×N

Gain N ducats. Ducats count toward your final score.

### Any Secondary

Choose one of the three secondary colors (Orange, Green, or Purple) and add it to your color wheel. This ability never fizzles.

### Any Primary

Choose one of the three primary colors (Red, Yellow, or Blue) and add it to your color wheel. This ability never fizzles.

### Change Tertiary

Swap one tertiary color on your wheel for a different tertiary. First choose a tertiary color you have stored (Vermilion, Amber, Chartreuse, Teal, Indigo, or Magenta) to remove, then choose a different tertiary color to gain. If you have no tertiary colors stored, the ability fizzles.

## End of Game

The game ends when either:
- Any player reaches **15 or more points** after a complete round, OR
- **Round 10** is completed.

Each player's score = buyer stars + ducats. The player with the highest score wins.

---

## Card Reference

### Basic Dye Cards (3 types, 1 per player in starting decks)

| Name | Color | Destroy Ability |
|------|-------|-----------------|
| Basic Red | 1 Red | Sell |
| Basic Yellow | 1 Yellow | Sell |
| Basic Blue | 1 Blue | Sell |

### Dye Cards (15 unique, 4 copies each in draft deck)

**Primary Dyes** (3 pips of one color):

| Name | Colors | Destroy Ability |
|------|--------|-----------------|
| Kermes | 3 Red | Sell |
| Weld | 3 Yellow | Sell |
| Woad | 3 Blue | Sell |

**Secondary Dyes** (2 pips of adjacent colors):

| Name | Colors | Destroy Ability |
|------|--------|-----------------|
| Madder | 1 Orange, 1 Red | Workshop ×3 |
| Turmeric | 1 Orange, 1 Yellow | Workshop ×3 |
| Dyer's Greenweed | 1 Green, 1 Yellow | Workshop ×3 |
| Verdigris | 1 Green, 1 Blue | Workshop ×3 |
| Orchil | 1 Purple, 1 Red | Workshop ×3 |
| Logwood | 1 Purple, 1 Blue | Workshop ×3 |

**Tertiary Dyes** (1 pip of a tertiary color):

| Name | Colors | Destroy Ability |
|------|--------|-----------------|
| Vermilion | 1 Vermilion | Mix Colors ×2 |
| Saffron | 1 Amber | Mix Colors ×2 |
| Persian Berries | 1 Chartreuse | Mix Colors ×2 |
| Azurite | 1 Teal | Mix Colors ×2 |
| Indigo | 1 Indigo | Mix Colors ×2 |
| Cochineal | 1 Magenta | Mix Colors ×2 |

### Starting Material Cards (3 types, 1 per player in starting deck)

| Name | Material Gain | Destroy Ability |
|------|--------------|-----------------|
| Ceramics | 1 Ceramics | Workshop ×2 |
| Paintings | 1 Paintings | Sell |
| Textiles | 1 Textiles | Draw Cards ×2 |

### Draft Material Cards (15 unique in draft deck)

| Name | Material Gain | Color Pip | Destroy Ability |
|------|--------------|-----------|-----------------|
| Fine Ceramics | 2x Ceramics | — | Workshop ×2 |
| Fine Paintings | 2x Paintings | — | Sell |
| Fine Textiles | 2x Textiles | — | Draw Cards ×2 |
| Terra Cotta | 1 Ceramics | Red | Workshop ×2 |
| Ochre Ware | 1 Ceramics | Yellow | Workshop ×2 |
| Cobalt Ware | 1 Ceramics | Blue | Workshop ×2 |
| Cinnabar & Canvas | 1 Paintings | Red | Sell |
| Orpiment & Canvas | 1 Paintings | Yellow | Sell |
| Ultramarine & Canvas | 1 Paintings | Blue | Sell |
| Alizarin & Fabric | 1 Textiles | Red | Draw Cards ×2 |
| Fustic & Fabric | 1 Textiles | Yellow | Draw Cards ×2 |
| Pastel & Fabric | 1 Textiles | Blue | Draw Cards ×2 |
| Clay & Canvas | 1 Ceramics + 1 Paintings | — | Destroy Cards ×1 |
| Clay & Fabric | 1 Ceramics + 1 Textiles | — | Destroy Cards ×1 |
| Canvas & Fabric | 1 Paintings + 1 Textiles | — | Destroy Cards ×1 |

### Action Cards (5 unique in draft deck, 3 copies each; plus Chalk as a starter)

All action cards have **Destroy Cards ×1** as their destroy ability. Their workshop abilities (triggered when chosen during Workshop resolution) are:

| Name | Workshop Abilities | Notes |
|------|-------------------|-------|
| Chalk | Any Primary | Starter card (1 per player, not in draft deck) |
| Alum | Gain Ducats ×1 | Draft deck |
| Cream of Tartar | Draw Cards ×3 | Draft deck |
| Gum Arabic | Any Secondary | Draft deck |
| Potash | Workshop ×3 | Draft deck |
| Vinegar | Change Tertiary | Draft deck |

### Buyer Cards (51 total, single deck)

Buyers do not have names. Each buyer requires 2 of a specific material type and a set of colors as its cost.

| Material | Stars | Requirement | Count |
|----------|-------|-------------|-------|
| Textiles | 2 | One tertiary OR one secondary + one primary | 15 |
| Ceramics | 3 | One tertiary + one primary | 18 |
| Paintings | 4 | One tertiary + one secondary | 18 |

All 51 buyers are shuffled into a single deck. 6 buyers are displayed face-up at all times.
