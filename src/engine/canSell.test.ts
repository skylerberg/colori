import { describe, expect, it } from 'vitest';
import { canSell } from './wasmEngine';
import { ALL_COLORS } from '../data/colors';
import type { Color, GameState, MaterialType, PlayerState, SellCard } from '../data/types';

// `canSell` restates the engine's affordability rule so the UI can grey out
// buyers without a round trip; these pin it to the cases the Rust tests cover.
function stateWith(
  sellCard: SellCard,
  stock: { colors?: Color[]; materials?: MaterialType[]; boats?: number; wildPrimaries?: number },
): GameState {
  const colorWheel = Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>;
  for (const c of stock.colors ?? []) colorWheel[c]++;
  const materials: Record<MaterialType, number> = { Textiles: 0, Ceramics: 0, Paintings: 0 };
  for (const m of stock.materials ?? []) materials[m]++;
  const player = {
    buyers: [{ instanceId: 7, card: sellCard }],
    colorWheel,
    materials,
    boats: stock.boats ?? 0,
    wildPrimaries: stock.wildPrimaries ?? 0,
  } as unknown as PlayerState;
  return {
    players: [player],
    phase: { type: 'action', actionState: { currentPlayerIndex: 0, abilityStack: [] } },
  } as unknown as GameState;
}

describe('canSell', () => {
  it('needs the material and every colour', () => {
    expect(canSell(stateWith('PotterAmber', { colors: ['Amber'], materials: ['Ceramics'] }), 7)).toBe(true);
    expect(canSell(stateWith('PotterAmber', { colors: ['Amber'] }), 7)).toBe(false);
    expect(canSell(stateWith('PotterAmber', { materials: ['Ceramics'] }), 7)).toBe(false);
  });

  it('asks nothing but colours of an illuminator or a glassmaker', () => {
    expect(canSell(stateWith('IlluminatorTeal', { colors: ['Teal'] }), 7)).toBe(true);
    expect(canSell(stateWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue', 'Blue'] }), 7)).toBe(true);
    expect(canSell(stateWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue'] }), 7)).toBe(false);
  });

  it('lets a wild primary cover a missing primary but not a tertiary', () => {
    expect(canSell(stateWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue'], wildPrimaries: 1 }), 7)).toBe(true);
    expect(canSell(stateWith('GlassmakerAmberBlue', { colors: ['Blue', 'Blue'], wildPrimaries: 3 }), 7)).toBe(false);
  });

  it('takes a boat from a shipwright on top of its textiles', () => {
    const colors: Color[] = ['Purple', 'Green'];
    expect(canSell(stateWith('ShipwrightPurpleGreen', { colors, materials: ['Textiles'] }), 7)).toBe(false);
    expect(canSell(stateWith('ShipwrightPurpleGreen', { colors, materials: ['Textiles'], boats: 1 }), 7)).toBe(true);
    expect(canSell(stateWith('ShipwrightPurpleGreen', { colors, boats: 1 }), 7)).toBe(false);
    expect(canSell(stateWith('ShipwrightPurpleGreen', { colors, boats: 2 }), 7)).toBe(true);
  });

  it('counts both of a tilemaker\'s ceramics', () => {
    expect(canSell(stateWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics'] }), 7)).toBe(false);
    expect(canSell(stateWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics', 'Ceramics'] }), 7)).toBe(true);
    expect(canSell(stateWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics'], boats: 1 }), 7)).toBe(true);
  });
});
