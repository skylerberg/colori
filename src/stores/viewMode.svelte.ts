export type ViewMode = '2d' | '3d';

const STORAGE_KEY = 'colori-view-mode';

let viewMode: ViewMode = $state(
  (typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) as ViewMode : null) ?? '2d'
);

export function getViewMode(): ViewMode {
  return viewMode;
}

export function setViewMode(mode: ViewMode) {
  viewMode = mode;
  localStorage.setItem(STORAGE_KEY, mode);
}

export function toggleViewMode() {
  setViewMode(viewMode === '2d' ? '3d' : '2d');
}
