<script lang="ts">
  import type { LobbyPlayer } from '../network/types';

  let { role, roomCode, lobbyPlayers, playerCount, onSetPlayerCount, onStartGame, onJoin, onBack }: {
    role: 'host' | 'guest';
    roomCode: string;
    lobbyPlayers: LobbyPlayer[];
    playerCount: number;
    onSetPlayerCount?: (count: number) => void;
    onStartGame?: () => void;
    onJoin?: (name: string, code: string) => void;
    onBack: () => void;
  } = $props();

  let guestName = $state('');
  let joinCode = $state('');
  let joined = $state(false);

  function handleJoin() {
    const name = guestName.trim() || 'Player';
    const code = joinCode.trim().toUpperCase();
    if (!code) return;
    onJoin?.(name, code);
    joined = true;
  }

  let humanPlayerCount = $derived(lobbyPlayers.length);
  let canStart = $derived(humanPlayerCount >= 2 || playerCount >= 2);
</script>

<div class="lobby-screen">
  <button class="back-btn" onclick={onBack}>&larr; Back</button>

  {#if role === 'host'}
    <h2>Host Online Game</h2>

    <div class="room-code-section">
      <span class="room-code-label">Room Code:</span>
      <span class="room-code">{roomCode}</span>
      <p class="room-code-hint">Share this code with other players</p>
    </div>

    <div class="player-count-section">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label>Total Players:</label>
      <div class="count-buttons">
        {#each [2, 3, 4, 5] as count}
          <button
            class="count-btn"
            class:active={playerCount === count}
            onclick={() => onSetPlayerCount?.(count)}
          >
            {count}
          </button>
        {/each}
      </div>
    </div>

    <div class="players-section">
      <h3>Players ({humanPlayerCount} / {playerCount})</h3>
      <div class="player-list">
        {#each lobbyPlayers as player}
          <div class="player-row" class:disconnected={!player.connected}>
            <span class="player-name">
              {player.name}
              {#if player.isHost}<span class="host-badge">Host</span>{/if}
            </span>
            <span class="player-status" class:connected={player.connected}>
              {player.connected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        {/each}
        {#each { length: Math.max(0, playerCount - humanPlayerCount) } as _}
          <div class="player-row ai-slot">
            <span class="player-name">AI Player</span>
            <span class="ai-badge">AI</span>
          </div>
        {/each}
      </div>
    </div>

    <button class="start-btn" onclick={onStartGame} disabled={!canStart}>
      Start Game
    </button>

  {:else}
    <h2>Join Online Game</h2>

    {#if !joined}
      <div class="join-form">
        <div class="input-group">
          <label for="guest-name">Your Name:</label>
          <input
            id="guest-name"
            type="text"
            bind:value={guestName}
            placeholder="Player"
          />
        </div>
        <div class="input-group">
          <label for="join-code">Room Code:</label>
          <input
            id="join-code"
            type="text"
            bind:value={joinCode}
            placeholder="ABC123"
            maxlength="6"
            style="text-transform: uppercase"
          />
        </div>
        <button class="join-btn" onclick={handleJoin} disabled={!joinCode.trim()}>
          Join
        </button>
      </div>
    {:else}
      <div class="waiting-section">
        <div class="players-section">
          <h3>Players</h3>
          <div class="player-list">
            {#each lobbyPlayers as player}
              <div class="player-row">
                <span class="player-name">
                  {player.name}
                  {#if player.isHost}<span class="host-badge">Host</span>{/if}
                </span>
                <span class="player-status connected">Connected</span>
              </div>
            {/each}
          </div>
        </div>
        <p class="waiting-text">Waiting for host to start the game...</p>
      </div>
    {/if}
  {/if}
</div>

<style>
  .lobby-screen {
    max-width: 450px;
    margin: 1rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .back-btn {
    align-self: flex-start;
    padding: 6px 14px;
    font-size: 0.9rem;
    background: #eee;
    border: 1px solid #ccc;
    border-radius: 6px;
    cursor: pointer;
  }

  .back-btn:hover {
    background: #ddd;
  }

  h2 {
    color: #4a3728;
    font-size: 1.5rem;
    text-align: center;
  }

  .room-code-section {
    text-align: center;
    padding: 16px;
    border: 2px solid #d4a017;
    border-radius: 12px;
    background: #fffde7;
  }

  .room-code-label {
    font-size: 0.9rem;
    color: #666;
    display: block;
    margin-bottom: 4px;
  }

  .room-code {
    font-size: 2.5rem;
    font-weight: 800;
    color: #4a3728;
    letter-spacing: 6px;
  }

  .room-code-hint {
    font-size: 0.8rem;
    color: #999;
    margin-top: 4px;
  }

  .player-count-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .player-count-section label {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .count-buttons {
    display: flex;
    gap: 8px;
  }

  .count-btn {
    width: 48px;
    height: 48px;
    font-size: 1.2rem;
    font-weight: 700;
    border-radius: 50%;
    border: 2px solid #999;
    background: #fff;
    cursor: pointer;
  }

  .count-btn.active {
    border-color: #2a6bcf;
    background: #2a6bcf;
    color: #fff;
  }

  .players-section {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 12px;
    background: #fff;
  }

  .players-section h3 {
    font-size: 0.9rem;
    color: #4a3728;
    margin-bottom: 8px;
  }

  .player-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .player-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border: 1px solid #eee;
    border-radius: 6px;
    background: #fafafa;
  }

  .player-row.disconnected {
    opacity: 0.6;
  }

  .player-row.ai-slot {
    border-style: dashed;
    border-color: #ccc;
    background: #f5f5f5;
  }

  .player-name {
    font-weight: 600;
    font-size: 0.9rem;
  }

  .host-badge {
    font-size: 0.7rem;
    background: #d4a017;
    color: #fff;
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 6px;
    font-weight: 700;
  }

  .ai-badge {
    font-size: 0.75rem;
    background: #e67e22;
    color: #fff;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 700;
  }

  .player-status {
    font-size: 0.8rem;
    font-weight: 600;
  }

  .player-status.connected {
    color: #27ae60;
  }

  .player-row.disconnected .player-status {
    color: #e74c3c;
  }

  .start-btn {
    padding: 12px 24px;
    font-size: 1.1rem;
    font-weight: 700;
    background: #2a6bcf;
    color: #fff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }

  .start-btn:hover:not(:disabled) {
    background: #1e56a8;
  }

  .start-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .join-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-group label {
    font-size: 0.85rem;
    font-weight: 600;
  }

  .input-group input {
    padding: 10px 12px;
    border: 2px solid #ccc;
    border-radius: 6px;
    font-size: 1rem;
  }

  .input-group input:focus {
    outline: none;
    border-color: #2a6bcf;
  }

  .join-btn {
    padding: 12px 24px;
    font-size: 1.1rem;
    font-weight: 700;
    background: #4a3728;
    color: #fff;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }

  .join-btn:hover:not(:disabled) {
    background: #3a2a1e;
  }

  .join-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .waiting-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .waiting-text {
    text-align: center;
    color: #666;
    font-style: italic;
    font-size: 0.95rem;
  }
</style>
