<script lang="ts">
  import type { GameState, StructuredGameLog } from './data/types';
  import type { LobbyPlayer, SanitizedGameState } from './network/types';
  import SetupScreen from './components/SetupScreen.svelte';
  import GameScreen from './components/GameScreen.svelte';
  import ScoreScreen from './components/ScoreScreen.svelte';
  import MainMenu from './components/MainMenu.svelte';
  import LobbyScreen from './components/LobbyScreen.svelte';
  import OnlineGameScreen from './components/OnlineGameScreen.svelte';
  import ZoneEditor from './components/ZoneEditor.svelte';
  import CardPreview from './components/CardPreview.svelte';
  import { saveGame, loadGame, clearSavedGame } from './persistence';
  import { NetworkManager } from './network/networkManager';
  import { HostController } from './network/hostController';
  import { GuestController } from './network/guestController';
  import { sanitizedToGameState } from './network/stateAdapter';
  import { GameLogAccumulator } from './gameLog';
  import { initEngine } from './engine/wasmEngine';
  import { resetTutorial } from './tutorial/tutorial';

  let engineReady = $state(false);
  initEngine().then(() => { engineReady = true; });

  type AppScreen =
    | { type: 'mainMenu' }
    | { type: 'localSetup' }
    | { type: 'lobby'; role: 'host' | 'guest' }
    | { type: 'onlineGame'; role: 'host' | 'guest' }
    | { type: 'localGame' }
    | { type: 'score' }
    | { type: 'zoneEditor' };

  const saved = loadGame();
  let screen: AppScreen = $state({ type: 'mainMenu' });
  let gameState: GameState | null = $state(saved?.gameState ?? null);
  let gameStartTime: number | null = $state(saved?.gameStartTime ?? null);
  let savedGameLog: string[] = $state(saved?.gameLog ?? []);
  let aiIterations: number[] | null = $state(saved?.aiIterations ?? null);
  let aiStyle: string | null = $state(saved?.aiStyle ?? null);
  let hasSavedGame = $state(saved !== null && saved.gameState.phase.type !== 'gameOver');

  // Structured logging state
  let gameLogAccumulator: GameLogAccumulator | null = $state(
    saved?.structuredLog ? GameLogAccumulator.fromLog(saved.structuredLog) : null
  );
  let finalGameLog: StructuredGameLog | null = $state(null);

  // Online state
  let networkManager: NetworkManager | null = $state(null);
  let hostController: HostController | null = $state(null);
  let guestController: GuestController | null = $state(null);
  let roomCode = $state('');
  let lobbyPlayers: LobbyPlayer[] = $state([]);
  let lobbyPlayerCount = $state(2);
  let hostName = $state('');

  type RejoinInfo = { roomCode: string; name: string; token: string };
  let rejoinInfo: RejoinInfo | null = $state(loadRejoinInfo());
  let initialGuestState: SanitizedGameState | null = $state(null);

  function persistRejoinInfo(info: RejoinInfo | null) {
    rejoinInfo = info;
    try {
      if (info) sessionStorage.setItem('colori-rejoin', JSON.stringify(info));
      else sessionStorage.removeItem('colori-rejoin');
    } catch {}
  }

  function loadRejoinInfo(): RejoinInfo | null {
    try {
      const raw = sessionStorage.getItem('colori-rejoin');
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      if (parsed && parsed.roomCode && parsed.name && parsed.token) return parsed as RejoinInfo;
    } catch {}
    return null;
  }

  // -- History helpers --

  function pushScreen(newScreen: AppScreen) {
    history.pushState({ screen: newScreen }, '');
    screen = newScreen;
  }

  function replaceScreen(newScreen: AppScreen) {
    history.replaceState({ screen: newScreen }, '');
    screen = newScreen;
  }

  function backToMainMenu() {
    screen = { type: 'mainMenu' };
    history.back();
  }

  // Initialize history state and popstate handler
  $effect(() => {
    history.replaceState({ screen: { type: 'mainMenu' } }, '');

    function handlePopState(event: PopStateEvent) {
      const currentScreen = screen;

      // Clean up network if leaving lobby or online game
      if (currentScreen.type === 'lobby' || currentScreen.type === 'onlineGame') {
        cleanupNetwork();
        gameState = null;
        gameStartTime = null;
      }

      // Determine target screen from event state
      const targetScreen: AppScreen = event.state?.screen ?? { type: 'mainMenu' };

      // Validate the target screen can be displayed
      if (targetScreen.type === 'localGame' && gameState === null) {
        replaceScreen({ type: 'mainMenu' });
        return;
      }
      if (targetScreen.type === 'onlineGame') {
        // Online games can't be resumed via history
        replaceScreen({ type: 'mainMenu' });
        return;
      }
      if (targetScreen.type === 'lobby') {
        // Lobby can't be resumed via history
        replaceScreen({ type: 'mainMenu' });
        return;
      }
      if (targetScreen.type === 'score' && gameState === null) {
        replaceScreen({ type: 'mainMenu' });
        return;
      }

      screen = targetScreen;
    }

    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  });

  // -- Local game handlers --

  function handleGameStarted(state: GameState, iterations: number[], style: string) {
    gameState = state;
    aiIterations = iterations;
    aiStyle = style;
    gameStartTime = Date.now();
    savedGameLog = [];
    gameLogAccumulator = new GameLogAccumulator(state, iterations, style);
    finalGameLog = null;
    saveGame(state, gameStartTime!, [], iterations, gameLogAccumulator!.getLog(), style);
    replaceScreen({ type: 'localGame' });
  }

  function handleGameUpdated(state: GameState, log: string[]) {
    gameState = state;
    if (state.phase.type === 'gameOver') {
      clearSavedGame();
      hasSavedGame = false;
      if (gameLogAccumulator) {
        gameLogAccumulator.finalize(state);
        finalGameLog = gameLogAccumulator.getLog();
      }
      replaceScreen({ type: 'score' });
    } else {
      saveGame(state, gameStartTime!, log, aiIterations ?? undefined, gameLogAccumulator?.getLog(), aiStyle ?? undefined);
    }
  }

  function handlePlayAgain() {
    gameState = null;
    gameStartTime = null;
    savedGameLog = [];
    aiIterations = null;
    aiStyle = null;
    gameLogAccumulator = null;
    finalGameLog = null;
    clearSavedGame();
    hasSavedGame = false;
    backToMainMenu();
  }

  function handleLeaveGame() {
    clearSavedGame();
    gameState = null;
    gameStartTime = null;
    savedGameLog = [];
    aiIterations = null;
    aiStyle = null;
    gameLogAccumulator = null;
    finalGameLog = null;
    hasSavedGame = false;
    backToMainMenu();
  }

  function handleLeaveOnlineGame() {
    cleanupNetwork();
    gameState = null;
    gameStartTime = null;
    backToMainMenu();
  }

  // -- Navigation --

  function goToLocalSetup() {
    pushScreen({ type: 'localSetup' });
  }

  function resumeGame() {
    pushScreen({ type: 'localGame' });
  }

  function goToMainMenu() {
    cleanupNetwork();
    backToMainMenu();
  }

  function goToZoneEditor() {
    pushScreen({ type: 'zoneEditor' });
  }

  function handleHowToPlay() {
    resetTutorial();
    goToLocalSetup();
  }

  // -- Online game handlers --

  function hostOnlineGame() {
    networkManager = new NetworkManager();
    roomCode = networkManager.createRoom();

    hostController = new HostController(networkManager, hostName || 'Host');
    hostController.onLobbyUpdated = (players) => {
      lobbyPlayers = [...players];
    };

    lobbyPlayers = [...hostController.getLobbyPlayers()];
    lobbyPlayerCount = hostController.getPlayerCount();
    pushScreen({ type: 'lobby', role: 'host' });
  }

  function joinOnlineGame() {
    networkManager = new NetworkManager();
    guestController = new GuestController(networkManager);

    guestController.onLobbyUpdated = (players, playerCount) => {
      lobbyPlayers = [...players];
      lobbyPlayerCount = playerCount;
    };

    guestController.onGameStarted = (state: SanitizedGameState) => {
      initialGuestState = state;
      gameState = sanitizedToGameState(state);
      gameStartTime = state.gameStartTime ?? Date.now();
      const token = guestController!.getRejoinToken();
      const name = guestController!.getMyName();
      if (token && name && roomCode) {
        persistRejoinInfo({ roomCode, name, token });
      }
      replaceScreen({ type: 'onlineGame', role: 'guest' });
    };

    guestController.onError = (message: string, context?: string) => {
      // Only show blocking alerts for join-time errors; in-game errors surface via OnlineGameScreen banner.
      if (context === 'join' || !screen || screen.type !== 'onlineGame') {
        alert(message);
      }
    };

    guestController.onHostDisconnected = (reason) => {
      alert(reason === 'intentional' ? 'Host ended the game' : 'Host disconnected');
      persistRejoinInfo(null);
      goToMainMenu();
    };

    pushScreen({ type: 'lobby', role: 'guest' });
  }

  function handleGuestJoin(name: string, code: string) {
    networkManager!.join(code);
    roomCode = code;
    guestController!.join(name);
  }

  function rejoinOnlineGame() {
    if (!rejoinInfo) return;
    const info = rejoinInfo;
    networkManager = new NetworkManager();
    guestController = new GuestController(networkManager);

    guestController.onLobbyUpdated = (players, playerCount) => {
      lobbyPlayers = [...players];
      lobbyPlayerCount = playerCount;
    };

    guestController.onGameStarted = (state: SanitizedGameState) => {
      initialGuestState = state;
      gameState = sanitizedToGameState(state);
      gameStartTime = state.gameStartTime ?? Date.now();
      replaceScreen({ type: 'onlineGame', role: 'guest' });
    };

    guestController.onSanitizedStateChanged = (state) => {
      // First state update from a successful rejoin acts as "game resumed".
      if (screen.type !== 'onlineGame') {
        initialGuestState = state;
        gameState = sanitizedToGameState(state);
        gameStartTime = state.gameStartTime ?? Date.now();
        replaceScreen({ type: 'onlineGame', role: 'guest' });
      }
    };

    guestController.onError = (message: string, context?: string) => {
      if (context === 'join') {
        alert(`Rejoin failed: ${message}`);
        persistRejoinInfo(null);
        cleanupNetwork();
        goToMainMenu();
      }
    };

    guestController.onHostDisconnected = (reason) => {
      alert(reason === 'intentional' ? 'Host ended the game' : 'Host disconnected');
      persistRejoinInfo(null);
      goToMainMenu();
    };

    roomCode = info.roomCode;
    networkManager.join(info.roomCode);
    guestController.rejoin(info.name, info.token);
    pushScreen({ type: 'lobby', role: 'guest' });
  }

  function dismissRejoinInfo() {
    persistRejoinInfo(null);
  }

  function handleSetHostName(name: string) {
    hostName = name;
    hostController?.setHostName(name || 'Host');
  }

  function handleSetPlayerCount(count: number) {
    lobbyPlayerCount = count;
    hostController?.setPlayerCount(count);
  }

  function handleStartOnlineGame() {
    if (!hostController) return;
    hostController.startGame();
    gameState = hostController.getGameState();
    gameStartTime = hostController.getGameStartTime();
    replaceScreen({ type: 'onlineGame', role: 'host' });
  }

  function handleOnlineGameOver(finalState: GameState, structuredLog?: StructuredGameLog) {
    persistRejoinInfo(null);
    cleanupNetwork();
    gameState = finalState;
    finalGameLog = structuredLog ?? null;
    replaceScreen({ type: 'score' });
  }

  function cleanupNetwork() {
    // If we were hosting an active game, politely notify guests before tearing down transport.
    if (hostController && hostController.getGameState()) {
      hostController.announceHostLeaving();
    }
    hostController?.cleanup();
    guestController?.leave();
    networkManager?.leave();
    networkManager = null;
    hostController = null;
    guestController = null;
    roomCode = '';
    lobbyPlayers = [];
    lobbyPlayerCount = 2;
  }

  // Warn on refresh/close during an active online game.
  $effect(() => {
    function onBeforeUnload(e: BeforeUnloadEvent) {
      if (screen.type === 'onlineGame' || screen.type === 'lobby') {
        e.preventDefault();
        e.returnValue = '';
      }
    }
    window.addEventListener('beforeunload', onBeforeUnload);
    return () => window.removeEventListener('beforeunload', onBeforeUnload);
  });
</script>

<main>
  {#if screen.type !== 'localGame' && screen.type !== 'onlineGame'}
    <h1 class="title">Colori</h1>
    {#if screen.type === 'mainMenu'}
      <p class="subtitle">Vendecolori di Venezia</p>
    {/if}
  {/if}
  {#if !engineReady}
    <p>Loading...</p>
  {:else if screen.type === 'mainMenu'}
    <MainMenu
      onLocalGame={goToLocalSetup}
      onHostOnline={hostOnlineGame}
      onJoinOnline={joinOnlineGame}
      {hasSavedGame}
      onResumeGame={resumeGame}
      onHowToPlay={handleHowToPlay}
      rejoinAvailable={rejoinInfo !== null}
      rejoinRoomCode={rejoinInfo?.roomCode ?? ''}
      onRejoinOnline={rejoinOnlineGame}
      onDismissRejoin={dismissRejoinInfo}
    />
  {:else if screen.type === 'localSetup'}
    <SetupScreen onGameStarted={handleGameStarted} />
  {:else if screen.type === 'lobby'}
    <LobbyScreen
      role={screen.role}
      {roomCode}
      {lobbyPlayers}
      playerCount={lobbyPlayerCount}
      {hostName}
      onSetHostName={handleSetHostName}
      onSetPlayerCount={handleSetPlayerCount}
      onStartGame={handleStartOnlineGame}
      onJoin={handleGuestJoin}
      onBack={goToMainMenu}
    />
  {:else if screen.type === 'localGame' && gameState !== null}
    <GameScreen {gameState} {gameStartTime} onGameUpdated={handleGameUpdated} initialGameLog={savedGameLog} onLeaveGame={handleLeaveGame} {gameLogAccumulator} aiIterations={aiIterations ?? gameState.aiPlayers.map(() => 100000)} aiStyle={aiStyle ?? 'ga'} />
  {:else if screen.type === 'onlineGame' && gameState !== null}
    <OnlineGameScreen
      role={screen.role}
      {hostController}
      {guestController}
      {initialGuestState}
      onGameOver={handleOnlineGameOver}
      gameStartTime={gameStartTime ?? Date.now()}
      onLeaveGame={handleLeaveOnlineGame}
    />
  {:else if screen.type === 'score' && gameState !== null}
    <ScoreScreen {gameState} {gameStartTime} onPlayAgain={handlePlayAgain} structuredLog={finalGameLog} />
  {:else if screen.type === 'zoneEditor'}
    <ZoneEditor onClose={goToMainMenu} />
  {/if}
</main>

<CardPreview />

<style>
  main {
    text-align: center;
  }
  .title {
    font-family: var(--font-display);
    font-size: clamp(1.8rem, 5vw, 2.5rem);
    font-weight: 700;
    letter-spacing: clamp(2px, 0.5vw, 4px);
    margin-bottom: 0.25rem;
    background: linear-gradient(135deg, #c9a84c, #e8d48b, #c9a84c);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  .subtitle {
    font-family: var(--font-body);
    font-size: clamp(0.95rem, 2.5vw, 1.1rem);
    font-style: italic;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
    letter-spacing: 1px;
  }
</style>
