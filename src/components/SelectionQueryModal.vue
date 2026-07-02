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
                >
                    ▶ Run
                </n-button>
                <n-tag v-if="errMsg" type="error">{{ errMsg }}</n-tag>
            </div>
        </template>

        <!-- Results view -->
        <template v-else>
            <div class="results-header">
                <n-button text @click="view = 'editor'">← Query</n-button>
                <span class="result-info">{{ resultInfo }}</span>
            </div>
            <n-data-table
                :columns="resultCols"
                :data="resultRows"
                :max-height="480"
                size="small"
                style="font-size: 12px;"
            />
        </template>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NButton, NDataTable, NTag } from 'naive-ui'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
    show: boolean
    entriesJson: string
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

const view    = ref<'editor' | 'results'>('editor')
const sqlText = ref('')
const loading = ref(false)
const errMsg  = ref('')

const resultCols = ref<any[]>([])
const resultRows = ref<any[]>([])
const resultInfo = ref('')

const EDITOR_OPTIONS = {
    minimap:              { enabled: false },
    fontSize:             13,
    wordWrap:             'on' as const,
    scrollBeyondLastLine: false,
    automaticLayout:      true,
}

async function run() {
    const sql = sqlText.value.trim()
    if (!sql) return

    loading.value = true
    errMsg.value  = ''
    try {
        const raw = await invoke<string>('execute_query', {
            entriesJson: props.entriesJson,
            sql,
        })
        const result = JSON.parse(raw)
        if (result.err_msg) {
            errMsg.value = result.err_msg
        } else {
            resultCols.value = (result.headers as string[]).map(h => ({
                title:     h,
                key:       h,
                resizable: true,
                ellipsis:  { tooltip: true },
            }))
            resultRows.value = result.body
            resultInfo.value = `${result.row_count} rows × ${result.col_count} cols`
            errMsg.value     = ''
            view.value       = 'results'
            emit('query-executed', sql)
        }
    } finally {
        loading.value = false
    }
}

watch(() => props.initialQuery, q => {
    if (q) {
        sqlText.value = q
        view.value    = 'editor'
        errMsg.value  = ''
    }
})

watch(() => props.show, open => {
    if (!open) {
        view.value       = 'editor'
        errMsg.value     = ''
        resultCols.value = []
        resultRows.value = []
        resultInfo.value = ''
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
.results-header {
    display:         flex;
    align-items:     center;
    justify-content: space-between;
    margin-bottom:   8px;
}
.result-info {
    font-size: 12px;
    opacity:   0.6;
}
</style>
