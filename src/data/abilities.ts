import type { Ability, AnyCardData } from './types';

export function formatSingleAbility(a: Ability): string {
  switch (a.type) {
    case 'workshop': return `Workshop x${a.count}`;
    case 'drawCards': return `Draw x${a.count}`;
    case 'mixColors': return `Mix x${a.count}`;
    case 'destroyCards': return 'Move a card from workshop to draft pool';
    case 'sell': return 'Sell';
    case 'gainDucats': return `Gain ${a.count} Ducat(s)`;
    case 'gainSecondary': return 'Any Secondary';
    case 'gainPrimary': return 'Any Primary';
    case 'changeTertiary': return 'Change Tertiary';
    case 'moveToDrafted': return 'Move to Drafted';
    case 'moveToWorkshop': return 'Move to Workshop';
  }
}

export function formatAbility(c: AnyCardData): string {
  if (c.kind === 'sellCard') return '';
  return formatSingleAbility(c.ability);
}
