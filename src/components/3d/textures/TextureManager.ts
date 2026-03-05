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
}
