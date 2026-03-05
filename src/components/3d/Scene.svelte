<script lang="ts">
  import { Canvas } from '@threlte/core';
  import type { GameState, Choice } from '../../data/types';
  import SceneContent from './SceneContent.svelte';

  let { gameState, activePlayerIndex, onAction, elapsedSeconds = 0 }: {
    gameState: GameState;
    activePlayerIndex: number;
    onAction?: (choice: Choice) => void;
    elapsedSeconds?: number;
  } = $props();
</script>

<Canvas>
  <SceneContent {gameState} {activePlayerIndex} {onAction} {elapsedSeconds} />
</Canvas>
