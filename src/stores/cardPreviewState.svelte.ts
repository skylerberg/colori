let _previewCard = $state<string | null>(null);
let _previewArtUrl = $state<string | null>(null);
let _previewLabel = $state<string>('');

export const cardPreviewState = {
  get card() { return _previewCard; },
  get artUrl() { return _previewArtUrl; },
  get label() { return _previewLabel; },
  open(card: string, artUrl: string, label: string) {
    _previewCard = card;
    _previewArtUrl = artUrl;
    _previewLabel = label;
  },
  close() {
    _previewCard = null;
    _previewArtUrl = null;
    _previewLabel = '';
  }
};
