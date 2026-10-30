import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";
export default defineConfig({
  plugins: [
    // Sonner also injects these styles at runtime. Keep them in app.css so the
    // production CSP needs no inline-style exception for toast positioning.
    {
      name: "static-sonner-styles",
      enforce: "pre",
      transform(code, id) {
        if (!id.replaceAll("\\", "/").endsWith("/sonner/dist/index.mjs"))
          return;
        const injector = /function __insertCSS\(code\) \{[\s\S]*?\n\}/;
        if (!injector.test(code))
          throw new Error(
            "Sonner style injector changed; review CSP integration",
          );
        return {
          code: code.replace(injector, "function __insertCSS() {}"),
          map: null,
        };
      },
    },
    react(),
    tailwindcss(),
    {
      name: "version-assets",
      apply: "build",
      transformIndexHtml: {
        order: "post",
        handler: (html) =>
          html.replace(/(\/(?:app\.js|app\.css))"/g, '$1?v={{ASSET_VERSION}}"'),
      },
    },
  ],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/favicon.svg": "http://127.0.0.1:8080",
    },
  },
  build: {
    outDir: "../web/dist",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        chunkFileNames: "chunk-[name].js",
        assetFileNames: (asset) =>
          asset.names?.some((name) => name.endsWith(".css"))
            ? "app.css"
            : "[name][extname]",
      },
    },
  },
  test: { environment: "jsdom", setupFiles: ["./src/test-setup.js"] },
});
