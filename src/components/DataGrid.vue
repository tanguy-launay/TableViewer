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
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, h } from 'vue'
import { NDataTable } from 'naive-ui'

const props = defineProps<{
    columns: any[]
    data: any[]        // already sliced by parent
}>()

// ── Copy row on idx double-click ──────────────────────────────────────────
async function copyRow(row: Record<string, any>) {
    // Strip the internal idx column, copy everything else as formatted JSON
    const { idx: _idx, ...rest } = row
    const text = JSON.stringify(rest, null, 2)
    try {
        await navigator.clipboard.writeText(text)
    } catch {
        // Tauri webview fallback
        const ta = document.createElement('textarea')
        ta.value = text
        document.body.appendChild(ta)
        ta.select()
        document.execCommand('copy')
        document.body.removeChild(ta)
    }
}

// Wrap the idx column: keep any existing render (e.g. search highlight) but
// add a dblclick handler on the wrapping element.
const processedColumns = computed(() =>
    props.columns.map(col => {
        if (col.key !== 'idx') return col

        const base = col.render   // may already be a highlight render from useSearch

        return {
            ...col,
            render: (row: Record<string, any>) => {
                const inner = base
                    ? base(row)
                    : String(row['idx'] ?? '')

                return h('span', {
                    class: 'idx-cell',
                    title: 'Double-click to copy row',
                    onDblclick: () => copyRow(row),
                }, [inner])
            },
        }
    })
)

// ── ResizeObserver: fill the container ───────────────────────────────────
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

.idx-cell:hover {
    opacity: 0.7;
}
</style>
