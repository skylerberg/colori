import { describe, expect, it } from 'vitest';
import type { Color, PlayerState } from './data/types';
import { ALL_COLORS, mixInputs, mixResult } from './data/colors';
import { logChange, migrateLog, playEntry, snapshot } from './logEntries';

function player(): PlayerState {
  return {
    deck: [],
    buyers: [{ instanceId: 3, card: 'PotterAmber' }],
    workshoppedCards: [],
    workshopCards: [{ instanceId: 1, card: 'BasicRed' }, { instanceId: 8, card: 'StarterCeramics' }],
    draftedCards: [{ instanceId: 20, card: 'Cinnabar' }],
    colorWheel: Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>,
    materials: { Textiles: 0, Ceramics: 0, Paintings: 0 },
    completedSellCards: [],
    ducats: 1,
    wildPrimaries: 0,
    boats: 1,
  };
}

describe('mixInputs', () => {
  it('names the two colors either side of the one they make', () => {
    for (const color of ALL_COLORS) {
      const inputs = mixInputs(color);
      if (inputs === null) continue;
      expect(mixResult(...inputs)).toBe(color);
    }
    expect(mixInputs('Orange')).toEqual(['Red', 'Yellow']);
    expect(mixInputs('Fuchsia')).toEqual(['Purple', 'Red']);
    expect(mixInputs('Blue')).toBeNull();
  });
});

describe('playEntry', () => {
  it('records a trash and the workshop answering it, with what they gave', () => {
    const before = player();
    const after = snapshot(before);
    after.draftedCards = [];
    after.workshopCards = [];
    after.workshoppedCards = before.workshopCards;
    after.colorWheel.Red = 1;
    after.materials.Ceramics = 1;

    const entry = playEntry(2, 0, { type: 'trashAndWorkshop', card: 'Cinnabar', workshopCards: ['StarterCeramics', 'BasicRed'] }, before, after);
    expect(entry).toEqual({
      kind: 'play',
      round: 2,
      player: 0,
      moves: [
        { type: 'trash', card: { instanceId: 20, card: 'Cinnabar' } },
        { type: 'workshop', cards: [{ instanceId: 8, card: 'StarterCeramics' }, { instanceId: 1, card: 'BasicRed' }] },
      ],
      change: { colors: { Red: 1 }, materials: { Ceramics: 1 }, ducats: 0, boats: 0, wildPrimaries: 0, points: 0 },
    });
  });

  it('names the buyer drawn unseen from what arrived', () => {
    const before = player();
    const after = snapshot(before);
    after.buyers.push({ instanceId: 9, card: 'DyerGreen' });
    const entry = playEntry(1, 1, { type: 'drawBuyer' }, before, after);
    expect(entry?.kind === 'play' && entry.moves).toEqual([{ type: 'claim', buyer: 'DyerGreen', unseen: true }]);
  });

  it('counts a sale in points and what it spent', () => {
    const before = player();
    before.colorWheel.Amber = 1;
    const after = snapshot(before);
    after.colorWheel.Amber = 0;
    after.boats = 0;
    after.buyers = [];
    after.completedSellCards = [{ instanceId: 3, card: 'PotterAmber' }];
    const change = logChange(before, after);
    expect(change).toEqual({ colors: { Amber: -1 }, materials: {}, ducats: 0, boats: -1, wildPrimaries: 0, points: 3 });
  });

  it('leaves draft picks out', () => {
    expect(playEntry(1, 0, { type: 'draftPick', card: 'Alum' }, player(), player())).toBeNull();
  });

  it('shows the cards a turn trashed on its way out', () => {
    const before = player();
    const after = snapshot(before);
    after.draftedCards = [];
    const entry = playEntry(3, 0, { type: 'endTurn', keepRemaining: false }, before, after);
    expect(entry?.kind === 'play' && entry.moves).toEqual([
      { type: 'trashWithoutUsing', cards: [{ instanceId: 20, card: 'Cinnabar' }] },
      { type: 'endTurn' },
    ]);
  });

  it('keeps a turn that ends empty quiet', () => {
    const before = player();
    before.draftedCards = [];
    const entry = playEntry(3, 0, { type: 'endTurn', keepRemaining: false }, before, before);
    expect(entry?.kind === 'play' && entry.moves).toEqual([{ type: 'endTurn' }]);
  });
});

describe('migrateLog', () => {
  it('keeps the lines of a log saved before entries were structured', () => {
    expect(migrateLog(['Round 1 began'])).toEqual([{ kind: 'note', round: 0, player: null, text: 'Round 1 began' }]);
  });
});
