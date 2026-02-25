import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  base: '/',
  plugins: [svelte(), wasm()],
  build: {
    minify: 'terser',
  },
  worker: {
    plugins: () => [wasm()],
  },
  test: {
    include: ['src/**/*.test.ts'],
    benchmark: {
      include: ['src/**/*.bench.ts'],
    },
  },
})
