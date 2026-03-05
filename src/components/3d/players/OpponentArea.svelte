<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { PlayerState } from '../../../data/types';
  import PlayerTableau3D from './PlayerTableau3D.svelte';

  let { player, playerName, position = [0, 0.07, -1.7], rotation = 0, isAI = false }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
    rotation?: number;
    isAI?: boolean;
  } = $props();

  let deckSize = $derived(player.deck.length + player.discard.length);
  let score = $derived(player.completedBuyers.length);
</script>

<T.Group position={position} rotation.y={rotation}>
  <!-- Name label -->
  <Text
    text={`${playerName}${isAI ? ' (AI)' : ''}`}
    position={[0, 0.35, 0]}
    fontSize={0.06}
    color="#ffe8cc"
    anchorX="center"
    anchorY="middle"
    outlineWidth={0.003}
    outlineColor="#1a1410"
    fontWeight="bold"
  />

  <!-- Score label -->
  {#if score > 0}
    <Text
      text={`Score: ${score}`}
      position={[0, 0.28, 0]}
      fontSize={0.045}
      color="#ffd700"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.002}
      outlineColor="#1a1410"
      fontWeight="bold"
    />
  {/if}

  <!-- Tableau with all card zones integrated -->
  <PlayerTableau3D
    colorWheel={player.colorWheel}
    materials={player.materials}
    draftedCards={player.draftedCards}
    workshopCards={player.workshopCards}
    completedBuyers={player.completedBuyers}
    {deckSize}
    position={[0, 0, 0]}
    scale={1}
    interactive={false}
  />
</T.Group>
