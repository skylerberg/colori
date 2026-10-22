import type { Color, MaterialType } from './types';

// Where things sit on the printed boards, in pixels of their art
// (public/boards), exported 2000px wide. The player boards are 342.9 x 190.5
// mm and the Buyer's Board 304.8 x 203.2 mm, so card sizes follow from the
// printed card stock: bridge cards (57 x 87 mm) for the draft deck, mini cards
// (44 x 67 mm) for sell cards.

export interface Rect { x: number; y: number; width: number; height: number }
export interface Point { x: number; y: number }

export const BOARD_ART_WIDTH = 2000;

const PLAYER_PX_PER_MM = BOARD_ART_WIDTH / 342.9;
const BUYERS_PX_PER_MM = BOARD_ART_WIDTH / 304.8;

export const PLAYER_BOARD_HEIGHT = 1111;
export const PLAYER_CARD = { width: 57 * PLAYER_PX_PER_MM, height: 87 * PLAYER_PX_PER_MM };
export const PLAYER_SELL_CARD = { width: 44 * PLAYER_PX_PER_MM, height: 67 * PLAYER_PX_PER_MM };
// Top cards tuck halfway under the top edge, workshop cards under the bottom.
export const TUCK = PLAYER_CARD.height / 2;

export const DRAFT_SLOT_X = [273, 650, 1027, 1405];
export const WORKSHOP_SLOT_X = [211, 555, 899, 1243, 1587];

// Slot 1 is always open; the others open as the rounds go on. Cards sit
// centred in the printed frames, which are a little larger than a card.
export const BUYER_SLOTS: Point[] = [
  { x: 1387, y: 102 },
  { x: 1695, y: 104 },
  { x: 1387, y: 542 },
];
export const COMPLETED_SLOT: Point = { x: 1698, y: 542 };

export const DECK: Rect = { x: 51, y: 503, width: PLAYER_CARD.width, height: PLAYER_CARD.height };
export const DUCAT_BAG: Point = { x: 1279, y: 973 };
export const MATERIALS_CIRCLE: Rect = { x: 30, y: 90, width: 395, height: 380 };
// Counters beside the printed gondola, wild primary bowl and material icons.
export const BOATS: Point = { x: 286, y: 281 };
export const WILD_PRIMARIES: Point = { x: 949, y: 537 };
export const MATERIALS: Record<MaterialType, Point> = {
  Paintings: { x: 148, y: 280 },
  Ceramics: { x: 395, y: 276 },
  Textiles: { x: 269, y: 415 },
};
// The reminder strip: a goldCoin buys a workshop pick, a mix or a sale.
export const DUCAT_PURCHASE_ICONS = {
  workshop: { x: 1488, y: 996 },
  mixColors: { x: 1567, y: 996 },
  sell: { x: 1637, y: 996 },
} as const;
export const DUCAT_PURCHASE_ICON_RADIUS = 34;
export const DUCAT_STRIP: Rect = { x: 1350, y: 960, width: 348, height: 73 };

// Where each character's face is, for cropping a portrait out of the board.
export const PORTRAIT_FACES: Point[] = [
  { x: 485, y: 790 },
  { x: 433, y: 815 },
  { x: 480, y: 810 },
  { x: 474, y: 805 },
  { x: 480, y: 815 },
  { x: 481, y: 805 },
];
export const PORTRAIT_SIZE = 160;

// ── The color wheel ──
// Twelve regions: tertiaries around the outside, and inside the rim three
// primaries meeting at the hub with a secondary petal between each pair. A
// petal is a circle clipped by the rim; spokes run from the hub to each
// petal's tip. Angles are degrees clockwise from straight up.

export const WHEEL = { cx: 901.5, cy: 538, outer: 414, rim: 275, petalCenter: 262, petalRadius: 143 };
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
  Red: { x: 943, y: 404 },
  Orange: { x: 1109, y: 469 },
  Yellow: { x: 1083, y: 649 },
  Green: { x: 939, y: 774 },
  Blue: { x: 791, y: 649 },
  Purple: { x: 759, y: 469 },
  Vermilion: { x: 939, y: 222 },
  Amber: { x: 1203, y: 500 },
  Chartreuse: { x: 939, y: 905 },
  Teal: { x: 631, y: 740 },
  Ultramarine: { x: 539, y: 708 },
  Fuchsia: { x: 623, y: 335 },
};

export const WHEEL_REGIONS: WheelRegion[] = (Object.keys(WHEEL_COUNTERS) as Color[]).map(color => ({
  color,
  path: regionPath(color),
  counter: WHEEL_COUNTERS[color],
}));

// ── The Buyer's Board ──
// Cropped to the right of the draft deck and trash, which only the physical
// game needs, so the sell cards get the room.

export const BUYERS_BOARD_WIDTH = 1552;
export const BUYERS_BOARD_HEIGHT = 1333;
const BUYERS_SELL_CARD = { width: 44 * BUYERS_PX_PER_MM, height: 67 * BUYERS_PX_PER_MM };

export const BUYERS_BOARD = {
  sellDeck: { x: 46, y: 270, ...BUYERS_SELL_CARD } as Rect,
  display: [
    { x: 439, y: 270 }, { x: 833, y: 270 }, { x: 1228, y: 270 },
    { x: 43, y: 857 }, { x: 437, y: 857 }, { x: 832, y: 856 }, { x: 1228, y: 857 },
  ].map(p => ({ ...p, ...BUYERS_SELL_CARD })) as Rect[],
  // One circle per round along the top: the round track.
  rounds: [112, 332, 549, 767, 987, 1202, 1422].map(x => ({ x, y: 115 })) as Point[],
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
