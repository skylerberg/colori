import { getContext, setContext } from 'svelte';
import type { Color, DucatPurchase } from '../data/types';

// What can be done by clicking the boards right now. The game layout owns it;
// the phase view in the controls panel fills it in for the seat it plays, and
// clears its part when it goes away.
export interface TableInteraction {
  // Whose player board the board handlers act on, or -1.
  seat: number;
  destroyDrafted: ((instanceId: number) => void) | null;
  workshop: {
    selectable: number[];
    selected: number[];
    onClick: (instanceId: number) => void;
  } | null;
  buyers: {
    selectable: number[];
    onClick: (instanceId: number) => void;
  } | null;
  wheel: {
    // The wheel as the moves planned so far would leave it.
    wheel: Record<Color, number>;
    wildPrimaries: number;
    pickable: Color[];
    selected: Color[];
    // What picking each pickable color would make, when that is a mix.
    mixesInto: Partial<Record<Color, Color>>;
    onClick: (color: Color) => void;
  } | null;
  ducats: {
    available: DucatPurchase[];
    onSpend: (purchase: DucatPurchase) => void;
  } | null;
  market: {
    onClaim: (instanceId: number) => void;
    onDrawUnseen: (() => void) | null;
  } | null;
}

const KEY = Symbol('tableInteraction');

export function emptyTableInteraction(): TableInteraction {
  return { seat: -1, destroyDrafted: null, workshop: null, buyers: null, wheel: null, ducats: null, market: null };
}

export function provideTableInteraction(interaction: TableInteraction): void {
  setContext(KEY, interaction);
}

export function tableInteraction(): TableInteraction {
  return getContext<TableInteraction>(KEY);
}
