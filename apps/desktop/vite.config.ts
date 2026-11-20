/// <reference types="vitest/config" />
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vite';

// Dev server and build for the Tauri window; Vitest uses the same config.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // The translation file lives at the repository root (shared with Rust).
    fs: { allow: ['.', '../../locales'] },
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/lib/**/*.ts'],
      // tauri.ts only forwards to the Tauri API; bindings.ts is generated.
      exclude: [
        'src/lib/**/*.test.ts',
        'src/lib/tauri.ts',
        'src/lib/types.ts',
        'src/lib/bindings.ts',
      ],
      thresholds: { lines: 90 },
    },
  },
});
