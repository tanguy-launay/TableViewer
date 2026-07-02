import { ref } from "vue";
import type { Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save, message } from "@tauri-apps/plugin-dialog";
import type { FileEntry } from "../types";

const MAX_HISTORY = 10;

export function useQuery(
  fileEntries: Ref<FileEntry[]>,
  sql: Ref<string>,
  buildEntriesJson: () => string,
) {
  const tbHeaders = ref<any[]>([]);
  const tbBody = ref<any[]>([]);
  const tbRows = ref(0);
  const tbCols = ref(0);

  const queryHistory = ref<string[]>([]);
  const pinnedQueries = ref<string[]>([]);
  const showExportCsvModal = ref(false);
  const exportCsvSep = ref(",");

  function togglePin(q: string) {
    if (pinnedQueries.value.includes(q)) {
      pinnedQueries.value = pinnedQueries.value.filter((p) => p !== q);
    } else {
      pinnedQueries.value = [q, ...pinnedQueries.value];
    }
  }

  function deleteHistory(q: string) {
    queryHistory.value  = queryHistory.value.filter(h => h !== q)
    pinnedQueries.value = pinnedQueries.value.filter(p => p !== q)
  }

  function pushHistory(q: string) {
    queryHistory.value = [
      q,
      ...queryHistory.value.filter((h) => h !== q),
    ].slice(0, MAX_HISTORY);
  }

  async function exportHistoryItem(sql: string, format: 'csv' | 'parquet') {
    if (!sql.trim() || fileEntries.value.length === 0) return
    if (format === 'parquet') {
      const dest = await save({ filters: [{ name: 'Parquet', extensions: ['parquet'] }] })
      if (!dest) return
      const result = JSON.parse(
        await invoke<string>('export_parquet', { entriesJson: buildEntriesJson(), sql, dest })
      )
      if ('err_msg' in result) await message(result.err_msg, { title: 'Export error', kind: 'error' })
      else                     await message(`Saved to ${dest}`, { title: 'Export complete' })
    } else {
      const dest = await save({ filters: [{ name: 'CSV', extensions: ['csv'] }] })
      if (!dest) return
      const result = JSON.parse(
        await invoke<string>('export_csv_file', {
          entriesJson: buildEntriesJson(), sql, dest,
          sep: ','.charCodeAt(0),
        })
      )
      if ('err_msg' in result) await message(result.err_msg, { title: 'Export error', kind: 'error' })
      else                     await message(`Saved to ${dest}`, { title: 'Export complete' })
    }
  }

  async function setUi(jStr: string) {
    const table = JSON.parse(jStr);
    if ("err_msg" in table) {
      await message(table.err_msg, { title: "Query error", kind: "error" });
    } else {
      tbHeaders.value = table.headers;
      tbBody.value = table.body;
      tbRows.value = table.row_count;
      tbCols.value = table.col_count;
    }
  }

  async function executeSql() {
    if (!sql.value.trim() || fileEntries.value.length === 0) return;
    pushHistory(sql.value);
    const result = await invoke<string>("execute_query", {
      entriesJson: buildEntriesJson(),
      sql: sql.value,
    });
    await setUi(result);
  }

  async function exportHistory() {
    if (queryHistory.value.length === 0) return;
    const dest = await save({
      filters: [{ name: "SQL File", extensions: ["sql"] }],
    });
    if (!dest) return;
    const content = queryHistory.value
      .map((q, i) => `-- Query ${i + 1}\n${q.trimEnd()};`)
      .join("\n\n");
    const result = JSON.parse(
      await invoke<string>("write_text_file", { content, dest }),
    );
    if ("err_msg" in result) {
      await message(result.err_msg, { title: "Export error", kind: "error" });
    } else {
      await message(`Saved to ${dest}`, { title: "History exported" });
    }
  }

  async function doExport(format: "parquet" | "csv") {
    if (!sql.value.trim() || fileEntries.value.length === 0) return;
    if (format === "csv") {
      exportCsvSep.value = ",";
      showExportCsvModal.value = true;
      return;
    }
    const dest = await save({
      filters: [{ name: "Parquet", extensions: ["parquet"] }],
    });
    if (!dest) return;
    const result = JSON.parse(
      await invoke<string>("export_parquet", {
        entriesJson: buildEntriesJson(),
        sql: sql.value,
        dest,
      }),
    );
    if ("err_msg" in result) {
      await message(result.err_msg, { title: "Export error", kind: "error" });
    } else {
      await message(`Saved to ${dest}`, { title: "Export complete" });
    }
  }

  async function confirmExportCsv() {
    const dest = await save({
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!dest) return;
    const result = JSON.parse(
      await invoke<string>("export_csv_file", {
        entriesJson: buildEntriesJson(),
        sql: sql.value,
        dest,
        sep: exportCsvSep.value.charCodeAt(0),
      }),
    );
    if ("err_msg" in result) {
      await message(result.err_msg, { title: "Export error", kind: "error" });
    } else {
      await message(`Saved to ${dest}`, { title: "Export complete" });
    }
  }

  return {
    tbHeaders,
    tbBody,
    tbRows,
    tbCols,
    queryHistory,
    pinnedQueries,
    togglePin,
    pushHistory,
    deleteHistory,
    exportHistoryItem,
    executeSql,
    exportHistory,
    showExportCsvModal,
    exportCsvSep,
    doExport,
    confirmExportCsv,
  };
}
