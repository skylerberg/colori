import * as THREE from 'three';
import type { Card, BuyerCard } from '../../../data/types';
import { getCardArtUrl, getBuyerArtUrl } from '../../../data/cardArt';

const textureLoader = new THREE.TextureLoader();
const textureCache = new Map<string, THREE.Texture>();
const loadingPromises = new Map<string, Promise<THREE.Texture>>();

// Placeholder texture (solid gray) used while real textures load
let placeholderTexture: THREE.Texture | null = null;

function getPlaceholder(): THREE.Texture {
  if (!placeholderTexture) {
    const canvas = document.createElement('canvas');
    canvas.width = 64;
    canvas.height = 64;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#8b7355';
    ctx.fillRect(0, 0, 64, 64);
    placeholderTexture = new THREE.CanvasTexture(canvas);
    placeholderTexture.colorSpace = THREE.SRGBColorSpace;
  }
  return placeholderTexture;
}

function configureTexture(texture: THREE.Texture): void {
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.minFilter = THREE.LinearMipMapLinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = true;
}

function loadTexture(url: string): Promise<THREE.Texture> {
  const cached = textureCache.get(url);
  if (cached) return Promise.resolve(cached);

  const existing = loadingPromises.get(url);
  if (existing) return existing;

  const promise = new Promise<THREE.Texture>((resolve, reject) => {
    textureLoader.load(
      url,
      (texture) => {
        configureTexture(texture);
        textureCache.set(url, texture);
        loadingPromises.delete(url);
        resolve(texture);
      },
      undefined,
      (error) => {
        loadingPromises.delete(url);
        reject(error);
      }
    );
  });

  loadingPromises.set(url, promise);
  return promise;
}

/** Get a card texture synchronously (returns placeholder if not yet loaded, triggers async load) */
export function getCardTexture(card: Card): THREE.Texture {
  const url = getCardArtUrl(card);
  const cached = textureCache.get(url);
  if (cached) return cached;
  // Start loading in background
  loadTexture(url);
  return getPlaceholder();
}

/** Get a buyer card texture synchronously */
export function getBuyerTexture(buyer: BuyerCard): THREE.Texture {
  const url = getBuyerArtUrl(buyer);
  const cached = textureCache.get(url);
  if (cached) return cached;
  loadTexture(url);
  return getPlaceholder();
}

/** Async load a card texture */
export function loadCardTexture(card: Card): Promise<THREE.Texture> {
  return loadTexture(getCardArtUrl(card));
}

/** Async load a buyer card texture */
export function loadBuyerTexture(buyer: BuyerCard): Promise<THREE.Texture> {
  return loadTexture(getBuyerArtUrl(buyer));
}

/** Preload an array of card textures in the background */
export function preloadCards(cards: Card[]): void {
  for (const card of cards) {
    loadTexture(getCardArtUrl(card));
  }
}

/** Preload an array of buyer card textures in the background */
export function preloadBuyers(buyers: BuyerCard[]): void {
  for (const buyer of buyers) {
    loadTexture(getBuyerArtUrl(buyer));
  }
}

/** Preload cards and buyers with a progress callback */
export function preloadWithProgress(
  cards: Card[],
  buyers: BuyerCard[],
  onProgress?: (loaded: number, total: number) => void,
): Promise<void> {
  const cardUrls = cards.map(getCardArtUrl);
  const buyerUrls = buyers.map(getBuyerArtUrl);
  const allUrls = [...new Set([...cardUrls, ...buyerUrls])]; // deduplicate
  const total = allUrls.length;
  if (total === 0) {
    onProgress?.(0, 0);
    return Promise.resolve();
  }
  let loaded = 0;
  const promises = allUrls.map((url) =>
    loadTexture(url)
      .then(() => {
        loaded++;
        onProgress?.(loaded, total);
      })
      .catch(() => {
        loaded++;
        onProgress?.(loaded, total);
      })
  );
  return Promise.all(promises).then(() => {});
}

// ── Procedural Card Back Texture ──

let cardBackTexture: THREE.Texture | null = null;

/** Generate a dark Renaissance-patterned card back texture */
export function getCardBackTexture(): THREE.Texture {
  if (cardBackTexture) return cardBackTexture;

  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = Math.round(size * 1.4); // 5:7 ratio
  const ctx = canvas.getContext('2d')!;
  const h = canvas.height;

  // Dark base
  ctx.fillStyle = '#1a120c';
  ctx.fillRect(0, 0, size, h);

  // Border frame
  const borderWidth = 16;
  ctx.strokeStyle = '#b8860b';
  ctx.lineWidth = 3;
  ctx.strokeRect(borderWidth, borderWidth, size - borderWidth * 2, h - borderWidth * 2);
  ctx.strokeStyle = '#8b6914';
  ctx.lineWidth = 1.5;
  ctx.strokeRect(borderWidth + 6, borderWidth + 6, size - (borderWidth + 6) * 2, h - (borderWidth + 6) * 2);

  // Corner flourishes
  const cornerSize = 40;
  const corners: [number, number, number, number][] = [
    [borderWidth + 10, borderWidth + 10, 1, 1],
    [size - borderWidth - 10, borderWidth + 10, -1, 1],
    [borderWidth + 10, h - borderWidth - 10, 1, -1],
    [size - borderWidth - 10, h - borderWidth - 10, -1, -1],
  ];

  ctx.strokeStyle = '#b8860b';
  ctx.lineWidth = 2;
  for (const [cx, cy, dx, dy] of corners) {
    ctx.beginPath();
    ctx.moveTo(cx, cy + dy * cornerSize);
    ctx.quadraticCurveTo(cx, cy, cx + dx * cornerSize, cy);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(cx + dx * 8, cy + dy * cornerSize * 0.6);
    ctx.quadraticCurveTo(cx + dx * 8, cy + dy * 8, cx + dx * cornerSize * 0.6, cy + dy * 8);
    ctx.stroke();
  }

  // Center diamond motif
  const centerX = size / 2;
  const centerY = h / 2;
  const diamondW = 60;
  const diamondH = 80;

  ctx.strokeStyle = '#b8860b';
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(centerX, centerY - diamondH);
  ctx.lineTo(centerX + diamondW, centerY);
  ctx.lineTo(centerX, centerY + diamondH);
  ctx.lineTo(centerX - diamondW, centerY);
  ctx.closePath();
  ctx.stroke();

  // Inner diamond
  const innerW = 35;
  const innerH = 50;
  ctx.strokeStyle = '#8b6914';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(centerX, centerY - innerH);
  ctx.lineTo(centerX + innerW, centerY);
  ctx.lineTo(centerX, centerY + innerH);
  ctx.lineTo(centerX - innerW, centerY);
  ctx.closePath();
  ctx.stroke();

  // "C" letter in center for Colori
  ctx.fillStyle = '#b8860b';
  ctx.font = 'bold 48px serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('C', centerX, centerY);

  // Repeating small motif pattern in background
  ctx.globalAlpha = 0.12;
  ctx.fillStyle = '#b8860b';
  const spacing = 32;
  for (let row = 0; row < h / spacing; row++) {
    for (let col = 0; col < size / spacing; col++) {
      const px = col * spacing + spacing / 2;
      const py = row * spacing + spacing / 2;
      // Skip if too close to center diamond
      const dist = Math.abs(px - centerX) / diamondW + Math.abs(py - centerY) / diamondH;
      if (dist < 1.3) continue;
      // Skip if outside inner border
      if (px < borderWidth + 12 || px > size - borderWidth - 12 ||
          py < borderWidth + 12 || py > h - borderWidth - 12) continue;
      // Small cross/star motif
      ctx.fillRect(px - 1, py - 4, 2, 8);
      ctx.fillRect(px - 4, py - 1, 8, 2);
    }
  }
  ctx.globalAlpha = 1;

  cardBackTexture = new THREE.CanvasTexture(canvas);
  cardBackTexture.colorSpace = THREE.SRGBColorSpace;
  return cardBackTexture;
}

/** Clean up all cached textures */
export function disposeAll(): void {
  for (const texture of textureCache.values()) {
    texture.dispose();
  }
  textureCache.clear();
  loadingPromises.clear();
  if (placeholderTexture) {
    placeholderTexture.dispose();
    placeholderTexture = null;
  }
  if (cardBackTexture) {
    cardBackTexture.dispose();
    cardBackTexture = null;
  }
}
