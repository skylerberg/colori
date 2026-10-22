import { describe, expect, it } from 'vitest';
import { canSell, saleShortfall } from './wasmEngine';
import { ALL_COLORS } from '../data/colors';
import type { Color, MaterialType, PlayerState, Buyer } from '../data/types';

// `canSell` restates the engine's affordability rule so the UI can grey out
// buyers without a round trip; these pin it to the cases the Rust tests cover.
function playerWith(
  buyer: Buyer,
  stock: { colors?: Color[]; materials?: MaterialType[]; boats?: number; bowls?: number },
): PlayerState {
  const colorWheel = Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>;
  for (const c of stock.colors ?? []) colorWheel[c]++;
  const materials: Record<MaterialType, number> = { Textiles: 0, Ceramics: 0, Paintings: 0 };
  for (const m of stock.materials ?? []) materials[m]++;
  return {
    buyers: [{ instanceId: 7, card: buyer }],
    colorWheel,
    materialsWheel: materials,
    boats: stock.boats ?? 0,
    bowls: stock.bowls ?? 0,
  } as unknown as PlayerState;
}

describe('canSell', () => {
  it('needs the material and every colour', () => {
    expect(canSell(playerWith('PotterAmber', { colors: ['Amber'], materials: ['Ceramics'] }), 7)).toBe(true);
    expect(canSell(playerWith('PotterAmber', { colors: ['Amber'] }), 7)).toBe(false);
    expect(canSell(playerWith('PotterAmber', { materials: ['Ceramics'] }), 7)).toBe(false);
  });

  it('asks nothing but colours of an illuminator or a glassmaker', () => {
    expect(canSell(playerWith('IlluminatorTeal', { colors: ['Teal'] }), 7)).toBe(true);
    expect(canSell(playerWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue', 'Blue'] }), 7)).toBe(true);
    expect(canSell(playerWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue'] }), 7)).toBe(false);
  });

  it('lets a bowl cover a missing primary but not a tertiary', () => {
    expect(canSell(playerWith('GlassmakerAmberBlue', { colors: ['Amber', 'Blue'], bowls: 1 }), 7)).toBe(true);
    expect(canSell(playerWith('GlassmakerAmberBlue', { colors: ['Blue', 'Blue'], bowls: 3 }), 7)).toBe(false);
  });

  it('takes a boat from a shipwright on top of its textiles', () => {
    const colors: Color[] = ['Purple', 'Green'];
    expect(canSell(playerWith('ShipwrightPurpleGreen', { colors, materials: ['Textiles'] }), 7)).toBe(false);
    expect(canSell(playerWith('ShipwrightPurpleGreen', { colors, materials: ['Textiles'], boats: 1 }), 7)).toBe(true);
    expect(canSell(playerWith('ShipwrightPurpleGreen', { colors, boats: 1 }), 7)).toBe(false);
    expect(canSell(playerWith('ShipwrightPurpleGreen', { colors, boats: 2 }), 7)).toBe(true);
  });

  it('counts both of a tilemaker\'s ceramics', () => {
    expect(canSell(playerWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics'] }), 7)).toBe(false);
    expect(canSell(playerWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics', 'Ceramics'] }), 7)).toBe(true);
    expect(canSell(playerWith('TilemakerTeal', { colors: ['Teal'], materials: ['Ceramics'], boats: 1 }), 7)).toBe(true);
  });
});

describe('saleShortfall', () => {
  it('names each missing material and colour once per unit', () => {
    const player = playerWith('TilemakerTeal', {});
    expect(saleShortfall(player, 'TilemakerTeal')).toEqual(['Ceramics', 'Ceramics', 'Teal']);
  });

  it('spends boats and bowls before calling anything missing', () => {
    const player = playerWith('GlassmakerAmberBlue', { colors: ['Amber'], bowls: 1 });
    expect(saleShortfall(player, 'GlassmakerAmberBlue')).toEqual(['Blue']);
    expect(saleShortfall(playerWith('PotterAmber', { colors: ['Amber'], boats: 1 }), 'PotterAmber')).toEqual([]);
  });

  it('asks a shipwright for its own boat before covering textiles', () => {
    const colors: Color[] = ['Purple', 'Green'];
    expect(saleShortfall(playerWith('ShipwrightPurpleGreen', { colors, boats: 1 }), 'ShipwrightPurpleGreen')).toEqual(['Textiles']);
    expect(saleShortfall(playerWith('ShipwrightPurpleGreen', { colors }), 'ShipwrightPurpleGreen')).toEqual(['Boat', 'Textiles']);
  });
});
