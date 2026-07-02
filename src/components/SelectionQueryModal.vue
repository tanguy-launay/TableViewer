<template>
    <n-modal
        v-model:show="localShow"
        preset="card"
        title="Query"
        style="width: min(860px, 92vw); max-height: 92vh;"
        :segmented="{ content: true }"
    >
        <!-- Editor view -->
        <template v-if="view === 'editor'">
            <vue-monaco-editor
                v-model:value="sqlText"
                language="sql"
                theme="vs-dark"
                height="300px"
                :options="EDITOR_OPTIONS"
            />
            <div class="run-bar">
                <n-button
                    type="primary"
                    :disabled="!sqlText.trim() || loading"
                    :loading="loading"
                    @click="run"
                >▶ Run</n-button>
                <n-tag v-if="errMsg" type="error">{{ errMsg }}</n-tag>
            </div>
        </template>

        <!-- Results view -->
        <template v-else>
            <!-- shape + pagination bar (mirrors main frame) -->
            <div class="shape-bar">
                <span class="shape-left">{{ resultInfo }}</span>
                <n-pagination
                    v-model:page="currentPage"
                    :page-count="pageCount"
                    size="small"
                />
                <span class="shape-right">{{ PAGE_SIZE }}/page</span>
            </div>

            <n-data-table
                :columns="resultCols"
                :data="pagedRows"
                :max-height="460"
                size="small"
                style="font-size: 12px;"
            />

            <n-button text size="small" class="back-btn" @click="view = 'editor'">
                ← Query
            </n-button>
        </template>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NButton, NDataTable, NTag, NPagination } from 'naive-ui'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
    show:         boolean
    entriesJson:  string
    initialQuery: string
}>()

const emit = defineEmits<{
    (e: 'update:show', v: boolean): void
    (e: 'query-executed', sql: string): void
}>()

const localShow = computed({
    get: () => props.show,
    set: v => emit('update:show', v),
})

// ── Views ─────────────────────────────────────────────────────────────────
const view    = ref<'editor' | 'results'>('editor')
const sqlText = ref('')
const loading = ref(false)
const errMsg  = ref('')

// ── Results + pagination ──────────────────────────────────────────────────
const PAGE_SIZE   = 100
const currentPage = ref(1)
const resultCols  = ref<any[]>([])
const allRows     = ref<any[]>([])   // full result set (never sliced)
const resultInfo  = ref('')

const pageCount = computed(() =>
    Math.max(1, Math.ceil(allRows.value.length / PAGE_SIZE))
)

const pagedRows = computed(() => {
    const start = (currentPage.value - 1) * PAGE_SIZE
    return allRows.value.slice(start, start + PAGE_SIZE)
})

// ── Editor options ────────────────────────────────────────────────────────
const EDITOR_OPTIONS = {
    minimap:              { enabled: false },
    fontSize:             13,
    wordWrap:             'on' as const,
    scrollBeyondLastLine: false,
    automaticLayout:      true,
}

// ── Run ───────────────────────────────────────────────────────────────────
async function run() {
    const sql = sqlText.value.trim()
    if (!sql) return

    loading.value = true
    errMsg.value  = ''
    try {
        const raw    = await invoke<string>('execute_query', { entriesJson: props.entriesJson, sql })
        const result = JSON.parse(raw)
        if (result.err_msg) {
            errMsg.value = result.err_msg
        } else {
            resultCols.value = (result.headers as any[]).map(h => ({ ...h, ellipsis: { tooltip: true } }))
            allRows.value    = result.body as any[]
            resultInfo.value = `${result.row_count} rows × ${result.col_count} cols`
            currentPage.value = 1
            view.value        = 'results'
            emit('query-executed', sql)
        }
    } finally {
        loading.value = false
    }
}

// ── Sync initialQuery ─────────────────────────────────────────────────────
watch(() => props.initialQuery, q => {
    if (q) { sqlText.value = q; view.value = 'editor'; errMsg.value = '' }
})

// ── Reset on close ────────────────────────────────────────────────────────
watch(() => props.show, open => {
    if (!open) {
        view.value        = 'editor'
        errMsg.value      = ''
        resultCols.value  = []
        allRows.value     = []
        resultInfo.value  = ''
        currentPage.value = 1
    }
})
</script>

<style scoped>
.run-bar {
    display:     flex;
    align-items: center;
    gap:         10px;
    margin-top:  10px;
}

/* mirrors the main-frame shape bar */
.shape-bar {
    display:         flex;
    align-items:     center;
    padding:         0 2px;
    height:          34px;
    margin-bottom:   6px;
    border-bottom:   1px solid var(--sidebar-border, #2e2e3a);
}

.shape-left {
    flex:        1;
    font-size:   11px;
    opacity:     0.6;
    white-space: nowrap;
    overflow:    hidden;
    text-overflow: ellipsis;
}

.shape-right {
    flex:        1;
    font-size:   11px;
    opacity:     0.6;
    white-space: nowrap;
    text-align:  right;
}

.back-btn {
    margin-top: 8px;
    opacity:    0.7;
}
.back-btn:hover { opacity: 1; }
</style>
