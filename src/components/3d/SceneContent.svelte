<script lang="ts">
  import { T, useTask } from '@threlte/core';
  import { OrbitControls } from '@threlte/extras';
  import type { GameState, Choice } from '../../data/types';
  import * as THREE from 'three';

  import Room from './environment/Room.svelte';
  import Table from './environment/Table.svelte';
  import Lighting from './environment/Lighting.svelte';
  import CandleGroup from './environment/CandleGroup.svelte';
  import Shelves from './environment/Shelves.svelte';
  import DryingLine from './environment/DryingLine.svelte';
  import Workbench from './environment/Workbench.svelte';
  import WindowArea from './environment/WindowArea.svelte';
  import DustMotes from './particles/DustMotes.svelte';
  import AmbientDust from './particles/AmbientDust.svelte';
  import CandleSmoke from './particles/CandleSmoke.svelte';
  import Effects from './postprocessing/Effects.svelte';
  import CardHand3D from './cards/CardHand3D.svelte';
  import BuyerDisplay3D from './cards/BuyerDisplay3D.svelte';
  import DraftZone3D from './cards/DraftZone3D.svelte';
  import ActionZone3D from './cards/ActionZone3D.svelte';
  import PlayerArea from './players/PlayerArea.svelte';
  import OpponentArea from './players/OpponentArea.svelte';

  let { gameState, activePlayerIndex, onAction }: {
    gameState: GameState;
    activePlayerIndex: number;
    onAction?: (choice: Choice) => void;
  } = $props();

  // Camera idle sway
  let cameraRef: THREE.PerspectiveCamera | undefined = $state();
  let time = 0;

  useTask((delta) => {
    time += delta;
    if (cameraRef) {
      const baseX = 0;
      const baseY = 2.0;
      const baseZ = 4.5;
      cameraRef.position.x = baseX + Math.sin(time * 0.15) * 0.03;
      cameraRef.position.y = baseY + Math.sin(time * 0.2) * 0.015;
      cameraRef.position.z = baseZ + Math.cos(time * 0.12) * 0.02;
    }
  });

  // Determine which player areas are opponents
  let localPlayerIndex = $derived(
    gameState.phase.type === 'draft' ? gameState.phase.draftState.currentPlayerIndex :
    gameState.phase.type === 'action' ? gameState.phase.actionState.currentPlayerIndex :
    0
  );

  let opponentIndices = $derived(
    gameState.players.map((_, i) => i).filter(i => i !== localPlayerIndex)
  );

  // Opponent seat positions around the table
  function getOpponentPosition(opponentIndex: number, totalOpponents: number): [number, number, number] {
    const positions: [number, number, number][] = [
      [0, 0.07, -1.7],       // center-back (across)
      [-2.5, 0.07, -1.3],    // left-back
      [2.5, 0.07, -1.3],     // right-back
      [-3, 0.07, 0],          // left-side
    ];
    return positions[opponentIndex % positions.length];
  }
</script>

<!-- Camera -->
<T.PerspectiveCamera
  bind:ref={cameraRef}
  makeDefault
  position={[0, 2.0, 4.5]}
  fov={55}
  near={0.1}
  far={50}
>
  <OrbitControls
    enablePan={false}
    enableZoom={true}
    enableRotate={true}
    minDistance={2}
    maxDistance={10}
    maxPolarAngle={Math.PI / 2.2}
    minPolarAngle={Math.PI / 6}
    target={[0, 0.1, -0.5]}
  />
</T.PerspectiveCamera>

<!-- Fog -->
<T.Fog args={['#1a1410', 8, 18]} attach="fog" />

<!-- Environment -->
<Lighting />
<Room />
<Table />
<CandleGroup />
<Shelves />
<DryingLine />
<Workbench />
<WindowArea />

<!-- Particles -->
<DustMotes />
<AmbientDust />
<CandleSmoke />

<!-- Game Objects -->
{#if gameState.phase.type === 'draft'}
  <DraftZone3D {gameState} {onAction} />
{:else if gameState.phase.type === 'action'}
  <ActionZone3D {gameState} {onAction} />
{/if}

<!-- Buyer display in center of table -->
<BuyerDisplay3D buyers={gameState.buyerDisplay} {onAction} {gameState} />

<!-- Opponent areas -->
{#each opponentIndices as oppIdx, i}
  <OpponentArea
    player={gameState.players[oppIdx]}
    playerName={gameState.playerNames[oppIdx]}
    position={getOpponentPosition(i, opponentIndices.length)}
    isAI={gameState.aiPlayers[oppIdx]}
  />
{/each}

<!-- Post-processing -->
<Effects />
