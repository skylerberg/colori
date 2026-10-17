<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import type { ErrorReport } from '../errorReport';

  let { report, actions }: { report: ErrorReport; actions?: Snippet } = $props();

  let copied = $state(false);
  let showFull = $state(false);
  let fullText: HTMLTextAreaElement | undefined = $state();
  const canShare = typeof navigator.share === 'function';

  async function copy() {
    try {
      await navigator.clipboard.writeText(report.text);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // No clipboard on an insecure origin, or permission refused: select the
      // text for the player to copy themselves.
      showFull = true;
      await tick();
      fullText?.select();
    }
  }

  async function share() {
    try {
      await navigator.share({ title: 'Colori error report', text: report.text });
    } catch {
      // The player closed the share sheet.
    }
  }
</script>

<div class="error-report" role="alert">
  <strong>{report.title}</strong>
  <pre>{report.message}</pre>
  <p class="hint">Please send this report to the developer.</p>
  <div class="buttons">
    <button class="report-btn" onclick={copy}>{copied ? 'Copied' : 'Copy report'}</button>
    {#if canShare}
      <button class="report-btn" onclick={share}>Share report</button>
    {/if}
    {@render actions?.()}
  </div>
  <details bind:open={showFull}>
    <summary>Full report</summary>
    <textarea readonly rows="8" bind:this={fullText} value={report.text}></textarea>
  </details>
</div>

<style>
  .error-report {
    background: rgba(254, 242, 242, 0.95);
    border: 1px solid #e74c3c;
    border-radius: 8px;
    padding: 12px 16px;
    color: #333;
  }

  strong {
    color: #c0392b;
    font-size: 0.9rem;
  }

  pre {
    margin: 8px 0;
    padding: 8px;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.75rem;
    text-align: left;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: all;
  }

  .hint {
    margin: 0 0 8px;
    font-size: 0.8rem;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .report-btn {
    padding: 8px 16px;
    font-size: 0.8rem;
    background: #fff;
    color: #c0392b;
    border: 1px solid #e74c3c;
    border-radius: 4px;
    cursor: pointer;
    min-height: 44px;
  }

  summary {
    margin-top: 8px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    margin-top: 4px;
    font-family: monospace;
    font-size: 0.7rem;
  }
</style>
