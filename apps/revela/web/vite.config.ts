import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { VitePWA } from 'vite-plugin-pwa';

// GitHub Pages sirve el repo en https://nikporras.github.io/revela/
const base = '/revela/';
// Versión visible en el diagnóstico: el commit del despliegue, o "local".
const version = (process.env.GITHUB_SHA ?? 'local').slice(0, 7);

export default defineConfig({
  base,
  define: { __REVELA_VERSION__: JSON.stringify(version) },
  plugins: [
    svelte(),
    VitePWA({
      // La versión nueva espera a que el usuario toque "Actualizar" (src/lib/actualizar.ts).
      registerType: 'prompt',
      injectRegister: false,
      manifest: {
        name: 'Revela',
        short_name: 'Revela',
        description: 'Editor de fotos gratuito que trabaja en tu teléfono, sin nube.',
        lang: 'es',
        dir: 'ltr',
        start_url: base,
        scope: base,
        display: 'standalone',
        orientation: 'portrait',
        background_color: '#0e0f12',
        theme_color: '#0e0f12',
        icons: [
          { src: 'icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: 'icons/icon-512.png', sizes: '512x512', type: 'image/png' },
          { src: 'icons/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        // Todo, incluido el motor (.wasm), queda guardado para usar la app sin conexión.
        globPatterns: ['**/*.{js,css,html,wasm,png,svg,ico,webmanifest}'],
        maximumFileSizeToCacheInBytes: 32 * 1024 * 1024,
        cleanupOutdatedCaches: true,
      },
    }),
  ],
  worker: { format: 'es' },
  build: { target: 'es2022' },
});
