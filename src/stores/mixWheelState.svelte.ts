import type { Color } from '../data/types';

let _simulatedWheel = $state<Record<Color, number> | null>(null);
let _simulatedSilver = $state(0);
let _selectedColors = $state<Color[]>([]);
let _onColorClick = $state<((color: Color) => void) | null>(null);

export const mixWheelState = {
  get simulatedWheel() { return _simulatedWheel; },
  set simulatedWheel(v: Record<Color, number> | null) { _simulatedWheel = v; },
  // Silver left after the mixes planned so far: how many empty primaries can
  // still be picked.
  get simulatedSilver() { return _simulatedSilver; },
  set simulatedSilver(v: number) { _simulatedSilver = v; },
  get selectedColors() { return _selectedColors; },
  set selectedColors(v: Color[]) { _selectedColors = v; },
  get onColorClick() { return _onColorClick; },
  set onColorClick(v: ((color: Color) => void) | null) { _onColorClick = v; },
  clear() {
    _simulatedWheel = null;
    _simulatedSilver = 0;
    _selectedColors = [];
    _onColorClick = null;
  }
};
