<script lang="ts">
  import type { GameState } from './data/types';
  import type { LobbyPlayer, SanitizedGameState } from './network/types';
  import SetupScreen from './components/SetupScreen.svelte';
  import GameScreen from './components/GameScreen.svelte';
  import ScoreScreen from './components/ScoreScreen.svelte';
  import MainMenu from './components/MainMenu.svelte';
  import LobbyScreen from './components/LobbyScreen.svelte';
  import OnlineGameScreen from './components/OnlineGameScreen.svelte';
  import { saveGame, loadGame, clearSavedGame } from './persistence';
  import { NetworkManager } from './network/networkManager';
  import { HostController } from './network/hostController';
  import { GuestController } from './network/guestController';
  import { sanitizedToGameState } from './network/stateAdapter';

  type AppScreen =
    | { type: 'mainMenu' }
    | { type: 'localSetup' }
    | { type: 'lobby'; role: 'host' | 'guest' }
    | { type: 'onlineGame'; role: 'host' | 'guest' }
    | { type: 'localGame' }
    | { type: 'score' };

  const saved = loadGame();
  let screen: AppScreen = $state({ type: 'mainMenu' });
  let gameState: GameState | null = $state(saved?.gameState ?? null);
  let gameStartTime: number | null = $state(saved?.gameStartTime ?? null);
  let savedGameLog: string[] = $state(saved?.gameLog ?? []);
  let hasSavedGame = $state(saved !== null && saved.gameState.phase.type !== 'gameOver');

  // Online state
  let networkManager: NetworkManager | null = $state(null);
  let hostController: HostController | null = $state(null);
  let guestController: GuestController | null = $state(null);
  let roomCode = $state('');
  let lobbyPlayers: LobbyPlayer[] = $state([]);
  let lobbyPlayerCount = $state(2);

  // -- Local game handlers --

  function handleGameStarted(state: GameState) {
    gameState = state;
    gameStartTime = Date.now();
    savedGameLog = [];
    saveGame(state, gameStartTime!, []);
    screen = { type: 'localGame' };
  }

  function handleGameUpdated(state: GameState, log: string[]) {
    gameState = state;
    if (state.phase.type === 'gameOver') {
      clearSavedGame();
      hasSavedGame = false;
      screen = { type: 'score' };
    } else {
      saveGame(state, gameStartTime!, log);
    }
  }

  function handlePlayAgain() {
    gameState = null;
    gameStartTime = null;
    savedGameLog = [];
    clearSavedGame();
    hasSavedGame = false;
    screen = { type: 'mainMenu' };
  }

  function handleLeaveGame() {
    clearSavedGame();
    gameState = null;
    gameStartTime = null;
    savedGameLog = [];
    hasSavedGame = false;
    screen = { type: 'mainMenu' };
  }

  function handleLeaveOnlineGame() {
    cleanupNetwork();
    gameState = null;
    gameStartTime = null;
    screen = { type: 'mainMenu' };
  }

  // -- Navigation --

  function goToLocalSetup() {
    screen = { type: 'localSetup' };
  }

  function resumeGame() {
    screen = { type: 'localGame' };
  }

  function goToMainMenu() {
    cleanupNetwork();
    screen = { type: 'mainMenu' };
  }

  // -- Online game handlers --

  function hostOnlineGame() {
    networkManager = new NetworkManager();
    roomCode = networkManager.createRoom();

    hostController = new HostController(networkManager, 'Host');
    hostController.onLobbyUpdated = (players) => {
      lobbyPlayers = [...players];
    };

    lobbyPlayers = [...hostController.getLobbyPlayers()];
    lobbyPlayerCount = hostController.getPlayerCount();
    screen = { type: 'lobby', role: 'host' };
  }

  function joinOnlineGame() {
    networkManager = new NetworkManager();
    guestController = new GuestController(networkManager);

    guestController.onLobbyUpdated = (players, playerCount) => {
      lobbyPlayers = [...players];
      lobbyPlayerCount = playerCount;
    };

    guestController.onGameStarted = (state: SanitizedGameState) => {
      gameState = sanitizedToGameState(state);
      gameStartTime = Date.now();
      screen = { type: 'onlineGame', role: 'guest' };
    };

    guestController.onError = (message: string) => {
      alert(message);
    };

    guestController.onHostDisconnected = () => {
      alert('Host disconnected');
      goToMainMenu();
    };

    screen = { type: 'lobby', role: 'guest' };
  }

  function handleGuestJoin(name: string, code: string) {
    networkManager!.join(code);
    roomCode = code;
    guestController!.join(name);
  }

  function handleSetPlayerCount(count: number) {
    lobbyPlayerCount = count;
    hostController?.setPlayerCount(count);
  }

  function handleStartOnlineGame() {
    if (!hostController) return;
    hostController.startGame();
    gameState = hostController.getGameState();
    gameStartTime = Date.now();
    screen = { type: 'onlineGame', role: 'host' };
  }

  function handleOnlineGameOver(finalState: GameState) {
    gameState = finalState;
    screen = { type: 'score' };
  }

  function cleanupNetwork() {
    hostController?.cleanup();
    networkManager?.leave();
    networkManager = null;
    hostController = null;
    guestController = null;
    roomCode = '';
    lobbyPlayers = [];
    lobbyPlayerCount = 2;
  }
</script>

<main>
  <h1>Colori</h1>
  {#if screen.type === 'mainMenu'}
    <MainMenu
      onLocalGame={goToLocalSetup}
      onHostOnline={hostOnlineGame}
      onJoinOnline={joinOnlineGame}
      {hasSavedGame}
      onResumeGame={resumeGame}
    />
  {:else if screen.type === 'localSetup'}
    <SetupScreen onGameStarted={handleGameStarted} />
  {:else if screen.type === 'lobby'}
    <LobbyScreen
      role={screen.role}
      {roomCode}
      {lobbyPlayers}
      playerCount={lobbyPlayerCount}
      onSetPlayerCount={handleSetPlayerCount}
      onStartGame={handleStartOnlineGame}
      onJoin={handleGuestJoin}
      onBack={goToMainMenu}
    />
  {:else if screen.type === 'localGame' && gameState !== null}
    <GameScreen {gameState} {gameStartTime} onGameUpdated={handleGameUpdated} initialGameLog={savedGameLog} onLeaveGame={handleLeaveGame} />
  {:else if screen.type === 'onlineGame' && gameState !== null}
    <OnlineGameScreen
      role={screen.role}
      {hostController}
      {guestController}
      onGameOver={handleOnlineGameOver}
      gameStartTime={gameStartTime ?? Date.now()}
      onLeaveGame={handleLeaveOnlineGame}
    />
  {:else if screen.type === 'score' && gameState !== null}
    <ScoreScreen {gameState} {gameStartTime} onPlayAgain={handlePlayAgain} />
  {/if}
</main>

<style>
  main {
    text-align: center;
  }
  h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
    color: #4a3728;
  }
</style>
