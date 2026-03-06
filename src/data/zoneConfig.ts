export interface ZoneConfig {
  id: string;
  label: string;
  x: number;      // percentage from left (-50 to 150 to allow overhang)
  y: number;      // percentage from top (-50 to 150 to allow overhang)
  width: number;   // percentage width
  height: number;  // percentage height
}

// Zones within the player's workshop tableau area
// Negative y = hanging above the tableau, y > 100 = hanging below
export const DEFAULT_ZONES: ZoneConfig[] = [
  { id: 'colorWheel', label: 'Color Wheel', x: 27.2, y: 14.8, width: 41.2, height: 67.7 },
  { id: 'buyers', label: 'Completed Buyers', x: 2.5, y: 23.1, width: 20.4, height: 48.6 },
  { id: 'draft1', label: 'Draft Slot 1', x: 1.8, y: -41.4, width: 21.5, height: 48.3 },
  { id: 'draft2', label: 'Draft Slot 2', x: 26.4, y: -41.2, width: 22.1, height: 48.1 },
  { id: 'draft3', label: 'Draft Slot 3', x: 51.4, y: -41.2, width: 22, height: 48.1 },
  { id: 'draft4', label: 'Draft Slot 4', x: 76.4, y: -41.6, width: 21.9, height: 48.5 },
  { id: 'materialTextiles', label: 'Textiles', x: 71.4, y: 26.5, width: 8.3, height: 43.9 },
  { id: 'materialCeramics', label: 'Ceramics', x: 81.1, y: 26.7, width: 7.5, height: 43.5 },
  { id: 'materialPaintings', label: 'Paintings', x: 89.8, y: 26.5, width: 8.3, height: 43.9 },
  { id: 'workshop', label: 'Workshop Cards', x: 1.6, y: 89.2, width: 96.4, height: 27.6 },
];

export function getZone(zones: ZoneConfig[], id: string): ZoneConfig | undefined {
  return zones.find(z => z.id === id);
}

export function loadZones(): ZoneConfig[] {
  try {
    const saved = localStorage.getItem('colori-zone-config');
    if (saved) return JSON.parse(saved);
  } catch {}
  return DEFAULT_ZONES;
}

export function saveZones(zones: ZoneConfig[]): void {
  localStorage.setItem('colori-zone-config', JSON.stringify(zones));
}
