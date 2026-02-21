import type { Color } from './types';

export const WHEEL_ORDER: Color[] = [
  'Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse',
  'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta'
];

export const ALL_COLORS = WHEEL_ORDER;

export function colorIndex(color: Color): number {
  return WHEEL_ORDER.indexOf(color);
}

// Can two colors be mixed? They must be exactly 2 positions apart on the wheel (wrapping).
export function canMix(a: Color, b: Color): boolean {
  const diff = Math.abs(colorIndex(a) - colorIndex(b));
  return diff === 2 || diff === WHEEL_ORDER.length - 2;
}

// Returns the color produced by mixing a and b (the one between them).
// Throws if they can't be mixed.
export function mixResult(a: Color, b: Color): Color {
  if (!canMix(a, b)) throw new Error(`Cannot mix ${a} and ${b}`);
  const ai = colorIndex(a);
  const bi = colorIndex(b);
  const diff = bi - ai;
  // Handle wrapping
  if (diff === 2 || diff === -(WHEEL_ORDER.length - 2)) {
    return WHEEL_ORDER[(ai + 1) % WHEEL_ORDER.length];
  } else {
    return WHEEL_ORDER[(bi + 1) % WHEEL_ORDER.length];
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
