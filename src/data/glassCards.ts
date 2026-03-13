import type { GlassCard } from './types';

export interface GlassCardData {
  name: string;
  description: string;
}

const DATA: Record<GlassCard, GlassCardData> = {
  GlassWorkshop: { name: 'Glass Workshop', description: 'Workshop 1 card' },
  GlassDraw: { name: 'Glass Draw', description: 'Draw 1 card' },
  GlassMix: { name: 'Glass Mix', description: 'Mix 1 pair of colors' },
  GlassExchange: { name: 'Glass Exchange', description: 'Exchange 1 material for another' },
  GlassMoveDrafted: { name: 'Glass Move', description: 'Move a drafted card to workshop' },
  GlassUnmix: { name: 'Glass Unmix', description: 'Unmix a non-primary color' },
  GlassTertiaryDucat: { name: 'Glass Ducat', description: 'Convert a tertiary color to 1 ducat' },
  GlassReworkshop: { name: 'Glass Reworkshop', description: 'Un-rotate a workshopped card' },
  GlassGainPrimary: { name: 'Glass Primary', description: 'Gain 1 primary color' },
  GlassDestroyClean: { name: 'Glass Destroy', description: 'Destroy a workshop card' },
  GlassKeepBoth: { name: 'Glass Keep Both', description: 'Keep all remaining draft cards (passive)' },
};

export const GLASS_CARD_ORDER: GlassCard[] = [
  'GlassWorkshop', 'GlassDraw', 'GlassMix', 'GlassExchange', 'GlassMoveDrafted',
  'GlassUnmix', 'GlassTertiaryDucat', 'GlassReworkshop', 'GlassGainPrimary',
  'GlassDestroyClean', 'GlassKeepBoth',
];

export function getGlassCardData(glass: GlassCard): GlassCardData {
  return DATA[glass];
}
