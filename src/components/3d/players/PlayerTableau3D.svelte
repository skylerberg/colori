<script module lang="ts">
  import * as THREE from 'three';

  // Module-level texture cache shared across all instances
  let cachedTableauTexture: THREE.Texture | null = null;
  let textureLoadPromise: Promise<THREE.Texture> | null = null;

  function loadTableauTexture(): Promise<THREE.Texture> {
    if (cachedTableauTexture) return Promise.resolve(cachedTableauTexture);
    if (textureLoadPromise) return textureLoadPromise;

    textureLoadPromise = new Promise((resolve) => {
      const loader = new THREE.TextureLoader();
      loader.load('/3d/workshop-tableau.png', (tex) => {
        tex.colorSpace = THREE.SRGBColorSpace;
        tex.wrapS = THREE.ClampToEdgeWrapping;
        tex.wrapT = THREE.ClampToEdgeWrapping;
        tex.minFilter = THREE.LinearMipMapLinearFilter;
        tex.magFilter = THREE.LinearFilter;
        tex.generateMipmaps = true;
        cachedTableauTexture = tex;
        resolve(tex);
      });
    });
    return textureLoadPromise;
  }
</script>

<script lang="ts">
  import { T } from '@threlte/core';
  import { Text } from '@threlte/extras';
  import type { Color, MaterialType, CardInstance, BuyerInstance } from '../../../data/types';
  import { colorToHex, WHEEL_ORDER } from '../../../data/colors';
  import PigmentObject3D from './PigmentObject3D.svelte';
  import Card3D from '../cards/Card3D.svelte';

  let {
    colorWheel,
    materials,
    draftedCards = [],
    workshopCards = [],
    completedBuyers = [],
    deckSize = 0,
    position = [0, 0, 0],
    scale = 1,
    interactive = false,
    mixMode = false,
    onPigmentClick,
    selectedMixColors = [],
  }: {
    colorWheel: Record<Color, number>;
    materials: Record<MaterialType, number>;
    draftedCards?: CardInstance[];
    workshopCards?: CardInstance[];
    completedBuyers?: BuyerInstance[];
    deckSize?: number;
    position?: [number, number, number];
    scale?: number;
    interactive?: boolean;
    mixMode?: boolean;
    onPigmentClick?: (color: Color) => void;
    selectedMixColors?: Color[];
  } = $props();

  // ──────────────────────────────────────────────
  // Layout constants (all in local units)
  // The tableau is oriented in local space as:
  //   X = left/right (along the table edge)
  //   Z = depth (toward/away from table center, negative = toward center)
  // ──────────────────────────────────────────────

  /** Total tableau width (fits within one pentagon side ~3.29 units) */
  const TABLEAU_W = 2.6;
  /** Total tableau depth — matches workshop-tableau.png aspect ratio (1328:800 ≈ 1.66:1) */
  const TABLEAU_D = 2.6 / (1328 / 800); // ≈ 1.566

  // ── Color Wheel Zone ──
  // Positions derived from the workshop-tableau.png image:
  //   Wheel center at ~pixel (420, 370) of 1328×800
  //   UV: (0.316, 0.463) → local: x = (0.316-0.5)*W, z = -(0.463-0.5)*D
  const WHEEL_CENTER_X = (0.316 - 0.5) * TABLEAU_W;  // ≈ -0.478
  const WHEEL_CENTER_Z = -(0.463 - 0.5) * TABLEAU_D;  // ≈ 0.058
  // Mid-ring of color wheel: ~210px radius → 210/1328 * W ≈ 0.41
  const PIGMENT_RADIUS = (210 / 1328) * TABLEAU_W;    // ≈ 0.411
  const SEGMENT_ANGLE = (2 * Math.PI) / 12;

  function getPigmentPos(index: number): [number, number, number] {
    // Arrange 12 pigments in a circle, starting from top (Red at 12 o'clock)
    const angle = index * SEGMENT_ANGLE - Math.PI / 2;
    return [
      WHEEL_CENTER_X + Math.cos(angle) * PIGMENT_RADIUS,
      0,
      WHEEL_CENTER_Z - Math.sin(angle) * PIGMENT_RADIUS,
    ];
  }

  // ── Material Slots Zone (right half) ──
  const MAT_ZONE_X = 0.7;
  const MAT_ZONE_Z = 0.0;
  const MAT_SLOT_SPACING = 0.35;

  const MATERIAL_SLOTS: { type: MaterialType; x: number; z: number; color: string; label: string }[] = [
    { type: 'Textiles', x: MAT_ZONE_X, z: MAT_ZONE_Z - MAT_SLOT_SPACING, color: '#c0392b', label: 'T' },
    { type: 'Ceramics', x: MAT_ZONE_X, z: MAT_ZONE_Z, color: '#8b6914', label: 'C' },
    { type: 'Paintings', x: MAT_ZONE_X + 0.35, z: MAT_ZONE_Z - MAT_SLOT_SPACING, color: '#2a6bcf', label: 'P' },
  ];

  // ── Card Zones ──
  // Cards are 0.5x0.7 at full scale; we use CARD_SCALE inside the tableau
  const CARD_SCALE = 0.3;
  const CARD_W = 0.5 * CARD_SCALE; // 0.15
  const CARD_H = 0.7 * CARD_SCALE; // 0.21
  const CARD_SPACING = 0.2;

  // Drafted cards row: 4 slots at negative z (toward table center)
  const DRAFT_SLOTS = 4;
  const DRAFT_ROW_Z = -TABLEAU_D / 2 - CARD_H / 2 - 0.05; // just outside top edge
  const DRAFT_START_X = -(DRAFT_SLOTS - 1) * CARD_SPACING / 2;

  // Workshop cards row: at positive z (player-facing edge)
  const WORKSHOP_ROW_Z = TABLEAU_D / 2 + CARD_H / 2 + 0.05; // just outside bottom edge
  const MAX_WORKSHOP_DISPLAY = 5;

  // Completed buyers: far right, stacked
  const BUYERS_X = TABLEAU_W / 2 + CARD_W / 2 + 0.05;
  const BUYERS_START_Z = -0.3;
  const BUYERS_SPACING = 0.22;

  // Deck stack: below completed buyers
  const DECK_X = BUYERS_X;
  const DECK_Z = 0.3;

  // Always load the workshop tableau texture for the board background
  let tableauTexture = $state<THREE.Texture | null>(null);

  loadTableauTexture().then((tex) => {
    tableauTexture = tex;
  });

  // Mix mode hover state
  let hoveredPigment = $state<number | null>(null);
</script>

<T.Group position={position} scale={[scale, scale, scale]}>
  <!-- Tableau board (workshop-tableau.png texture, like a physical board on the table) -->
  <T.Mesh rotation.x={-Math.PI / 2} position.y={0.003} receiveShadow castShadow>
    <T.PlaneGeometry args={[TABLEAU_W, TABLEAU_D]} />
    {#if tableauTexture}
      <T.MeshStandardMaterial
        map={tableauTexture}
        roughness={0.75}
        metalness={0.02}
      />
    {:else}
      <T.MeshStandardMaterial color="#1e1812" roughness={0.9} metalness={0.02} />
    {/if}
  </T.Mesh>

  <!-- Pigment objects physically sitting on top of the color wheel -->
  {#each WHEEL_ORDER as color, i}
    {@const pigmentPos = getPigmentPos(i)}
    {@const isSelected = selectedMixColors.includes(color)}
    {@const isHovered = hoveredPigment === i}
    {@const hasColor = colorWheel[color] > 0}
    {@const isClickable = mixMode && interactive && hasColor && !!onPigmentClick}

    <PigmentObject3D
      {color}
      count={colorWheel[color]}
      position={pigmentPos}
    />

    <!-- Mix mode: invisible click target overlay above each pigment -->
    {#if mixMode && interactive}
      <T.Mesh
        position={[pigmentPos[0], 0.12, pigmentPos[2]]}
        rotation.x={-Math.PI / 2}
        onclick={isClickable ? () => onPigmentClick!(color) : undefined}
        onpointerenter={isClickable ? () => { hoveredPigment = i; document.body.style.cursor = 'pointer'; } : undefined}
        onpointerleave={isClickable ? () => { hoveredPigment = null; document.body.style.cursor = 'auto'; } : undefined}
      >
        <T.CircleGeometry args={[0.06, 16]} />
        <T.MeshBasicMaterial transparent={true} opacity={0} />
      </T.Mesh>
    {/if}

    <!-- Glow ring for selected colors in mix mode -->
    {#if isSelected}
      <T.Mesh
        position={[pigmentPos[0], 0.01, pigmentPos[2]]}
        rotation.x={-Math.PI / 2}
      >
        <T.RingGeometry args={[0.04, 0.06, 24]} />
        <T.MeshBasicMaterial color="#ffffff" transparent={true} opacity={0.7} />
      </T.Mesh>
    {/if}

    <!-- Hover highlight ring -->
    {#if isHovered && isClickable}
      <T.Mesh
        position={[pigmentPos[0], 0.01, pigmentPos[2]]}
        rotation.x={-Math.PI / 2}
      >
        <T.RingGeometry args={[0.04, 0.055, 24]} />
        <T.MeshBasicMaterial color={colorToHex(color)} transparent={true} opacity={0.5} />
      </T.Mesh>
    {/if}
  {/each}

  <!-- ════════ MATERIAL SLOTS ZONE (right) ════════ -->

  {#each MATERIAL_SLOTS as slot}
    {@const count = materials[slot.type]}

    <!-- Material token cubes stacked -->
    {#each Array(Math.min(count, 5)) as _, ci}
      <T.Mesh
        position={[slot.x, 0.03 + ci * 0.05, slot.z]}
        castShadow
      >
        <T.BoxGeometry args={[0.07, 0.05, 0.07]} />
        <T.MeshStandardMaterial
          color={slot.color}
          roughness={0.5}
          metalness={0.2}
        />
      </T.Mesh>
    {/each}

    <!-- Label -->
    <Text
      text={`${slot.label}: ${count}`}
      position={[slot.x, 0.01, slot.z + 0.17]}
      fontSize={0.035}
      color="#ffe8cc"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.002}
      outlineColor="#000000"
      fontWeight="bold"
    />
  {/each}

  <!-- ════════ DRAFTED CARDS ZONE (top, toward table center) ════════ -->

  <!-- Empty slot indicators (always visible) -->
  {#each Array(DRAFT_SLOTS) as _, i}
    {@const slotX = DRAFT_START_X + i * CARD_SPACING}
    {#if i >= draftedCards.length}
      <T.Mesh position={[slotX, 0.003, DRAFT_ROW_Z]} rotation.x={-Math.PI / 2}>
        <T.PlaneGeometry args={[CARD_W, CARD_H]} />
        <T.MeshStandardMaterial color="#2a2018" roughness={0.9} transparent={true} opacity={0.4} />
      </T.Mesh>
    {/if}
  {/each}

  <!-- Drafted cards rendered on top of slots -->
  {#each draftedCards.slice(0, DRAFT_SLOTS) as ci, i}
    {@const slotX = DRAFT_START_X + i * CARD_SPACING}
    <Card3D
      card={ci.card}
      position={[slotX, 0.02, DRAFT_ROW_Z]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={CARD_SCALE}
    />
  {/each}

  <!-- ════════ WORKSHOP CARDS ZONE (bottom, player-facing edge) ════════ -->

  {#each workshopCards.slice(0, MAX_WORKSHOP_DISPLAY) as ci, i}
    {@const totalCards = Math.min(workshopCards.length, MAX_WORKSHOP_DISPLAY)}
    {@const startX = -(totalCards - 1) * CARD_SPACING / 2}
    <Card3D
      card={ci.card}
      position={[startX + i * CARD_SPACING, 0.02, WORKSHOP_ROW_Z]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={CARD_SCALE}
    />
  {/each}

  <!-- ════════ COMPLETED BUYERS (right side, outside border) ════════ -->

  {#each completedBuyers.slice(0, 4) as bi, i}
    <Card3D
      buyerCard={bi.card}
      position={[BUYERS_X, 0.02, BUYERS_START_Z + i * BUYERS_SPACING]}
      rotation={[-Math.PI / 2, 0, 0]}
      faceUp={true}
      scale={CARD_SCALE}
    />
  {/each}

  <!-- ════════ DECK STACK (right side, below buyers) ════════ -->

  {#if deckSize > 0}
    <T.Mesh position={[DECK_X, 0.02 + Math.min(deckSize * 0.002, 0.05), DECK_Z]} castShadow>
      <T.BoxGeometry args={[CARD_W, Math.min(deckSize * 0.004, 0.08), CARD_H]} />
      <T.MeshStandardMaterial color="#2a1f18" roughness={0.8} />
    </T.Mesh>
    <Text
      text={`${deckSize}`}
      position={[DECK_X, 0.1, DECK_Z]}
      fontSize={0.03}
      color="#ffe8cc"
      anchorX="center"
      anchorY="middle"
      outlineWidth={0.002}
      outlineColor="#000000"
    />
  {/if}
</T.Group>
