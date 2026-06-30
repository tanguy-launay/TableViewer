/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// Monaco ESM subpath declarations (we import selectively to exclude unused languages)
declare module "monaco-editor/esm/vs/editor/editor.api" {
  export * from "monaco-editor";
}
declare module "monaco-editor/esm/vs/basic-languages/sql/sql.contribution" {
  // side-effect import — registers the SQL tokeniser with Monaco
}
