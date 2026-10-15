import type { Color } from '../data/types';

let _simulatedWheel = $state<Record<Color, number> | null>(null);
let _simulatedWildPrimaries = $state(0);
let _selectedColors = $state<Color[]>([]);
let _onColorClick = $state<((color: Color) => void) | null>(null);

export const mixWheelState = {
  get simulatedWheel() { return _simulatedWheel; },
  set simulatedWheel(v: Record<Color, number> | null) { _simulatedWheel = v; },
  // Wild primaries left after the mixes planned so far: how many empty primaries can
  // still be picked.
  get simulatedWildPrimaries() { return _simulatedWildPrimaries; },
  set simulatedWildPrimaries(v: number) { _simulatedWildPrimaries = v; },
  get selectedColors() { return _selectedColors; },
  set selectedColors(v: Color[]) { _selectedColors = v; },
  get onColorClick() { return _onColorClick; },
  set onColorClick(v: ((color: Color) => void) | null) { _onColorClick = v; },
  clear() {
    _simulatedWheel = null;
    _simulatedWildPrimaries = 0;
    _selectedColors = [];
    _onColorClick = null;
  }
};
