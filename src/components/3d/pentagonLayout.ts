/**
 * Pentagon table geometry constants and helper functions.
 * Used by Table.svelte, SceneContent.svelte, and player positioning.
 */

/** Circumradius of the pentagon table (center to vertex) */
export const PENTAGON_CIRCUMRADIUS = 2.8;

/** Number of sides */
export const PENTAGON_SIDES = 5;

/** Side length of the pentagon */
export const PENTAGON_SIDE_LENGTH = 2 * PENTAGON_CIRCUMRADIUS * Math.sin(Math.PI / PENTAGON_SIDES);

/** Inradius / apothem (center to edge midpoint) */
export const PENTAGON_INRADIUS = PENTAGON_CIRCUMRADIUS * Math.cos(Math.PI / PENTAGON_SIDES);

/**
 * Pentagon vertices in 3D (xz-plane), starting from top vertex going clockwise.
 * Oriented so that edge 2 (between vertices 2 and 3) faces positive z (camera).
 * After the rotateX(-PI/2) transform in Table.svelte:
 *   Shape (x, y) -> 3D (x, 0, y)
 */
export function getPentagonVertex(i: number, radius: number = PENTAGON_CIRCUMRADIUS): [number, number] {
  const angle = (2 * Math.PI * i) / PENTAGON_SIDES - Math.PI / 2;
  // Shape coords map to 3D as (x, 0, y), so return [x3d, z3d]
  const sx = radius * Math.cos(angle);
  const sy = radius * Math.sin(angle);
  return [sx, sy]; // 3D: (sx, 0, sy)
}

export interface EdgeInfo {
  /** Index of this edge (0-4) */
  index: number;
  /** Midpoint in 3D xz-plane */
  midpoint: [number, number, number];
  /** Rotation around y-axis so that "forward" (local +z) points outward from table center */
  rotationY: number;
  /** Side length */
  sideLength: number;
}

/**
 * Get info for each pentagon edge.
 * Edge i connects vertex i to vertex (i+1) % 5.
 *
 * Edge ordering (with current orientation):
 *   Edge 0: v0-v1 (back-right, far from camera)
 *   Edge 1: v1-v2 (right side)
 *   Edge 2: v2-v3 (front, closest to camera — local player's edge)
 *   Edge 3: v3-v4 (left side)
 *   Edge 4: v4-v0 (back-left)
 */
export function getPentagonEdges(): EdgeInfo[] {
  const edges: EdgeInfo[] = [];
  for (let i = 0; i < PENTAGON_SIDES; i++) {
    const [x1, z1] = getPentagonVertex(i);
    const [x2, z2] = getPentagonVertex((i + 1) % PENTAGON_SIDES);
    const mx = (x1 + x2) / 2;
    const mz = (z1 + z2) / 2;
    const dx = x2 - x1;
    const dz = z2 - z1;
    const edgeLen = Math.sqrt(dx * dx + dz * dz);

    // Outward normal direction (away from center)
    // The midpoint itself points outward from center, so normalize it
    const dist = Math.sqrt(mx * mx + mz * mz);
    const nx = mx / dist;
    const nz = mz / dist;

    // Rotation so that the player's "forward" (+z in local space) faces outward
    // atan2(nx, nz) gives the angle from +z axis to the outward normal
    const rotY = Math.atan2(nx, nz);

    edges.push({
      index: i,
      midpoint: [mx, 0.07, mz],
      rotationY: rotY,
      sideLength: edgeLen,
    });
  }
  return edges;
}

/**
 * Inward offset to position player groups so tableaux sit ON the table surface
 * rather than straddling the edge. This shifts the group center toward the table
 * center by this many units from the edge midpoint.
 */
export const PLAYER_INSET = 0.7;

/**
 * Get inset midpoint for a player position (shifted toward center from edge midpoint).
 * This ensures the tableau and surrounding cards sit within the table surface.
 */
export function getPlayerPosition(edge: EdgeInfo): [number, number, number] {
  const [mx, y, mz] = edge.midpoint;
  const dist = Math.sqrt(mx * mx + mz * mz);
  const nx = mx / dist;
  const nz = mz / dist;
  return [mx - nx * PLAYER_INSET, y, mz - nz * PLAYER_INSET];
}

/** The local player's edge index (front edge, closest to camera at positive z) */
export const LOCAL_PLAYER_EDGE = 2;

/**
 * Get the edge assignments for each player seat.
 * Local player (index 0) is always on edge 2 (front).
 * Opponents fill remaining edges clockwise: 3, 1, 4, 0.
 * This distributes players symmetrically around the table.
 */
export function getPlayerEdgeOrder(playerCount: number): number[] {
  // Edge 2 is local player, then clockwise: 3, 1, 4, 0
  const order = [2, 3, 1, 4, 0];
  return order.slice(0, playerCount);
}
