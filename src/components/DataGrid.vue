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
        <pre class="cell-pre">{{ cellDisplay }}</pre>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, h } from 'vue'
import { NDataTable, NModal } from 'naive-ui'

const props = defineProps<{
    columns: any[]
    data: any[]        // already sliced by parent
}>()

// ── Cell detail modal ─────────────────────────────────────────────────────
const showCellModal = ref(false)
const cellLabel     = ref('')
const cellDisplay   = ref('')

function openCell(colTitle: string, raw: any) {
    cellLabel.value = colTitle
    // Try to pretty-print JSON (arrays, objects serialised by Polars)
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

// ── Column post-processing ────────────────────────────────────────────────
// - idx:       double-click → copy row (existing)
// - all other: single-click → open cell detail modal
const processedColumns = computed(() =>
    props.columns.map(col => {
        const base = col.render  // may be a search-highlight render from useSearch

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
}
</style>
