<script lang="ts">
  import type { CardInstance } from '../../../data/types';
  import Card3D from './Card3D.svelte';
  import { spring } from 'svelte/motion';

  let { cards, interactive = false, selectedCard, onCardClick }: {
    cards: CardInstance[];
    interactive?: boolean;
    selectedCard?: string;
    onCardClick?: (card: CardInstance) => void;
  } = $props();

  // Fan spread arc layout with natural curve
  const ARC_RADIUS = 4.0;
  const MAX_TOTAL_ANGLE = Math.PI / 3.5; // ~51 degrees (tighter)
  const CARD_TILT = Math.PI / 18;        // 10 degrees inward tilt
  const BASE_Y = 0.15;                   // Table surface + offset
  const BASE_Z = 1.5;                    // Near player edge
  const VERTICAL_CURVE = 0.08;           // Cards curve down at edges

  interface CardTransform {
    position: [number, number, number];
    rotation: [number, number, number];
  }

  let layout = $derived(computeLayout(cards.length));

  // Animated card count for draw transitions
  const smoothCount = spring(0, { stiffness: 0.08, damping: 0.6 });

  $effect(() => {
    smoothCount.set(cards.length);
  });

  function computeLayout(count: number): CardTransform[] {
    if (count === 0) return [];
    const totalAngle = Math.min(MAX_TOTAL_ANGLE, count * (Math.PI / 40));
    const startAngle = -totalAngle / 2;
    const angleStep = count > 1 ? totalAngle / (count - 1) : 0;
    const centerIndex = (count - 1) / 2;

    return Array.from({ length: count }, (_, i) => {
      const angle = startAngle + i * angleStep;
      const distFromCenter = Math.abs(i - centerIndex) / Math.max(centerIndex, 1);
      // Cards at edges dip down slightly for a natural hand feel
      const verticalDip = distFromCenter * distFromCenter * VERTICAL_CURVE;
      const x = ARC_RADIUS * Math.sin(angle);
      const y = BASE_Y + ARC_RADIUS * (1 - Math.cos(angle)) * 0.3 - verticalDip + i * 0.001;
      const z = BASE_Z - distFromCenter * 0.05; // Edges slightly closer

      return {
        position: [x, y, z] as [number, number, number],
        rotation: [CARD_TILT, 0, -angle] as [number, number, number],
      };
    });
  }
</script>

{#each cards as ci, i}
  {@const transform = layout[i]}
  {#if transform}
    <Card3D
      card={ci.card}
      position={transform.position}
      rotation={transform.rotation}
      {interactive}
      selected={selectedCard === ci.card}
      faceUp={true}
      onclick={() => onCardClick?.(ci)}
    />
  {/if}
{/each}
