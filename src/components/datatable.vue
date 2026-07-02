<template>
    <n-config-provider :theme="isDark ? darkTheme : null">
        <div :class="['app-root', isDark ? 'theme-dark' : 'theme-light']" :style="appStyle">
            <div class="layout">

                <AppSidebar
                    :open="sidebarOpen"
                    :entries="fileEntries"
                    :schema-map="schemaMap"
                    :expanded-schemas="expandedSchemas"
                    :history="queryHistory"
                    :pinned-queries="pinnedQueries"
                    @open-file="openFile"
                    @remove-entry="removeEntry"
                    @update-alias="updateAlias"
                    @toggle-schema="toggleSchema"
                    @export-history="exportHistory"
                    @clear-history="queryHistory = []"
                    @use-history="onUseHistory"
                    @toggle-pin="togglePin"
                />

                <div class="main-panel">
                    <AppToolbar
                        v-model:sql="sql"
                        v-model:search-text="searchText"
                        v-model:search-mode="searchMode"
                        :settings-options="settingsOptions"
                        @toggle-sidebar="sidebarOpen = !sidebarOpen"
                        @execute="runQuery"
                        @expand-sql="showSqlModal = true"
                        @settings-select="handleSettings"
                    />

                    <!-- Shape + pagination bar -->
                    <div v-if="tbRows > 0" class="shape-bar">
                        <span class="shape-left">
                            {{ tbRows }} rows &times; {{ tbCols }} cols
                            <template v-if="searchText.trim()">
                                &nbsp;·&nbsp; {{ filteredBody.length }} matching
                            </template>
                        </span>
                        <n-pagination
                            v-model:page="currentPage"
                            :page-count="pageCount"
                            size="small"
                        />
                        <span class="shape-right">{{ pageSize }}/page</span>
                    </div>

                    <DataGrid
                        :columns="renderedHeaders"
                        :data="slicedData"
                    />
                </div>
            </div>
        </div>

        <!-- SQL editor modal (Monaco) -->
        <SqlModal
            v-model="showSqlModal"
            :sql="sql"
            :is-dark="isDark"
            @execute="onModalExecute"
        />

        <!-- CSV open modal -->
        <n-modal
            v-model:show="showCsvModal"
            preset="dialog"
            title="CSV Separator"
            negative-text="Cancel"
            positive-text="Open"
            @positive-click="confirmCsvOpen"
        >
            <n-select v-model:value="pendingSep" :options="CSV_SEP_OPTIONS" />
        </n-modal>

        <!-- CSV export modal -->
        <n-modal
            v-model:show="showExportCsvModal"
            preset="dialog"
            title="CSV Export Delimiter"
            negative-text="Cancel"
            positive-text="Save…"
            @positive-click="confirmExportCsv"
        >
            <n-select v-model:value="exportCsvSep" :options="CSV_SEP_OPTIONS" />
        </n-modal>
    </n-config-provider>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NConfigProvider, NModal, NSelect, NPagination } from 'naive-ui'
import { darkTheme } from 'naive-ui'

import AppSidebar from './AppSidebar.vue'
import AppToolbar from './AppToolbar.vue'
import DataGrid   from './DataGrid.vue'
import SqlModal   from './SqlModal.vue'

import { useThemeZoom, ZOOM_MIN, ZOOM_MAX } from '../composables/useThemeZoom'
import { useFiles, CSV_SEP_OPTIONS } from '../composables/useFiles'
import { useQuery } from '../composables/useQuery'
import { useSearch } from '../composables/useSearch'

// ── Theme + zoom ──────────────────────────────────────────────────────────
const { isDark, zoom, appStyle, zoomIn, zoomOut, zoomReset, handleZoomKey } = useThemeZoom()

// ── SQL (shared: seeded by useFiles on first open, executed by useQuery) ──
const sql = ref('')

// ── Files + schema ────────────────────────────────────────────────────────
const {
    fileEntries, schemaMap, expandedSchemas,
    showCsvModal, pendingSep,
    buildEntriesJson,
    openFile, confirmCsvOpen,
    removeEntry, updateAlias, toggleSchema,
} = useFiles(sql)

// ── Query + history + export ──────────────────────────────────────────────
const {
    tbHeaders, tbBody, tbRows, tbCols,
    queryHistory, pinnedQueries, togglePin,
    executeSql, exportHistory,
    showExportCsvModal, exportCsvSep,
    doExport, confirmExportCsv,
} = useQuery(fileEntries, sql, buildEntriesJson)

// ── Search / filter ───────────────────────────────────────────────────────
const { searchText, searchMode, currentPage, filteredBody, renderedHeaders } = useSearch(tbHeaders, tbBody)

// ── SQL modal (Monaco editor) ─────────────────────────────────────────────
const showSqlModal = ref(false)

function runQuery() {
    currentPage.value = 1
    executeSql()
}

function onModalExecute(newSql: string) {
    sql.value = newSql
    currentPage.value = 1
    executeSql()
}

function onUseHistory(q: string) {
    sql.value = q
    runQuery()
}

// ── Sidebar ───────────────────────────────────────────────────────────────
const sidebarOpen = ref(true)

onMounted(() => {
    window.addEventListener('keydown', handleZoomKey)
})
onUnmounted(() => {
    window.removeEventListener('keydown', handleZoomKey)
})

// ── Pagination ────────────────────────────────────────────────────────────
const pageSize = ref(100)

const pageCount = computed(() =>
    Math.max(1, Math.ceil(filteredBody.value.length / pageSize.value))
)

const slicedData = computed(() => {
    const start = (currentPage.value - 1) * pageSize.value
    return filteredBody.value.slice(start, start + pageSize.value)
})

// ── Gear / settings menu ──────────────────────────────────────────────────
const settingsOptions = computed(() => [
    {
        label: isDark.value ? '☀️  Light mode' : '🌙  Dark mode',
        key: 'toggle-theme',
    },
    { type: 'divider', key: 'd1' },
    {
        label: 'Rows per page',
        key: 'rows',
        children: [100, 200, 500, 1000].map(n => ({
            label: (pageSize.value === n ? '✓ ' : '  ') + n,
            key: `rows-${n}`,
        })),
    },
    { type: 'divider', key: 'd2' },
    {
        label: 'Export as Parquet',
        key: 'export-parquet',
        disabled: !sql.value.trim() || fileEntries.value.length === 0,
    },
    {
        label: 'Export as CSV',
        key: 'export-csv',
        disabled: !sql.value.trim() || fileEntries.value.length === 0,
    },
    { type: 'divider', key: 'd3' },
    { label: `⊞  Zoom in  — ${zoom.value}%`, key: 'zoom-in',    disabled: zoom.value >= ZOOM_MAX },
    { label: `⊟  Zoom out`,                  key: 'zoom-out',   disabled: zoom.value <= ZOOM_MIN },
    { label: `↺  Reset zoom`,                key: 'zoom-reset', disabled: zoom.value === 100 },
])

function handleSettings(key: string) {
    if      (key === 'toggle-theme')   isDark.value = !isDark.value
    else if (key.startsWith('rows-')) {
        pageSize.value    = Number(key.slice(5))
        currentPage.value = 1
    }
    else if (key === 'export-parquet') doExport('parquet')
    else if (key === 'export-csv')     doExport('csv')
    else if (key === 'zoom-in')        zoomIn()
    else if (key === 'zoom-out')       zoomOut()
    else if (key === 'zoom-reset')     zoomReset()
}
</script>

<style>
/* ── Global reset ────────────────────────────────────────────────────────── */
* { box-sizing: border-box; }

body {
    margin: 0;
    transition: background 0.15s;
}

/* ── Theme tokens ────────────────────────────────────────────────────────── */
.theme-light {
    --sidebar-bg:     #f5f5f7;
    --sidebar-border: #e0e0e0;
    --history-hover:  #e8e8ec;
    --empty-color:    #aaa;
    --shape-bar-bg:   #efefef;
    --shape-bar-color:#666;
    background: #ffffff;
    color: #333;
}

.theme-dark {
    --sidebar-bg:     #1e1e26;
    --sidebar-border: #2e2e3a;
    --history-hover:  #2a2a38;
    --empty-color:    #555;
    --shape-bar-bg:   #1a1a22;
    --shape-bar-color:#888;
    background: #18181c;
    color: #ffffffd1;
}

/* ── App shell ───────────────────────────────────────────────────────────── */
.app-root {
    height: 100vh;
    overflow: hidden;
}

.layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
}

.main-panel {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.shape-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 10px;
    height: 36px;
    background: var(--shape-bar-bg);
    border-bottom: 1px solid var(--sidebar-border);
}

.shape-left {
    flex: 1;
    font-size: 11px;
    color: var(--shape-bar-color);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.shape-right {
    flex: 1;
    font-size: 11px;
    color: var(--shape-bar-color);
    white-space: nowrap;
    text-align: right;
}

/* ── Search highlight (used inside table cell render functions) ───────────── */
mark.hl {
    background: #ffe066;
    color: #000;
    border-radius: 2px;
    padding: 0 1px;
}
.theme-dark mark.hl {
    background: #b8860b;
    color: #fff;
}
</style>
