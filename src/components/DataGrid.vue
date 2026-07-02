<template>
    <div id="table_container" ref="containerEl">
        <n-data-table
            :max-height="bodyMaxHeight"
            :columns="processedColumns"
            :data="data"
            size="small"
            style="font-size: smaller; font-weight: 550;"
        />
    </div>

    <!-- Cell detail modal — opens on single-click of any non-idx cell -->
    <n-modal
        v-model:show="showCellModal"
        preset="card"
        :title="cellLabel"
        style="width: min(760px, 92vw); max-height: 80vh;"
        :segmented="{ content: true }"
    >
        <!-- Search bar + Query-selection button -->
        <div class="cell-search-row">
            <n-input
                v-model:value="modalSearch"
                placeholder="Search in cell…"
                size="small"
                clearable
                style="flex: 1"
            />
            <n-button
                size="small"
                :type="modalRegex ? 'primary' : 'default'"
                title="Regex"
                @click="modalRegex = !modalRegex"
            >.*</n-button>
            <n-button
                size="small"
                :type="modalCase ? 'primary' : 'default'"
                title="Case sensitive"
                @click="modalCase = !modalCase"
            >Aa</n-button>
            <span v-if="modalSearch" class="cell-match-count">
                {{ matchCount }} match{{ matchCount !== 1 ? 'es' : '' }}
            </span>
            <!-- Appears once the user has selected text in the pre below -->
            <n-button
                v-if="lastSel"
                size="small"
                type="info"
                title="Query selected text"
                @click="querySelection"
            >⚡ Query</n-button>
        </div>

        <!-- eslint-disable-next-line vue/no-v-html -->
        <pre
            class="cell-pre"
            v-html="highlightedContent"
            @mouseup="onPreMouseUp"
        />
    </n-modal>

    <!-- Selection query modal -->
    <SelectionQueryModal
        v-model:show="showSelQuery"
        v-model:selections="selectionList"
        :entries-json="entriesJson"
    />
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, h } from 'vue'
import { NDataTable, NModal, NInput, NButton } from 'naive-ui'
import SelectionQueryModal from './SelectionQueryModal.vue'

const props = defineProps<{
    columns: any[]
    data: any[]
    entriesJson: string
}>()

// ── Cell detail modal ─────────────────────────────────────────────────────
const showCellModal = ref(false)
const cellLabel     = ref('')
const cellDisplay   = ref('')

// ── Search inside modal ───────────────────────────────────────────────────
const modalSearch = ref('')
const modalRegex  = ref(false)
const modalCase   = ref(false)
const matchCount  = ref(0)

const highlightedContent = computed(() => {
    const escaped = cellDisplay.value
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')

    if (!modalSearch.value.trim()) {
        matchCount.value = 0
        return escaped
    }
    try {
        const flags = 'g' + (modalCase.value ? '' : 'i')
        const re = modalRegex.value
            ? new RegExp(modalSearch.value, flags)
            : new RegExp(modalSearch.value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), flags)
        let n = 0
        const result = escaped.replace(re, m => { n++; return `<mark class="search-hit">${m}</mark>` })
        matchCount.value = n
        return result
    } catch {
        matchCount.value = 0
        return escaped
    }
})

function openCell(colTitle: string, raw: any) {
    modalSearch.value = ''
    lastSel.value     = ''
    cellLabel.value   = colTitle
    if (typeof raw === 'string') {
        const trimmed = raw.trim()
        if ((trimmed.startsWith('[') || trimmed.startsWith('{')) && (trimmed.endsWith(']') || trimmed.endsWith('}'))) {
            try {
                cellDisplay.value = JSON.stringify(JSON.parse(trimmed), null, 2)
                showCellModal.value = true
                return
            } catch { /* not valid JSON, fall through */ }
        }
    }
    cellDisplay.value = String(raw ?? '')
    showCellModal.value = true
}

// ── Copy row on idx double-click ──────────────────────────────────────────
async function copyRow(row: Record<string, any>) {
    const { idx: _idx, ...rest } = row
    const text = JSON.stringify(rest, null, 2)
    try {
        await navigator.clipboard.writeText(text)
    } catch {
        const ta = document.createElement('textarea')
        ta.value = text
        document.body.appendChild(ta)
        ta.select()
        document.execCommand('copy')
        document.body.removeChild(ta)
    }
}

// ── Selection → query builder ─────────────────────────────────────────────
// Save selection on mouseup (selection is cleared when the button is clicked,
// so we capture it early and store it in lastSel).
const lastSel       = ref('')
const showSelQuery  = ref(false)
const selectionList = ref<string[]>([])

function onPreMouseUp() {
    const sel = window.getSelection()?.toString().trim() ?? ''
    if (sel) lastSel.value = sel
}

function querySelection() {
    if (!lastSel.value) return
    if (showSelQuery.value) {
        // modal already open → append as next $n
        selectionList.value = [...selectionList.value, lastSel.value]
    } else {
        selectionList.value = [lastSel.value]
        showSelQuery.value  = true
    }
    lastSel.value = ''
}

// ── Column post-processing ────────────────────────────────────────────────
const processedColumns = computed(() =>
    props.columns.map(col => {
        const base = col.render

        if (col.key === 'idx') {
            return {
                ...col,
                render: (row: Record<string, any>) => {
                    const inner = base ? base(row) : String(row['idx'] ?? '')
                    return h('span', {
                        class: 'idx-cell',
                        title: 'Double-click to copy row',
                        onDblclick: () => copyRow(row),
                    }, [inner])
                },
            }
        }

        return {
            ...col,
            render: (row: Record<string, any>) => {
                const inner = base ? base(row) : String(row[col.key] ?? '')
                return h('span', {
                    class: 'data-cell',
                    title: 'Click to view full content',
                    onClick: () => openCell(String(col.title ?? col.key), row[col.key]),
                }, [inner])
            },
        }
    })
)

// ── ResizeObserver ────────────────────────────────────────────────────────
const containerEl   = ref<HTMLElement | null>(null)
const bodyMaxHeight = ref(400)
let ro: ResizeObserver | null = null

onMounted(() => {
    if (!containerEl.value) return
    ro = new ResizeObserver(([entry]) => {
        bodyMaxHeight.value = Math.max(entry.contentRect.height, 80)
    })
    ro.observe(containerEl.value)
})

onUnmounted(() => ro?.disconnect())
</script>

<style scoped>
#table_container {
    flex: 1;
    min-height: 0;
    margin: 0 5px 5px;
    overflow: hidden;
}

.idx-cell {
    display: block;
    width: 100%;
    cursor: copy;
    user-select: none;
}
.idx-cell:hover { opacity: 0.7; }

.data-cell {
    display: block;
    width: 100%;
    cursor: pointer;
}
.data-cell:hover { opacity: 0.75; }

.cell-pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: monospace;
    font-size: 13px;
    line-height: 1.6;
    max-height: 60vh;
    overflow-y: auto;
    cursor: text;
}

/* ── Modal search bar ──────────────────────────────────────────────────── */
.cell-search-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
}

.cell-match-count {
    font-size: 11px;
    opacity: 0.6;
    white-space: nowrap;
}

/* Search hit highlight (inside v-html pre) */
:deep(mark.search-hit) {
    background: #ffe066;
    color: #000;
    border-radius: 2px;
    padding: 0 1px;
}
</style>
