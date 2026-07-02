<template>
    <div :class="['sidebar', { 'sidebar--collapsed': !open }]">

        <!-- ── Files ─────────────────────────────────────────────── -->
        <div class="sidebar-section">
            <div class="sidebar-header">
                <span class="sidebar-label">Files</span>
                <n-dropdown trigger="click" :options="FILETYPE_OPTIONS" @select="$emit('open-file', $event)">
                    <n-button size="small" type="primary">+ Open</n-button>
                </n-dropdown>
            </div>

            <div v-if="entries.length === 0" class="sidebar-empty">No files open</div>

            <div v-for="entry in entries" :key="entry.id" class="file-entry">
                <n-tag size="small" :type="typeColor(entry.filetype)" class="type-tag">
                    {{ entry.filetype }}
                </n-tag>
                <n-input
                    :value="entry.alias"
                    size="small"
                    class="alias-input"
                    placeholder="alias"
                    @update:value="$emit('update-alias', entry.id, $event)"
                />
                <n-button
                    size="small" quaternary circle
                    class="remove-btn"
                    @click="$emit('remove-entry', entry.id)"
                >✕</n-button>
            </div>
        </div>

        <div class="sidebar-divider" />

        <!-- ── Schema ────────────────────────────────────────────── -->
        <div class="sidebar-section">
            <div class="sidebar-header">
                <span class="sidebar-label">Schema</span>
            </div>

            <div v-if="entries.length === 0" class="sidebar-empty">No files open</div>

            <div v-for="entry in entries" :key="entry.id" class="schema-table">
                <div class="schema-table-header" @click="$emit('toggle-schema', entry.id)">
                    <span class="schema-chevron">{{ expandedSchemas[entry.id] ? '▾' : '▸' }}</span>
                    <span class="schema-alias">{{ entry.alias }}</span>
                    <span class="schema-count" v-if="schemaMap[entry.id]">
                        {{ schemaMap[entry.id].length }}
                    </span>
                </div>
                <div v-if="expandedSchemas[entry.id]" class="schema-columns">
                    <div
                        v-for="col in schemaMap[entry.id]" :key="col.name"
                        class="schema-col"
                        :title="col.name + ' — ' + col.dtype"
                    >
                        <span class="schema-col-name">{{ col.name }}</span>
                        <span class="schema-col-type">{{ col.dtype }}</span>
                    </div>
                </div>
            </div>
        </div>

        <div class="sidebar-divider" />

        <!-- ── History ───────────────────────────────────────────── -->
        <div class="sidebar-section">
            <div class="sidebar-header">
                <span class="sidebar-label">History</span>
                <div style="display:flex;gap:4px">
                    <n-button size="small" quaternary @click="$emit('export-history')">Export</n-button>
                    <n-button size="small" quaternary @click="$emit('clear-history')">Clear</n-button>
                </div>
            </div>

            <div v-if="pinnedQueries.length > 0" class="history-section">
                <span class="history-section-label">📌 Pinned</span>
                <div
                    v-for="q in pinnedQueries" :key="'pin-'+q"
                    class="history-item history-item--pinned"
                    :title="q"
                    @click="$emit('use-history', q)"
                >
                    <span class="history-item-text">{{ q }}</span>
                    <span class="pin-btn pin-btn--active" title="Unpin" @click.stop="$emit('toggle-pin', q)">📌</span>
                </div>
            </div>

            <div v-if="unpinnedHistory.length === 0 && pinnedQueries.length === 0" class="sidebar-empty">No history yet</div>

            <div v-if="unpinnedHistory.length > 0" class="history-section">
                <span v-if="pinnedQueries.length > 0" class="history-section-label">Recent</span>
                <div
                    v-for="(q, i) in unpinnedHistory" :key="i"
                    class="history-item"
                    :title="q"
                    @click="$emit('use-history', q)"
                >
                    <span class="history-item-text">{{ q }}</span>
                    <span class="pin-btn" title="Pin" @click.stop="$emit('toggle-pin', q)">📌</span>
                </div>
            </div>
        </div>

    </div>
</template>

<script setup lang="ts">
import { NButton, NDropdown, NInput, NTag } from 'naive-ui'
import { computed } from 'vue'
import { FILETYPE_OPTIONS } from '../composables/useFiles'
import type { FileEntry, ColumnInfo } from '../types'

const props = defineProps<{
    open: boolean
    entries: FileEntry[]
    schemaMap: Record<string, ColumnInfo[]>
    expandedSchemas: Record<string, boolean>
    history: string[]
    pinnedQueries: string[]
}>()

const unpinnedHistory = computed(() =>
    props.history.filter(q => !props.pinnedQueries.includes(q))
)

defineEmits<{
    (e: 'open-file', key: string): void
    (e: 'remove-entry', id: string): void
    (e: 'update-alias', id: string, alias: string): void
    (e: 'toggle-schema', id: string): void
    (e: 'export-history'): void
    (e: 'clear-history'): void
    (e: 'use-history', q: string): void
    (e: 'toggle-pin', q: string): void
}>()

function typeColor(ft: string): 'success' | 'info' | 'warning' | 'error' | 'default' {
    if (ft === 'parquet')               return 'success'
    if (ft === 'arrow')                 return 'info'
    if (ft === 'csv')                   return 'warning'
    if (ft === 'json' || ft === 'jsonl') return 'error'
    if (ft === 'yaml' || ft === 'yml')  return 'info'
    if (ft === 'xml')                   return 'warning'
    if (ft === 'toml')                  return 'default'
    if (ft === 'duckdb')                return 'warning'
    if (ft === 'sqlite')                return 'default'
    return 'default'
}
</script>

<style scoped>
.sidebar {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--sidebar-border);
    padding: 6px 0;
    transition: width 0.2s ease, padding 0.2s ease, border 0.2s ease;
}

.sidebar--collapsed {
    width: 0;
    padding: 0;
    border-right: none;
}

.sidebar-section  { padding: 6px 10px; }

.sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
}

.sidebar-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.5;
}

.sidebar-divider {
    height: 1px;
    background: var(--sidebar-border);
    margin: 2px 0;
}

.sidebar-empty {
    font-size: 12px;
    color: var(--empty-color);
    padding: 2px 0;
}

/* File entries */
.file-entry {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 5px;
}

.type-tag    { flex-shrink: 0; }
.alias-input { flex: 1; min-width: 0; }
.remove-btn  { flex-shrink: 0; }

/* History */
.history-section { margin-bottom: 4px; }

.history-section-label {
    display: block;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.4;
    padding: 2px 4px 1px;
    margin-top: 4px;
}

.history-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-family: monospace;
    padding: 3px 4px 3px 6px;
    border-radius: 4px;
    cursor: pointer;
    margin-bottom: 2px;
}
.history-item:hover { background: var(--history-hover); }
.history-item:hover .pin-btn { opacity: 0.5; }

.history-item--pinned { border-left: 2px solid #18a058; padding-left: 4px; }

.history-item-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.pin-btn {
    flex-shrink: 0;
    font-size: 11px;
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.1s;
    line-height: 1;
}
.pin-btn--active { opacity: 0.7; }
.pin-btn--active:hover { opacity: 1; }

/* Schema */
.schema-table  { margin-bottom: 2px; }

.schema-table-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 4px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    user-select: none;
}
.schema-table-header:hover { background: var(--history-hover); }

.schema-chevron { flex-shrink: 0; font-size: 10px; width: 10px; }
.schema-alias   { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.schema-count   { flex-shrink: 0; font-size: 10px; opacity: 0.45; }

.schema-columns { padding: 2px 0 4px 16px; }

.schema-col {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 1px 2px;
    gap: 8px;
}

.schema-col-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    font-family: monospace;
}

.schema-col-type {
    flex-shrink: 0;
    font-size: 10px;
    font-family: monospace;
    opacity: 0.55;
}
</style>
