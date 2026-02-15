import { defineConfig } from "vite";

export default defineConfig({
  root: "src",
  server: {
    port: 1420,
    strictPort: false,
    host: "127.0.0.1",
  },
});
