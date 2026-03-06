<script lang="ts">
  import type { Color } from '../data/types';
  import { colorToHex, textColorForBackground } from '../data/colors';
  import { WORKSHOP_WHEEL_REGIONS } from '../data/wheelPaths';

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

  // Compute label positions using polar coordinates from wheel center
  const WC = 128; // wheel center in 256 viewBox
  const LABEL_RADIUS = { primary: 48, secondary: 70, tertiary: 100 };
  const COLOR_LABEL: Record<string, { angle: number; ring: 'primary' | 'secondary' | 'tertiary' }> = {
    'Red': { angle: 0, ring: 'primary' },
    'Yellow': { angle: 120, ring: 'primary' },
    'Blue': { angle: 240, ring: 'primary' },
    'Orange': { angle: 60, ring: 'secondary' },
    'Green': { angle: 180, ring: 'secondary' },
    'Purple': { angle: 300, ring: 'secondary' },
    'Vermilion': { angle: 30, ring: 'tertiary' },
    'Amber': { angle: 90, ring: 'tertiary' },
    'Chartreuse': { angle: 150, ring: 'tertiary' },
    'Teal': { angle: 210, ring: 'tertiary' },
    'Indigo': { angle: 270, ring: 'tertiary' },
    'Magenta': { angle: 330, ring: 'tertiary' },
  };

  function labelXY(color: string): [number, number] {
    const info = COLOR_LABEL[color];
    const r = LABEL_RADIUS[info.ring];
    const rad = (info.angle * Math.PI) / 180;
    return [WC + r * Math.sin(rad), WC - r * Math.cos(rad)];
  }

  let { wheel, interactive = false, onColorClick, selectedColors = [], size = 200, hidden = false }: {
    wheel: Record<Color, number>;
    interactive?: boolean;
    onColorClick?: (color: Color) => void;
    selectedColors?: Color[];
    size?: number;
    hidden?: boolean;
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

<div class="color-wheel-container" class:hidden>
    <svg width={size} height={size} viewBox="0 0 256 256" fill="none">
      <!-- Filled color regions (behind strokes) -->
      <!-- Inner ring: primaries -->
      <path d="M 92.68 53.92 A 82 82 0 0 1 175.23 61.43 C 167.13 77.91 163.95 92.81 167.29 109.96 L 127.36 129.67 L 92.28 108.76 C 104.97 92.97 103.93 75.85 92.68 53.92 Z" fill="#e63946" />
      <path d="M 127.36 129.67 L 167.29 109.96 C 172.12 129.99 186.94 142.29 206.61 147.99 A 82 82 0 0 1 165.72 200.2 C 156.42 185.38 143.41 173.81 127.36 173.81 Z" fill="#f2e205" />
      <path d="M 127.36 129.67 L 127.36 173.81 C 111.47 173.81 98.54 182.04 86.81 198.76 A 82 82 0 0 1 46.15 126.82 C 66.13 124.41 80.07 121.06 92.28 108.76 Z" fill="#3b82f6" />
      <!-- Secondary lobes: crescent shapes between curved dividers and inner circle -->
      <!-- Orange: lobe between Red and Yellow (path 119 curve + circle arc back) -->
      <path d="M 175.23 61.43 C 167.13 77.91 163.95 92.81 167.29 109.96 C 172.12 129.99 186.94 142.29 206.61 147.99 A 82 82 0 0 0 175.23 61.43 Z" fill="#f4a261" />
      <!-- Purple: lobe between Red and Blue (path 120 curve + circle arc back) -->
      <path d="M 92.68 53.92 C 103.93 75.85 104.97 92.97 92.28 108.76 C 80.07 121.06 66.13 124.41 46.15 126.82 A 82 82 0 0 1 92.68 53.92 Z" fill="#a855f7" />
      <!-- Green: lobe between Yellow and Blue (path 123 curve + circle arc back) -->
      <path d="M 86.81 198.76 C 98.54 182.04 111.47 173.81 127.36 173.81 C 143.41 173.81 156.42 185.38 165.72 200.2 A 82 82 0 0 1 86.81 198.76 Z" fill="#2ecc71" />
      <!-- Outer ring: tertiaries -->
      <path d="M 127.44 6.74 A 122 122 0 0 1 245.67 97.13 L 204.48 99.46 A 82 82 0 0 0 127.44 46.22 Z" fill="#e76f51" />
      <path d="M 245.67 97.13 A 122 122 0 0 1 222.29 203.86 L 188.73 183.32 A 82 82 0 0 0 204.48 99.46 Z" fill="#e9c46a" />
      <path d="M 222.29 203.86 A 122 122 0 0 1 127.44 249.67 L 127.44 210.02 A 82 82 0 0 0 188.73 183.32 Z" fill="#a7c957" />
      <path d="M 127.44 249.67 A 122 122 0 0 1 25.71 195.17 L 58.95 172.61 A 82 82 0 0 0 127.44 210.02 Z" fill="#219ebc" />
      <path d="M 25.71 195.17 A 122 122 0 0 1 23.62 63.57 L 59.44 82.91 A 82 82 0 0 0 58.95 172.61 Z" fill="#6366f1" />
      <path d="M 23.62 63.57 A 122 122 0 0 1 127.44 6.74 L 127.44 46.22 A 82 82 0 0 0 59.44 82.91 Z" fill="#d63384" />
      <!-- Stroke paths (hand-drawn lines) -->
      <path d="M 127.68,6.41 C 60.93,6.41 6,60.96 6,128.42 C 6,195.61 60.69,250 127.44,250 C 194.69,250 249.72,195.45 249.72,128.42 C 249.72,61.23 195.29,6.41 127.68,6.41 Z" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 127.44,6.74 V 46.22" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 245.67,97.13 L 204.48,99.46" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 222.29,203.86 L 188.73,183.32" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 127.44,249.67 V 210.02" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 25.71,195.17 L 58.95,172.61" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 23.62,63.57 L 59.44,82.91" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 127.44,209.7 C 174.99,209.7 209.31,170.55 209.31,128.34 C 209.31,85.28 174.75,46.22 127.44,46.22 C 80.87,46.22 46.15,83.87 46.15,128.34 C 46.15,172.04 80.87,209.7 127.44,209.7 Z" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 175.23,61.43 C 167.13,77.91 163.95,92.81 167.29,109.96 C 172.12,129.99 186.94,142.29 206.61,147.99" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 92.68,53.92 C 103.93,75.85 104.97,92.97 92.28,108.76 C 80.07,121.06 66.13,124.41 46.15,126.82" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 92.28,108.76 L 127.36,129.67 L 167.61,112.1" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 127.36,129.67 L 128.4,173.81" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <path d="M 86.81,198.76 C 98.54,182.04 111.47,173.81 127.36,173.81 C 143.41,173.81 156.42,185.38 165.72,200.2" stroke="black" stroke-width="1.7" stroke-miterlimit="10"/>
      <!-- Interactive hit targets (invisible, on top of strokes) -->
      {#if interactive}
        {#each WORKSHOP_WHEEL_REGIONS as region}
          <path
            d={region.path}
            fill="transparent"
            class:clickable={interactive && wheel[region.color] > 0}
            class:selected={isSelected(region.color)}
            onclick={() => handleClick(region.color)}
          />
        {/each}
      {/if}
      <!-- Color count labels -->
      {#each WORKSHOP_WHEEL_REGIONS as region, i}
        {@const count = wheel[region.color]}
        {@const [cx, cy] = labelXY(region.color)}
        {@const hex = colorToHex(region.color)}
        {@const textColor = textColorForBackground(hex)}
        {@const countSize = i < 3 ? 22 : i < 6 ? 14 : 13}
        <text
          x={cx}
          y={cy}
          text-anchor="middle"
          dominant-baseline="central"
          fill={textColor}
          font-size={countSize}
          font-weight="700"
          font-family="Cinzel, serif"
          opacity={count > 0 ? 1 : 0.25}
          style="pointer-events: none;"
        >{count}</text>
      {/each}
    </svg>
</div>

<style>
  .color-wheel-container {
    display: inline-block;
    filter: drop-shadow(0 2px 6px rgba(44, 30, 18, 0.2));
  }

  .color-wheel-container.hidden {
    filter: none;
  }

  path.clickable {
    cursor: pointer;
  }

  path.clickable:hover {
    fill: rgba(255, 255, 255, 0.15);
  }

  path.selected {
    fill: rgba(201, 168, 76, 0.3);
  }
</style>
