<script lang="ts">
  let {
    onLocalGame, onHostOnline, onJoinOnline, hasSavedGame, onResumeGame, onHowToPlay,
    rejoinAvailable = false, rejoinRoomCode = '', onRejoinOnline, onDismissRejoin,
  }: {
    onLocalGame: () => void;
    onHostOnline: () => void;
    onJoinOnline: () => void;
    hasSavedGame: boolean;
    onResumeGame: () => void;
    onHowToPlay: () => void;
    rejoinAvailable?: boolean;
    rejoinRoomCode?: string;
    onRejoinOnline?: () => void;
    onDismissRejoin?: () => void;
  } = $props();
</script>

<div class="main-menu">
  <div class="menu-buttons">
    {#if rejoinAvailable}
      <button class="menu-btn resume-btn" onclick={onRejoinOnline}>Rejoin Online Game ({rejoinRoomCode})</button>
      <button class="menu-btn dismiss-btn" onclick={onDismissRejoin}>Dismiss rejoin</button>
    {/if}
    {#if hasSavedGame}
      <button class="menu-btn resume-btn" onclick={onResumeGame}>Resume Game</button>
    {/if}
    <button class="menu-btn local-btn" onclick={onLocalGame}>Local Game</button>
    <button class="menu-btn online-btn" onclick={onHostOnline}>Host Online Game</button>
    <button class="menu-btn online-btn" onclick={onJoinOnline}>Join Online Game</button>
    <button class="menu-btn tutorial-btn" onclick={onHowToPlay}>How to Play</button>
  </div>
</div>

<style>
  .main-menu {
    width: 100%;
    max-width: 400px;
    margin: 1.5rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    align-items: center;
    padding: 0 1rem;
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
    font-size: clamp(0.95rem, 2.5vw, 1.05rem);
    font-weight: 600;
    letter-spacing: 1.5px;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    width: 100%;
    min-height: 48px;
    transition: background 0.2s, transform 0.2s, box-shadow 0.2s;
  }

  @media (min-width: 768px) {
    .main-menu {
      margin: 2rem auto;
      gap: 2rem;
      padding: 0;
    }
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

  .tutorial-btn {
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
  }

  .tutorial-btn:hover {
    background: #3a2a1e;
  }

  .dismiss-btn {
    background: transparent;
    color: var(--text-secondary, #6b5744);
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    font-size: 0.8rem;
    padding: 8px 14px;
    min-height: 36px;
  }

  .dismiss-btn:hover {
    background: rgba(201, 168, 76, 0.08);
  }

</style>
