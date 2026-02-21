<script lang="ts">
  let { playerName, onReady }: {
    playerName: string;
    onReady: () => void;
  } = $props();
</script>

<div class="pass-overlay">
  <div class="pass-content">
    <h2>Pass the device to</h2>
    <div class="player-name">{playerName}</div>
    <button class="ready-btn" onclick={onReady}>Ready</button>
  </div>
</div>

<style>
  .pass-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.92);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .pass-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    color: #fff;
  }

  h2 {
    font-size: 1.2rem;
    font-weight: 400;
    color: #ccc;
  }

  .player-name {
    font-size: 2.5rem;
    font-weight: 700;
    color: #fff;
  }

  .ready-btn {
    padding: 14px 48px;
    font-size: 1.2rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 10px;
    margin-top: 1rem;
  }

  .ready-btn:hover {
    background: #1e56a8;
  }
</style>
