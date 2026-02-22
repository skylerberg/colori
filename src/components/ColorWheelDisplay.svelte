<script lang="ts">
  import type { Color } from '../data/types';
  import { colorToHex, textColorForBackground } from '../data/colors';

  interface WheelSegment {
    color: Color;
    ring: 'inner' | 'middle' | 'outer';
    startAngleDeg: number;
    spanDeg: number;
    isExtension?: boolean;
  }

  const WHEEL_SEGMENTS: WheelSegment[] = [
    // Inner ring - primaries (120° each)
    { color: 'Red', ring: 'inner', startAngleDeg: 300, spanDeg: 120 },
    { color: 'Yellow', ring: 'inner', startAngleDeg: 60, spanDeg: 120 },
    { color: 'Blue', ring: 'inner', startAngleDeg: 180, spanDeg: 120 },
    // Middle ring - primary extensions and secondaries (60° each)
    { color: 'Red', ring: 'middle', startAngleDeg: 330, spanDeg: 60, isExtension: true },
    { color: 'Orange', ring: 'middle', startAngleDeg: 30, spanDeg: 60 },
    { color: 'Yellow', ring: 'middle', startAngleDeg: 90, spanDeg: 60, isExtension: true },
    { color: 'Green', ring: 'middle', startAngleDeg: 150, spanDeg: 60 },
    { color: 'Blue', ring: 'middle', startAngleDeg: 210, spanDeg: 60, isExtension: true },
    { color: 'Purple', ring: 'middle', startAngleDeg: 270, spanDeg: 60 },
    // Outer ring - tertiaries (60° each)
    { color: 'Vermilion', ring: 'outer', startAngleDeg: 0, spanDeg: 60 },
    { color: 'Amber', ring: 'outer', startAngleDeg: 60, spanDeg: 60 },
    { color: 'Chartreuse', ring: 'outer', startAngleDeg: 120, spanDeg: 60 },
    { color: 'Teal', ring: 'outer', startAngleDeg: 180, spanDeg: 60 },
    { color: 'Indigo', ring: 'outer', startAngleDeg: 240, spanDeg: 60 },
    { color: 'Magenta', ring: 'outer', startAngleDeg: 300, spanDeg: 60 },
  ];

  const RING_RADII: Record<string, { inner: number; outer: number }> = {
    inner: { inner: 0.12, outer: 0.38 },
    middle: { inner: 0.42, outer: 0.64 },
    outer: { inner: 0.68, outer: 0.90 },
  };

  const FONT_SCALE: Record<string, number> = {
    inner: 0.07,
    middle: 0.055,
    outer: 0.045,
  };

  let { wheel, interactive = false, onColorClick, selectedColors = [], size = 200 }: {
    wheel: Record<Color, number>;
    interactive?: boolean;
    onColorClick?: (color: Color) => void;
    selectedColors?: Color[];
    size?: number;
  } = $props();

  let half = $derived(size / 2);

  function toRad(deg: number): number {
    return (deg * Math.PI) / 180;
  }

  function pointAt(angleDeg: number, radius: number): { x: number; y: number } {
    const rad = toRad(angleDeg);
    return {
      x: half + radius * Math.sin(rad),
      y: half - radius * Math.cos(rad),
    };
  }

  function segmentPath(seg: WheelSegment): string {
    const radii = RING_RADII[seg.ring];
    const rInner = half * radii.inner;
    const rOuter = half * radii.outer;
    const endDeg = seg.startAngleDeg + seg.spanDeg;
    const largeArc = seg.spanDeg > 180 ? 1 : 0;

    const p1 = pointAt(seg.startAngleDeg, rOuter);
    const p2 = pointAt(endDeg, rOuter);
    const p3 = pointAt(endDeg, rInner);
    const p4 = pointAt(seg.startAngleDeg, rInner);

    return [
      `M ${p1.x} ${p1.y}`,
      `A ${rOuter} ${rOuter} 0 ${largeArc} 1 ${p2.x} ${p2.y}`,
      `L ${p3.x} ${p3.y}`,
      `A ${rInner} ${rInner} 0 ${largeArc} 0 ${p4.x} ${p4.y}`,
      `Z`,
    ].join(' ');
  }

  function labelPos(seg: WheelSegment): { x: number; y: number } {
    const radii = RING_RADII[seg.ring];
    const rMid = half * (radii.inner + radii.outer) / 2;
    const midAngle = seg.startAngleDeg + seg.spanDeg / 2;
    return pointAt(midAngle, rMid);
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
    {#each WHEEL_SEGMENTS as seg}
      {@const pos = labelPos(seg)}
      {@const fontSize = Math.round(size * FONT_SCALE[seg.ring])}
      <path
        d={segmentPath(seg)}
        fill={colorToHex(seg.color)}
        stroke={isSelected(seg.color) ? '#fff' : '#333'}
        stroke-width={isSelected(seg.color) ? 3 : 0.5}
        opacity={wheel[seg.color] > 0 ? 1 : 0.3}
        class:clickable={interactive && wheel[seg.color] > 0}
        onclick={() => handleClick(seg.color)}
        role={interactive && wheel[seg.color] > 0 ? 'button' : undefined}
        tabindex={interactive && wheel[seg.color] > 0 ? 0 : undefined}
      />
      {#if !seg.isExtension}
        <text
          x={pos.x}
          y={pos.y}
          text-anchor="middle"
          dominant-baseline="central"
          font-size={fontSize}
          font-weight="bold"
          fill={textColorForBackground(colorToHex(seg.color))}
          pointer-events="none"
        >
          {seg.color[0]}
        </text>
        {#if wheel[seg.color] > 0 && size >= 140}
          <text
            x={pos.x}
            y={pos.y + fontSize * 0.9}
            text-anchor="middle"
            dominant-baseline="central"
            font-size={Math.round(fontSize * 0.75)}
            font-weight="bold"
            fill={textColorForBackground(colorToHex(seg.color))}
            pointer-events="none"
          >
            {wheel[seg.color]}
          </text>
        {/if}
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
