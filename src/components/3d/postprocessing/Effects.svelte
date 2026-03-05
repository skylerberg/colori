<script lang="ts">
  import { useThrelte } from '@threlte/core';
  import * as THREE from 'three';
  import { onMount, onDestroy } from 'svelte';

  const { renderer } = useThrelte();

  // Apply color grading via renderer tone mapping instead of post-processing passes.
  // This avoids extra render passes entirely — zero GPU overhead.
  let previousToneMapping: THREE.ToneMapping | undefined;
  let previousExposure: number | undefined;

  onMount(() => {
    if (!renderer) return;
    previousToneMapping = renderer.toneMapping;
    previousExposure = renderer.toneMappingExposure;

    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
  });

  onDestroy(() => {
    if (!renderer) return;
    if (previousToneMapping !== undefined) renderer.toneMapping = previousToneMapping;
    if (previousExposure !== undefined) renderer.toneMappingExposure = previousExposure;
  });
</script>
