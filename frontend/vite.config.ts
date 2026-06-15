import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const rust = 'http://127.0.0.1:8766'

// Dev-only proxy. React now owns the whole UI (auth + app + public link);
// only the JSON API is served by Rust. In production Caddy handles /api.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: rust, changeOrigin: false },
    },
  },
})
