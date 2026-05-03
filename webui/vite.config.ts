import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'node:path'

const devTarget = process.env.VITE_DEV_API_TARGET ?? 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [vue()],
  build: {
    rollupOptions: {
      input: {
        index: resolve(__dirname, 'index.html'),
        signin: resolve(__dirname, 'signin.html'),
        session: resolve(__dirname, 'session.html'),
        nodes: resolve(__dirname, 'nodes.html'),
        peers: resolve(__dirname, 'peers.html'),
        compose: resolve(__dirname, 'compose.html'),
      },
    },
  },
  server: {
    proxy: {
      '/api': {
        target: devTarget,
        changeOrigin: true,
      },
    },
  },
})