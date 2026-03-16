import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
//
// CSS strategy:
// - ALL widget CSS lives in widget.css and is imported with ?inline
// - The ?inline import returns CSS as a string for manual injection into Shadow DOM
// - We do NOT use vite-plugin-css-injected-by-js because it injects into document.head
//   which does NOT reach Shadow DOM; all CSS MUST be injected into the shadow root manually
export default defineConfig({
  plugins: [
    react(),
  ],
  define: {
    'process.env': {},
    'global': 'globalThis',
  },
  build: {
    cssCodeSplit: false,
    lib: {
      entry: 'src/main.tsx',
      name: 'efofxWidget',
      fileName: () => 'embed.js',
      formats: ['iife'],
    },
    rollupOptions: {
      output: {
        entryFileNames: 'embed.js',
        assetFileNames: 'embed.[ext]',
        exports: 'named',
      },
    },
  },
})
