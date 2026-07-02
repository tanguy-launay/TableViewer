import { ref, watch } from "vue";
import type { Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { FileEntry, ColumnInfo } from "../types";

export const FILETYPE_OPTIONS = [
  { label: "parquet(s)", key: "parquet" },
  { label: "feather(s)", key: "arrow" },
  { label: "csv(s)", key: "csv" },
  { label: "json", key: "json" },
  { label: "jsonl", key: "jsonl" },
  { label: "yaml", key: "yaml" },
  { label: "xml", key: "xml" },
  { label: "toml", key: "toml" },
  { label: "DuckDB", key: "duckdb" },
  { label: "SQLite", key: "sqlite" },
];

export const CSV_SEP_OPTIONS = [
  { label: "comma (,)", value: "," },
  { label: "tab (\\t)", value: "\t" },
  { label: "semicolon (;)", value: ";" },
];

const Extensions: Record<string, string[]> = {
  parquet: ["parquet"],
  arrow: ["arrow", "ipc", "feather"],
  csv: ["csv", "tsv"],
  json: ["json"],
  jsonl: ["jsonl", "ndjson"],
  yaml: ["yaml", "yml"],
  xml: ["xml"],
  toml: ["toml"],
  duckdb: ["duckdb"],
  sqlite: ["sqlite", "sqlite3", "db"],
};

// File types that go through JaqModal before import
const JAQ_TYPES = new Set(["json", "jsonl", "yaml", "xml", "toml"]);

export function useFiles(sql: Ref<string>) {
  const fileEntries = ref<FileEntry[]>([]);
  const schemaMap = ref<Record<string, ColumnInfo[]>>({});
  const expandedSchemas = ref<Record<string, boolean>>({});

  // CSV open modal
  const showCsvModal = ref(false);
  const pendingSep = ref(",");
  const pendingFiles = ref<string[]>([]);

  // Jaq preprocessing modal
  const showJaqModal = ref(false);
  const jaqPendingPaths = ref<string[]>([]);
  const jaqPendingType = ref("");

  function buildEntriesJson(): string {
    return JSON.stringify(
      fileEntries.value.map((e) => ({
        alias: e.alias,
        filenames: e.filenames,
        filetype: e.filetype,
        sep: e.filetype === "csv" ? e.sep.charCodeAt(0) : null,
        jaq_filter: e.jaq_filter ?? null,
      })),
    );
  }

  function makeAlias(filenames: string[]): string {
    const base = (filenames[0] ?? "table")
      .split("/")
      .pop()!
      .replace(/\.[^.]+$/, "")
      .replace(/[^a-zA-Z0-9]/g, "_");
    const safe = /^[0-9]/.test(base) ? `t_${base}` : base || "table";
    const taken = fileEntries.value.map((e) => e.alias);
    if (!taken.includes(safe)) return safe;
    let i = 1;
    while (taken.includes(`${safe}_${i}`)) i++;
    return `${safe}_${i}`;
  }

  function addEntry(
    filenames: string[],
    filetype: FileEntry["filetype"],
    sep: string,
    jaq_filter = "",
  ) {
    const alias = makeAlias(filenames);
    fileEntries.value.push({
      id: crypto.randomUUID(),
      alias,
      filenames,
      filetype,
      sep,
      jaq_filter,
    });
    if (!sql.value) sql.value = `SELECT * FROM ${alias} LIMIT 100`;
  }

  function removeEntry(id: string) {
    fileEntries.value = fileEntries.value.filter((e) => e.id !== id);
  }

  function updateAlias(id: string, alias: string) {
    const entry = fileEntries.value.find((e) => e.id === id);
    if (entry) entry.alias = alias;
  }

  function toggleSchema(id: string) {
    expandedSchemas.value[id] = !expandedSchemas.value[id];
  }

  // Reload schema whenever the file list changes
  watch(
    () => fileEntries.value.map((e) => e.id).join(","),
    async () => {
      if (fileEntries.value.length === 0) {
        schemaMap.value = {};
        return;
      }
      const res = JSON.parse(
        await invoke<string>("get_schema", {
          entriesJson: buildEntriesJson(),
        }),
      );
      if (!res.ok) return;
      const map: Record<string, ColumnInfo[]> = {};
      res.tables.forEach((t: { columns: ColumnInfo[] }, i: number) => {
        map[fileEntries.value[i].id] = t.columns;
      });
      schemaMap.value = map;
    },
  );

  async function openFile(key: string) {
    const selected = await open({
      multiple: true,
      filters: [
        { name: "Table File(s)", extensions: Extensions[key] ?? ["*"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (!Array.isArray(selected) || selected.length === 0) return;

    if (key === "csv") {
      pendingFiles.value = selected;
      pendingSep.value = ",";
      showCsvModal.value = true;
    } else if (JAQ_TYPES.has(key)) {
      // Open JaqModal for structured text formats
      jaqPendingPaths.value = selected;
      jaqPendingType.value = key;
      showJaqModal.value = true;
    } else {
      addEntry(selected, key as FileEntry["filetype"], ",");
    }
  }

  function confirmCsvOpen() {
    addEntry(pendingFiles.value, "csv", pendingSep.value);
  }

  function confirmJaqOpen(filter: string) {
    addEntry(
      jaqPendingPaths.value,
      jaqPendingType.value as FileEntry["filetype"],
      ",",
      filter,
    );
  }

  return {
    fileEntries,
    schemaMap,
    expandedSchemas,
    showCsvModal,
    pendingSep,
    showJaqModal,
    jaqPendingPaths,
    jaqPendingType,
    buildEntriesJson,
    openFile,
    confirmCsvOpen,
    confirmJaqOpen,
    removeEntry,
    updateAlias,
    toggleSchema,
  };
}
