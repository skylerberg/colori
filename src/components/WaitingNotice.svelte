<script lang="ts">
  let { text, detail }: { text: string; detail?: string } = $props();
</script>

<div class="waiting">
  <span class="spinner"></span>
  <div>
    <div class="panel-heading">{text}</div>
    {#if detail}<p class="panel-hint">{detail}</p>{/if}
  </div>
</div>

<style>
  .waiting {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }

  .spinner {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    border: 3px solid rgba(201, 168, 76, 0.3);
    border-top-color: #c9a84c;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
