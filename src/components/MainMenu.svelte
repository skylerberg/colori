<script lang="ts">
  let { onLocalGame, onHostOnline, onJoinOnline, hasSavedGame, onResumeGame, onZoneEditor }: {
    onLocalGame: () => void;
    onHostOnline: () => void;
    onJoinOnline: () => void;
    hasSavedGame: boolean;
    onResumeGame: () => void;
    onZoneEditor: () => void;
  } = $props();
</script>

<div class="main-menu">
  <div class="menu-buttons">
    {#if hasSavedGame}
      <button class="menu-btn resume-btn" onclick={onResumeGame}>Resume Game</button>
    {/if}
    <button class="menu-btn local-btn" onclick={onLocalGame}>Local Game</button>
    <button class="menu-btn online-btn" onclick={onHostOnline}>Host Online Game</button>
    <button class="menu-btn online-btn" onclick={onJoinOnline}>Join Online Game</button>
    <button class="menu-btn editor-btn" onclick={onZoneEditor}>Zone Editor</button>
  </div>
</div>

<style>
  .main-menu {
    max-width: 400px;
    margin: 2rem auto;
    display: flex;
    flex-direction: column;
    gap: 2rem;
    align-items: center;
  }

  .menu-buttons {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }

  .menu-btn {
    padding: 14px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    width: 100%;
    transition: background 0.2s, transform 0.2s, box-shadow 0.2s;
  }

  .menu-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(44, 30, 18, 0.2);
  }

  .local-btn {
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
  }

  .local-btn:hover {
    background: #3a2a1e;
  }

  .online-btn {
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
  }

  .online-btn:hover {
    background: #3a2a1e;
  }

  .resume-btn {
    background: linear-gradient(135deg, #c9a84c, #e8d48b);
    color: var(--bg-deep, #2c1e12);
  }

  .resume-btn:hover {
    background: linear-gradient(135deg, #b8960f, #d4c070);
  }

  .editor-btn {
    background: var(--text-tertiary, #9a8775);
    color: var(--text-on-dark, #f5ede0);
  }

  .editor-btn:hover {
    background: #8a7765;
  }
</style>
