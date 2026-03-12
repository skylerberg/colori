<script lang="ts">
  import { cardPreviewState } from '../stores/cardPreviewState.svelte';

  function close() {
    cardPreviewState.close();
  }
</script>

{#if cardPreviewState.card}
<div class="preview-overlay" onclick={close} onkeydown={(e) => e.key === 'Escape' && close()} role="dialog" tabindex="-1">
  <div class="preview-content">
    <img src={cardPreviewState.artUrl} alt={cardPreviewState.label} class="preview-image" />
    <div class="preview-label">{cardPreviewState.label}</div>
  </div>
  <button class="preview-close" onclick={close}>&times;</button>
</div>
{/if}

<style>
  .preview-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(0, 0, 0, 0.92);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    max-width: 90vw;
    max-height: 90vh;
    cursor: default;
  }

  .preview-image {
    max-width: 85vw;
    max-height: 80vh;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: 0 0 40px rgba(201, 168, 76, 0.3), 0 0 80px rgba(0, 0, 0, 0.5);
  }

  .preview-label {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: clamp(1rem, 3vw, 1.5rem);
    color: var(--accent-gold, #c9a84c);
    text-align: center;
    letter-spacing: 1px;
  }

  .preview-close {
    position: fixed;
    top: 1rem;
    right: 1rem;
    width: 48px;
    height: 48px;
    border: 2px solid rgba(201, 168, 76, 0.5);
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    color: var(--accent-gold, #c9a84c);
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    min-height: 48px;
  }

  .preview-close:hover {
    background: rgba(201, 168, 76, 0.2);
  }
</style>
