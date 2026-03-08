<script lang="ts">
  import type { Color } from '../data/types';
  import { colorToHex, textColorForBackground } from '../data/colors';

  interface InnerSegment {
    color: Color;
    type: 'primary' | 'secondary';
    startAngle: number;
    endAngle: number;
    bowStart: number;
    bowEnd: number;
  }

  interface OuterSegment {
    color: Color;
    startAngle: number;
    endAngle: number;
  }

  const HALF = 128;
  const INNER_R = 0.64 * HALF;   // ~82
  const OUTER_R = 0.92 * HALF;   // ~118
  const BOW_FACTOR = 0.45;
  const SECONDARY_APEX_R = 0.2 * INNER_R;  // ~16 — how far from center the secondary tips reach

  const PRIMARIES: InnerSegment[] = [
    { color: 'Red',    type: 'primary',   startAngle: 330, endAngle: 30,  bowStart: 0,   bowEnd: 0 },
    { color: 'Yellow', type: 'primary',   startAngle: 90,  endAngle: 150, bowStart: 120, bowEnd: 120 },
    { color: 'Blue',   type: 'primary',   startAngle: 210, endAngle: 270, bowStart: 240, bowEnd: 240 },
  ];

  const SECONDARIES: InnerSegment[] = [
    { color: 'Orange', type: 'secondary', startAngle: 30,  endAngle: 90,  bowStart: 0,   bowEnd: 120 },
    { color: 'Green',  type: 'secondary', startAngle: 150, endAngle: 210, bowStart: 120, bowEnd: 240 },
    { color: 'Purple', type: 'secondary', startAngle: 270, endAngle: 330, bowStart: 240, bowEnd: 0 },
  ];

  const ALL_INNER = [...PRIMARIES, ...SECONDARIES];

  const OUTER_SEGMENTS: OuterSegment[] = [
    { color: 'Vermilion',  startAngle: 0,   endAngle: 60 },
    { color: 'Amber',      startAngle: 60,  endAngle: 120 },
    { color: 'Chartreuse', startAngle: 120, endAngle: 180 },
    { color: 'Teal',       startAngle: 180, endAngle: 240 },
    { color: 'Indigo',     startAngle: 240, endAngle: 300 },
    { color: 'Magenta',    startAngle: 300, endAngle: 360 },
  ];

  let { wheel, interactive = false, onColorClick, selectedColors = [], size = 200, hidden = false }: {
    wheel: Record<Color, number>;
    interactive?: boolean;
    onColorClick?: (color: Color) => void;
    selectedColors?: Color[];
    size?: number;
    hidden?: boolean;
  } = $props();

  function toRad(deg: number): number {
    return (deg * Math.PI) / 180;
  }

  function pointAt(angleDeg: number, radius: number): { x: number; y: number } {
    const rad = toRad(angleDeg);
    return {
      x: HALF + radius * Math.sin(rad),
      y: HALF - radius * Math.cos(rad),
    };
  }

  function controlPoint(_dividerAngle: number, bowTowardAngle: number): { x: number; y: number } {
    const r = BOW_FACTOR * INNER_R;
    return pointAt(bowTowardAngle, r);
  }

  function segmentSpan(seg: InnerSegment): number {
    let span = seg.endAngle - seg.startAngle;
    if (span < 0) span += 360;
    return span;
  }

  function innerSegmentPath(seg: InnerSegment): string {
    const bStart = pointAt(seg.startAngle, INNER_R);
    const bEnd = pointAt(seg.endAngle, INNER_R);
    const cpStart = controlPoint(seg.startAngle, seg.bowStart);
    const cpEnd = controlPoint(seg.endAngle, seg.bowEnd);

    const span = segmentSpan(seg);
    const largeArc = span > 180 ? 1 : 0;

    if (seg.type === 'primary') {
      return [
        `M ${HALF} ${HALF}`,
        `Q ${cpStart.x} ${cpStart.y}, ${bStart.x} ${bStart.y}`,
        `A ${INNER_R} ${INNER_R} 0 ${largeArc} 1 ${bEnd.x} ${bEnd.y}`,
        `Q ${cpEnd.x} ${cpEnd.y}, ${HALF} ${HALF}`,
        `Z`,
      ].join(' ');
    } else {
      // Secondary: crescent shape starting from apex point, not center
      const midAngle = seg.startAngle + span / 2;
      const apex = pointAt(midAngle, SECONDARY_APEX_R);
      return [
        `M ${apex.x} ${apex.y}`,
        `Q ${cpStart.x} ${cpStart.y}, ${bStart.x} ${bStart.y}`,
        `A ${INNER_R} ${INNER_R} 0 ${largeArc} 1 ${bEnd.x} ${bEnd.y}`,
        `Q ${cpEnd.x} ${cpEnd.y}, ${apex.x} ${apex.y}`,
        `Z`,
      ].join(' ');
    }
  }

  function outerSegmentPath(seg: OuterSegment): string {
    const p1 = pointAt(seg.startAngle, OUTER_R);
    const p2 = pointAt(seg.endAngle, OUTER_R);
    const p3 = pointAt(seg.endAngle, INNER_R);
    const p4 = pointAt(seg.startAngle, INNER_R);

    let span = seg.endAngle - seg.startAngle;
    if (span < 0) span += 360;
    const largeArc = span > 180 ? 1 : 0;

    return [
      `M ${p1.x} ${p1.y}`,
      `A ${OUTER_R} ${OUTER_R} 0 ${largeArc} 1 ${p2.x} ${p2.y}`,
      `L ${p3.x} ${p3.y}`,
      `A ${INNER_R} ${INNER_R} 0 ${largeArc} 0 ${p4.x} ${p4.y}`,
      `Z`,
    ].join(' ');
  }

  // Label positions
  const PRIMARY_LABEL_R = 38;
  const SECONDARY_LABEL_R = 58;
  const TERTIARY_LABEL_R = (INNER_R + OUTER_R) / 2;

  interface LabelInfo {
    color: Color;
    x: number;
    y: number;
    letterSize: number;
    countSize: number;
    yOffset: number;
  }

  function centerAngle(startAngle: number, endAngle: number): number {
    let span = endAngle - startAngle;
    if (span < 0) span += 360;
    return startAngle + span / 2;
  }

  const labels: LabelInfo[] = [
    // Primaries
    ...PRIMARIES.map(seg => {
      const angle = centerAngle(seg.startAngle, seg.endAngle);
      const pos = pointAt(angle, PRIMARY_LABEL_R);
      return { color: seg.color, x: pos.x, y: pos.y, letterSize: 18, countSize: 20, yOffset: 10 };
    }),
    // Secondaries
    ...SECONDARIES.map(seg => {
      const angle = centerAngle(seg.startAngle, seg.endAngle);
      const pos = pointAt(angle, SECONDARY_LABEL_R);
      return { color: seg.color, x: pos.x, y: pos.y, letterSize: 12, countSize: 13, yOffset: 7 };
    }),
    // Tertiaries
    ...OUTER_SEGMENTS.map(seg => {
      const angle = centerAngle(seg.startAngle, seg.endAngle);
      const pos = pointAt(angle, TERTIARY_LABEL_R);
      return { color: seg.color, x: pos.x, y: pos.y, letterSize: 11, countSize: 12, yOffset: 7 };
    }),
  ];

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
      <!-- Inner filled segments: primaries first (background), then secondaries on top -->
      {#each PRIMARIES as seg}
        <path d={innerSegmentPath(seg)} fill={colorToHex(seg.color)} />
      {/each}
      {#each SECONDARIES as seg}
        <path d={innerSegmentPath(seg)} fill={colorToHex(seg.color)} />
      {/each}
      <!-- Outer filled segments (tertiaries) -->
      {#each OUTER_SEGMENTS as seg}
        <path d={outerSegmentPath(seg)} fill={colorToHex(seg.color)} />
      {/each}
      <!-- Segment outlines -->
      {#each ALL_INNER as seg}
        <path d={innerSegmentPath(seg)} fill="none" stroke="black" stroke-width="1.7" />
      {/each}
      {#each OUTER_SEGMENTS as seg}
        <path d={outerSegmentPath(seg)} fill="none" stroke="black" stroke-width="1.7" />
      {/each}
      <!-- Interactive hit targets -->
      {#each ALL_INNER as seg}
        <path
          d={innerSegmentPath(seg)}
          fill="transparent"
          class:clickable={interactive && wheel[seg.color] > 0}
          class:selected={isSelected(seg.color)}
          onclick={() => handleClick(seg.color)}
        />
      {/each}
      {#each OUTER_SEGMENTS as seg}
        <path
          d={outerSegmentPath(seg)}
          fill="transparent"
          class:clickable={interactive && wheel[seg.color] > 0}
          class:selected={isSelected(seg.color)}
          onclick={() => handleClick(seg.color)}
        />
      {/each}
      <!-- Color letter + count labels -->
      {#each labels as label}
        {@const count = wheel[label.color]}
        {@const hex = colorToHex(label.color)}
        {@const textColor = textColorForBackground(hex)}
        <text
          x={label.x}
          y={label.y - label.yOffset}
          text-anchor="middle"
          dominant-baseline="central"
          fill={textColor}
          font-size={label.letterSize}
          font-weight="700"
          font-family="Cinzel, serif"
          opacity={count > 0 ? 1 : 0.25}
          style="pointer-events: none;"
        >{label.color[0]}</text>
        <text
          x={label.x}
          y={label.y + label.yOffset}
          text-anchor="middle"
          dominant-baseline="central"
          fill={textColor}
          font-size={label.countSize}
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
