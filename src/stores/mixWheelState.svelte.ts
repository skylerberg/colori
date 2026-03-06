import type { Color } from '../data/types';

let _simulatedWheel = $state<Record<Color, number> | null>(null);
let _selectedColors = $state<Color[]>([]);
let _onColorClick = $state<((color: Color) => void) | null>(null);

export const mixWheelState = {
  get simulatedWheel() { return _simulatedWheel; },
  set simulatedWheel(v: Record<Color, number> | null) { _simulatedWheel = v; },
  get selectedColors() { return _selectedColors; },
  set selectedColors(v: Color[]) { _selectedColors = v; },
  get onColorClick() { return _onColorClick; },
  set onColorClick(v: ((color: Color) => void) | null) { _onColorClick = v; },
  clear() {
    _simulatedWheel = null;
    _selectedColors = [];
    _onColorClick = null;
  }
};
