<script lang="ts">
  import { T } from '@threlte/core';
  import { HTML } from '@threlte/extras';
  import type { PlayerState } from '../../../data/types';
  import Card3D from '../cards/Card3D.svelte';

  let { player, playerName, position = [0, 0.07, -1.7], isAI = false }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
    isAI?: boolean;
  } = $props();

  // Show face-down stack representing their deck
  let deckSize = $derived(player.deck.length + player.discard.length);
  let completedBuyers = $derived(player.completedBuyers);
  let draftedCards = $derived(player.draftedCards);
</script>

<T.Group position={position}>
  <!-- Name label floating above -->
  <HTML position={[0, 0.5, 0]} transform sprite center>
    <div style="
      background: rgba(26, 20, 16, 0.8);
      color: #ffe8cc;
      padding: 2px 8px;
      border-radius: 4px;
      font-size: 11px;
      font-weight: 600;
      white-space: nowrap;
      backdrop-filter: blur(4px);
    ">
      {playerName}{isAI ? ' (AI)' : ''}
    </div>
  </HTML>

  <!-- Face-down deck stack -->
  {#if deckSize > 0}
    <T.Mesh position={[-0.4, 0.04, 0]} castShadow>
      <T.BoxGeometry args={[0.4, Math.min(deckSize * 0.003, 0.1), 0.56]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
  {/if}

  <!-- Drafted cards visible face-up (game rules: drafted cards are public) -->
  {#each draftedCards.slice(0, 4) as ci, i}
    <Card3D
      card={ci.card}
      position={[-0.1 + i * 0.35, 0.07, 0]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.6}
    />
  {/each}

  <!-- Completed buyers -->
  {#each completedBuyers.slice(0, 3) as bi, i}
    <Card3D
      buyerCard={bi.card}
      position={[-0.1 + i * 0.35, 0.07, 0.5]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.5}
    />
  {/each}
</T.Group>
