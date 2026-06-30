import { ref, watch } from 'vue'
import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open, message } from '@tauri-apps/plugin-dialog'
import type { FileEntry, ColumnInfo } from '../types'

export const FILETYPE_OPTIONS = [
    { label: 'parquet(s)', key: 'parquet' },
    { label: 'feather(s)', key: 'arrow'   },
    { label: 'csv(s)',     key: 'csv'      },
    { label: 'json',       key: 'json'     },
    { label: 'jsonl',      key: 'jsonl'    },
]

export const CSV_SEP_OPTIONS = [
    { label: 'comma (,)',     value: ','  },
    { label: 'tab (\\t)',     value: '\t' },
    { label: 'semicolon (;)', value: ';'  },
]

const Extensions: Record<string, string[]> = {
    parquet: ['parquet'],
    arrow:   ['arrow', 'ipc', 'feather'],
    csv:     ['csv'],
    json:    ['json'],
    jsonl:   ['jsonl', 'ndjson'],
}

// sql ref is passed in so addEntry can seed a default query on first file
export function useFiles(sql: Ref<string>) {
    const fileEntries     = ref<FileEntry[]>([])
    const schemaMap       = ref<Record<string, ColumnInfo[]>>({})
    const expandedSchemas = ref<Record<string, boolean>>({})
    const showCsvModal    = ref(false)
    const pendingSep      = ref(',')
    const pendingFiles    = ref<string[]>([])

    function buildEntriesJson(): string {
        return JSON.stringify(fileEntries.value.map(e => ({
            alias:     e.alias,
            filenames: e.filenames,
            filetype:  e.filetype,
            sep:       e.filetype === 'csv' ? e.sep.charCodeAt(0) : null,
        })))
    }

    function makeAlias(filenames: string[]): string {
        const base = (filenames[0] ?? 'table')
            .split('/').pop()!
            .replace(/\.[^.]+$/, '')
            .replace(/[^a-zA-Z0-9]/g, '_')
        const safe = /^[0-9]/.test(base) ? `t_${base}` : base || 'table'
        const taken = fileEntries.value.map(e => e.alias)
        if (!taken.includes(safe)) return safe
        let i = 1
        while (taken.includes(`${safe}_${i}`)) i++
        return `${safe}_${i}`
    }

    function addEntry(filenames: string[], filetype: FileEntry['filetype'], sep: string) {
        const alias = makeAlias(filenames)
        fileEntries.value.push({ id: crypto.randomUUID(), alias, filenames, filetype, sep })
        if (!sql.value) sql.value = `SELECT * FROM ${alias} LIMIT 100`
    }

    function removeEntry(id: string) {
        fileEntries.value = fileEntries.value.filter(e => e.id !== id)
    }

    function updateAlias(id: string, alias: string) {
        const entry = fileEntries.value.find(e => e.id === id)
        if (entry) entry.alias = alias
    }

    function toggleSchema(id: string) {
        expandedSchemas.value[id] = !expandedSchemas.value[id]
    }

    // Reload schema whenever the set of open files changes
    watch(
        () => fileEntries.value.map(e => e.id).join(','),
        async () => {
            if (fileEntries.value.length === 0) { schemaMap.value = {}; return }
            const res = JSON.parse(await invoke<string>('get_schema', {
                entriesJson: buildEntriesJson(),
            }))
            if (!res.ok) return
            const map: Record<string, ColumnInfo[]> = {}
            res.tables.forEach((t: { columns: ColumnInfo[] }, i: number) => {
                map[fileEntries.value[i].id] = t.columns
            })
            schemaMap.value = map
        }
    )

    async function openFile(key: string) {
        const selected = await open({
            multiple: true,
            filters: [
                { name: 'Table File(s)', extensions: Extensions[key] },
                { name: 'All Files',     extensions: ['*']            },
            ],
        })
        if (!Array.isArray(selected) || selected.length === 0) return
        if (key === 'csv') {
            pendingFiles.value = selected
            pendingSep.value   = ','
            showCsvModal.value = true
        } else if (key === 'json' || key === 'jsonl') {
            const res = JSON.parse(await invoke<string>('check_json_import', {
                paths: selected, filetype: key,
            }))
            if ('err_msg' in res) {
                await message(res.err_msg, { title: 'Import error', kind: 'error' })
                return
            }
            if (res.warning) await message(res.warning, { title: 'Import warning' })
            addEntry(selected, key as FileEntry['filetype'], ',')
        } else {
            addEntry(selected, key as FileEntry['filetype'], ',')
        }
    }

    function confirmCsvOpen() {
        addEntry(pendingFiles.value, 'csv', pendingSep.value)
    }

    return {
        fileEntries, schemaMap, expandedSchemas,
        showCsvModal, pendingSep,
        buildEntriesJson,
        openFile, confirmCsvOpen,
        removeEntry, updateAlias, toggleSchema,
    }
}
