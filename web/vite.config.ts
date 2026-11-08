import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    // Keep the shipped bundle readable instead of minified.
    minify: false
  },
  server: {
    port: 5000,
    proxy: {
      '/api': 'http://localhost:8000'
    }
  }
})