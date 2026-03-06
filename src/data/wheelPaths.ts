import type { Color } from './types';

export interface WheelRegion {
  color: Color;
  path: string; // SVG path d attribute in 256x256 viewBox space
}

// Hit regions derived from the actual hand-drawn color wheel SVG fill paths.
// These are in the 256x256 viewBox coordinate space (no scaling needed).

export const WORKSHOP_WHEEL_REGIONS: WheelRegion[] = [
  // ── Primaries (inner ring) ──
  {
    color: 'Red',
    path: 'M 92.68 53.92 A 82 82 0 0 1 175.23 61.43 C 167.13 77.91 163.95 92.81 167.29 109.96 L 127.36 129.67 L 92.28 108.76 C 104.97 92.97 103.93 75.85 92.68 53.92 Z',
  },
  {
    color: 'Yellow',
    path: 'M 127.36 129.67 L 167.29 109.96 C 172.12 129.99 186.94 142.29 206.61 147.99 A 82 82 0 0 1 165.72 200.2 C 156.42 185.38 143.41 173.81 127.36 173.81 Z',
  },
  {
    color: 'Blue',
    path: 'M 127.36 129.67 L 127.36 173.81 C 111.47 173.81 98.54 182.04 86.81 198.76 A 82 82 0 0 1 46.15 126.82 C 66.13 124.41 80.07 121.06 92.28 108.76 Z',
  },

  // ── Secondaries (crescent lobes between primaries) ──
  {
    color: 'Orange',
    path: 'M 175.23 61.43 C 167.13 77.91 163.95 92.81 167.29 109.96 C 172.12 129.99 186.94 142.29 206.61 147.99 A 82 82 0 0 0 175.23 61.43 Z',
  },
  {
    color: 'Green',
    path: 'M 86.81 198.76 C 98.54 182.04 111.47 173.81 127.36 173.81 C 143.41 173.81 156.42 185.38 165.72 200.2 A 82 82 0 0 1 86.81 198.76 Z',
  },
  {
    color: 'Purple',
    path: 'M 92.68 53.92 C 103.93 75.85 104.97 92.97 92.28 108.76 C 80.07 121.06 66.13 124.41 46.15 126.82 A 82 82 0 0 1 92.68 53.92 Z',
  },

  // ── Tertiaries (outer ring segments) ──
  {
    color: 'Vermilion',
    path: 'M 127.44 6.74 A 122 122 0 0 1 245.67 97.13 L 204.48 99.46 A 82 82 0 0 0 127.44 46.22 Z',
  },
  {
    color: 'Amber',
    path: 'M 245.67 97.13 A 122 122 0 0 1 222.29 203.86 L 188.73 183.32 A 82 82 0 0 0 204.48 99.46 Z',
  },
  {
    color: 'Chartreuse',
    path: 'M 222.29 203.86 A 122 122 0 0 1 127.44 249.67 L 127.44 210.02 A 82 82 0 0 0 188.73 183.32 Z',
  },
  {
    color: 'Teal',
    path: 'M 127.44 249.67 A 122 122 0 0 1 25.71 195.17 L 58.95 172.61 A 82 82 0 0 0 127.44 210.02 Z',
  },
  {
    color: 'Indigo',
    path: 'M 25.71 195.17 A 122 122 0 0 1 23.62 63.57 L 59.44 82.91 A 82 82 0 0 0 58.95 172.61 Z',
  },
  {
    color: 'Magenta',
    path: 'M 23.62 63.57 A 122 122 0 0 1 127.44 6.74 L 127.44 46.22 A 82 82 0 0 0 59.44 82.91 Z',
  },
];
