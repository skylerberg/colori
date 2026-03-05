<script lang="ts">
  import { Canvas } from '@threlte/core';
  import type { GameState, Choice } from '../../data/types';
  import SceneContent from './SceneContent.svelte';

  let { gameState, activePlayerIndex, onAction }: {
    gameState: GameState;
    activePlayerIndex: number;
    onAction?: (choice: Choice) => void;
  } = $props();
</script>

<Canvas>
  <SceneContent {gameState} {activePlayerIndex} {onAction} />
</Canvas>
