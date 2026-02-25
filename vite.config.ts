import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import wasm from 'vite-plugin-wasm'
import { resolve } from 'path'

export default defineConfig({
  base: '/',
  plugins: [svelte(), wasm()],
  build: {
    minify: 'terser',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        analysis: resolve(__dirname, 'analysis.html'),
      },
    },
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
