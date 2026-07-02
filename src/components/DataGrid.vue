<template>
    <div id="table_container" ref="containerEl">
        <n-data-table
            :max-height="bodyMaxHeight"
            :columns="processedColumns"
            :data="data"
            size="small"
            style="font-size: smaller; font-weight: 550;"
        />

        <!-- Hidden columns restore bar -->
        <div v-if="hiddenCols.size > 0" class="hidden-bar">
            <span class="hidden-bar-label">Hidden:</span>
            <span
                v-for="col in hiddenCols"
                :key="col"
                class="hidden-pill"
                @click="unhideCol(col)"
                :title="`Click to restore ${col}`"
            >{{ col }} ↩</span>
        </div>
    </div>

    <!-- Cell detail modal — opens on single-click of any non-idx cell -->
    <n-modal
        v-model:show="showCellModal"
        preset="card"
        :title="cellLabel"
        style="width: min(760px, 92vw); max-height: 80vh;"
        :segmented="{ content: true }"
    >
        <template #header-extra>
            <div class="cell-search-row">
                <n-input
                    v-model:value="modalSearch"
                    placeholder="Search in cell…"
                    size="small"
                    clearable
                    style="width: 220px"
                />
                <n-button
                    size="small"
                    :type="modalRegex && !modalFuzzy ? 'primary' : 'default'"
                    title="Regex"
                    @click="setMode('regex')"
                >.*</n-button>
                <n-button
                    size="small"
                    :type="modalFuzzy ? 'primary' : 'default'"
                    title="Fuzzy (all tokens must appear)"
                    @click="setMode('fuzzy')"
                >~</n-button>
                <n-button
                    size="small"
                    :type="modalCase ? 'primary' : 'default'"
                    title="Case sensitive"
                    @click="modalCase = !modalCase"
                >Aa</n-button>
                <span v-if="modalSearch" class="cell-match-count">
                    {{ matchCount }} match{{ matchCount !== 1 ? 'es' : '' }}
                </span>
            </div>
        </template>

        <!-- Right-click → dropdown menu  |  Ctrl+S → query editor directly -->
        <!-- eslint-disable-next-line vue/no-v-html -->
        <pre
            class="cell-pre"
            v-html="highlightedContent"
            @contextmenu.prevent="onContextMenu"
        />
    </n-modal>

    <!-- Context-menu dropdown (no preventDefault — avoids WebKit2GTK crash) -->
    <n-dropdown
        trigger="manual"
        :show="showCtxMenu"
        :options="CTX_OPTIONS"
        :x="ctxX"
        :y="ctxY"
        @clickoutside="showCtxMenu = false"
        @select="onCtxSelect"
    />
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, h } from 'vue'
import { NDataTable, NModal, NInput, NButton, NDropdown } from 'naive-ui'

const props = defineProps<{
    columns: any[]
    data: any[]
}>()

const emit = defineEmits<{
    (e: 'open-query-modal'): void
    (e: 'sort-col',   payload: { col: string; dir: 'asc' | 'desc' | null }): void
    (e: 'filter-col', col: string): void
}>()

// ── Cell detail modal ─────────────────────────────────────────────────────
const showCellModal = ref(false)
const cellLabel     = ref('')
const cellDisplay   = ref('')

// ── Search inside modal ───────────────────────────────────────────────────
const modalSearch = ref('')
const modalRegex  = ref(false)
const modalFuzzy  = ref(false)
const modalCase   = ref(false)
const matchCount  = ref(0)

// Mutual-exclusive mode helper: only one of regex / fuzzy active at a time
function setMode(mode: 'regex' | 'fuzzy') {
    if (mode === 'regex') { modalRegex.value = !modalRegex.value; if (modalRegex.value) modalFuzzy.value = false }
    if (mode === 'fuzzy') { modalFuzzy.value = !modalFuzzy.value; if (modalFuzzy.value) modalRegex.value = false }
}

const highlightedContent = computed(() => {
    const escaped = cellDisplay.value
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')

    if (!modalSearch.value.trim()) {
        matchCount.value = 0
        return escaped
    }

    // Fuzzy: every space-separated token must appear as a substring.
    // Each matching token is highlighted independently.
    if (modalFuzzy.value) {
        const tokens = modalSearch.value.trim().split(/\s+/).filter(Boolean)
        if (tokens.length === 0) { matchCount.value = 0; return escaped }
        const flags = 'g' + (modalCase.value ? '' : 'i')
        let result = escaped
        let total  = 0
        let allFound = true
        for (const tok of tokens) {
            const re = new RegExp(tok.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), flags)
            if (!re.test(result.replace(/<[^>]*>/g, ''))) { allFound = false; break }
            result = result.replace(re, m => { total++; return `<mark class="search-hit">${m}</mark>` })
        }
        matchCount.value = allFound ? total : 0
        return allFound ? result : escaped
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
    modalFuzzy.value  = false
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

// ── Right-click context menu ──────────────────────────────────────────────
const showCtxMenu = ref(false)
const ctxX        = ref(0)
const ctxY        = ref(0)

const CTX_OPTIONS = [
    { label: '⚡ Open Query Editor', key: 'query' },
]

function onContextMenu(e: MouseEvent) {
    ctxX.value = e.clientX
    ctxY.value = e.clientY
    showCtxMenu.value = true
}

function onCtxSelect(key: string) {
    showCtxMenu.value = false
    if (key === 'query') emit('open-query-modal')
}

// Ctrl+S while cell modal is open → query editor
function onKeyDown(e: KeyboardEvent) {
    if (showCellModal.value && (e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault()
        emit('open-query-modal')
    }
}

// ── Per-column sort / hide ────────────────────────────────────────────────
const sortState  = ref<{ col: string; dir: 'asc' | 'desc' } | null>(null)
const hiddenCols = ref<Set<string>>(new Set())

// Reset when the query result changes (new columns = new query)
watch(() => props.columns, () => {
    sortState.value  = null
    hiddenCols.value = new Set()
})

// ── Column sort / hide helpers ────────────────────────────────────────────
function cycleSort(col: string) {
    if (!sortState.value || sortState.value.col !== col) {
        sortState.value = { col, dir: 'asc' }
        emit('sort-col', { col, dir: 'asc' })
    } else if (sortState.value.dir === 'asc') {
        sortState.value = { col, dir: 'desc' }
        emit('sort-col', { col, dir: 'desc' })
    } else {
        sortState.value = null
        emit('sort-col', { col, dir: null })
    }
}

function hideCol(col: string) {
    const next = new Set(hiddenCols.value)
    next.add(col)
    hiddenCols.value = next
}

function unhideCol(col: string) {
    const next = new Set(hiddenCols.value)
    next.delete(col)
    hiddenCols.value = next
}

// ── Column post-processing ────────────────────────────────────────────────
const processedColumns = computed(() =>
    props.columns
        .filter(col => col.key === 'idx' || !hiddenCols.value.has(String(col.key)))
        .map(col => {
            const base = col.render

            if (col.key === 'idx') {
                return {
                    ...col,
                    render: (row: Record<string, any>) => {
                        const inner = base ? base(row) : String(row['idx'] ?? '')
                        return h('span', { class: 'idx-cell', title: 'Double-click to copy row', onDblclick: () => copyRow(row) }, [inner])
                    },
                }
            }

            const colKey = String(col.key)
            const s = sortState.value
            const isSorted = s?.col === colKey
            const sortIcon = isSorted ? (s!.dir === 'asc' ? '↑' : '↓') : '↕'

            return {
                ...col,
                // Custom header: name + sort + filter + hide icons
                title: () => h('div', { class: 'col-header' }, [
                    h('span', { class: 'col-name' }, String(col.title ?? colKey)),
                    h('div', { class: 'col-actions' }, [
                        h('span', {
                            class: ['col-act', 'col-act--sort', isSorted ? 'col-act--on' : ''],
                            title: `Sort by ${colKey}`,
                            onClick: (e: MouseEvent) => { e.stopPropagation(); cycleSort(colKey) },
                        }, sortIcon),
                        h('span', {
                            class: 'col-act col-act--filter',
                            title: `Filter by ${colKey}`,
                            onClick: (e: MouseEvent) => { e.stopPropagation(); emit('filter-col', colKey) },
                        }, '⊤'),
                        h('span', {
                            class: 'col-act col-act--hide',
                            title: `Hide ${colKey}`,
                            onClick: (e: MouseEvent) => { e.stopPropagation(); hideCol(colKey) },
                        }, '✕'),
                    ]),
                ]),
                render: (row: Record<string, any>) => {
                    const inner = base ? base(row) : String(row[colKey] ?? '')
                    return h('span', {
                        class: 'data-cell',
                        title: 'Click to view full content',
                        onClick: () => openCell(String(col.title ?? colKey), row[colKey]),
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
    document.addEventListener('keydown', onKeyDown)
    if (!containerEl.value) return
    ro = new ResizeObserver(([entry]) => {
        bodyMaxHeight.value = Math.max(entry.contentRect.height, 80)
    })
    ro.observe(containerEl.value)
})

onUnmounted(() => {
    document.removeEventListener('keydown', onKeyDown)
    ro?.disconnect()
})
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
    /* no margin-bottom — it's in the header */
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

/* ── Column header ────────────────────────────────────────────────────── */
.col-header {
    display:         flex;
    align-items:     center;
    justify-content: space-between;
    width:           100%;
    gap:             4px;
    min-width:       0;
}

.col-name {
    flex:           1;
    overflow:       hidden;
    text-overflow:  ellipsis;
    white-space:    nowrap;
}

.col-actions {
    display:     flex;
    align-items: center;
    gap:         1px;
    flex-shrink: 0;
}

.col-act {
    cursor:        pointer;
    font-size:     11px;
    padding:       1px 3px;
    border-radius: 3px;
    user-select:   none;
    line-height:   1;
    transition:    background 0.1s, opacity 0.1s;
}
.col-act:hover { background: rgba(128,128,128,0.25); }

/* Sort: always dim, bright when active */
.col-act--sort  { opacity: 0.25; }
.col-act--on    { opacity: 1; color: #18a058; }
:deep(.n-data-table-th:hover) .col-act--sort { opacity: 0.7; }

/* Filter + hide: hidden until th hover */
.col-act--filter,
.col-act--hide  { opacity: 0; }
:deep(.n-data-table-th:hover) .col-act--filter,
:deep(.n-data-table-th:hover) .col-act--hide  { opacity: 0.55; }
.col-act--filter:hover { opacity: 1 !important; }
.col-act--hide:hover   { opacity: 1 !important; color: #e06060; }

/* ── Hidden columns restore bar ─────────────────────────────────────── */
.hidden-bar {
    display:     flex;
    align-items: center;
    flex-wrap:   wrap;
    gap:         6px;
    padding:     4px 8px;
    font-size:   11px;
    opacity:     0.75;
    border-top:  1px solid var(--sidebar-border, #2e2e3a);
}

.hidden-bar-label {
    font-weight:    600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size:      10px;
}

.hidden-pill {
    padding:       2px 7px;
    border-radius: 10px;
    background:    rgba(128,128,128,0.15);
    cursor:        pointer;
    white-space:   nowrap;
    transition:    background 0.1s;
}
.hidden-pill:hover { background: rgba(128,128,128,0.3); }
</style>
