<script lang="ts">
  let { onLocalGame, onHostOnline, onJoinOnline, hasSavedGame, onResumeGame }: {
    onLocalGame: () => void;
    onHostOnline: () => void;
    onJoinOnline: () => void;
    hasSavedGame: boolean;
    onResumeGame: () => void;
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
    <a class="menu-btn analysis-btn" href="analysis.html">Analysis</a>
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
    font-size: 1.1rem;
    font-weight: 700;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    width: 100%;
  }

  .local-btn {
    background: #2a6bcf;
    color: #fff;
  }

  .local-btn:hover {
    background: #1e56a8;
  }

  .online-btn {
    background: #4a3728;
    color: #fff;
  }

  .online-btn:hover {
    background: #3a2a1e;
  }

  .resume-btn {
    background: #d4a017;
    color: #fff;
  }

  .resume-btn:hover {
    background: #b8890f;
  }

  .analysis-btn {
    background: #5a7a5a;
    color: #fff;
    text-align: center;
    text-decoration: none;
    display: block;
    box-sizing: border-box;
  }

  .analysis-btn:hover {
    background: #4a664a;
  }
</style>
