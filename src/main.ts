import { createApp } from "vue";
import App from "./App.vue";
import loader from "@monaco-editor/loader";

// Minimal Monaco: core editor API + SQL tokeniser only.
// Importing 'monaco-editor' would bundle every language; this pulls in just what we need.
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import "monaco-editor/esm/vs/basic-languages/sql/sql.contribution";

loader.config({ monaco });

createApp(App).mount("#app");
