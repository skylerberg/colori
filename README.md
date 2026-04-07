# Colori

A strategic deck-building card game for 2-5 players about dyeing materials and selling to buyers. Complete game rules are in `RULES.md`.

## Quick Start

```bash
npm install
npm run build:wasm
npm run dev
```

## Variant Configuration

| Field                | Type   | Default    | Description                       |
|----------------------|--------|------------|-----------------------------------|
| `name`               | string | auto       | Display name for this variant     |
| `iterations`         | number | 100        | MCTS iterations per move          |
| `explorationConstant`| number | 0.75       | UCB exploration constant          |
| `maxRolloutSteps`    | number | 200        | Max steps per rollout simulation  |
