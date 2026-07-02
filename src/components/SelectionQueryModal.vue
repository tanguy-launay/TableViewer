<template>
    <n-modal
        v-model:show="localShow"
        preset="card"
        title="Selection Query"
        style="width: min(860px, 92vw); max-height: 92vh;"
        :segmented="{ content: true }"
    >
        <!-- 1. Bindings ───────────────────────────────────────────────────── -->
        <div class="section-label">Bindings — referenced as $1, $2, … in the query</div>
        <div class="bindings-list">
            <div v-for="(_, idx) in localSels" :key="idx" class="binding-row">
                <code class="binding-label">${{ idx + 1 }}</code>
                <n-input
                    v-model:value="localSels[idx]"
                    size="small"
                    class="binding-input"
                />
                <n-button
                    size="small"
                    quaternary
                    type="error"
                    class="binding-remove"
                    @click="removeBinding(idx)"
                >✕</n-button>
            </div>
            <n-button dashed size="small" class="add-btn" @click="addBinding">
                ＋ Add binding
            </n-button>
        </div>

        <!-- 2. SQL editor ─────────────────────────────────────────────────── -->
        <div class="section-label sql-label">
            SQL — $1, $2, … are replaced with quoted string literals
        </div>
        <vue-monaco-editor
            v-model:value="sqlText"
            language="sql"
            theme="vs-dark"
            height="140px"
            :options="EDITOR_OPTIONS"
            @keydown="onEditorKeydown"
        />

        <!-- 3. Run bar ────────────────────────────────────────────────────── -->
        <div class="run-bar">
            <n-button
                type="primary"
                size="small"
                :disabled="!sqlText.trim()"
                :loading="running"
                @click="run"
            >▶ Run (Ctrl+Enter)</n-button>
            <span v-if="resultInfo" class="result-info">{{ resultInfo }}</span>
            <n-tag v-if="errMsg" type="error" size="small" class="err-tag">{{ errMsg }}</n-tag>
        </div>

        <!-- 4. Results table ──────────────────────────────────────────────── -->
        <div v-if="resultCols.length" class="results-wrap">
            <n-data-table
                :columns="resultCols"
                :data="resultRows"
                size="small"
                :max-height="340"
                style="font-size: 12px;"
            />
        </div>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NInput, NButton, NDataTable, NTag } from 'naive-ui'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
    show: boolean
    entriesJson: string
    selections: string[]
}>()

const emit = defineEmits<{
    (e: 'update:show', v: boolean): void
    (e: 'update:selections', v: string[]): void
}>()

// ── Show ──────────────────────────────────────────────────────────────────
const localShow = computed({
    get: () => props.show,
    set: v => emit('update:show', v),
})

// ── Selections ────────────────────────────────────────────────────────────
const localSels = ref<string[]>([...props.selections])

// Sync inbound changes (e.g. parent pushes a new $n while modal is open)
watch(
    () => props.selections,
    sels => { localSels.value = [...sels] },
)

// Propagate local edits upward
watch(
    localSels,
    sels => emit('update:selections', [...sels]),
    { deep: true },
)

function addBinding() {
    localSels.value = [...localSels.value, '']
}

function removeBinding(idx: number) {
    const next = [...localSels.value]
    next.splice(idx, 1)
    localSels.value = next
}

// ── Editor ────────────────────────────────────────────────────────────────
const sqlText = ref('')

const EDITOR_OPTIONS = {
    minimap:              { enabled: false },
    lineNumbers:          'off' as const,
    fontSize:             13,
    wordWrap:             'on' as const,
    scrollBeyondLastLine: false,
    automaticLayout:      true,
}

function onEditorKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault()
        run()
    }
}

// ── Results state ─────────────────────────────────────────────────────────
const running    = ref(false)
const errMsg     = ref('')
const resultInfo = ref('')
const resultCols = ref<any[]>([])
const resultRows = ref<any[]>([])

watch(
    () => props.show,
    open => {
        if (!open) {
            errMsg.value     = ''
            resultInfo.value = ''
            resultCols.value = []
            resultRows.value = []
        }
    },
)

// ── Run ───────────────────────────────────────────────────────────────────
async function run() {
    const query = sqlText.value.trim()
    if (!query) return

    // Substitute $n placeholders with single-quoted SQL string literals.
    // Iterate in descending numeric order so $10 is replaced before $1.
    let finalSql = query
    const sels = localSels.value
    for (let n = sels.length; n >= 1; n--) {
        const escaped = sels[n - 1].replace(/'/g, "''")
        finalSql = finalSql.split(`$${n}`).join(`'${escaped}'`)
    }

    running.value = true
    errMsg.value  = ''

    try {
        const raw    = await invoke<string>('execute_query', {
            entriesJson: props.entriesJson,
            sql: finalSql,
        })
        const result: any = JSON.parse(raw)

        if (result.err_msg) {
            errMsg.value     = result.err_msg
            resultCols.value = []
            resultRows.value = []
            resultInfo.value = ''
        } else {
            const headers: string[] = result.headers  ?? []
            const body: any[]       = result.body     ?? []
            const rowCount: number  = result.row_count ?? body.length
            const colCount: number  = result.col_count ?? headers.length

            resultCols.value = headers.map(h => ({
                title:     h,
                key:       h,
                resizable: true,
                ellipsis:  { tooltip: true },
            }))
            resultRows.value = body
            resultInfo.value =
                `${rowCount} row${rowCount !== 1 ? 's' : ''} × ${colCount} col${colCount !== 1 ? 's' : ''}`
        }
    } catch (e: any) {
        errMsg.value = String(e)
    } finally {
        running.value = false
    }
}
</script>

<style scoped>
.section-label {
    font-size: 11px;
    opacity: 0.55;
    margin-bottom: 6px;
    letter-spacing: 0.02em;
}

.sql-label {
    margin-top: 14px;
}

/* Bindings list */
.bindings-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 4px;
}

.binding-row {
    display: flex;
    align-items: center;
    gap: 6px;
}

.binding-label {
    font-size: 12px;
    min-width: 28px;
    text-align: right;
    opacity: 0.75;
    flex-shrink: 0;
    font-family: monospace;
}

.binding-input {
    flex: 1;
}

.binding-remove {
    flex-shrink: 0;
}

.add-btn {
    align-self: flex-start;
    font-size: 12px;
}

/* Run bar */
.run-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 10px;
    margin-bottom: 6px;
}

.result-info {
    font-size: 12px;
    opacity: 0.6;
}

.err-tag {
    font-family: monospace;
    font-size: 11px;
    max-width: 100%;
    white-space: pre-wrap;
    word-break: break-word;
    height: auto;
}

/* Results */
.results-wrap {
    margin-top: 6px;
}
</style>
