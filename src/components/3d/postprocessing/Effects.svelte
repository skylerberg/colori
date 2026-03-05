<script lang="ts">
  import { useThrelte } from '@threlte/core';
  import { onMount, onDestroy } from 'svelte';
  import { EffectComposer, RenderPass, EffectPass, BloomEffect, VignetteEffect } from 'postprocessing';

  const { renderer, scene, camera } = useThrelte();

  let composer: EffectComposer | undefined;
  let animationFrame: number;

  onMount(() => {
    if (!renderer || !scene || !camera.current) return;

    composer = new EffectComposer(renderer);
    composer.addPass(new RenderPass(scene, camera.current));

    const bloomEffect = new BloomEffect({
      intensity: 0.35,
      luminanceThreshold: 0.7,
      luminanceSmoothing: 0.9,
      mipmapBlur: true,
    });

    const vignetteEffect = new VignetteEffect({
      offset: 0.35,
      darkness: 0.55,
    });

    composer.addPass(new EffectPass(camera.current, bloomEffect, vignetteEffect));

    function render() {
      if (composer && camera.current) {
        composer.render();
      }
      animationFrame = requestAnimationFrame(render);
    }
    render();
  });

  onDestroy(() => {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    composer?.dispose();
  });
</script>
