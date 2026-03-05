<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { PlayerState, Color, Choice } from '../../../data/types';
  import Card3D from '../cards/Card3D.svelte';

  let { player, playerName, position = [0, 0.07, 2.2], mixMode = false, plannedMixes = [], onAction, onUndo, mixRemaining = 0 }: {
    player: PlayerState;
    playerName: string;
    position?: [number, number, number];
    mixMode?: boolean;
    plannedMixes?: [Color, Color][];
    onAction?: (choice: Choice) => void;
    onUndo?: () => void;
    mixRemaining?: number;
  } = $props();

  // Mix mode button hover states
  let submitHovered = $state(false);
  let undoHovered = $state(false);
</script>

<T.Group position={position}>
  <!-- Ducats display -->
  {#if player.ducats > 0}
    <T.Group position={[0.2, 0, 0]}>
      {#each Array(Math.min(player.ducats, 8)) as _, ci}
        <T.Mesh
          position={[ci * 0.08, 0.02, 0]}
          castShadow
        >
          <T.CylinderGeometry args={[0.04, 0.04, 0.015, 12]} />
          <T.MeshStandardMaterial
            color="#d4a017"
            roughness={0.3}
            metalness={0.6}
          />
        </T.Mesh>
      {/each}
      <Text
        text={`${player.ducats} Ducats`}
        position={[0.15, 0.15, 0]}
        fontSize={0.04}
        color="#ffd700"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.002}
        outlineColor="#000000"
        fontWeight="bold"
      />
    </T.Group>
  {/if}

  <!-- Completed buyers row -->
  {#each player.completedBuyers as bi, i}
    <Card3D
      buyerCard={bi.card}
      position={[0.8 + i * 0.4, 0.03, 0]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={0.5}
    />
  {/each}

  <!-- Mix mode UI -->
  {#if mixMode}
    <!-- Header -->
    <Text
      text={`Mix Colors (${mixRemaining} remaining)`}
      position={[-1.2, 0.7, 0]}
      fontSize={0.06}
      color="#ffe8cc"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.003}
      outlineColor="#1a1410"
      fontWeight="bold"
    />
    <Text
      text="Click two pigments on the wheel to mix"
      position={[-1.2, 0.6, 0]}
      fontSize={0.035}
      color="#aa9988"
      anchorX="center"
      anchorY="middle"
    />

    <!-- Submit / Skip button -->
    <T.Group position={[-0.5, 0.15, 0.5]}>
      <T.Mesh
        onclick={() => onAction?.({ type: 'mixAll', mixes: plannedMixes })}
        onpointerenter={() => { submitHovered = true; document.body.style.cursor = 'pointer'; }}
        onpointerleave={() => { submitHovered = false; document.body.style.cursor = 'auto'; }}
      >
        <T.BoxGeometry args={[0.35, 0.1, 0.12]} />
        <T.MeshStandardMaterial
          color={submitHovered ? '#3cb525' : '#2a6b14'}
          roughness={0.5}
          emissive={submitHovered ? '#3cb525' : '#000000'}
          emissiveIntensity={submitHovered ? 0.3 : 0}
        />
      </T.Mesh>
      <Text
        text={plannedMixes.length > 0 ? 'Submit' : 'Skip'}
        position={[0, 0.06, 0]}
        fontSize={0.04}
        color="#ffffff"
        anchorX="center"
        anchorY="middle"
        fontWeight="bold"
      />
    </T.Group>

    <!-- Undo button -->
    {#if plannedMixes.length > 0}
      <T.Group position={[-1.9, 0.15, 0.5]}>
        <T.Mesh
          onclick={() => onUndo?.()}
          onpointerenter={() => { undoHovered = true; document.body.style.cursor = 'pointer'; }}
          onpointerleave={() => { undoHovered = false; document.body.style.cursor = 'auto'; }}
        >
          <T.BoxGeometry args={[0.25, 0.1, 0.12]} />
          <T.MeshStandardMaterial
            color={undoHovered ? '#888' : '#555'}
            roughness={0.5}
            emissive={undoHovered ? '#888' : '#000000'}
            emissiveIntensity={undoHovered ? 0.2 : 0}
          />
        </T.Mesh>
        <Text
          text="Undo"
          position={[0, 0.06, 0]}
          fontSize={0.04}
          color="#ffffff"
          anchorX="center"
          anchorY="middle"
          fontWeight="bold"
        />
      </T.Group>
    {/if}
  {/if}
</T.Group>
