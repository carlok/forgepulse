import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 18745,
    strictPort: true,
    proxy: { '/api': 'http://api:18744' }
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/lib/**/*.ts'],
      thresholds: { statements: 80, branches: 80, functions: 80, lines: 80 }
    }
  }
});
