<template>
    <div id="toolbar">
        <n-button quaternary id="sidebar_toggle" title="Toggle sidebar"
            @click="$emit('toggle-sidebar')">☰</n-button>

        <div id="inputs_row">
            <!-- SQL input (2/3) -->
            <div id="sql_wrapper">
                <n-input
                    :value="sql"
                    clearable
                    placeholder="SELECT * FROM alias LIMIT 100"
                    @update:value="$emit('update:sql', $event)"
                    @keyup.enter="$emit('execute')"
                >
                    <template #suffix>
                        <span class="expand-btn" title="Open in editor (Ctrl+E)" @click="$emit('expand-sql')">⤢</span>
                        <span class="run-btn" title="Execute (Enter)" @click="$emit('execute')">▶</span>
                    </template>
                </n-input>
            </div>

            <!-- Filter input (1/3) -->
            <div id="filter_wrapper">
                <n-input
                    :value="searchText"
                    clearable
                    :placeholder="searchMode === 'regex' ? 'RegExp…' : searchMode === 'fuzzy' ? 'Fuzzy…' : 'Filter…'"
                    @update:value="$emit('update:search-text', $event)"
                >
                    <template #suffix>
                        <span
                            v-for="m in SEARCH_MODES" :key="m.value"
                            :class="['mode-btn', { active: searchMode === m.value }]"
                            :title="m.title"
                            @click.stop="$emit('update:search-mode', m.value)"
                        >{{ m.label }}</span>
                    </template>
                </n-input>
            </div>
        </div>

        <n-dropdown trigger="click" :options="settingsOptions" @select="$emit('settings-select', $event)">
            <n-button quaternary id="gear_btn" title="Settings">⚙</n-button>
        </n-dropdown>
    </div>
</template>

<script setup lang="ts">
import { NButton, NDropdown, NInput } from 'naive-ui'
import { SEARCH_MODES } from '../composables/useSearch'
import type { SearchMode } from '../types'

defineProps<{
    sql: string
    searchText: string
    searchMode: SearchMode
    settingsOptions: any[]
}>()

defineEmits<{
    (e: 'toggle-sidebar'): void
    (e: 'update:sql', val: string): void
    (e: 'update:search-text', val: string): void
    (e: 'update:search-mode', val: SearchMode): void
    (e: 'execute'): void
    (e: 'expand-sql'): void
    (e: 'settings-select', key: string): void
}>()
</script>

<style scoped>
#toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 5px;
    flex-shrink: 0;
}

/* inputs_row claims all free space between the two icon buttons,
   then splits it 2:1 between SQL input and filter input */
#inputs_row {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 8px;
    align-items: center;
}

#sql_wrapper    { flex: 2; min-width: 0; }
#filter_wrapper { flex: 1; min-width: 0; }

/* Make the NInput fill its wrapper (deep because it's a Naive UI internal) */
#sql_wrapper :deep(.n-input)    { width: 100%; }
#filter_wrapper :deep(.n-input) { width: 100%; }

.expand-btn {
    cursor: pointer;
    font-size: 13px;
    padding: 0 3px;
    opacity: 0.45;
    user-select: none;
    transition: opacity 0.1s;
}
.expand-btn:hover { opacity: 0.9; }

.run-btn {
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
    opacity: 0.6;
    user-select: none;
    transition: opacity 0.1s;
}
.run-btn:hover { opacity: 1; }

.mode-btn {
    cursor: pointer;
    font-size: 11px;
    font-family: monospace;
    padding: 1px 4px;
    border-radius: 3px;
    opacity: 0.4;
    user-select: none;
    transition: opacity 0.1s;
}
.mode-btn:hover  { opacity: 0.7; }
.mode-btn.active { opacity: 1; font-weight: 700; }
</style>
