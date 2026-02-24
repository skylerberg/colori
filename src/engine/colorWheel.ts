import type { Color, MaterialType } from '../data/types';
import { ALL_COLORS, canMix, mixResult } from '../data/colors';

export function createEmptyWheel(): Record<Color, number> {
  const wheel: Partial<Record<Color, number>> = {};
  for (const c of ALL_COLORS) wheel[c] = 0;
  return wheel as Record<Color, number>;
}

export function createStartingMaterials(): Record<MaterialType, number> {
  return { Textiles: 0, Ceramics: 0, Paintings: 0 };
}

export function storeColor(wheel: Record<Color, number>, color: Color): void {
  wheel[color]++;
}

export function removeColor(wheel: Record<Color, number>, color: Color): boolean {
  if (wheel[color] <= 0) return false;
  wheel[color]--;
  return true;
}

export function performMix(wheel: Record<Color, number>, a: Color, b: Color): boolean {
  if (!canMix(a, b)) return false;
  if (wheel[a] <= 0 || wheel[b] <= 0) return false;
  wheel[a]--;
  wheel[b]--;
  const result = mixResult(a, b);
  wheel[result]++;
  return true;
}

export function canPayCost(wheel: Record<Color, number>, cost: Color[]): boolean {
  const temp = { ...wheel };
  for (const c of cost) {
    if (temp[c] <= 0) return false;
    temp[c]--;
  }
  return true;
}

export function payCost(wheel: Record<Color, number>, cost: Color[]): boolean {
  if (!canPayCost(wheel, cost)) return false;
  for (const c of cost) wheel[c]--;
  return true;
}
