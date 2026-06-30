import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import monacoEditorPlugin from "vite-plugin-monaco-editor";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    // Bundle Monaco workers locally so the app works offline (Tauri)
    (monacoEditorPlugin as any).default({
      languageWorkers: ["editorWorkerService"],
    }),
  ],

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
}));
