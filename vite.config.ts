import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  base: '/',
  plugins: [svelte()],
  build: {
    minify: 'terser',
  },
  test: {
    include: ['src/**/*.test.ts'],
  },
})
