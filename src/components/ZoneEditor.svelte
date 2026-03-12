<script lang="ts">
  import { loadZones, saveZones, DEFAULT_ZONES, type ZoneConfig } from '../data/zoneConfig';

  let { onClose }: { onClose: () => void } = $props();

  let zones = $state<ZoneConfig[]>(loadZones().map(z => ({ ...z })));
  let selectedId = $state<string | null>(null);
  let tableauEl: HTMLDivElement | undefined = $state();
  let snapToGrid = $state(false);
  let gridSize = 5; // percentage

  // Drag state
  let dragging = $state(false);
  let resizing = $state(false);
  let resizeCorner = $state<'tl' | 'tr' | 'bl' | 'br'>('br');
  let dragStartMouse = { x: 0, y: 0 };
  let dragStartZone = { x: 0, y: 0, width: 0, height: 0 };

  const ZONE_COLORS: Record<string, string> = {
    colorWheel: '#f39c12',
    buyers: '#9b59b6',
    materialTextiles: '#e84393',
    materialCeramics: '#d35400',
    materialPaintings: '#8e44ad',
    draft1: '#3498db',
    draft2: '#2980b9',
    draft3: '#2471a3',
    draft4: '#1f618d',
    workshop: '#2ecc71',
  };

  const FALLBACK_COLORS = ['#e74c3c', '#e67e22', '#1abc9c', '#e84393', '#00cec9', '#6c5ce7'];

  function zoneColor(zone: ZoneConfig, index: number): string {
    return ZONE_COLORS[zone.id] ?? FALLBACK_COLORS[index % FALLBACK_COLORS.length];
  }

  function snap(value: number): number {
    if (!snapToGrid) return Math.round(value * 10) / 10;
    return Math.round(value / gridSize) * gridSize;
  }

  // Convert mouse position to percentage relative to the TABLEAU image (not the outer container)
  function getPercentPos(e: MouseEvent): { px: number; py: number } {
    if (!tableauEl) return { px: 0, py: 0 };
    const rect = tableauEl.getBoundingClientRect();
    return {
      px: ((e.clientX - rect.left) / rect.width) * 100,
      py: ((e.clientY - rect.top) / rect.height) * 100,
    };
  }

  function handleZoneMouseDown(e: MouseEvent, zone: ZoneConfig) {
    e.preventDefault();
    e.stopPropagation();
    selectedId = zone.id;
    dragging = true;
    const pos = getPercentPos(e);
    dragStartMouse = { x: pos.px, y: pos.py };
    dragStartZone = { x: zone.x, y: zone.y, width: zone.width, height: zone.height };
  }

  function handleCornerMouseDown(e: MouseEvent, zone: ZoneConfig, corner: 'tl' | 'tr' | 'bl' | 'br') {
    e.preventDefault();
    e.stopPropagation();
    selectedId = zone.id;
    resizing = true;
    resizeCorner = corner;
    const pos = getPercentPos(e);
    dragStartMouse = { x: pos.px, y: pos.py };
    dragStartZone = { x: zone.x, y: zone.y, width: zone.width, height: zone.height };
  }

  function handleMouseMove(e: MouseEvent) {
    if (!dragging && !resizing) return;
    const pos = getPercentPos(e);
    const dx = pos.px - dragStartMouse.x;
    const dy = pos.py - dragStartMouse.y;
    const zone = zones.find(z => z.id === selectedId);
    if (!zone) return;

    if (dragging) {
      zone.x = snap(dragStartZone.x + dx);
      zone.y = snap(dragStartZone.y + dy);
    } else if (resizing) {
      const minW = 5;
      const minH = 3;
      if (resizeCorner === 'br') {
        zone.width = snap(Math.max(minW, dragStartZone.width + dx));
        zone.height = snap(Math.max(minH, dragStartZone.height + dy));
      } else if (resizeCorner === 'bl') {
        const newW = Math.max(minW, dragStartZone.width - dx);
        zone.x = snap(dragStartZone.x + dragStartZone.width - newW);
        zone.width = snap(newW);
        zone.height = snap(Math.max(minH, dragStartZone.height + dy));
      } else if (resizeCorner === 'tr') {
        zone.width = snap(Math.max(minW, dragStartZone.width + dx));
        const newH = Math.max(minH, dragStartZone.height - dy);
        zone.y = snap(dragStartZone.y + dragStartZone.height - newH);
        zone.height = snap(newH);
      } else if (resizeCorner === 'tl') {
        const newW = Math.max(minW, dragStartZone.width - dx);
        zone.x = snap(dragStartZone.x + dragStartZone.width - newW);
        zone.width = snap(newW);
        const newH = Math.max(minH, dragStartZone.height - dy);
        zone.y = snap(dragStartZone.y + dragStartZone.height - newH);
        zone.height = snap(newH);
      }
    }
  }

  function handleMouseUp() {
    dragging = false;
    resizing = false;
  }

  function handleBackgroundClick() {
    if (!dragging && !resizing) {
      selectedId = null;
    }
  }

  function handleFieldInput(zone: ZoneConfig, field: 'x' | 'y' | 'width' | 'height', value: string) {
    const num = parseFloat(value);
    if (isNaN(num)) return;
    zone[field] = num;
  }

  function resetToDefaults() {
    zones = DEFAULT_ZONES.map(z => ({ ...z }));
    selectedId = null;
  }

  function exportJSON() {
    const json = JSON.stringify(zones, null, 2);
    navigator.clipboard.writeText(json);
  }

  function handleSave() {
    saveZones(zones);
  }

  // Group zones for the panel
  let draftZones = $derived(zones.filter(z => z.id.startsWith('draft')));
  let materialZones = $derived(zones.filter(z => z.id.startsWith('material')));
  let tableauZones = $derived(zones.filter(z => !z.id.startsWith('draft') && !z.id.startsWith('material')));
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} />

<div class="editor-page">
  <div class="editor-header">
    <h2>Workshop Tableau Zone Editor</h2>
    <p>Drag zones to reposition. Drag corners to resize. Cards hang off the top (drafted) and bottom (workshop).</p>
  </div>

  <div class="editor-body">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="canvas-wrapper" onmousedown={handleBackgroundClick}>
      <!-- The actual tableau image area -->
      <div class="tableau-canvas" bind:this={tableauEl}>
        <!-- Grid overlay -->
        {#if snapToGrid}
          <div class="grid-overlay">
            {#each Array(Math.floor(100 / gridSize)) as _, i}
              <div class="grid-line-v" style="left:{(i + 1) * gridSize}%"></div>
              <div class="grid-line-h" style="top:{(i + 1) * gridSize}%"></div>
            {/each}
          </div>
        {/if}

        <!-- Zone rectangles (positioned relative to tableau, can overflow) -->
        {#each zones as zone, i}
          {@const selected = zone.id === selectedId}
          {@const color = zoneColor(zone, i)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="editor-zone"
            class:selected
            style="left:{zone.x}%;top:{zone.y}%;width:{zone.width}%;height:{zone.height}%;border-color:{color};background:{color}33;"
            onmousedown={(e) => handleZoneMouseDown(e, zone)}
          >
            <span class="zone-label" style="background:{color};color:#fff;">{zone.label}</span>

            {#if selected}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="handle handle-tl" style="background:{color};" onmousedown={(e) => handleCornerMouseDown(e, zone, 'tl')}></div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="handle handle-tr" style="background:{color};" onmousedown={(e) => handleCornerMouseDown(e, zone, 'tr')}></div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="handle handle-bl" style="background:{color};" onmousedown={(e) => handleCornerMouseDown(e, zone, 'bl')}></div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="handle handle-br" style="background:{color};" onmousedown={(e) => handleCornerMouseDown(e, zone, 'br')}></div>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- Control panel -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="control-panel" onmousedown={(e) => e.stopPropagation()}>
      <label class="grid-toggle">
        <input type="checkbox" bind:checked={snapToGrid} />
        Snap to grid ({gridSize}%)
      </label>

      <div class="zone-list">
        {#snippet zoneItem(zone: import('../data/zoneConfig').ZoneConfig)}
          {@const color = zoneColor(zone, zones.indexOf(zone))}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="zone-item" class:zone-item-selected={zone.id === selectedId} style="border-left:3px solid {color};" onclick={() => selectedId = zone.id}>
            <div class="zone-item-label">{zone.label}</div>
            <div class="zone-fields">
              <label>x<input type="number" value={zone.x} step="1" oninput={(e) => handleFieldInput(zone, 'x', e.currentTarget.value)} /></label>
              <label>y<input type="number" value={zone.y} step="1" oninput={(e) => handleFieldInput(zone, 'y', e.currentTarget.value)} /></label>
              <label>w<input type="number" value={zone.width} step="1" oninput={(e) => handleFieldInput(zone, 'width', e.currentTarget.value)} /></label>
              <label>h<input type="number" value={zone.height} step="1" oninput={(e) => handleFieldInput(zone, 'height', e.currentTarget.value)} /></label>
            </div>
          </div>
        {/snippet}

        <div class="zone-group-label">Tableau</div>
        {#each tableauZones as zone}
          {@render zoneItem(zone)}
        {/each}

        {#if materialZones.length > 0}
          <div class="zone-group-label">Materials</div>
          {#each materialZones as zone}
            {@render zoneItem(zone)}
          {/each}
        {/if}

        {#if draftZones.length > 0}
          <div class="zone-group-label">Draft Slots (hang off top)</div>
          {#each draftZones as zone}
            {@render zoneItem(zone)}
          {/each}
        {/if}
      </div>

      <div class="panel-buttons">
        <button class="btn btn-save" onclick={handleSave}>Save</button>
        <button class="btn btn-export" onclick={exportJSON}>Copy JSON</button>
        <button class="btn btn-reset" onclick={resetToDefaults}>Reset</button>
        <button class="btn btn-close" onclick={onClose}>Close</button>
      </div>
    </div>
  </div>
</div>

<style>
  .editor-page {
    max-width: 1400px;
    margin: 0 auto;
    padding: 0.5rem;
  }

  .editor-header {
    text-align: center;
    margin-bottom: 1rem;
  }

  .editor-header h2 {
    color: #4a3728;
    font-size: 1rem;
    margin-bottom: 4px;
  }

  .editor-header p {
    font-size: 0.75rem;
    color: #888;
  }

  .editor-body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    align-items: flex-start;
  }

  /* Outer wrapper provides space for overhanging zones */
  .canvas-wrapper {
    flex: 1;
    padding: 40px 10px;
    background: #e8e0d5;
    border-radius: 10px;
    display: flex;
    justify-content: center;
    width: 100%;
  }

  /* The actual tableau image, maintains exact aspect ratio */
  .tableau-canvas {
    width: 100%;
    max-width: 100%;
    aspect-ratio: 1328 / 800;
    background-image: url('/workshop.webp');
    background-size: 100% 100%;
    border: 2px solid #8b6914;
    border-radius: 6px;
    position: relative;
    overflow: visible;
    cursor: default;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  }

  .grid-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .grid-line-v {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: rgba(255, 255, 255, 0.25);
  }

  .grid-line-h {
    position: absolute;
    left: 0;
    right: 0;
    height: 1px;
    background: rgba(255, 255, 255, 0.25);
  }

  .editor-zone {
    position: absolute;
    border: 2px solid;
    cursor: move;
    box-sizing: border-box;
    transition: box-shadow 0.1s;
    border-radius: 4px;
  }

  .editor-zone.selected {
    border-width: 3px;
    box-shadow: 0 0 12px rgba(255, 255, 255, 0.6);
    z-index: 10;
  }

  .zone-label {
    position: absolute;
    top: 2px;
    left: 2px;
    font-size: 9px;
    font-weight: 700;
    padding: 1px 4px;
    border-radius: 3px;
    pointer-events: none;
    white-space: nowrap;
  }

  .handle {
    position: absolute;
    width: 16px;
    height: 16px;
    border: 1px solid #fff;
    z-index: 11;
    border-radius: 2px;
  }

  .handle-tl { top: -8px; left: -8px; cursor: nwse-resize; }
  .handle-tr { top: -8px; right: -8px; cursor: nesw-resize; }
  .handle-bl { bottom: -8px; left: -8px; cursor: nesw-resize; }
  .handle-br { bottom: -8px; right: -8px; cursor: nwse-resize; }

  .control-panel {
    width: 100%;
    flex-shrink: 0;
    background: #f8f6f2;
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 12px;
    font-size: 12px;
    max-height: none;
    overflow-y: visible;
  }

  .grid-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
    cursor: pointer;
    font-size: 13px;
    color: #4a3728;
    min-height: 44px;
  }

  .grid-toggle input[type="checkbox"] {
    width: 20px;
    height: 20px;
  }

  .zone-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .zone-group-label {
    font-size: 11px;
    font-weight: 700;
    color: #4a3728;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: 8px;
    margin-bottom: 2px;
    padding-bottom: 2px;
    border-bottom: 1px solid #ddd;
  }

  .zone-group-label:first-child {
    margin-top: 0;
  }

  .zone-item {
    padding: 8px;
    background: #fff;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid #eee;
  }

  .zone-item-selected {
    background: #f0f4ff;
    border-color: #aab;
  }

  .zone-item-label {
    font-weight: 600;
    margin-bottom: 2px;
    font-size: 13px;
    color: #4a3728;
  }

  .zone-fields {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .zone-fields label {
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 11px;
    color: #888;
  }

  .zone-fields input {
    width: 52px;
    padding: 4px 6px;
    font-size: 12px;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 3px;
    color: #333;
    min-height: 36px;
  }

  .panel-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .btn {
    padding: 10px 16px;
    font-size: 13px;
    font-weight: 600;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    min-height: 44px;
  }

  .btn-save { background: #2ecc71; color: #fff; }
  .btn-save:hover { background: #27ae60; }
  .btn-export { background: #3498db; color: #fff; }
  .btn-export:hover { background: #2980b9; }
  .btn-reset { background: #e74c3c; color: #fff; }
  .btn-reset:hover { background: #c0392b; }
  .btn-close { background: #7f8c8d; color: #fff; }
  .btn-close:hover { background: #6c7a7b; }

  /* ===== RESPONSIVE OVERRIDES (mobile-first) ===== */

  /* Tablet (768px+): side-by-side layout */
  @media (min-width: 768px) {
    .editor-page {
      padding: 1rem;
    }

    .editor-header h2 {
      font-size: 1.3rem;
    }

    .editor-header p {
      font-size: 0.85rem;
    }

    .editor-body {
      flex-direction: row;
    }

    .canvas-wrapper {
      padding: 60px 15px;
    }

    .tableau-canvas {
      max-width: 800px;
    }

    .control-panel {
      width: 240px;
      max-height: calc(100vh - 120px);
      overflow-y: auto;
    }

    .zone-fields {
      flex-wrap: nowrap;
      gap: 4px;
    }

    .zone-fields input {
      width: 40px;
      min-height: unset;
      font-size: 10px;
      padding: 2px 4px;
    }

    .zone-fields label {
      font-size: 10px;
    }

    .btn {
      padding: 6px 12px;
      font-size: 11px;
      min-height: unset;
    }

    .zone-item {
      padding: 4px 6px;
    }

    .zone-item-label {
      font-size: 11px;
    }

    .grid-toggle {
      font-size: 12px;
      min-height: unset;
    }

    .grid-toggle input[type="checkbox"] {
      width: auto;
      height: auto;
    }

    .handle {
      width: 10px;
      height: 10px;
    }

    .handle-tl { top: -5px; left: -5px; }
    .handle-tr { top: -5px; right: -5px; }
    .handle-bl { bottom: -5px; left: -5px; }
    .handle-br { bottom: -5px; right: -5px; }
  }

  /* Desktop (1024px+) */
  @media (min-width: 1024px) {
    .canvas-wrapper {
      padding: 80px 20px;
    }

    .control-panel {
      width: 300px;
    }

    .zone-fields input {
      width: 42px;
    }
  }
</style>
