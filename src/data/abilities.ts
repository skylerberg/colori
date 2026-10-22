import type { Ability, AnyCardData } from './types';

export function formatSingleAbility(a: Ability): string {
  switch (a.type) {
    case 'workshop': return `Workshop ×${a.count}`;
    case 'mixColors': return `Mix ×${a.count}`;
    case 'liftCard': return 'Lift a card';
    case 'sell': return 'Sell to a buyer';
    case 'gainGoldCoins': return `Gain ${a.count} gold ${a.count === 1 ? 'coin' : 'coins'}`;
    case 'gainSecondary': return 'Any secondary';
    case 'gainBowls': return `Gain ${a.count} ${a.count === 1 ? 'bowl' : 'bowls'}`;
    case 'gainBoats': return `Gain ${a.count} ${a.count === 1 ? 'boat' : 'boats'}`;
  }
}

export function formatAbility(c: AnyCardData): string {
  if (c.kind === 'buyer') return '';
  return c.topActions.map(formatSingleAbility).join(', then ');
}
