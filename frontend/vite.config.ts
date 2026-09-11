/// <reference types="vitest/config" />
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    // Kehityksessä backend ajetaan erikseen (`cargo run`) portissa 8080.
    // Proxy pitää cookiet same-originina, joten CORS-asetuksia ei tarvita.
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8787',
        changeOrigin: false,
      },
    },
  },
  build: {
    // Recharts on iso; yksi 700 kt:n chunk on tälle sovellukselle hyväksyttävä.
    chunkSizeWarningLimit: 800,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
  },
})
