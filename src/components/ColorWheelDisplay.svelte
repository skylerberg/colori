<script lang="ts">
  import type { Color } from '../data/types';
  import { WHEEL_ORDER, colorToHex, textColorForBackground } from '../data/colors';

  let { wheel, interactive = false, onColorClick, selectedColors = [], size = 200 }: {
    wheel: Record<Color, number>;
    interactive?: boolean;
    onColorClick?: (color: Color) => void;
    selectedColors?: Color[];
    size?: number;
  } = $props();

  let cx = $derived(size / 2);
  let cy = $derived(size / 2);
  let outerRadius = $derived(size * 0.425);
  let innerRadius = $derived(size * 0.225);

  function segmentPath(index: number): string {
    const angleStep = (2 * Math.PI) / 12;
    const startAngle = index * angleStep - Math.PI / 2 - angleStep / 2;
    const endAngle = startAngle + angleStep;

    const x1 = cx + outerRadius * Math.cos(startAngle);
    const y1 = cy + outerRadius * Math.sin(startAngle);
    const x2 = cx + outerRadius * Math.cos(endAngle);
    const y2 = cy + outerRadius * Math.sin(endAngle);
    const x3 = cx + innerRadius * Math.cos(endAngle);
    const y3 = cy + innerRadius * Math.sin(endAngle);
    const x4 = cx + innerRadius * Math.cos(startAngle);
    const y4 = cy + innerRadius * Math.sin(startAngle);

    return `M ${x1} ${y1} A ${outerRadius} ${outerRadius} 0 0 1 ${x2} ${y2} L ${x3} ${y3} A ${innerRadius} ${innerRadius} 0 0 0 ${x4} ${y4} Z`;
  }

  function labelPosition(index: number): { x: number; y: number } {
    const angleStep = (2 * Math.PI) / 12;
    const angle = index * angleStep - Math.PI / 2;
    const r = (outerRadius + innerRadius) / 2;
    return { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
  }

  function outerLabelPosition(index: number): { x: number; y: number } {
    const angleStep = (2 * Math.PI) / 12;
    const angle = index * angleStep - Math.PI / 2;
    const r = outerRadius + size * 0.06;
    return { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
  }

  function handleClick(color: Color) {
    if (interactive && onColorClick && wheel[color] > 0) {
      onColorClick(color);
    }
  }

  function isSelected(color: Color): boolean {
    return selectedColors.includes(color);
  }
</script>

<div class="color-wheel-container">
  <svg width={size} height={size} viewBox="0 0 {size} {size}">
    {#each WHEEL_ORDER as color, i}
      {@const pos = labelPosition(i)}
      {@const outerPos = outerLabelPosition(i)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <path
        d={segmentPath(i)}
        fill={colorToHex(color)}
        stroke={isSelected(color) ? '#fff' : '#333'}
        stroke-width={isSelected(color) ? 3 : 1}
        opacity={wheel[color] > 0 ? 1 : 0.3}
        class:clickable={interactive && wheel[color] > 0}
        onclick={() => handleClick(color)}
        role={interactive && wheel[color] > 0 ? 'button' : undefined}
        tabindex={interactive && wheel[color] > 0 ? 0 : undefined}
      />
      <text
        x={pos.x}
        y={pos.y}
        text-anchor="middle"
        dominant-baseline="central"
        font-size={Math.round(size * 0.055)}
        font-weight="bold"
        fill={textColorForBackground(colorToHex(color))}
        pointer-events="none"
      >
        {color[0]}
      </text>
      {#if wheel[color] > 0}
        <text
          x={outerPos.x}
          y={outerPos.y}
          text-anchor="middle"
          dominant-baseline="central"
          font-size={Math.round(size * 0.045)}
          font-weight="bold"
          fill="#333"
          pointer-events="none"
        >
          {wheel[color]}
        </text>
      {/if}
    {/each}
  </svg>
</div>

<style>
  .color-wheel-container {
    display: inline-block;
  }

  path.clickable {
    cursor: pointer;
  }

  path.clickable:hover {
    filter: brightness(1.2);
  }
</style>
