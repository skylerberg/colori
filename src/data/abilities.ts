import type { Ability, AnyCardData } from './types';

export function formatSingleAbility(a: Ability): string {
  switch (a.type) {
    case 'workshop': return `Workshop ×${a.count}`;
    case 'mixColors': return `Mix ×${a.count}`;
    case 'liftCard': return 'Move a card to your top cards';
    case 'sell': return 'Sell';
    case 'gainGoldCoins': return `Gain ${a.count} ${a.count === 1 ? 'goldCoin' : 'goldCoins'}`;
    case 'gainSecondary': return 'Any secondary';
    case 'gainBowls': return `Gain ${a.count} wild ${a.count === 1 ? 'primary' : 'primaries'}`;
    case 'gainPrimaries': return 'Gain one of each primary';
    case 'gainBoats': return `Gain ${a.count} ${a.count === 1 ? 'boat' : 'boats'}`;
  }
}

export function formatAbility(c: AnyCardData): string {
  if (c.kind === 'buyer') return '';
  return c.topActions.map(formatSingleAbility).join(', then ');
}
