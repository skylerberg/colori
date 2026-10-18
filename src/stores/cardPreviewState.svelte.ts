let _previewCard = $state<string | null>(null);
let _previewArtUrl = $state<string | null>(null);
let _previewLabel = $state<string>('');
// A card the mouse rests on. Cards tucked under a board show only half their
// face, so the whole card floats beside it.
let _hover = $state<{ artUrl: string; label: string; anchor: DOMRect } | null>(null);

export const cardPreviewState = {
  get card() { return _previewCard; },
  get artUrl() { return _previewArtUrl; },
  get label() { return _previewLabel; },
  get hover() { return _hover; },
  open(card: string, artUrl: string, label: string) {
    _hover = null;
    _previewCard = card;
    _previewArtUrl = artUrl;
    _previewLabel = label;
  },
  close() {
    _previewCard = null;
    _previewArtUrl = null;
    _previewLabel = '';
  },
  showHover(artUrl: string, label: string, anchor: DOMRect) {
    _hover = { artUrl, label, anchor };
  },
  hideHover() {
    _hover = null;
  },
};
