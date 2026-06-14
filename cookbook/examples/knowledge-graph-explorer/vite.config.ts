import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "VITE_");
  const target = env.VITE_LEANCTX_BASE_URL || "http://127.0.0.1:8080";

  return {
    plugins: [react()],
    // esbuild >=0.28 errors when down-transpiling destructuring for the legacy
    // default target (es2020 + browser overrides) in some bundled deps (mermaid).
    // A modern target skips that lowering entirely; demo app only targets evergreen.
    build: {
      target: "es2022",
    },
    server: {
      port: 5173,
      strictPort: true,
      proxy: {
        "/leanctx": {
          target,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/leanctx/, ""),
        },
      },
    },
  };
});
