import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: {
    outDir: "build"
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
