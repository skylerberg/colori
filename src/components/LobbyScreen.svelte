<script lang="ts">
  import type { LobbyPlayer } from '../network/types';

  let { role, roomCode, lobbyPlayers, playerCount, hostName = '', onSetHostName, onSetPlayerCount, onStartGame, onJoin, onBack }: {
    role: 'host' | 'guest';
    roomCode: string;
    lobbyPlayers: LobbyPlayer[];
    playerCount: number;
    hostName?: string;
    onSetHostName?: (name: string) => void;
    onSetPlayerCount?: (count: number) => void;
    onStartGame?: () => void;
    onJoin?: (name: string, code: string) => void;
    onBack: () => void;
  } = $props();

  let guestName = $state('');
  let joinCode = $state('');
  let joined = $state(false);
  // svelte-ignore state_referenced_locally
  let localHostName = $state(hostName);
  let hostNameDebounce: ReturnType<typeof setTimeout> | null = null;

  function handleHostNameInput(value: string) {
    localHostName = value;
    if (hostNameDebounce) clearTimeout(hostNameDebounce);
    hostNameDebounce = setTimeout(() => {
      onSetHostName?.(value);
      hostNameDebounce = null;
    }, 300);
  }

  function commitHostName() {
    if (hostNameDebounce) {
      clearTimeout(hostNameDebounce);
      hostNameDebounce = null;
    }
    onSetHostName?.(localHostName);
  }

  function handleJoin() {
    const name = guestName.trim();
    const code = joinCode.trim().toUpperCase();
    if (!name || !code) return;
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

    <div class="input-group">
      <label for="host-name">Your Name:</label>
      <input
        id="host-name"
        type="text"
        value={localHostName}
        maxlength="20"
        oninput={(e) => handleHostNameInput(e.currentTarget.value)}
        onblur={commitHostName}
        placeholder="Host"
      />
    </div>

    <div class="room-code-section">
      <span class="room-code-label">Room Code:</span>
      <span class="room-code">{roomCode}</span>
      <p class="room-code-hint">Share this code with other players</p>
    </div>

    <div class="player-count-section">
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label>Total Players:</label>
      <div class="count-buttons">
        {#each [2, 3, 4] as count}
          <button
            class="count-btn"
            class:active={playerCount === count}
            disabled={count < lobbyPlayers.length}
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
          <div class="player-row" class:disconnected={!player.isConnected}>
            <span class="player-name">
              {player.name}
              {#if player.isHost}<span class="host-badge">Host</span>{/if}
            </span>
            <span class="player-status" class:connected={player.isConnected}>
              {player.isConnected ? 'Connected' : 'Disconnected'}
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

    <button class="start-btn" onclick={() => onStartGame?.()} disabled={!canStart}>
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
            maxlength="20"
            placeholder="Player"
          />
        </div>
        <div class="input-group">
          <label for="join-code">Room Code:</label>
          <input
            id="join-code"
            type="text"
            bind:value={joinCode}
            placeholder="ABCDEFGH"
            maxlength="8"
            style="text-transform: uppercase"
          />
        </div>
        <button class="join-btn" onclick={handleJoin} disabled={!joinCode.trim() || !guestName.trim()}>
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
    width: 100%;
    max-width: 450px;
    margin: 1rem auto;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    padding: 0 1rem;
  }

  .back-btn {
    align-self: flex-start;
    padding: 8px 14px;
    font-size: 0.9rem;
    background: var(--bg-panel, #ebe3d3);
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    cursor: pointer;
    min-height: 44px;
  }

  .back-btn:hover {
    background: #e0d6c3;
  }

  h2 {
    font-family: var(--font-display, 'Cinzel', serif);
    color: var(--text-primary, #2c1e12);
    font-size: clamp(1.3rem, 3.5vw, 1.5rem);
    text-align: center;
  }

  .room-code-section {
    text-align: center;
    padding: 12px;
    border: 2px solid var(--accent-gold, #c9a84c);
    border-radius: 12px;
    background: rgba(201, 168, 76, 0.08);
  }

  .room-code-label {
    font-size: 0.9rem;
    color: var(--text-secondary, #6b5744);
    display: block;
    margin-bottom: 4px;
  }

  .room-code {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: clamp(1.8rem, 6vw, 2.5rem);
    font-weight: 800;
    color: var(--text-primary, #2c1e12);
    letter-spacing: clamp(4px, 1.5vw, 6px);
    word-break: break-all;
    user-select: all;
  }

  .room-code-hint {
    font-size: 0.8rem;
    color: var(--text-tertiary, #9a8775);
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
    gap: 10px;
  }

  .count-btn {
    width: 48px;
    height: 48px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.2rem;
    font-weight: 700;
    border-radius: 50%;
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    background: var(--bg-panel, #ebe3d3);
    cursor: pointer;
  }

  .count-btn.active {
    border-color: var(--accent-gold, #c9a84c);
    background: var(--accent-gold, #c9a84c);
    color: var(--bg-deep, #2c1e12);
  }

  .players-section {
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 8px;
    padding: 12px;
    background: var(--bg-panel, #ebe3d3);
  }

  .players-section h3 {
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 0.85rem;
    color: var(--text-primary, #2c1e12);
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
    padding: 10px 12px;
    border: 1px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.4);
    min-height: 44px;
  }

  .player-row.disconnected {
    opacity: 0.6;
  }

  .player-row.ai-slot {
    border-style: dashed;
    border-color: var(--border-gold, rgba(201, 168, 76, 0.3));
    background: rgba(201, 168, 76, 0.04);
  }

  .player-name {
    font-weight: 600;
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .host-badge {
    font-size: 0.7rem;
    background: var(--accent-gold, #c9a84c);
    color: var(--bg-deep, #2c1e12);
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 6px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .ai-badge {
    font-size: 0.75rem;
    background: var(--accent-crimson, #8b2020);
    color: var(--text-on-dark, #f5ede0);
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .player-status {
    font-size: 0.8rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .player-status.connected {
    color: var(--accent-green, #3a6b3a);
  }

  .player-row.disconnected .player-status {
    color: var(--accent-crimson, #8b2020);
  }

  .start-btn {
    padding: 14px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    min-height: 48px;
    transition: background 0.2s, transform 0.2s;
  }

  .start-btn:hover:not(:disabled) {
    background: #3a2a1e;
    transform: translateY(-2px);
  }

  .start-btn:disabled {
    opacity: 0.4;
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
    border: 2px solid var(--border-gold, rgba(201, 168, 76, 0.3));
    border-radius: 6px;
    font-size: 1rem;
    min-height: 44px;
    width: 100%;
  }

  .input-group input:focus {
    outline: none;
    border-color: var(--accent-gold, #c9a84c);
  }

  .join-btn {
    padding: 14px 24px;
    font-family: var(--font-display, 'Cinzel', serif);
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 1.5px;
    background: var(--bg-deep, #2c1e12);
    color: var(--text-on-dark, #f5ede0);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    min-height: 48px;
    transition: background 0.2s, transform 0.2s;
  }

  .join-btn:hover:not(:disabled) {
    background: #3a2a1e;
    transform: translateY(-2px);
  }

  .join-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .waiting-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .waiting-text {
    text-align: center;
    color: var(--text-secondary, #6b5744);
    font-style: italic;
    font-size: 0.95rem;
  }

  @media (min-width: 768px) {
    .lobby-screen {
      padding: 0;
      gap: 1.5rem;
    }

    .room-code-section {
      padding: 16px;
    }
  }
</style>
