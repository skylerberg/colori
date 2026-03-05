<script lang="ts">
  import { T } from '@threlte/core';
  import { OrbitControls, interactivity } from '@threlte/extras';
  import type { GameState, Choice, Color } from '../../data/types';

  import Room from './environment/Room.svelte';
  import Table from './environment/Table.svelte';
  import Lighting from './environment/Lighting.svelte';
  import CandleGroup from './environment/CandleGroup.svelte';
  import Shelves from './environment/Shelves.svelte';
  import DryingLine from './environment/DryingLine.svelte';
  import Workbench from './environment/Workbench.svelte';
  import WindowArea from './environment/WindowArea.svelte';
  import WallTextiles from './environment/WallTextiles.svelte';
  import CandleSmoke from './particles/CandleSmoke.svelte';
  import Effects from './postprocessing/Effects.svelte';
  import BuyerDisplay3D from './cards/BuyerDisplay3D.svelte';
  import DraftZone3D from './cards/DraftZone3D.svelte';
  import ActionZone3D from './cards/ActionZone3D.svelte';
  import PlayerArea from './players/PlayerArea.svelte';
  import PlayerTableau3D from './players/PlayerTableau3D.svelte';
  import OpponentArea from './players/OpponentArea.svelte';
  import RoundInfo3D from './ui/RoundInfo3D.svelte';
  import AbilityPrompt3D from './ui/AbilityPrompt3D.svelte';

  // Enable pointer events (hover, click) on 3D meshes — required by Threlte v8+
  interactivity();

  let { gameState, activePlayerIndex, onAction, elapsedSeconds = 0 }: {
    gameState: GameState;
    activePlayerIndex: number;
    onAction?: (choice: Choice) => void;
    elapsedSeconds?: number;
  } = $props();

  import { getPentagonEdges, getPlayerEdgeOrder, getPlayerPosition, type EdgeInfo } from './pentagonLayout';

  // Local player is always index 0 in local games.
  // For the 3D view, we always show from the perspective of player 0.
  let localPlayerIndex = 0;

  let localPlayer = $derived(gameState.players[localPlayerIndex]);

  let opponentIndices = $derived(
    gameState.players.map((_, i) => i).filter(i => i !== localPlayerIndex)
  );

  // Pentagon edge layout for all players
  const pentagonEdges = getPentagonEdges();

  // Get edge assignments: local player edge first, then opponent edges
  let edgeOrder = $derived(getPlayerEdgeOrder(gameState.players.length));

  // Local player's edge info
  let localEdge = $derived(pentagonEdges[edgeOrder[0]]);

  // Get opponent edge info by opponent index (0-based among opponents)
  function getOpponentEdge(opponentIndex: number): EdgeInfo {
    // opponentIndex 0 maps to edgeOrder[1], etc.
    return pentagonEdges[edgeOrder[opponentIndex + 1]];
  }

  // Mix mode state for PlayerArea
  import { canMix, mixResult, ALL_COLORS } from '../../data/colors';

  let actionState = $derived(
    gameState.phase.type === 'action' ? gameState.phase.actionState : null
  );

  let topAbility = $derived(
    actionState && actionState.abilityStack.length > 0
      ? actionState.abilityStack[actionState.abilityStack.length - 1]
      : null
  );

  let isMixMode = $derived(topAbility?.type === 'mixColors');

  // Mix mode state lifted from MixColorPrompt3D so PlayerArea wheel can be interactive
  let plannedMixes: [Color, Color][] = $state([]);
  let selectedMixColors: Color[] = $state([]);
  let simulatedWheel: Record<Color, number> = $state(
    Object.fromEntries(ALL_COLORS.map(c => [c, 0])) as Record<Color, number>
  );

  // Reset mix state when entering/leaving mix mode
  $effect(() => {
    if (isMixMode && localPlayer) {
      simulatedWheel = { ...localPlayer.colorWheel };
      plannedMixes = [];
      selectedMixColors = [];
    }
  });

  function handleUndoMix() {
    if (plannedMixes.length === 0 || !localPlayer) return;
    const newWheel = { ...localPlayer.colorWheel };
    const newMixes = plannedMixes.slice(0, -1);
    for (const [a, b] of newMixes) {
      const result = mixResult(a, b);
      newWheel[a]--;
      newWheel[b]--;
      newWheel[result]++;
    }
    simulatedWheel = newWheel;
    plannedMixes = newMixes;
    selectedMixColors = [];
  }

  function handleMixSegmentClick(color: Color) {
    if (!isMixMode || !topAbility || topAbility.type !== 'mixColors') return;
    const remaining = topAbility.count;

    if (selectedMixColors.length === 0) {
      selectedMixColors = [color];
    } else if (selectedMixColors.length === 1) {
      const first = selectedMixColors[0];
      if (first === color) {
        selectedMixColors = [];
      } else if (canMix(first, color) && simulatedWheel[first] > 0 && simulatedWheel[color] > 0) {
        const result = mixResult(first, color);
        const newWheel = { ...simulatedWheel };
        newWheel[first]--;
        newWheel[color]--;
        newWheel[result]++;
        simulatedWheel = newWheel;
        plannedMixes = [...plannedMixes, [first, color]];
        selectedMixColors = [];

        if (plannedMixes.length === remaining && onAction) {
          onAction({ type: 'mixAll', mixes: plannedMixes });
        }
      } else {
        selectedMixColors = [color];
      }
    }
  }
</script>

<!-- Camera -->
<T.PerspectiveCamera
  makeDefault
  position={[0, 4.0, 5.0]}
  fov={55}
  near={0.1}
  far={50}
>
  <OrbitControls
    enablePan={false}
    enableZoom={true}
    enableRotate={true}
    minDistance={2}
    maxDistance={12}
    maxPolarAngle={Math.PI / 2.2}
    minPolarAngle={Math.PI / 6}
    target={[0, 0.0, 0.3]}
  />
</T.PerspectiveCamera>

<!-- Fog -->
<T.Fog args={['#e8f0f8', 18, 35]} attach="fog" />

<!-- Environment -->
<Lighting />
<Room />
<Table />
<CandleGroup />
<Shelves />
<DryingLine />
<Workbench />
<WindowArea />
<WallTextiles />

<CandleSmoke />

<!-- Round info floating above table -->
<RoundInfo3D round={gameState.round} {elapsedSeconds} />

<!-- Buyer display in center of table -->
<BuyerDisplay3D buyers={gameState.buyerDisplay} {onAction} {gameState} />

<!-- Local player group (inset from edge so tableau sits on table surface) -->
{#if localPlayer && localEdge}
  {@const localPos = getPlayerPosition(localEdge)}
  <T.Group position={localPos} rotation.y={localEdge.rotationY}>
    <!-- Tableau (pigments, materials, buyers, deck) -->
    <!-- Note: draftedCards/workshopCards omitted for local player since DraftZone3D/ActionZone3D handle them interactively -->
    <PlayerTableau3D
      colorWheel={isMixMode && simulatedWheel ? simulatedWheel : localPlayer.colorWheel}
      materials={localPlayer.materials}
      completedBuyers={localPlayer.completedBuyers}
      deckSize={localPlayer.deck.length + localPlayer.discard.length}
      position={[0, 0, 0]}
      interactive={true}
      mixMode={isMixMode}
      onPigmentClick={isMixMode ? handleMixSegmentClick : undefined}
      {selectedMixColors}
    />

    <!-- Draft/Action zone rendered in local player space -->
    {#if gameState.phase.type === 'draft'}
      <DraftZone3D {gameState} {onAction} />
    {:else if gameState.phase.type === 'action'}
      <ActionZone3D {gameState} {onAction} />
      {#if topAbility && topAbility.type !== 'mixColors'}
        <AbilityPrompt3D {gameState} onAction={onAction!} />
      {/if}
    {/if}

    <!-- Player area (ducats, mix buttons) on the outer edge side -->
    <PlayerArea
      player={localPlayer}
      playerName={gameState.playerNames[localPlayerIndex]}
      position={[0, 0, 0.9]}
      mixMode={isMixMode}
      {plannedMixes}
      onAction={onAction}
      onUndo={handleUndoMix}
      mixRemaining={topAbility?.type === 'mixColors' ? topAbility.count - plannedMixes.length : 0}
    />
  </T.Group>
{/if}

<!-- Opponent areas (also inset from edge) -->
{#each opponentIndices as oppIdx, i}
  {@const edge = getOpponentEdge(i)}
  <OpponentArea
    player={gameState.players[oppIdx]}
    playerName={gameState.playerNames[oppIdx]}
    position={getPlayerPosition(edge)}
    rotation={edge.rotationY}
    isAI={gameState.aiPlayers[oppIdx]}
  />
{/each}

<!-- Post-processing -->
<Effects />
