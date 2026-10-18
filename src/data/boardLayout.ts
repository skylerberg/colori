import type { Color, MaterialType } from './types';

// Where things sit on the printed boards, in pixels of their 2000px-wide art
// (public/boards). The player boards are 342.9 x 190.5 mm and the Buyer's
// Board 304.8 x 203.2 mm, so card sizes follow from the printed card stock:
// bridge cards (57 x 87 mm) for the draft deck, mini cards (44 x 67 mm) for
// sell cards.

export interface Rect { x: number; y: number; width: number; height: number }
export interface Point { x: number; y: number }

export const BOARD_ART_WIDTH = 2000;

const PLAYER_PX_PER_MM = BOARD_ART_WIDTH / 342.9;
const BUYERS_PX_PER_MM = BOARD_ART_WIDTH / 304.8;

export const PLAYER_BOARD_HEIGHT = 1111;
export const PLAYER_CARD = { width: 57 * PLAYER_PX_PER_MM, height: 87 * PLAYER_PX_PER_MM };
export const PLAYER_SELL_CARD = { width: 44 * PLAYER_PX_PER_MM, height: 67 * PLAYER_PX_PER_MM };
// Drafted cards tuck halfway under the top edge, workshop cards under the bottom.
export const TUCK = PLAYER_CARD.height / 2;

export const DRAFT_SLOT_X = [273, 650, 1027, 1405];
export const WORKSHOP_SLOT_X = [135, 479, 823, 1166, 1510];

// Slot 1 is always open; the others open as the rounds go on.
export const BUYER_SLOTS: Point[] = [
  { x: 1404, y: 102 },
  { x: 1698, y: 102 },
  { x: 1404, y: 528 },
];
export const COMPLETED_SLOT: Point = { x: 1698, y: 528 };
// The band across a closed slot that names the round it opens.
export const BUYER_SLOT_LABEL_Y = 230;
export const BUYER_SLOT_LABEL_HEIGHT = 100;

export const DECK: Rect = { x: 52, y: 528, width: PLAYER_CARD.width, height: PLAYER_CARD.height };
export const DUCAT_BAG: Point = { x: 1275, y: 965 };
// Counters beside the printed gondola, wild primary bowl and material icons.
export const BOATS: Point = { x: 292, y: 296 };
export const WILD_PRIMARIES: Point = { x: 958, y: 537 };
export const MATERIALS: Record<MaterialType, Point> = {
  Paintings: { x: 132, y: 292 },
  Ceramics: { x: 416, y: 289 },
  Textiles: { x: 269, y: 454 },
};
// The reminder strip: a ducat buys a workshop pick, a mix or a sale.
export const DUCAT_PURCHASE_ICONS = {
  workshop: { x: 1560, y: 996 },
  mixColors: { x: 1640, y: 996 },
  sell: { x: 1718, y: 996 },
} as const;
export const DUCAT_STRIP: Rect = { x: 1405, y: 958, width: 362, height: 76 };

// Where each character's face is, for cropping a portrait out of the board.
export const PORTRAIT_FACES: Point[] = [
  { x: 481, y: 800 },
  { x: 427, y: 850 },
  { x: 478, y: 785 },
  { x: 491, y: 830 },
  { x: 469, y: 795 },
  { x: 476, y: 800 },
];
export const PORTRAIT_SIZE = 210;

// ── The color wheel ──
// Twelve regions: tertiaries around the outside, and inside the rim three
// primaries meeting at the hub with a secondary petal between each pair. A
// petal is a circle clipped by the rim; spokes run from the hub to each
// petal's tip. Angles are degrees clockwise from straight up.

export const WHEEL = { cx: 910.5, cy: 538, outer: 414, rim: 275, petalCenter: 262, petalRadius: 143 };
const PETAL_TIP = WHEEL.petalCenter - WHEEL.petalRadius;
// Half the angle a petal's mouth spans along the rim.
const PETAL_HALF_ANGLE = Math.acos(
  (WHEEL.rim ** 2 + WHEEL.petalCenter ** 2 - WHEEL.petalRadius ** 2) / (2 * WHEEL.rim * WHEEL.petalCenter),
) * 180 / Math.PI;

const TERTIARY_START: Partial<Record<Color, number>> = {
  Vermilion: 0, Amber: 60, Chartreuse: 120, Teal: 180, Ultramarine: 240, Fuchsia: 300,
};
const SECONDARY_ANGLE: Partial<Record<Color, number>> = { Orange: 60, Green: 180, Purple: 300 };
const PRIMARY_ANGLE: Partial<Record<Color, number>> = { Red: 0, Yellow: 120, Blue: 240 };

function polar(r: number, deg: number): string {
  const rad = deg * Math.PI / 180;
  return `${(WHEEL.cx + r * Math.sin(rad)).toFixed(1)} ${(WHEEL.cy - r * Math.cos(rad)).toFixed(1)}`;
}

function regionPath(color: Color): string {
  const { outer, rim, petalRadius: p } = WHEEL;
  const a = PETAL_HALF_ANGLE;
  const tertiary = TERTIARY_START[color];
  if (tertiary !== undefined) {
    return `M ${polar(outer, tertiary)} A ${outer} ${outer} 0 0 1 ${polar(outer, tertiary + 60)} `
      + `L ${polar(rim, tertiary + 60)} A ${rim} ${rim} 0 0 0 ${polar(rim, tertiary)} Z`;
  }
  const secondary = SECONDARY_ANGLE[color];
  if (secondary !== undefined) {
    return `M ${polar(rim, secondary + a)} A ${p} ${p} 0 0 1 ${polar(rim, secondary - a)} `
      + `A ${rim} ${rim} 0 0 1 ${polar(rim, secondary + a)} Z`;
  }
  const primary = PRIMARY_ANGLE[color]!;
  const next = primary + 60;
  const previous = primary - 60;
  return `M ${polar(0, 0)} L ${polar(PETAL_TIP, next)} A ${p} ${p} 0 0 1 ${polar(rim, next - a)} `
    + `A ${rim} ${rim} 0 0 0 ${polar(rim, previous + a)} A ${p} ${p} 0 0 1 ${polar(PETAL_TIP, previous)} Z`;
}

export interface WheelRegion {
  color: Color;
  path: string;
  // Where its count sits: beside the printed bowl, shell or bottle, clear of
  // the color's name.
  counter: Point;
}

const WHEEL_COUNTERS: Record<Color, Point> = {
  Red: { x: 952, y: 404 },
  Orange: { x: 1118, y: 469 },
  Yellow: { x: 1092, y: 649 },
  Green: { x: 948, y: 774 },
  Blue: { x: 800, y: 649 },
  Purple: { x: 768, y: 469 },
  Vermilion: { x: 948, y: 222 },
  Amber: { x: 1212, y: 500 },
  Chartreuse: { x: 948, y: 905 },
  Teal: { x: 640, y: 740 },
  Ultramarine: { x: 548, y: 708 },
  Fuchsia: { x: 632, y: 335 },
};

export const WHEEL_REGIONS: WheelRegion[] = (Object.keys(WHEEL_COUNTERS) as Color[]).map(color => ({
  color,
  path: regionPath(color),
  counter: WHEEL_COUNTERS[color],
}));

// ── The Buyer's Board ──

export const BUYERS_BOARD_HEIGHT = 1333;
const BUYERS_SELL_CARD = { width: 44 * BUYERS_PX_PER_MM, height: 67 * BUYERS_PX_PER_MM };

export const BUYERS_BOARD = {
  draftDeck: { x: 37, y: 78, width: 370, height: 575 } as Rect,
  trash: { x: 33, y: 729, width: 374, height: 577 } as Rect,
  sellDeck: { x: 636, y: 296, ...BUYERS_SELL_CARD } as Rect,
  display: [
    { x: 982, y: 296 }, { x: 1329, y: 296 }, { x: 1676, y: 296 },
    { x: 633, y: 858 }, { x: 980, y: 858 }, { x: 1327, y: 858 }, { x: 1675, y: 858 },
  ].map(p => ({ ...p, ...BUYERS_SELL_CARD })) as Rect[],
  // One circle per round along the top: the round track.
  rounds: [560, 780, 997, 1215, 1435, 1650, 1870].map(x => ({ x, y: 115 })) as Point[],
  roundRadius: 92,
};

// Keep each card in the slot it landed in, so one leaving does not shuffle
// the rest along. New cards take the first free slots, in the order given.
export function assignSlots(previous: (number | null)[], ids: number[], slotCount: number): (number | null)[] {
  const present = new Set(ids);
  const slots: (number | null)[] = Array.from({ length: slotCount }, (_, i) => {
    const id = previous[i] ?? null;
    return id !== null && present.has(id) ? id : null;
  });
  const placed = new Set(slots);
  for (const id of ids) {
    if (placed.has(id)) continue;
    const free = slots.indexOf(null);
    if (free >= 0) slots[free] = id;
    else slots.push(id);
  }
  return slots;
}
