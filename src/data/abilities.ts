import type { Ability, AnyCardData } from './types';

export function formatSingleAbility(a: Ability): string {
  switch (a.type) {
    case 'workshop': return `Workshop ×${a.count}`;
    case 'mixColors': return `Mix ×${a.count}`;
    case 'moveToDraftPool': return 'Move a card to your draft pool';
    case 'sell': return 'Sell';
    case 'gainDucats': return `Gain ${a.count} ${a.count === 1 ? 'ducat' : 'ducats'}`;
    case 'gainSecondary': return 'Any secondary';
    case 'gainWildPrimaries': return `Gain ${a.count} wild ${a.count === 1 ? 'primary' : 'primaries'}`;
    case 'gainBoats': return `Gain ${a.count} ${a.count === 1 ? 'boat' : 'boats'}`;
  }
}

export function formatAbility(c: AnyCardData): string {
  if (c.kind === 'sellCard') return '';
  return c.destroyAbilities.map(formatSingleAbility).join(', then ');
}
