import type { Color } from './types';

export const WHEEL_ORDER: Color[] = [
  'Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse',
  'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta'
];

export const ALL_COLORS = WHEEL_ORDER;

export function colorIndex(color: Color): number {
  return WHEEL_ORDER.indexOf(color);
}

const PRIMARY_INDICES = new Set([0, 4, 8]);    // Red, Yellow, Blue
const SECONDARY_INDICES = new Set([2, 6, 10]); // Orange, Green, Purple

// Can two colors be mixed?
// Valid mixes: primary+primary, or primary+secondary that are 2 apart on the wheel.
export function canMix(a: Color, b: Color): boolean {
  const ai = colorIndex(a);
  const bi = colorIndex(b);
  const diff = Math.abs(ai - bi);
  const distance = Math.min(diff, WHEEL_ORDER.length - diff);

  if (PRIMARY_INDICES.has(ai) && PRIMARY_INDICES.has(bi)) {
    return ai !== bi;
  }

  const oneIsPrimary = PRIMARY_INDICES.has(ai) || PRIMARY_INDICES.has(bi);
  const oneIsSecondary = SECONDARY_INDICES.has(ai) || SECONDARY_INDICES.has(bi);
  return oneIsPrimary && oneIsSecondary && distance === 2;
}

// Returns the color produced by mixing a and b.
// Throws if they can't be mixed.
export function mixResult(a: Color, b: Color): Color {
  if (!canMix(a, b)) throw new Error(`Cannot mix ${a} and ${b}`);
  const ai = colorIndex(a);
  const bi = colorIndex(b);
  const n = WHEEL_ORDER.length;
  // Find midpoint along the shorter arc
  const forwardDist = (bi - ai + n) % n;
  if (forwardDist <= n / 2) {
    return WHEEL_ORDER[(ai + forwardDist / 2) % n];
  } else {
    const backDist = n - forwardDist;
    return WHEEL_ORDER[(ai - backDist / 2 + n) % n];
  }
}

// CSS color for display
export function colorToHex(color: Color): string {
  const map: Record<Color, string> = {
    'Red': '#e63946',
    'Vermilion': '#e76f51',
    'Orange': '#f4a261',
    'Amber': '#e9c46a',
    'Yellow': '#f2e205',
    'Chartreuse': '#a7c957',
    'Green': '#2d6a4f',
    'Teal': '#219ebc',
    'Blue': '#264653',
    'Indigo': '#3a0ca3',
    'Purple': '#7209b7',
    'Magenta': '#d63384',
  };
  return map[color];
}

export function blendColors(colors: Color[]): string {
  if (colors.length === 0) return '#000000';
  const hexes = colors.map(c => colorToHex(c));
  let r = 0, g = 0, b = 0;
  for (const hex of hexes) {
    r += parseInt(hex.slice(1, 3), 16);
    g += parseInt(hex.slice(3, 5), 16);
    b += parseInt(hex.slice(5, 7), 16);
  }
  const n = hexes.length;
  r = Math.round(r / n);
  g = Math.round(g / n);
  b = Math.round(b / n);
  return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
}

export function textColorForBackground(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const yiq = (r * 299 + g * 587 + b * 114) / 1000;
  return yiq >= 128 ? '#000000' : '#ffffff';
}
