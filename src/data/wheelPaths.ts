import type { Color } from './types';

export interface WheelRegion {
  color: Color;
  path: string; // SVG path d attribute in 0-100 viewBox space
  center: [number, number]; // [x, y] center of the region in 0-100 space
}

// Convert angle (degrees clockwise from north) + radius to (x, y) in 0-100 space
function polar(angleDeg: number, r: number): [number, number] {
  const rad = (angleDeg * Math.PI) / 180;
  return [50 + r * Math.sin(rad), 50 - r * Math.cos(rad)];
}

function fmt(n: number): string {
  return n.toFixed(2);
}

function moveTo(angleDeg: number, r: number): string {
  const [x, y] = polar(angleDeg, r);
  return `M ${fmt(x)} ${fmt(y)}`;
}

function lineTo(angleDeg: number, r: number): string {
  const [x, y] = polar(angleDeg, r);
  return `L ${fmt(x)} ${fmt(y)}`;
}

// Circular arc (using the positional radius as SVG arc radius)
function arcTo(angleDeg: number, r: number, largeArc: number, sweep: number): string {
  const [x, y] = polar(angleDeg, r);
  return `A ${fmt(r)} ${fmt(r)} 0 ${largeArc} ${sweep} ${fmt(x)} ${fmt(y)}`;
}

// Lobe arc to a point (uses R_LOBE_ARC as the SVG arc radius, not the positional radius)
function lobeArcTo(angleDeg: number, r: number): string {
  const [x, y] = polar(angleDeg, r);
  return `A ${fmt(R_LOBE_ARC)} ${fmt(R_LOBE_ARC)} 0 0 1 ${fmt(x)} ${fmt(y)}`;
}

function segCenter(angleDeg: number, r: number): [number, number] {
  return polar(angleDeg, r);
}

// ── Ring radii ──
const R_HUB = 5;
const R_INNER = 16;
const R_MIDDLE = 31;
const R_OUTER = 46;
const R_LOBE_ARC = 10; // SVG arc radius for secondary semicircular lobes

// Compute the radius of the "triple point" where two primaries and one secondary meet.
// This is where the lobe arc intersects the radial at the secondary's center angle.
function computeTripleRadius(): number {
  const [x1, y1] = polar(30, R_INNER);  // Orange lobe endpoint 1
  const [x2, y2] = polar(90, R_INNER);  // Orange lobe endpoint 2
  const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
  const dx = x2 - x1, dy = y2 - y1;
  const chord = Math.sqrt(dx * dx + dy * dy);
  const h = Math.sqrt(R_LOBE_ARC * R_LOBE_ARC - (chord * chord) / 4);
  // Two possible arc centers; pick the one farther from wheel center (50,50)
  const px = -dy / chord, py = dx / chord;
  const c1x = mx + h * px, c1y = my + h * py;
  const c2x = mx - h * px, c2y = my - h * py;
  const d1 = (c1x - 50) ** 2 + (c1y - 50) ** 2;
  const d2 = (c2x - 50) ** 2 + (c2y - 50) ** 2;
  const [cx, cy] = d1 > d2 ? [c1x, c1y] : [c2x, c2y];
  // Deepest point on lobe: from arc center toward wheel center
  const dirX = 50 - cx, dirY = 50 - cy;
  const dirLen = Math.sqrt(dirX * dirX + dirY * dirY);
  const deepX = cx + R_LOBE_ARC * dirX / dirLen;
  const deepY = cy + R_LOBE_ARC * dirY / dirLen;
  return Math.sqrt((deepX - 50) ** 2 + (deepY - 50) ** 2);
}

const R_TRIPLE = computeTripleRadius();

// ── Path builders ──

// Primary: stepped shape with lobe concavities carved out
// 120° hub wedge + 60° middle extension, with curved boundaries matching secondary lobes
function primaryRegion(
  innerStart: number, // start of 120° inner sector
  innerEnd: number,   // end of 120° inner sector
  extStart: number,   // start of 60° middle extension
  extEnd: number,     // end of 60° middle extension
): string {
  return [
    moveTo(innerStart, R_HUB),
    arcTo(innerEnd, R_HUB, 0, 1),        // Hub arc CW 120°
    lineTo(innerEnd, R_TRIPLE),           // Radial out to triple point
    lobeArcTo(extEnd, R_INNER),           // Lobe sub-arc (shared boundary with secondary)
    lineTo(extEnd, R_MIDDLE),             // Radial out to middle ring
    arcTo(extStart, R_MIDDLE, 0, 0),      // Middle ring arc CCW 60°
    lineTo(extStart, R_INNER),            // Radial in to inner boundary
    lobeArcTo(innerStart, R_TRIPLE),      // Lobe sub-arc (shared boundary with other secondary)
    'Z',
  ].join(' ');
}

// Secondary: semicircular lobe shape in the middle ring
// Outer boundary follows R_MIDDLE arc, inner boundary is a semicircular lobe curving toward center
function secondaryRegion(startAngle: number, endAngle: number): string {
  return [
    moveTo(startAngle, R_INNER),
    lineTo(startAngle, R_MIDDLE),         // Radial out
    arcTo(endAngle, R_MIDDLE, 0, 1),      // Outer arc CW 60°
    lineTo(endAngle, R_INNER),            // Radial in
    lobeArcTo(startAngle, R_INNER),       // Lobe arc curving toward center
    'Z',
  ].join(' ');
}

// Simple ring segment for tertiaries
function ringSegment(startAngle: number, endAngle: number, rIn: number, rOut: number): string {
  return [
    moveTo(startAngle, rIn),
    lineTo(startAngle, rOut),
    arcTo(endAngle, rOut, 0, 1),
    lineTo(endAngle, rIn),
    arcTo(startAngle, rIn, 0, 0),
    'Z',
  ].join(' ');
}

export const WORKSHOP_WHEEL_REGIONS: WheelRegion[] = [
  // ── Primaries: stepped regions with lobe concavities ──
  {
    color: 'Red',
    path: primaryRegion(300, 60, 330, 30),
    center: segCenter(0, (R_HUB + R_INNER) / 2),
  },
  {
    color: 'Yellow',
    path: primaryRegion(60, 180, 90, 150),
    center: segCenter(120, (R_HUB + R_INNER) / 2),
  },
  {
    color: 'Blue',
    path: primaryRegion(180, 300, 210, 270),
    center: segCenter(240, (R_HUB + R_INNER) / 2),
  },

  // ── Secondaries: semicircular lobe shapes in middle ring ──
  {
    color: 'Orange',
    path: secondaryRegion(30, 90),
    center: segCenter(60, (R_TRIPLE + R_MIDDLE) / 2),
  },
  {
    color: 'Green',
    path: secondaryRegion(150, 210),
    center: segCenter(180, (R_TRIPLE + R_MIDDLE) / 2),
  },
  {
    color: 'Purple',
    path: secondaryRegion(270, 330),
    center: segCenter(300, (R_TRIPLE + R_MIDDLE) / 2),
  },

  // ── Tertiaries: ring segments in outer ring ──
  {
    color: 'Vermilion',
    path: ringSegment(0, 60, R_MIDDLE, R_OUTER),
    center: segCenter(30, (R_MIDDLE + R_OUTER) / 2),
  },
  {
    color: 'Amber',
    path: ringSegment(60, 120, R_MIDDLE, R_OUTER),
    center: segCenter(90, (R_MIDDLE + R_OUTER) / 2),
  },
  {
    color: 'Chartreuse',
    path: ringSegment(120, 180, R_MIDDLE, R_OUTER),
    center: segCenter(150, (R_MIDDLE + R_OUTER) / 2),
  },
  {
    color: 'Teal',
    path: ringSegment(180, 240, R_MIDDLE, R_OUTER),
    center: segCenter(210, (R_MIDDLE + R_OUTER) / 2),
  },
  {
    color: 'Indigo',
    path: ringSegment(240, 300, R_MIDDLE, R_OUTER),
    center: segCenter(270, (R_MIDDLE + R_OUTER) / 2),
  },
  {
    color: 'Magenta',
    path: ringSegment(300, 360, R_MIDDLE, R_OUTER),
    center: segCenter(330, (R_MIDDLE + R_OUTER) / 2),
  },
];
