// The action panel's conveniences, which say what a click does rather than
// what the game allows: what to do with cards a turn leaves undecided when it
// ends, and whether the last workshop pick confirms the workshop on its own.
// Both start on, and both are remembered between turns and games.

const KEY = 'colori-action-prefs';

interface ActionPrefs {
  keepRemaining: boolean;
  autoWorkshop: boolean;
}

function load(): ActionPrefs {
  const defaults: ActionPrefs = { keepRemaining: true, autoWorkshop: true };
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return defaults;
    const saved = JSON.parse(raw) as Partial<ActionPrefs>;
    return {
      keepRemaining: typeof saved.keepRemaining === 'boolean' ? saved.keepRemaining : true,
      autoWorkshop: typeof saved.autoWorkshop === 'boolean' ? saved.autoWorkshop : true,
    };
  } catch {
    // No storage, or something else in its place: the defaults still work.
    return defaults;
  }
}

const saved = load();
let _keepRemaining = $state(saved.keepRemaining);
let _autoWorkshop = $state(saved.autoWorkshop);

function save(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ keepRemaining: _keepRemaining, autoWorkshop: _autoWorkshop }));
  } catch {
    // Storage unavailable — the choices last for this page only.
  }
}

export const actionPrefs = {
  get keepRemaining() { return _keepRemaining; },
  get autoWorkshop() { return _autoWorkshop; },
  setKeepRemaining(value: boolean) {
    _keepRemaining = value;
    save();
  },
  setAutoWorkshop(value: boolean) {
    _autoWorkshop = value;
    save();
  },
};
