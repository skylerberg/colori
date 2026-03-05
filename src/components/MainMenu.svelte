<script lang="ts">
  import { getViewMode, setViewMode, type ViewMode } from '../stores/viewMode.svelte';

  let { onLocalGame, onHostOnline, onJoinOnline, hasSavedGame, onResumeGame }: {
    onLocalGame: () => void;
    onHostOnline: () => void;
    onJoinOnline: () => void;
    hasSavedGame: boolean;
    onResumeGame: () => void;
  } = $props();

  let viewMode = $derived(getViewMode());

  function handleViewToggle(mode: ViewMode) {
    setViewMode(mode);
  }
</script>

<div class="main-menu">
  <div class="menu-buttons">
    {#if hasSavedGame}
      <button class="menu-btn resume-btn" onclick={onResumeGame}>Resume Game</button>
    {/if}
    <button class="menu-btn local-btn" onclick={onLocalGame}>Local Game</button>
    <button class="menu-btn online-btn" onclick={onHostOnline}>Host Online Game</button>
    <button class="menu-btn online-btn" onclick={onJoinOnline}>Join Online Game</button>
  </div>

  <div class="view-toggle">
    <span class="toggle-label">View Mode</span>
    <div class="toggle-group">
      <button
        class="toggle-btn"
        class:active={viewMode === '2d'}
        onclick={() => handleViewToggle('2d')}
      >2D</button>
      <button
        class="toggle-btn"
        class:active={viewMode === '3d'}
        onclick={() => handleViewToggle('3d')}
      >3D</button>
    </div>
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

  .view-toggle {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .toggle-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: #4a3728;
  }

  .toggle-group {
    display: flex;
    border: 2px solid #4a3728;
    border-radius: 6px;
    overflow: hidden;
  }

  .toggle-btn {
    padding: 6px 20px;
    font-size: 0.9rem;
    font-weight: 700;
    border: none;
    cursor: pointer;
    background: #fff;
    color: #4a3728;
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn.active {
    background: #4a3728;
    color: #fff;
  }

  .toggle-btn:hover:not(.active) {
    background: #f0e8d8;
  }

</style>
